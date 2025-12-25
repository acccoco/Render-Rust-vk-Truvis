use crate::scene_data::SceneRenderData;
use crate::scene_manager::SceneManager;
use ash::vk;
use glam::Vec4Swizzles;
use itertools::Itertools;
use slotmap::Key;
use std::path::PathBuf;
use truvis_asset::asset_hub::AssetHub;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::{
    commands::{
        barrier::{GfxBarrierMask, GfxBufferBarrier},
        command_buffer::GfxCommandBuffer,
    },
    raytracing::acceleration::GfxAcceleration,
    resources::special_buffers::structured_buffer::GfxStructuredBuffer,
};
use truvis_model::components::instance::Instance;
use truvis_render_base::bindless_manager::{BindlessManager, BindlessSrvHandle};
use truvis_render_base::frame_counter::FrameCounter;
use truvis_render_base::pipeline_settings::FrameLabel;
use truvis_resource::gfx_resource_manager::GfxResourceManager;
use truvis_resource::handles::GfxTextureHandle;
use truvis_resource::texture::{GfxTexture, ImageLoader};
use truvis_shader_binding::truvisl;

/// 构建 Gpu Scene 所需的所有 buffer
struct GpuSceneBuffers {
    scene_buffer: GfxStructuredBuffer<truvisl::GPUScene>,
    light_buffer: GfxStructuredBuffer<truvisl::PointLight>,
    light_stage_buffer: GfxStructuredBuffer<truvisl::PointLight>,
    material_buffer: GfxStructuredBuffer<truvisl::PBRMaterial>,
    material_stage_buffer: GfxStructuredBuffer<truvisl::PBRMaterial>,
    geometry_buffer: GfxStructuredBuffer<truvisl::Geometry>,
    geometry_stage_buffer: GfxStructuredBuffer<truvisl::Geometry>,
    instance_buffer: GfxStructuredBuffer<truvisl::Instance>,
    instance_stage_buffer: GfxStructuredBuffer<truvisl::Instance>,
    material_indirect_buffer: GfxStructuredBuffer<u32>,
    material_indirect_stage_buffer: GfxStructuredBuffer<u32>,
    geometry_indirect_buffer: GfxStructuredBuffer<u32>,
    geometry_indirect_stage_buffer: GfxStructuredBuffer<u32>,

    // TODO 使用 frame id 来标记是否过期，scene manager 里面也需要有相应的标记
    tlas: Option<GfxAcceleration>,
}
// init & destroy
impl GpuSceneBuffers {
    fn new(frame_label: FrameLabel) -> Self {
        let max_light_cnt = 512;
        let max_material_cnt = 1024;
        let max_geometry_cnt = 1024 * 8;
        let max_instance_cnt = 1024;

        GpuSceneBuffers {
            scene_buffer: GfxStructuredBuffer::new_ubo(1, format!("scene buffer-{}", frame_label)),
            light_buffer: GfxStructuredBuffer::new_ssbo(max_light_cnt, format!("light buffer-{}", frame_label)),
            light_stage_buffer: GfxStructuredBuffer::new_stage_buffer(
                max_light_cnt,
                format!("light stage buffer-{}", frame_label),
            ),
            material_buffer: GfxStructuredBuffer::new_ssbo(
                max_material_cnt,
                format!("material buffer-{}", frame_label),
            ),
            material_stage_buffer: GfxStructuredBuffer::new_stage_buffer(
                max_material_cnt,
                format!("material stage buffer-{}", frame_label),
            ),
            geometry_buffer: GfxStructuredBuffer::new_ssbo(
                max_geometry_cnt,
                format!("geometry buffer-{}", frame_label),
            ),
            geometry_stage_buffer: GfxStructuredBuffer::new_stage_buffer(
                max_geometry_cnt,
                format!("geometry stage buffer-{}", frame_label),
            ),
            instance_buffer: GfxStructuredBuffer::new_ssbo(
                max_instance_cnt,
                format!("instance buffer-{}", frame_label),
            ),
            instance_stage_buffer: GfxStructuredBuffer::new_stage_buffer(
                max_instance_cnt,
                format!("instance stage buffer-{}", frame_label),
            ),
            material_indirect_buffer: GfxStructuredBuffer::new_ssbo(
                max_instance_cnt * 8,
                format!("instance material buffer-{}", frame_label),
            ),
            material_indirect_stage_buffer: GfxStructuredBuffer::new_stage_buffer(
                max_instance_cnt * 8,
                format!("instance material stage buffer-{}", frame_label),
            ),
            geometry_indirect_buffer: GfxStructuredBuffer::new_ssbo(
                max_instance_cnt * 8,
                format!("instance geometry buffer-{}", frame_label),
            ),
            geometry_indirect_stage_buffer: GfxStructuredBuffer::new_stage_buffer(
                max_instance_cnt * 8,
                format!("instance geometry stage buffer-{}", frame_label),
            ),
            tlas: None,
        }
    }
}

