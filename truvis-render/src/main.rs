use ash::vk;
use imgui::Ui;
use itertools::Itertools;
use model_manager::component::mesh::SimpleMesh;
use model_manager::manager::instance_manager::InstanceManager;
use model_manager::manager::mat_manager::MatManager;
use model_manager::manager::mesh_manager::MeshManager;
use model_manager::vertex::vertex_3d::VertexLayoutAos3D;
use model_manager::vertex::vertex_pnu::VertexLayoutAosPosNormalUv;
use model_manager::vertex::VertexLayout;
use shader_binding::shader;
use shader_layout_macro::ShaderLayout;
use std::mem::offset_of;
use std::rc::Rc;
use truvis_assimp::SceneLoader;
use truvis_render::app::{AppCtx, OuterApp, TruvisApp};
use truvis_render::frame_context::FrameContext;
use truvis_render::platform::camera::Camera;
use truvis_render::renderer::framebuffer::RenderBuffer;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::pipeline::RhiGraphicsPipelineCreateInfo;
use truvis_rhi::core::synchronize::RhiBufferBarrier;
use truvis_rhi::shader_cursor::ShaderCursor;
use truvis_rhi::{
    basic::{color::LabelColor, FRAME_ID_MAP},
    core::{
        buffer::{RhiBuffer, RhiBufferCreateInfo},
        command_queue::RhiSubmitInfo,
        descriptor::{RhiDescriptorSet, RhiDescriptorSetLayout},
        pipeline::RhiGraphicsPipeline,
    },
    rhi::Rhi,
};

#[derive(ShaderLayout)]
struct SceneShaderBindings {
    #[binding = 0]
    #[descriptor_type = "UNIFORM_BUFFER_DYNAMIC"]
    #[stage = "VERTEX | FRAGMENT"]
    _scene: (),
}

#[derive(ShaderLayout)]
struct MeshShaderBindings {
    #[binding = 0]
    #[descriptor_type = "UNIFORM_BUFFER_DYNAMIC"]
    #[stage = "VERTEX | FRAGMENT"]
    _mesh: (),
}

#[derive(ShaderLayout)]
struct MaterialShaderBindings {
    #[binding = 0]
    #[descriptor_type = "UNIFORM_BUFFER_DYNAMIC"]
    #[stage = "FRAGMENT"]
    _mat: (),
}

#[derive(ShaderLayout)]
struct BindlessTextureBindings {
    #[binding = 0]
    #[descriptor_type = "COMBINED_IMAGE_SAMPLER"]
    #[stage = "FRAGMENT"]
    #[count = 128]
    #[flags = "PARTIALLY_BOUND | UPDATE_AFTER_BIND"]
    _textures: (),

    #[binding = 1]
    #[descriptor_type = "STORAGE_IMAGE"]
    #[stage = "FRAGMENT"]
    #[count = 128]
    #[flags = "PARTIALLY_BOUND | UPDATE_AFTER_BIND"]
    _images: (),
}

struct PhongAppDescriptorSetLayouts {
    scene_layout: RhiDescriptorSetLayout<SceneShaderBindings>,
    mesh_layout: RhiDescriptorSetLayout<MeshShaderBindings>,
    material_layout: RhiDescriptorSetLayout<MaterialShaderBindings>,
}

