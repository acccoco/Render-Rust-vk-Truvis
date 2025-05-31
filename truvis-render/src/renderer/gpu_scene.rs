use crate::renderer::bindless::BindlessManager;
use crate::renderer::scene_manager::TheWorld;
use ash::vk;
use glam::Vec4Swizzles;
use itertools::Itertools;
use model_manager::component::{DrsGeometry3D, DrsInstance};
use shader_binding::shader;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use truvis_rhi::core::acceleration::RhiAcceleration;
use truvis_rhi::core::buffer::RhiStructuredBuffer;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::synchronize::{RhiBarrierMask, RhiBufferBarrier};
use truvis_rhi::rhi::Rhi;

/// 数据以顺序的方式存储，同时查找时间为 O(1)
#[derive(Default)]
struct FlattenMap<T: std::hash::Hash + Eq + Copy> {
    /// 顺序形式存储的数据
    linear_storage: Vec<T>,

    /// 用于查找数据的 HashMap
    query_table: HashMap<T, usize>,
}
impl<T: std::hash::Hash + Eq + Copy> FlattenMap<T> {
    /// 找到数据在 store 中的索引
    #[inline]
    pub fn at(&self, key: &T) -> Option<usize> {
        self.query_table.get(key).copied()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.linear_storage.iter()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.linear_storage.len()
    }

    /// 清空所有数据
    #[inline]
    pub fn clear(&mut self) {
        self.linear_storage.clear();
        self.query_table.clear();
    }

    /// 为数据预留空间
    #[inline]
    pub fn reserve(&mut self, cnt: usize) {
        self.linear_storage.reserve(cnt);
    }

    /// 将数据插入到 store 中
    #[inline]
    pub fn insert(&mut self, key: T) {
        if self.query_table.contains_key(&key) {
            panic!("key already exists in store");
        }

        let idx = self.linear_storage.len();
        self.linear_storage.push(key);
        self.query_table.insert(key, idx);
    }
}

/// 构建 Gpu Scene 所需的所有 buffer
struct GpuSceneBuffers {
    scene_buffer: RhiStructuredBuffer<shader::Scene>,
    light_buffer: RhiStructuredBuffer<shader::PointLight>,
    light_stage_buffer: RhiStructuredBuffer<shader::PointLight>,
    material_buffer: RhiStructuredBuffer<shader::PBRMaterial>,
    material_stage_buffer: RhiStructuredBuffer<shader::PBRMaterial>,
    geometry_buffer: RhiStructuredBuffer<shader::Geometry>,
    geometry_stage_buffer: RhiStructuredBuffer<shader::Geometry>,
    instance_buffer: RhiStructuredBuffer<shader::Instance>,
    instance_stage_buffer: RhiStructuredBuffer<shader::Instance>,
    material_indirect_buffer: RhiStructuredBuffer<u32>,
    material_indirect_stage_buffer: RhiStructuredBuffer<u32>,
    geometry_indirect_buffer: RhiStructuredBuffer<u32>,
    geometry_indirect_stage_buffer: RhiStructuredBuffer<u32>,
    tlas: Option<RhiAcceleration>,
}
impl GpuSceneBuffers {
    fn new(rhi: &Rhi, frame_label: usize) -> Self {
        let max_light_cnt = 512;
        let max_material_cnt = 1024;
        let max_geometry_cnt = 1024 * 8;
        let max_instance_cnt = 1024;

        GpuSceneBuffers {
            scene_buffer: RhiStructuredBuffer::new_ubo(rhi, 1, format!("scene buffer-{}", frame_label)),
            light_buffer: RhiStructuredBuffer::new_ubo(rhi, max_light_cnt, format!("light buffer-{}", frame_label)),
            light_stage_buffer: RhiStructuredBuffer::new_stage_buffer(
                rhi,
                max_light_cnt,
                format!("light stage buffer-{}", frame_label),
            ),
            material_buffer: RhiStructuredBuffer::new_ubo(
                rhi,
                max_material_cnt,
                format!("material buffer-{}", frame_label),
            ),
            material_stage_buffer: RhiStructuredBuffer::new_stage_buffer(
                rhi,
                max_material_cnt,
                format!("material stage buffer-{}", frame_label),
            ),
            geometry_buffer: RhiStructuredBuffer::new_ubo(
                rhi,
                max_geometry_cnt,
                format!("geometry buffer-{}", frame_label),
            ),
            geometry_stage_buffer: RhiStructuredBuffer::new_stage_buffer(
                rhi,
                max_geometry_cnt,
                format!("geometry stage buffer-{}", frame_label),
            ),
            instance_buffer: RhiStructuredBuffer::new_ubo(
                rhi,
                max_instance_cnt,
                format!("instance buffer-{}", frame_label),
            ),
            instance_stage_buffer: RhiStructuredBuffer::new_stage_buffer(
                rhi,
                max_instance_cnt,
                format!("instance stage buffer-{}", frame_label),
            ),
            material_indirect_buffer: RhiStructuredBuffer::new_ubo(
                rhi,
                max_instance_cnt * 8,
                format!("instance material buffer-{}", frame_label),
            ),
            material_indirect_stage_buffer: RhiStructuredBuffer::new_stage_buffer(
                rhi,
                max_instance_cnt * 8,
                format!("instance material stage buffer-{}", frame_label),
            ),
            geometry_indirect_buffer: RhiStructuredBuffer::new_ubo(
                rhi,
                max_instance_cnt * 8,
                format!("instance geometry buffer-{}", frame_label),
            ),
            geometry_indirect_stage_buffer: RhiStructuredBuffer::new_stage_buffer(
                rhi,
                max_instance_cnt * 8,
                format!("instance geometry stage buffer-{}", frame_label),
            ),
            tlas: None,
        }
    }
}