/// 用于构建传输到 GPU 的场景数据
pub struct GpuScene {
    scene_render_data: SceneRenderData,

    /// mesh 在 geometry buffer 中的 start idx
    // mesh_geometry_map: SecondaryMap<MeshHandle, usize>,
    all_mesh_startup_index: Vec<usize>,

    gpu_scene_buffers: [GpuSceneBuffers; FrameCounter::fif_count()],

    sky_texture_handle: GfxTextureHandle,
    uv_checker_texture_handle: GfxTextureHandle,
}
// getter
impl GpuScene {
    #[inline]
    pub fn tlas(&self, frame_label: FrameLabel) -> Option<&GfxAcceleration> {
        self.gpu_scene_buffers[*frame_label].tlas.as_ref()
    }

    #[inline]
    pub fn scene_buffer(&self, frame_label: FrameLabel) -> &GfxStructuredBuffer<truvisl::GPUScene> {
        &self.gpu_scene_buffers[*frame_label].scene_buffer
    }
}
// new & init
impl GpuScene {
    pub fn new(gfx_resource_manager: &mut GfxResourceManager, bindless_manager: &mut BindlessManager) -> Self {
        let sky_path = TruvisPath::resources_path("sky.jpg");
        let uv_checker_path = TruvisPath::resources_path("uv_checker.png");
        let sky_image = ImageLoader::load_image(&PathBuf::from(&sky_path));
        let uv_checker_image = ImageLoader::load_image(&PathBuf::from(&uv_checker_path));

        let sky_texture = GfxTexture::new(sky_image, &sky_path);
        let uv_checker_texture = GfxTexture::new(uv_checker_image, &uv_checker_path);

        let sky_texture_handle = gfx_resource_manager.register_texture(sky_texture);
        let uv_checker_texture_handle = gfx_resource_manager.register_texture(uv_checker_texture);

        bindless_manager.register_srv_with_texture(sky_texture_handle);
        bindless_manager.register_srv_with_texture(uv_checker_texture_handle);

        Self {
            scene_render_data: Default::default(),
            all_mesh_startup_index: Vec::default(),

            gpu_scene_buffers: FrameCounter::frame_labes().map(GpuSceneBuffers::new),

            sky_texture_handle,
            uv_checker_texture_handle,
        }
    }
}
impl Drop for GpuScene {
    fn drop(&mut self) {}
}
// destroy
impl GpuScene {
    pub fn destroy(self) {}
    pub fn destroy_mut(&mut self) {}
}
// tools
impl GpuScene {
    /// # Phase: Before Render
    ///
    /// 将已经准备好的 GPU 格式的场景数据写入 Device Buffer 中
    pub fn prepare_render_data(
        &mut self,
        cmd: &GfxCommandBuffer,
        barrier_mask: GfxBarrierMask,
        frame_counter: &FrameCounter,
        scene_render_data: SceneRenderData,
        scene_manager: &SceneManager,
        bindless_manager: &BindlessManager,
        asset_hub: &AssetHub,
    ) {
        let _span = tracy_client::span!("GpuScene::upload_to_buffer");
        self.scene_render_data = scene_render_data;

        // 记录每个 mesh 在 geometry buffer 中的 startup index
        {
            self.all_mesh_startup_index.reserve(self.scene_render_data.all_meshes.len());

            let mut mesh_startup_idx = 0;
            for mesh_handle in &self.scene_render_data.all_meshes {
                let mesh = scene_manager.get_mesh(*mesh_handle).unwrap();
                self.all_mesh_startup_index.push(mesh_startup_idx);
                mesh_startup_idx += mesh.geometries.len();
            }
        }

        self.upload_mesh_buffer(cmd, barrier_mask, scene_manager, frame_counter);
        self.upload_instance_buffer(cmd, barrier_mask, scene_manager, frame_counter);
        self.upload_material_buffer(cmd, barrier_mask, scene_manager, bindless_manager, asset_hub, frame_counter);
        self.upload_light_buffer(cmd, barrier_mask, scene_manager, frame_counter);

        // 需要确保 instance 先与 tlas 构建
        self.build_tlas(scene_manager, frame_counter);

        self.upload_scene_buffer(cmd, frame_counter, barrier_mask, scene_manager, bindless_manager);
    }