struct PhongAppDescriptorSets {
    scene_set: RhiDescriptorSet<SceneShaderBindings>,
    mesh_set: RhiDescriptorSet<MeshShaderBindings>,
    material_set: RhiDescriptorSet<MaterialShaderBindings>,
}

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct Light {
    pos: glam::Vec3,
    pos_padding__: i32,

    color: glam::Vec3,
    color_padding__: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct SceneUboPerDraw {
    projection: glam::Mat4,
    view: glam::Mat4,

    l1: Light,
    l2: Light,
    l3: Light,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MeshUBO {
    model: glam::Mat4,
    trans_inv_model: glam::Mat4,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MaterialUBO {
    color: glam::Vec4,
}

struct PhongUniformBuffer {
    scene_uniform_buffer: RhiBuffer,
    mesh_uniform_buffer: RhiBuffer,
    material_uniform_buffer: RhiBuffer,
}

struct PhongApp {
    _descriptor_set_layouts: PhongAppDescriptorSetLayouts,

    _bindless_layout: RhiDescriptorSetLayout<BindlessTextureBindings>,
    _bindless_descriptor_set: RhiDescriptorSet<BindlessTextureBindings>,

    mesh_manager: MeshManager,
    _mat_manager: MatManager,
    instance_manager: InstanceManager,

    /// 每帧独立的 descriptor set
    descriptor_sets: Vec<PhongAppDescriptorSets>,
    pipeline_simple: RhiGraphicsPipeline,

    pipeline_3d: RhiGraphicsPipeline,
    /// 每个 instance 的数据
    // instance_descriptor_sets: Vec<RhiDescriptorSet<MeshShaderBindings>>,

    /// BOX
    cube: SimpleMesh,
    /// 每帧独立的 uniform buffer
    uniform_buffers: Vec<PhongUniformBuffer>,

    mesh_ubo_offset_align: vk::DeviceSize,
    _mat_ubo_offset_align: vk::DeviceSize,

    mesh_ubo: Vec<MeshUBO>,
    mat_ubo: Vec<MaterialUBO>,
    scene_ubo_per_draw: SceneUboPerDraw,

    camera: Camera,

    push: shader::PushConstants,
}

impl PhongApp {
    fn create_descriptor_sets(
        rhi: &Rhi,
        _render_context: &FrameContext,
        frames_in_flight: usize,
    ) -> (PhongAppDescriptorSetLayouts, Vec<PhongAppDescriptorSets>) {
        let scene_descriptor_set_layout = RhiDescriptorSetLayout::<SceneShaderBindings>::new(
            rhi,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            "phong-scene",
        );
        let mesh_descriptor_set_layout = RhiDescriptorSetLayout::<MeshShaderBindings>::new(
            rhi,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            "phong-mesh",
        );
        let material_descriptor_set_layout = RhiDescriptorSetLayout::<MaterialShaderBindings>::new(
            rhi,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            "phong-material",
        );

        let layouts = PhongAppDescriptorSetLayouts {
            scene_layout: scene_descriptor_set_layout,
            mesh_layout: mesh_descriptor_set_layout,
            material_layout: material_descriptor_set_layout,
        };

        let sets = (0..frames_in_flight)
            .map(|idx| FRAME_ID_MAP[idx])
            .map(|tag| PhongAppDescriptorSets {
                scene_set: RhiDescriptorSet::<SceneShaderBindings>::new(
                    rhi,
                    rhi.descriptor_pool(),
                    &layouts.scene_layout,
                    &format!("phong-scene-{}", tag),
                ),
                mesh_set: RhiDescriptorSet::<MeshShaderBindings>::new(
                    rhi,
                    rhi.descriptor_pool(),
                    &layouts.mesh_layout,
                    &format!("phong-mesh-{}", tag),
                ),
                material_set: RhiDescriptorSet::<MaterialShaderBindings>::new(
                    rhi,
                    rhi.descriptor_pool(),
                    &layouts.material_layout,
                    &format!("phong-material-{}", tag),
                ),
            })
            .collect_vec();

        (layouts, sets)
    }

    fn create_pipeline(
        rhi: &Rhi,
        render_ctx: &mut FrameContext,
        descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    ) -> (RhiGraphicsPipeline, RhiGraphicsPipeline) {
        let mut ci = RhiGraphicsPipelineCreateInfo::default();
        ci.vertex_shader_stage("shader/build/phong/phong.vs.slang.spv".to_string(), "main".to_string());
        ci.fragment_shader_stage("shader/build/phong/phong.ps.slang.spv".to_string(), "main".to_string());
        ci.push_constant_ranges(vec![vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(size_of::<shader::PushConstants>() as u32)]);
        ci.descriptor_set_layouts(descriptor_set_layouts);
        ci.attach_info(vec![render_ctx.color_format()], Some(render_ctx.depth_format()), None);
        ci.vertex_binding(VertexLayoutAosPosNormalUv::vertex_input_bindings());
        ci.vertex_attribute(VertexLayoutAosPosNormalUv::vertex_input_attributes());
        ci.color_blend_attach_states(vec![vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)]);

        let simple_pipe = RhiGraphicsPipeline::new(rhi.device.clone(), &ci, "phong-simple-pipe");

        ci.vertex_shader_stage("shader/build/phong/phong3d.vs.slang.spv".to_string(), "main".to_string());
        ci.vertex_binding(VertexLayoutAos3D::vertex_input_bindings());
        ci.vertex_attribute(VertexLayoutAos3D::vertex_input_attributes());

        let d3_pipe = RhiGraphicsPipeline::new(rhi.device.clone(), &ci, "phong-d3-pipe");

        (simple_pipe, d3_pipe)
    }

    /// mesh ubo 数量：32 个
    /// material ubo 数量：32 个
    fn create_uniform_buffers(
        rhi: &Rhi,
        frames_in_flight: usize,
        mesh_ubo_align: &mut vk::DeviceSize,
        mat_ubo_align: &mut vk::DeviceSize,
    ) -> Vec<PhongUniformBuffer> {
        let mesh_instance_count = 32;
        let material_instance_count = 32;

        *mesh_ubo_align = rhi.device.align_ubo_size(size_of::<MeshUBO>() as vk::DeviceSize);
        *mat_ubo_align = rhi.device.align_ubo_size(size_of::<MaterialUBO>() as vk::DeviceSize);

        let scene_buffer_ci = Rc::new(RhiBufferCreateInfo::new(
            size_of::<SceneUboPerDraw>() as vk::DeviceSize * frames_in_flight as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        ));
        let mesh_buffer_ci = Rc::new(RhiBufferCreateInfo::new(
            *mesh_ubo_align * mesh_instance_count * frames_in_flight as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        ));
        let material_buffer_ci = Rc::new(RhiBufferCreateInfo::new(
            *mat_ubo_align * material_instance_count * frames_in_flight as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        ));

        let ubo_alloc_ci = Rc::new(vk_mem::AllocationCreateInfo {
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        });

        (0..frames_in_flight)
            .map(|idx| FRAME_ID_MAP[idx])
            .map(|tag| PhongUniformBuffer {
                scene_uniform_buffer: RhiBuffer::new(
                    rhi,
                    scene_buffer_ci.clone(),
                    ubo_alloc_ci.clone(),
                    None,
                    &format!("scene-ubo-{}", tag),
                ),
                mesh_uniform_buffer: RhiBuffer::new(
                    rhi,
                    mesh_buffer_ci.clone(),
                    ubo_alloc_ci.clone(),
                    Some(*mesh_ubo_align),
                    &format!("mesh-ubo-{}", tag),
                ),
                material_uniform_buffer: RhiBuffer::new(
                    rhi,
                    material_buffer_ci.clone(),
                    ubo_alloc_ci.clone(),
                    Some(*mat_ubo_align),
                    &format!("material-ubo-{}", tag),
                ),
            })
            .collect_vec()
    }

    /// 将场景的信息更新到 uniform buffer 中去，并且和 descriptor set 绑定起来
    fn update_scene_uniform(&mut self, rhi: &Rhi, frame_index: usize) {
        let PhongUniformBuffer {
            scene_uniform_buffer,
            mesh_uniform_buffer,
            material_uniform_buffer,
        } = &mut self.uniform_buffers[frame_index];

        scene_uniform_buffer.transfer_data_by_mem_map(std::slice::from_ref(&self.scene_ubo_per_draw));
        mesh_uniform_buffer.transfer_data_by_mem_map(&self.mesh_ubo);
        material_uniform_buffer.transfer_data_by_mem_map(&self.mat_ubo);

        let PhongAppDescriptorSets {
            scene_set,
            mesh_set,
            material_set,
        } = &mut self.descriptor_sets[frame_index];

        let scene_write = SceneShaderBindings::scene().write_buffer(
            scene_set.handle,
            0,
            vec![scene_uniform_buffer.get_descriptor_buffer_info_ubo::<SceneUboPerDraw>()],
        );
        let mesh_write = MeshShaderBindings::mesh().write_buffer(
            mesh_set.handle,
            0,
            vec![mesh_uniform_buffer.get_descriptor_buffer_info_ubo::<MeshUBO>()],
        );
        let mat_write = MaterialShaderBindings::mat().write_buffer(
            material_set.handle,
            0,
            vec![material_uniform_buffer.get_descriptor_buffer_info_ubo::<MaterialUBO>()],
        );

        rhi.device.write_descriptor_sets(&[scene_write, mesh_write, mat_write]);
    }

    fn draw_simple_obj(&self, cmd: &RhiCommandBuffer, app_ctx: &mut AppCtx) {
        let frame_id = app_ctx.render_context.current_frame_index();

        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline_simple.pipeline);

        let swapchain_extend = app_ctx.render_context.swapchain_extent();
        cmd.cmd_set_viewport(
            0,
            &[vk::Viewport {
                x: 0.0,
                y: swapchain_extend.height as f32,
                width: swapchain_extend.width as f32,
                height: -(swapchain_extend.height as f32),
                min_depth: 0.0,
                max_depth: 1.0,
            }],
        );
        cmd.cmd_set_scissor(
            0,
            &[vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: swapchain_extend,
            }],
        );

        cmd.cmd_push_constants(
            self.pipeline_simple.pipeline_layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            bytemuck::bytes_of(&self.push),
        );

        // scene data
        cmd.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline_simple.pipeline_layout,
            0,
            &[self.descriptor_sets[frame_id].scene_set.handle],
            &[0],
        );

        // per mat
        cmd.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline_simple.pipeline_layout,
            2,
            &[self.descriptor_sets[frame_id].material_set.handle],
            // TODO 只使用一个材质
            &[0],
        );

        // index 和 vertex 暂且就用同一个
        cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&self.cube.vertex_buffer), &[0]);
        cmd.cmd_bind_index_buffer(&self.cube.index_buffer, 0, vk::IndexType::UINT32);

        for (mesh_idx, _) in self.mesh_ubo.iter().enumerate() {
            cmd.bind_descriptor_sets(
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_simple.pipeline_layout,
                1,
                &[self.descriptor_sets[frame_id].mesh_set.handle],
                &[(self.mesh_ubo_offset_align * mesh_idx as u64) as u32],
            );
            cmd.draw_indexed(self.cube.index_cnt, 0, 1, 0, 0);
            // cmd.cmd_draw(VertexPosNormalUvAoS::shape_box().len() as u32, 1, 0, 0);
        }
    }

    fn draw_3d_obj(&self, cmd: &RhiCommandBuffer, app_ctx: &mut AppCtx) {
        let frame_id = app_ctx.render_context.current_frame_index();

        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline_3d.pipeline);
        let swapchain_extend = app_ctx.render_context.swapchain_extent();
        cmd.cmd_set_viewport(
            0,
            &[vk::Viewport {
                x: 0.0,
                y: swapchain_extend.height as f32,
                width: swapchain_extend.width as f32,
                height: -(swapchain_extend.height as f32),
                min_depth: 0.0,
                max_depth: 1.0,
            }],
        );
        cmd.cmd_set_scissor(
            0,
            &[vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: swapchain_extend,
            }],
        );

        cmd.cmd_push_constants(
            self.pipeline_3d.pipeline_layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            bytemuck::bytes_of(&self.push),
        );

        // scene data
        cmd.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline_3d.pipeline_layout,
            0,
            &[self.descriptor_sets[frame_id].scene_set.handle],
            &[0],
        );

        // per mat
        cmd.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline_3d.pipeline_layout,
            2,
            &[self.descriptor_sets[frame_id].material_set.handle],
            // TODO 只使用一个材质
            &[0],
        );

        // FIXME 随便绑定一下，否则会报错
        cmd.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline_3d.pipeline_layout,
            1,
            &[self.descriptor_sets[frame_id].mesh_set.handle],
            &[0],
        );

        for (_, instance) in &self.instance_manager.ins_map {
            let transform_data = [instance.transform, instance.transform.inverse()];

            cmd.cmd_push_constants(
                self.pipeline_3d.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                offset_of!(shader::PushConstants, model) as u32,
                bytemuck::bytes_of(&transform_data),
            );

            for mesh_id in &instance.meshes {
                let mesh = self.mesh_manager.mesh_map.get(mesh_id).unwrap();
                cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&mesh.vertex_buffer), &[0]);
                cmd.cmd_bind_index_buffer(&mesh.index_buffer, 0, vk::IndexType::UINT32);
                cmd.draw_indexed(mesh.index_cnt, 0, 1, 0, 0);
            }
        }
    }
}