/// 用于构建传输到 GPU 的场景数据
pub struct GpuScene {
    scene_mgr: Rc<RefCell<TheWorld>>,
    bindless_mgr: Rc<RefCell<BindlessManager>>,

    /// GPU 中以顺序存储的 instance
    ///
    /// CPU 执行绘制时，会使用这个顺序来绘制实例
    flatten_instances: Vec<uuid::Uuid>,

    /// GPU 中以顺序存储的材质信息
    flatten_materials: FlattenMap<uuid::Uuid>,

    /// GPU 中以顺序存储的 mesh 信息
    ///
    /// 每个 mesh 会被分为多个 submesh，且每个 mesh 的 submesh 会被顺序存储
    flatten_meshes: FlattenMap<uuid::Uuid>,

    /// mesh 在 geometry buffer 中的 idx
    mesh_geometry_map: HashMap<uuid::Uuid, usize>,

    gpu_scene_buffers: Vec<GpuSceneBuffers>,
}
// getter
impl GpuScene {
    #[inline]
    pub fn tlas(&self, frame_label: usize) -> Option<&RhiAcceleration> {
        self.gpu_scene_buffers[frame_label].tlas.as_ref()
    }
}
impl GpuScene {
    pub fn new(
        rhi: &Rhi,
        scene_mgr: Rc<RefCell<TheWorld>>,
        bindless_mgr: Rc<RefCell<BindlessManager>>,
        frame_in_flight: usize,
    ) -> Self {
        Self {
            scene_mgr,
            bindless_mgr,

            flatten_instances: vec![],
            flatten_materials: FlattenMap::default(),
            flatten_meshes: FlattenMap::default(),
            mesh_geometry_map: HashMap::new(),

            gpu_scene_buffers: (0..frame_in_flight).map(|i| GpuSceneBuffers::new(rhi, i)).collect(),
        }
    }

    pub fn scene_device_address(&self, frame_idx: usize) -> vk::DeviceAddress {
        self.gpu_scene_buffers[frame_idx].scene_buffer.device_address()
    }