    /// 绘制场景中的所有实例
    ///
    /// before_draw(instance_idx, submesh_idx)
    pub fn draw(&self, cmd: &GfxCommandBuffer, scene_manager: &SceneManager, mut before_draw: impl FnMut(u32, u32)) {
        let _span = tracy_client::span!("GpuScene::draw");
        for (instance_idx, instance_handle) in self.scene_render_data.all_instances.iter().enumerate() {
            let instance = scene_manager.get_instance(*instance_handle).unwrap();
            let mesh = scene_manager.get_mesh(instance.mesh).unwrap();
            for (submesh_idx, geometry) in mesh.geometries.iter().enumerate() {
                geometry.cmd_bind_index_buffer(cmd);
                geometry.cmd_bind_vertex_buffers(cmd);

                before_draw(instance_idx as u32, submesh_idx as u32);
                cmd.draw_indexed(geometry.index_cnt(), 0, 1, 0, 0);
            }
        }
    }

    /// 将整个场景的数据上传到 scene buffer 中去
    fn upload_scene_buffer(
        &mut self,
        cmd: &GfxCommandBuffer,
        frame_counter: &FrameCounter,
        barrier_mask: GfxBarrierMask,
        scene_manager: &SceneManager,
        bindless_manager: &BindlessManager,
    ) {
        let crt_gpu_buffers = &self.gpu_scene_buffers[*frame_counter.frame_label()];
        let scene_data = truvisl::GPUScene {
            all_instances: crt_gpu_buffers.instance_buffer.device_address(),
            all_mats: crt_gpu_buffers.material_buffer.device_address(),
            all_geometries: crt_gpu_buffers.geometry_buffer.device_address(),
            instance_material_map: crt_gpu_buffers.material_indirect_buffer.device_address(),
            instance_geometry_map: crt_gpu_buffers.geometry_indirect_buffer.device_address(),
            point_lights: crt_gpu_buffers.light_buffer.device_address(),
            spot_lights: 0, // TODO 暂时无用
            point_light_count: scene_manager.point_light_map().len() as u32,
            spot_light_count: 0, // TODO 暂时无用

            sky: bindless_manager.get_shader_srv_handle_with_texture(self.sky_texture_handle).0,
            sky_sampler_type: truvisl::ESamplerType_LinearClamp,
            uv_checker: bindless_manager.get_shader_srv_handle_with_texture(self.uv_checker_texture_handle).0,
            uv_checker_sampler_type: truvisl::ESamplerType_LinearClamp,
        };

        cmd.cmd_update_buffer(crt_gpu_buffers.scene_buffer.vk_buffer(), 0, bytemuck::bytes_of(&scene_data));
        cmd.buffer_memory_barrier(
            vk::DependencyFlags::empty(),
            &[GfxBufferBarrier::default().mask(barrier_mask).buffer(
                crt_gpu_buffers.scene_buffer.vk_buffer(),
                0,
                vk::WHOLE_SIZE,
            )],
        );
    }