impl OuterApp for PhongApp {
    fn init(rhi: &Rhi, render_context: &mut FrameContext) -> Self {
        // TODO 通过其他方式获取到 frames in flight
        let (layouts, sets) = Self::create_descriptor_sets(rhi, render_context, 3);
        let (pipe_simple, pipe_3d) = Self::create_pipeline(
            rhi,
            render_context,
            vec![
                layouts.scene_layout.layout,
                layouts.mesh_layout.layout,
                layouts.material_layout.layout,
            ],
        );

        let mut mesh_ubo_offset_align = 0;
        let mut mat_ubo_offset_align = 0;
        let uniform_buffers =
            Self::create_uniform_buffers(rhi, 3, &mut mesh_ubo_offset_align, &mut mat_ubo_offset_align);
        let cube = VertexLayoutAosPosNormalUv::cube(rhi);

        let mut mesh_manager = MeshManager::default();
        let mut mat_manager = MatManager::default();
        let mut instance_manager = InstanceManager::default();
        SceneLoader::load_model(
            rhi,
            std::path::Path::new("assets/obj/spot.obj"),
            &mut instance_manager,
            &mut mesh_manager,
            &mut mat_manager,
        );

        let rot =
            glam::Mat4::from_euler(glam::EulerRot::XYZ, 30f32.to_radians(), 40f32.to_radians(), 50f32.to_radians());
        let mesh_trans = [
            glam::Mat4::from_translation(glam::vec3(10.0, 0.0, 0.0)) * rot,
            glam::Mat4::from_translation(glam::vec3(0.0, 10.0, 0.0)) * rot,
            glam::Mat4::from_translation(glam::vec3(0.0, 15.0, 10.0)),
            glam::Mat4::from_translation(glam::vec3(0.0, 0.0, 0.0)) * rot,
            glam::Mat4::from_translation(glam::vec3(0.0, -10.0, 0.0)) * rot,
        ];

        let camera = Camera {
            position: glam::vec3(20.0, 0.0, 0.0),
            euler_yaw_deg: 90.0,
            ..Default::default()
        };

        let bindless_layout = RhiDescriptorSetLayout::<BindlessTextureBindings>::new(
            rhi,
            vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
            "bindless-layout",
        );
        let bindless_descriptor_set = RhiDescriptorSet::<BindlessTextureBindings>::new(
            rhi,
            rhi.descriptor_pool(),
            &bindless_layout,
            "bindless-descriptor-set",
        );

        Self {
            _descriptor_set_layouts: layouts,
            descriptor_sets: sets,

            mesh_manager,
            _mat_manager: mat_manager,
            instance_manager,

            _bindless_layout: bindless_layout,
            _bindless_descriptor_set: bindless_descriptor_set,

            pipeline_simple: pipe_simple,
            pipeline_3d: pipe_3d,
            cube,
            uniform_buffers,
            mesh_ubo_offset_align,
            _mat_ubo_offset_align: mat_ubo_offset_align,
            mesh_ubo: mesh_trans
                .iter()
                .map(|trans| MeshUBO {
                    model: *trans,
                    trans_inv_model: trans.inverse().transpose(),
                })
                .collect_vec(),
            mat_ubo: vec![MaterialUBO {
                color: glam::Vec4::new(1.0, 0.0, 0.0, 1.0),
            }],
            scene_ubo_per_draw: SceneUboPerDraw {
                projection: camera.get_projection_matrix(),
                view: camera.get_view_matrix(),
                l1: Light {
                    pos: glam::vec3(-20.0, 40.0, 0.0),
                    color: glam::vec3(5.0, 6.0, 1.0) * 2.0,
                    ..Default::default()
                },
                l2: Light {
                    pos: glam::vec3(40.0, 40.0, -30.0),
                    color: glam::vec3(1.0, 6.0, 7.0) * 3.0,
                    ..Default::default()
                },
                l3: Light {
                    pos: glam::vec3(40.0, 40.0, 30.0),
                    color: glam::vec3(5.0, 1.0, 8.0) * 3.0,
                    ..Default::default()
                },
            },
            push: shader::PushConstants {
                camera_pos: camera.position.into(),
                camera_dir: camera.camera_forward().into(),
                frame_id: 0,
                delta_time_ms: 0.0,
                mouse: Default::default(),
                resolution: Default::default(),
                time: 0.0,
                frame_rate: 0.0,
                ..Default::default()
            },
            camera,
        }
    }