    /// 在每一帧开始时调用，将场景数据转换为 GPU 可读的形式
    pub fn prepare_render_data(&mut self, frame_idx: usize) {
        self.bindless_mgr.borrow_mut().prepare_render_data(frame_idx);

        self.flatten_material_data();
        self.flatten_mesh_data();
        self.flatten_instance_data();
    }

    /// 将已经准备好的 GPU 格式的场景数据写入 Device Buffer 中
    pub fn upload_to_buffer(
        &mut self,
        rhi: &Rhi,
        frame_label: usize,
        cmd: &RhiCommandBuffer,
        barrier_mask: RhiBarrierMask,
    ) {
        self.upload_mesh_buffer(frame_label, cmd, barrier_mask);
        self.upload_instance_buffer(frame_label, cmd, barrier_mask);
        self.upload_material_buffer(frame_label, cmd, barrier_mask);
        self.upload_light_buffer(frame_label, cmd, barrier_mask);

        // 需要确保 instance 先与 tlas 构建
        self.build_tlas(rhi, frame_label);

        self.upload_scene_buffer(frame_label, cmd, barrier_mask);
    }

    /// 绘制场景中的所有示例
    ///
    /// before_draw(instance_idx, submesh_idx)
    pub fn draw(&self, cmd: &RhiCommandBuffer, mut before_draw: impl FnMut(u32, u32)) {
        let scene_mgr = self.scene_mgr.borrow();
        for (instance_idx, instance_uuid) in self.flatten_instances.iter().enumerate() {
            let instance = scene_mgr.get_instance(instance_uuid).unwrap();
            let mesh = scene_mgr.get_mesh(&instance.mesh).unwrap();
            for (submesh_idx, geometry) in mesh.geometries.iter().enumerate() {
                cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&geometry.vertex_buffer), &[0]);
                cmd.cmd_bind_index_buffer(&geometry.index_buffer, 0, DrsGeometry3D::index_type());

                before_draw(instance_idx as u32, submesh_idx as u32);
                cmd.draw_indexed(geometry.index_cnt(), 0, 1, 0, 0);
            }
        }
    }

    /// 将所有的实例转换为 Vector，准备上传到 GPU
    ///
    /// # 注
    ///
    /// 后续绘制时，也会使用 instance vector 中的顺序和 index
    fn flatten_instance_data(&mut self) {
        let scene_mgr = self.scene_mgr.borrow();

        self.flatten_instances.clear();
        self.flatten_instances.reserve(scene_mgr.instance_map().len());

        for (instance_uuid, _) in scene_mgr.instance_map().iter() {
            self.flatten_instances.push(*instance_uuid);
        }
    }

    /// 在每一帧绘制之前，将所有的 mesh 转换为 Vector，准备上传到 GPU
    ///
    /// 记录每个 mesh 在 geometry buffer 中的起始 idx 和长度
    fn flatten_mesh_data(&mut self) {
        let scene_mgr = self.scene_mgr.borrow();

        self.mesh_geometry_map.clear();
        self.flatten_meshes.clear();
        self.flatten_meshes.reserve(scene_mgr.mesh_map().len());

        let mut geometry_idx = 0;
        for (mesh_id, mesh) in scene_mgr.mesh_map().iter() {
            self.flatten_meshes.insert(*mesh_id);
            self.mesh_geometry_map.insert(*mesh_id, geometry_idx);
            geometry_idx += mesh.geometries.len();
        }
    }

    /// 在每一帧的绘制之前，将所有的材质转换为 Vector，准备上传到 GPU
    fn flatten_material_data(&mut self) {
        let scene_mgr = self.scene_mgr.borrow();

        self.flatten_materials.clear();
        self.flatten_materials.reserve(scene_mgr.mat_map().len());

        for (mat_id, _) in scene_mgr.mat_map().iter() {
            self.flatten_materials.insert(*mat_id);
        }
    }

    /// 将整个场景的数据上传到 scene buffer 中去
    fn upload_scene_buffer(&mut self, frame_label: usize, cmd: &RhiCommandBuffer, barrier_mask: RhiBarrierMask) {
        let scene_mgr = self.scene_mgr.borrow();
        let crt_gpu_buffers = &self.gpu_scene_buffers[frame_label];
        let scene_data = shader::Scene {
            all_instances: crt_gpu_buffers.instance_buffer.device_address(),
            all_mats: crt_gpu_buffers.material_buffer.device_address(),
            all_geometries: crt_gpu_buffers.geometry_buffer.device_address(),
            instance_material_map: crt_gpu_buffers.material_indirect_buffer.device_address(),
            instance_geometry_map: crt_gpu_buffers.geometry_indirect_buffer.device_address(),
            point_lights: crt_gpu_buffers.light_buffer.device_address(),
            spot_lights: 0, // TODO 暂时无用
            point_light_count: scene_mgr.point_light_map().len() as u32,
            spot_light_count: 0, // TODO 暂时无用
            tlas: crt_gpu_buffers.tlas.as_ref().map_or(vk::DeviceAddress::default(), |tlas| tlas.get_device_address()),

            _padding_0: 0,
            _padding_1: 0,
        };

        cmd.cmd_update_buffer(crt_gpu_buffers.scene_buffer.handle(), 0, bytemuck::bytes_of(&scene_data));
        cmd.buffer_memory_barrier(
            vk::DependencyFlags::empty(),
            &[RhiBufferBarrier::default().mask(barrier_mask).buffer(
                crt_gpu_buffers.scene_buffer.handle(),
                0,
                vk::WHOLE_SIZE,
            )],
        );
    }

    /// 将数据转换为 shader 中的实例数据
    ///
    /// 其中 buffer 可以是 stage buffer 的内存映射
    fn upload_instance_buffer(&mut self, frame_label: usize, cmd: &RhiCommandBuffer, barrier_mask: RhiBarrierMask) {
        let crt_gpu_buffers = &mut self.gpu_scene_buffers[frame_label];

        let crt_instance_stage_buffer = &mut crt_gpu_buffers.instance_stage_buffer;
        let crt_geometry_indirect_stage_buffer = &mut crt_gpu_buffers.geometry_indirect_stage_buffer;
        let crt_material_indirect_stage_buffer = &mut crt_gpu_buffers.material_indirect_stage_buffer;

        crt_instance_stage_buffer.map();
        crt_geometry_indirect_stage_buffer.map();
        crt_material_indirect_stage_buffer.map();

        let instance_buffer_slices = crt_instance_stage_buffer.mapped_slice();
        let material_indirect_buffer_slices = crt_material_indirect_stage_buffer.mapped_slice();
        let geometry_indirect_buffer_slices = crt_geometry_indirect_stage_buffer.mapped_slice();

        if instance_buffer_slices.len() < self.flatten_instances.len() {
            panic!("instance cnt can not be larger than buffer");
        }

        let scene_mgr = self.scene_mgr.borrow();

        let mut crt_geometry_indirect_idx = 0;
        let mut crt_material_indirect_idx = 0;
        for (instance_idx, instance_uuid) in self.flatten_instances.iter().enumerate() {
            let instance = scene_mgr.get_instance(instance_uuid).unwrap();
            let submesh_cnt = instance.materials.len();
            if geometry_indirect_buffer_slices.len() < crt_geometry_indirect_idx + submesh_cnt {
                panic!("instance geometry cnt can not be larger than buffer");
            }
            if material_indirect_buffer_slices.len() < crt_material_indirect_idx + submesh_cnt {
                panic!("instance material cnt can not be larger than buffer");
            }

            instance_buffer_slices[instance_idx] = shader::Instance {
                geometry_indirect_idx: crt_geometry_indirect_idx as u32,
                geometry_count: submesh_cnt as u32,
                material_indirect_idx: crt_material_indirect_idx as u32,
                material_count: submesh_cnt as u32,
                model: instance.transform.into(),
                inv_model: instance.transform.inverse().into(),
            };

            // TODO 对于 mesh 来说，可能不需要间接的索引 buffer，因为 mesh 在 geometry buffer 中是连续的
            // 首先将 instance 需要的 geometry 的实际索引，写入一个间接索引 buffer: geometry_indirect_buffer，
            // 然后获得 instance 数据在间接索引 buffer 中的起始 idx 和长度，将这个值写入到 shader::Instance 中
            let geometry_idx_start = self.mesh_geometry_map.get(&instance.mesh).unwrap();
            for submesh_idx in 0..instance.materials.len() {
                let geometry_idx = geometry_idx_start + submesh_idx;
                geometry_indirect_buffer_slices[crt_geometry_indirect_idx + submesh_idx] = geometry_idx as u32;
            }
            crt_geometry_indirect_idx += submesh_cnt;

            for material_uuid in instance.materials.iter() {
                let material_idx = self.flatten_materials.at(material_uuid).unwrap();
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
    fn upload_material_buffer(&mut self, frame_label: usize, cmd: &RhiCommandBuffer, barrier_mask: RhiBarrierMask) {
        let crt_gpu_buffers = &mut self.gpu_scene_buffers[frame_label];
        let crt_material_stage_buffer = &mut crt_gpu_buffers.material_stage_buffer;
        crt_material_stage_buffer.map();
        let material_buffer_slices = crt_material_stage_buffer.mapped_slice();
        if material_buffer_slices.len() < self.flatten_materials.len() {
            panic!("material cnt can not be larger than buffer");
        }

        let scene_mgr = self.scene_mgr.borrow();
        let bindless_mgr = self.bindless_mgr.borrow();
        for (mat_idx, mat_uuid) in self.flatten_materials.iter().enumerate() {
            let mat = scene_mgr.mat_map().get(mat_uuid).unwrap();
            material_buffer_slices[mat_idx] = shader::PBRMaterial {
                base_color: mat.diffuse.xyz().into(),
                emissive: mat.emissive.xyz().into(),
                metallic: 0.5,
                roughness: 0.5,
                diffuse_map: bindless_mgr
                    .get_texture_idx(&mat.diffuse_map)
                    .unwrap_or(shader::TextureHandle { index: 0 }),
                normal_map: shader::TextureHandle { index: 0 },

                _padding_1: Default::default(),
                _padding_2: Default::default(),
            };
        }

        helper::flush_copy_and_barrier(
            cmd,
            crt_material_stage_buffer,
            &mut crt_gpu_buffers.material_buffer,
            barrier_mask,
        );
    }

    fn upload_light_buffer(&mut self, frame_label: usize, cmd: &RhiCommandBuffer, barrier_mask: RhiBarrierMask) {
        let crt_gpu_buffers = &mut self.gpu_scene_buffers[frame_label];
        let crt_light_stage_buffer = &mut crt_gpu_buffers.light_stage_buffer;
        crt_light_stage_buffer.map();
        let light_buffer_slices = crt_light_stage_buffer.mapped_slice();
        let scene_mgr = self.scene_mgr.borrow();
        if light_buffer_slices.len() < scene_mgr.point_light_map().len() {
            panic!("light cnt can not be larger than buffer");
        }

        for (light_idx, (_, point_light)) in scene_mgr.point_light_map().iter().enumerate() {
            light_buffer_slices[light_idx] = shader::PointLight {
                pos: point_light.pos,
                color: point_light.color,

                _color_padding: Default::default(),
                _pos_padding: Default::default(),
            };
        }

        helper::flush_copy_and_barrier(cmd, crt_light_stage_buffer, &mut crt_gpu_buffers.light_buffer, barrier_mask);
    }

    /// 将 mesh 数据以 geometry 的形式上传到 GPU
    fn upload_mesh_buffer(&mut self, frame_label: usize, cmd: &RhiCommandBuffer, barrier_mask: RhiBarrierMask) {
        let crt_gpu_buffers = &mut self.gpu_scene_buffers[frame_label];
        let crt_geometry_stage_buffer = &mut crt_gpu_buffers.geometry_stage_buffer;
        crt_geometry_stage_buffer.map();
        // let crt_geometry_stage_buffer = &mut crt_gpu_buffers.geometry_stage_buffer;
        let geometry_buffer_slices = crt_geometry_stage_buffer.mapped_slice();
        let scene_mgr = self.scene_mgr.borrow();

        let mut crt_geometry_idx = 0;
        for mesh_uuid in self.flatten_meshes.iter() {
            let mesh = scene_mgr.mesh_map().get(mesh_uuid).unwrap();
            if geometry_buffer_slices.len() < crt_geometry_idx + mesh.geometries.len() {
                panic!("geometry cnt can not be larger than buffer");
            }
            for (submesh_idx, geometry) in mesh.geometries.iter().enumerate() {
                geometry_buffer_slices[crt_geometry_idx + submesh_idx] = shader::Geometry {
                    position_buffer: geometry.vertex_buffer.device_address(),
                    index_buffer: geometry.index_buffer.device_address(),

                    normal_buffer: vk::DeviceAddress::default(), // TODO 暂时无用
                    uv_buffer: vk::DeviceAddress::default(),     // TODO 暂时无用
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
    fn get_as_instance_info(&self, instance: &DrsInstance, custom_idx: u32) -> vk::AccelerationStructureInstanceKHR {
        let scene_mgr = self.scene_mgr.borrow();
        let mesh = scene_mgr.get_mesh(&instance.mesh).expect("Mesh not found");
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

    fn build_tlas(&mut self, rhi: &Rhi, frame_label: usize) {
        if self.flatten_instances.is_empty() {
            // 没有实例数据，直接返回
            return;
        }

        if self.gpu_scene_buffers[frame_label].tlas.is_some() {
            // 已经构建过 tlas，直接返回
            return;
        }

        let scene_mgr = self.scene_mgr.borrow();
        let instance_infos = self
            .flatten_instances
            .iter()
            .map(|ins_uuid| scene_mgr.get_instance(ins_uuid).unwrap())
            .enumerate()
            // TODO 这里暂时将 instance 的 index 作为 custom index
            .map(|(ins_idx, ins)| self.get_as_instance_info(ins, ins_idx as u32))
            .collect_vec();
        let tlas = RhiAcceleration::build_tlas_sync(
            rhi,
            &instance_infos,
            vk::BuildAccelerationStructureFlagsKHR::empty(),
            "scene tlas",
        );

        self.gpu_scene_buffers[frame_label].tlas = Some(tlas);
    }
}

mod helper {
    use ash::vk;
    use truvis_rhi::core::buffer::RhiBuffer;
    use truvis_rhi::core::command_buffer::RhiCommandBuffer;
    use truvis_rhi::core::synchronize::{RhiBarrierMask, RhiBufferBarrier};

    /// 三个操作：
    /// 1. 将 stage buffer 的数据 *全部* flush 到 buffer 中
    /// 2. 从 stage buffer 中将 *所有* 数据复制到目标 buffer 中
    /// 3. 添加 barrier，确保后续访问时 copy 已经完成且数据可用
    pub fn flush_copy_and_barrier(
        cmd: &RhiCommandBuffer,
        stage_buffer: &mut RhiBuffer,
        dst: &mut RhiBuffer,
        barrier_mask: RhiBarrierMask,
    ) {
        let buffer_size = stage_buffer.size();
        {
            stage_buffer.flush(0, buffer_size);
            stage_buffer.unmap();
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
            &[RhiBufferBarrier::default().mask(barrier_mask).buffer(dst.handle(), 0, vk::WHOLE_SIZE)],
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