    /// 将数据转换为 shader 中的实例数据
    ///
    /// 其中 buffer 可以是 stage buffer 的内存映射
    fn upload_instance_buffer(
        &mut self,
        cmd: &GfxCommandBuffer,
        barrier_mask: GfxBarrierMask,
        scene_manager: &SceneManager,
        frame_counter: &FrameCounter,
    ) {
        let _span = tracy_client::span!("upload_instance_buffer");
        let crt_gpu_buffers = &mut self.gpu_scene_buffers[*frame_counter.frame_label()];

        let crt_instance_stage_buffer = &mut crt_gpu_buffers.instance_stage_buffer;
        let crt_geometry_indirect_stage_buffer = &mut crt_gpu_buffers.geometry_indirect_stage_buffer;
        let crt_material_indirect_stage_buffer = &mut crt_gpu_buffers.material_indirect_stage_buffer;

        let instance_buffer_slices = crt_instance_stage_buffer.mapped_slice();
        let material_indirect_buffer_slices = crt_material_indirect_stage_buffer.mapped_slice();
        let geometry_indirect_buffer_slices = crt_geometry_indirect_stage_buffer.mapped_slice();

        if instance_buffer_slices.len() < self.scene_render_data.all_instances.len() {
            panic!("instance cnt can not be larger than buffer");
        }

        let mut crt_geometry_indirect_idx = 0;
        let mut crt_material_indirect_idx = 0;
        for (instance_idx, instance_handle) in self.scene_render_data.all_instances.iter().enumerate() {
            let instance = scene_manager.get_instance(*instance_handle).unwrap();
            let submesh_cnt = instance.materials.len();
            if geometry_indirect_buffer_slices.len() < crt_geometry_indirect_idx + submesh_cnt {
                panic!("instance geometry cnt can not be larger than buffer");
            }
            if material_indirect_buffer_slices.len() < crt_material_indirect_idx + submesh_cnt {
                panic!("instance material cnt can not be larger than buffer");
            }

            instance_buffer_slices[instance_idx] = truvisl::Instance {
                geometry_indirect_idx: crt_geometry_indirect_idx as u32,
                geometry_count: submesh_cnt as u32,
                material_indirect_idx: crt_material_indirect_idx as u32,
                material_count: submesh_cnt as u32,
                model: instance.transform.into(),
                inv_model: instance.transform.inverse().into(),
            };

            // TODO 对于 mesh 来说，可能不需要间接的索引 buffer，因为 mesh 在 geometry
            //  buffer 中是连续的 首先将 instance 需要的 geometry
            //  的实际索引，写入一个间接索引 buffer: geometry_indirect_buffer，
            //  然后获得 instance 数据在间接索引 buffer 中的起始 idx 和长度，将这个值写入到
            //  truvisl::Instance 中
            let mesh_startup_index =
                self.all_mesh_startup_index[self.scene_render_data.all_meshes.get_index_of(&instance.mesh).unwrap()];
            for submesh_idx in 0..instance.materials.len() {
                let geometry_idx = mesh_startup_index + submesh_idx;
                geometry_indirect_buffer_slices[crt_geometry_indirect_idx + submesh_idx] = geometry_idx as u32;
            }
            crt_geometry_indirect_idx += submesh_cnt;

            for material_handle in instance.materials.iter() {
                let material_idx = self.scene_render_data.all_materials.get_index_of(material_handle).unwrap();
                material_indirect_buffer_slices[crt_material_indirect_idx] = material_idx as u32;
                crt_material_indirect_idx += 1;
            }
        }

        helper::flush_copy_and_barrier(
            cmd,
            crt_instance_stage_buffer,
            &mut crt_gpu_buffers.instance_buffer,
            barrier_mask,
        );
        helper::flush_copy_and_barrier(
            cmd,
            crt_geometry_indirect_stage_buffer,
            &mut crt_gpu_buffers.geometry_indirect_buffer,
            barrier_mask,
        );
        helper::flush_copy_and_barrier(
            cmd,
            crt_material_indirect_stage_buffer,
            &mut crt_gpu_buffers.material_indirect_buffer,
            barrier_mask,
        );
    }