    fn draw_ui(&mut self, ui: &mut Ui) {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }

    fn update(&mut self, app_ctx: &mut AppCtx) {
        // camera controller
        if app_ctx.input_state.right_button_pressed {
            let delta = app_ctx.input_state.crt_mouse_pos - app_ctx.input_state.last_mouse_pos;
            let delta = delta * (app_ctx.timer.delta_time_s as f64) * 100.0;

            self.camera.rotate_yaw(delta.x as f32);
            self.camera.rotate_pitch(delta.y as f32);

            let move_speed = 10_f32;
            if let Some(true) = app_ctx.input_state.key_pressed.get(&winit::keyboard::KeyCode::KeyW) {
                self.camera.move_forward(app_ctx.timer.delta_time_s * move_speed);
            }
            if let Some(true) = app_ctx.input_state.key_pressed.get(&winit::keyboard::KeyCode::KeyS) {
                self.camera.move_forward(-app_ctx.timer.delta_time_s * move_speed);
            }
            if let Some(true) = app_ctx.input_state.key_pressed.get(&winit::keyboard::KeyCode::KeyA) {
                self.camera.move_right(-app_ctx.timer.delta_time_s * move_speed);
            }
            if let Some(true) = app_ctx.input_state.key_pressed.get(&winit::keyboard::KeyCode::KeyD) {
                self.camera.move_right(app_ctx.timer.delta_time_s * move_speed);
            }
            if let Some(true) = app_ctx.input_state.key_pressed.get(&winit::keyboard::KeyCode::KeyE) {
                self.camera.move_up(-app_ctx.timer.delta_time_s * move_speed);
            }
            if let Some(true) = app_ctx.input_state.key_pressed.get(&winit::keyboard::KeyCode::KeyQ) {
                self.camera.move_up(app_ctx.timer.delta_time_s * move_speed);
            }
        }

        self.push.camera_pos = self.camera.position.into();
        self.push.camera_dir = self.camera.camera_forward().into();
        self.scene_ubo_per_draw.view = self.camera.get_view_matrix();
        self.scene_ubo_per_draw.projection = self.camera.get_projection_matrix();
        self.push.scene_buffer_ptr = unsafe {
            let frame_id = app_ctx.render_context.current_frame_index();
            app_ctx.rhi.device.get_buffer_device_address(
                &vk::BufferDeviceAddressInfo::default()
                    .buffer(self.uniform_buffers[frame_id].scene_uniform_buffer.handle()),
            )
        };

        app_ctx.rhi.device.debug_utils.begin_queue_label(
            app_ctx.rhi.graphics_queue.handle,
            "[main-pass]update",
            LabelColor::COLOR_PASS,
        );
        {
            self.update_scene_uniform(app_ctx.rhi, app_ctx.render_context.current_frame_index());
        }
        app_ctx.rhi.device.debug_utils.end_queue_label(app_ctx.rhi.graphics_queue.handle);
    }

    fn draw(&self, app_ctx: &mut AppCtx) {
        let frame_id = app_ctx.render_context.current_frame_index();

        let color_attach = RenderBuffer::get_color_attachment(app_ctx.render_context.current_present_image_view());
        let depth_attach = RenderBuffer::get_depth_attachment(app_ctx.render_context.depth_view.handle());
        let render_info = RenderBuffer::get_render_info(
            vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: app_ctx.render_context.swapchain_extent(),
            },
            std::slice::from_ref(&color_attach),
            &depth_attach,
        );

        let cmd = FrameContext::alloc_command_buffer(app_ctx.render_context, "[main-pass]render");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[phong-pass]draw");
        {
            // 更新 per draw 的场景数据 ubo
            cmd.cmd_update_buffer(
                self.uniform_buffers[frame_id].scene_uniform_buffer.handle(),
                0,
                bytemuck::bytes_of(&self.scene_ubo_per_draw),
            );
            cmd.buffer_memory_barrier(
                vk::DependencyFlags::empty(),
                &[RhiBufferBarrier::default()
                    .buffer(self.uniform_buffers[frame_id].scene_uniform_buffer.handle(), 0, vk::WHOLE_SIZE)
                    .src_mask(vk::PipelineStageFlags2::TRANSFER, vk::AccessFlags2::TRANSFER_WRITE)
                    .dst_mask(vk::PipelineStageFlags2::VERTEX_SHADER, vk::AccessFlags2::SHADER_READ)],
            );

            // 开始渲染之前，准备 per mesh 的数据

            cmd.cmd_begin_rendering(&render_info);

            cmd.begin_label("[phong-pass]simple-draw", LabelColor::COLOR_PASS);
            self.draw_simple_obj(&cmd, app_ctx);
            cmd.end_label();

            cmd.begin_label("[phong-pass]3d-draw", LabelColor::COLOR_PASS);
            self.draw_3d_obj(&cmd, app_ctx);
            cmd.end_label();

            cmd.end_rendering();
        }
        cmd.end();

        app_ctx.rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[cmd])], None);
    }

    fn rebuild(&mut self, _rhi: &Rhi, render_context: &mut FrameContext) {
        self.camera.asp =
            render_context.swapchain_extent().width as f32 / render_context.swapchain_extent().height as f32;
    }
}

fn main() {
    TruvisApp::<PhongApp>::run();
}