    /// 将 material 数据上传到 material buffer 中
    fn upload_material_buffer(
        &mut self,
        cmd: &GfxCommandBuffer,
        barrier_mask: GfxBarrierMask,
        scene_manager: &SceneManager,
        bindless_manager: &BindlessManager,
        asset_hub: &AssetHub,
        frame_counter: &FrameCounter,
    ) {
        let _span = tracy_client::span!("upload_material_buffer");
        let crt_gpu_buffers = &mut self.gpu_scene_buffers[*frame_counter.frame_label()];
        let crt_material_stage_buffer = &mut crt_gpu_buffers.material_stage_buffer;
        let material_buffer_slices = crt_material_stage_buffer.mapped_slice();
        if material_buffer_slices.len() < self.scene_render_data.all_materials.len() {
            panic!("material cnt can not be larger than buffer");
        }

        for (mat_idx, mat_handle) in self.scene_render_data.all_materials.iter().enumerate() {
            let mat = scene_manager.mat_map().get(*mat_handle).unwrap();

            let mut diffuse_bindless_handle = BindlessSrvHandle::null();
            if !mat.diffuse_map.is_empty() {
                let diffuse_texture_handle = asset_hub.get_texture_by_path(std::path::Path::new(&mat.diffuse_map));
                diffuse_bindless_handle = bindless_manager.get_shader_srv_handle_with_texture(diffuse_texture_handle);
            }

            // 暂不支持法线贴图
            let normal_bindless_handle = BindlessSrvHandle::null();

            material_buffer_slices[mat_idx] = truvisl::PBRMaterial {
                base_color: mat.base_color.xyz().into(),
                emissive: mat.emissive.xyz().into(),
                metallic: mat.metallic,
                roughness: mat.roughness,
                diffuse_map: diffuse_bindless_handle.0,
                diffuse_map_sampler_type: truvisl::ESamplerType_LinearRepeat,
                normal_map: normal_bindless_handle.0,
                normal_map_sampler_type: truvisl::ESamplerType_LinearRepeat,
                opaque: mat.opaque,
                _padding_1: Default::default(),
                _padding_2: Default::default(),
                _padding_3: Default::default(),
            };
        }

        helper::flush_copy_and_barrier(
            cmd,
            crt_material_stage_buffer,
            &mut crt_gpu_buffers.material_buffer,
            barrier_mask,
        );
    }

    fn upload_light_buffer(
        &mut self,
        cmd: &GfxCommandBuffer,
        barrier_mask: GfxBarrierMask,
        scene_manager: &SceneManager,
        frame_counter: &FrameCounter,
    ) {
        let _span = tracy_client::span!("upload_light_buffer");
        let crt_gpu_buffers = &mut self.gpu_scene_buffers[*frame_counter.frame_label()];
        let crt_light_stage_buffer = &mut crt_gpu_buffers.light_stage_buffer;
        let light_buffer_slices = crt_light_stage_buffer.mapped_slice();
        if light_buffer_slices.len() < scene_manager.point_light_map().len() {
            panic!("light cnt can not be larger than buffer");
        }

        for (light_idx, (_, point_light)) in scene_manager.point_light_map().iter().enumerate() {
            light_buffer_slices[light_idx] = truvisl::PointLight {
                pos: point_light.pos,
                color: point_light.color,

                _color_padding: Default::default(),
                _pos_padding: Default::default(),
            };
        }

        helper::flush_copy_and_barrier(cmd, crt_light_stage_buffer, &mut crt_gpu_buffers.light_buffer, barrier_mask);
    }

    /// 将 mesh 数据以 geometry 的形式上传到 GPU
    fn upload_mesh_buffer(
        &mut self,
        cmd: &GfxCommandBuffer,
        barrier_mask: GfxBarrierMask,
        scene_manager: &SceneManager,
        frame_counter: &FrameCounter,
    ) {
        let _span = tracy_client::span!("upload_mesh_buffer");
        let crt_gpu_buffers = &mut self.gpu_scene_buffers[*frame_counter.frame_label()];
        let crt_geometry_stage_buffer = &mut crt_gpu_buffers.geometry_stage_buffer;
        let geometry_buffer_slices = crt_geometry_stage_buffer.mapped_slice();

        let mut crt_geometry_idx = 0;
        for mesh_handle in self.scene_render_data.all_meshes.iter() {
            let mesh = scene_manager.mesh_map().get(*mesh_handle).unwrap();
            if geometry_buffer_slices.len() < crt_geometry_idx + mesh.geometries.len() {
                panic!("geometry cnt can not be larger than buffer");
            }
            for (submesh_idx, geometry) in mesh.geometries.iter().enumerate() {
                geometry_buffer_slices[crt_geometry_idx + submesh_idx] = truvisl::Geometry {
                    position_buffer: geometry.vertex_buffer.pos_address(),
                    normal_buffer: geometry.vertex_buffer.normal_address(),
                    tangent_buffer: geometry.vertex_buffer.tangent_address(),
                    uv_buffer: geometry.vertex_buffer.uv_address(),
                    index_buffer: geometry.index_buffer.device_address(),
                };
            }
            crt_geometry_idx += mesh.geometries.len();
        }

        helper::flush_copy_and_barrier(
            cmd,
            crt_geometry_stage_buffer,
            &mut crt_gpu_buffers.geometry_buffer,
            barrier_mask,
        );
    }
}
// ray tracing
impl GpuScene {
    /// 根据 instance 信息获得加速结构的 instance 信息
    fn get_as_instance_info(
        &self,
        instance: &Instance,
        custom_idx: u32,
        scene_manager: &SceneManager,
    ) -> vk::AccelerationStructureInstanceKHR {
        let mesh = scene_manager.get_mesh(instance.mesh).expect("Mesh not found");
        vk::AccelerationStructureInstanceKHR {
            // 3x4 row-major matrix
            transform: helper::get_rt_matrix(&instance.transform),
            instance_custom_index_and_mask: vk::Packed24_8::new(custom_idx, 0xFF),
            instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                0, // TODO 暂时使用同一个 hit group
                vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as u8,
            ),
            acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                device_handle: mesh.blas_device_address.unwrap(),
            },
        }
    }

    fn build_tlas(&mut self, scene_manager: &SceneManager, frame_counter: &FrameCounter) {
        let _span = tracy_client::span!("build_tlas");
        if self.scene_render_data.all_instances.is_empty() {
            // 没有实例数据，直接返回
            return;
        }

        if self.gpu_scene_buffers[*frame_counter.frame_label()].tlas.is_some() {
            // 已经构建过 tlas，直接返回
            return;
        }

        let instance_infos = self
            .scene_render_data
            .all_instances
            .iter()
            .map(|ins_handle| (ins_handle, scene_manager.get_instance(*ins_handle).unwrap()))
            // BUG custom idx 的有效位数只有 24 位，如果场景内 instance 过多，可能会溢出
            .map(|(ins_handle, ins)| self.get_as_instance_info(ins, ins_handle.data().as_ffi() as u32, scene_manager))
            .collect_vec();
        let tlas = GfxAcceleration::build_tlas_sync(
            &instance_infos,
            vk::BuildAccelerationStructureFlagsKHR::empty(),
            format!("scene-{}-{}", frame_counter.frame_label(), frame_counter.frame_id),
        );

        self.gpu_scene_buffers[*frame_counter.frame_label()].tlas = Some(tlas);
    }
}

mod helper {
    use ash::vk;
    use truvis_gfx::{
        commands::{
            barrier::{GfxBarrierMask, GfxBufferBarrier},
            command_buffer::GfxCommandBuffer,
        },
        resources::buffer::GfxBuffer,
    };
    /// 三个操作：
    /// 1. 将 stage buffer 的数据 *全部* flush 到 buffer 中
    /// 2. 从 stage buffer 中将 *所有* 数据复制到目标 buffer 中
    /// 3. 添加 barrier，确保后续访问时 copy 已经完成且数据可用
    pub fn flush_copy_and_barrier(
        cmd: &GfxCommandBuffer,
        stage_buffer: &mut GfxBuffer,
        dst: &mut GfxBuffer,
        barrier_mask: GfxBarrierMask,
    ) {
        let buffer_size = stage_buffer.size();
        {
            stage_buffer.flush(0, buffer_size);
        }
        cmd.cmd_copy_buffer(
            stage_buffer,
            dst,
            &[vk::BufferCopy {
                size: buffer_size,
                ..Default::default()
            }],
        );
        cmd.buffer_memory_barrier(
            vk::DependencyFlags::empty(),
            &[GfxBufferBarrier::default().mask(barrier_mask).buffer(dst.vk_buffer(), 0, vk::WHOLE_SIZE)],
        );
    }

    pub fn get_rt_matrix(trans: &glam::Mat4) -> vk::TransformMatrixKHR {
        let c1 = &trans.x_axis;
        let c2 = &trans.y_axis;
        let c3 = &trans.z_axis;
        let c4 = &trans.w_axis;

        // 3x4 matrix, row-major order
        vk::TransformMatrixKHR {
            matrix: [
                c1.x, c2.x, c3.x, c4.x, // row 1
                c1.y, c2.y, c3.y, c4.y, // row 2
                c1.z, c2.z, c3.z, c4.z, // row 3
            ],
        }
    }
}
