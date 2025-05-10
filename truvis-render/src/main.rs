use ash::vk;
use imgui::Ui;
use itertools::Itertools;
use model_manager::component::instance::SimpleInstance;
use model_manager::component::mesh::SimpleMesh;
use model_manager::vertex::vertex_3d::VertexLayoutAos3D;
use model_manager::vertex::vertex_pnu::VertexLayoutAosPosNormalUv;
use model_manager::vertex::VertexLayout;
use shader_binding::shader;
use std::mem::offset_of;
use std::rc::Rc;
use truvis_render::app::{AppCtx, OuterApp, TruvisApp};
use truvis_render::frame_context::FrameContext;
use truvis_render::platform::camera::Camera;
use truvis_render::renderer::bindless::BindlessManager;
use truvis_render::renderer::frame_scene::FrameScene;
use truvis_render::renderer::framebuffer::FrameBuffer;
use truvis_render::renderer::scene_manager::SceneManager;
use truvis_rhi::core::buffer::{RhiBDABuffer, RhiStageBuffer};
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::pipeline::RhiGraphicsPipelineCreateInfo;
use truvis_rhi::core::synchronize::RhiBufferBarrier;
use truvis_rhi::{
    basic::color::LabelColor,
    core::{command_queue::RhiSubmitInfo, pipeline::RhiGraphicsPipeline},
    rhi::Rhi,
};

struct SimpleMainPass {
    pipeline: RhiGraphicsPipeline,

    bindless_manager: Rc<BindlessManager>,
}
impl SimpleMainPass {
    pub fn new(rhi: &Rhi, frame_context: &FrameContext, bindless_manager: Rc<BindlessManager>) -> Self {
        let mut ci = RhiGraphicsPipelineCreateInfo::default();
        ci.vertex_shader_stage("shader/build/phong/phong.vs.slang.spv".to_string(), "main".to_string());
        ci.fragment_shader_stage("shader/build/phong/phong.ps.slang.spv".to_string(), "main".to_string());
        ci.push_constant_ranges(vec![vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(size_of::<shader::DrawData>() as u32)]);
        // ci.descriptor_set_layouts(vec![bindless_manager.bindless_layout.layout]);
        ci.attach_info(vec![frame_context.color_format()], Some(frame_context.depth_format()), None);
        ci.vertex_binding(VertexLayoutAosPosNormalUv::vertex_input_bindings());
        ci.vertex_attribute(VertexLayoutAosPosNormalUv::vertex_input_attributes());
        ci.color_blend_attach_states(vec![vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)]);

        let simple_pipe = RhiGraphicsPipeline::new(rhi.device.clone(), &ci, "phong-simple-pipe");

        Self {
            pipeline: simple_pipe,
            bindless_manager,
        }
    }

    pub fn bind(&self, cmd: &RhiCommandBuffer, viewport: &vk::Rect2D, push_constant: &shader::DrawData) {
        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline);
        cmd.cmd_set_viewport(
            0,
            &[vk::Viewport {
                x: viewport.offset.x as f32,
                y: viewport.offset.y as f32 + viewport.extent.height as f32,
                width: viewport.extent.width as f32,
                height: -(viewport.extent.height as f32),
                min_depth: 0.0,
                max_depth: 1.0,
            }],
        );
        cmd.cmd_push_constants(
            self.pipeline.pipeline_layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            bytemuck::bytes_of(push_constant),
        );
        // cmd.bind_descriptor_sets(
        //     vk::PipelineBindPoint::GRAPHICS,
        //     self.pipeline.pipeline_layout,
        //     0,
        //     &[self.bindless_manager.bindless_set.handle],
        //     &[0],
        // );
    }

    pub fn draw(&self, cmd: &RhiCommandBuffer, app_ctx: &mut AppCtx) {
        // cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline_simple.pipeline);

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

        // cmd.cmd_push_constants(
        //     self.pipeline_simple.pipeline_layout,
        //     vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
        //     0,
        //     bytemuck::bytes_of(&self.push),
        // );
        //
        // // scene data
        // cmd.bind_descriptor_sets(
        //     vk::PipelineBindPoint::GRAPHICS,
        //     self.pipeline_simple.pipeline_layout,
        //     0,
        //     &[self.descriptor_sets[frame_id].scene_set.handle],
        //     &[0],
        // );
        //
        // // per mat
        // cmd.bind_descriptor_sets(
        //     vk::PipelineBindPoint::GRAPHICS,
        //     self.pipeline_simple.pipeline_layout,
        //     2,
        //     &[self.descriptor_sets[frame_id].material_set.handle],
        //     // TODO 只使用一个材质
        //     &[0],
        // );

        // index 和 vertex 暂且就用同一个
        // cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&self.cube.vertex_buffer), &[0]);
        // cmd.cmd_bind_index_buffer(&self.cube.index_buffer, 0, vk::IndexType::UINT32);
        //
        // for (mesh_idx, _) in self.mesh_ubo.iter().enumerate() {
        //     cmd.bind_descriptor_sets(
        //         vk::PipelineBindPoint::GRAPHICS,
        //         self.pipeline_simple.pipeline_layout,
        //         1,
        //         &[self.descriptor_sets[frame_id].mesh_set.handle],
        //         &[(self.mesh_ubo_offset_align * mesh_idx as u64) as u32],
        //     );
        //     cmd.draw_indexed(self.cube.index_cnt, 0, 1, 0, 0);
        //     // cmd.cmd_draw(VertexPosNormalUvAoS::shape_box().len() as u32, 1, 0, 0);
        // }
    }
}

struct Simple3DMainPass {
    pipeline: RhiGraphicsPipeline,
    bindless_manager: Rc<BindlessManager>,
}
impl Simple3DMainPass {
    pub fn new(rhi: &Rhi, frame_context: &FrameContext, bindless_manager: Rc<BindlessManager>) -> Self {
        let mut ci = RhiGraphicsPipelineCreateInfo::default();
        ci.vertex_shader_stage("shader/build/phong/phong3d.vs.slang.spv".to_string(), "main".to_string());
        ci.fragment_shader_stage("shader/build/phong/phong.ps.slang.spv".to_string(), "main".to_string());

        ci.vertex_binding(VertexLayoutAos3D::vertex_input_bindings());
        ci.vertex_attribute(VertexLayoutAos3D::vertex_input_attributes());

        ci.push_constant_ranges(vec![vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(size_of::<shader::DrawData>() as u32)]);
        ci.descriptor_set_layouts(vec![bindless_manager.bindless_layout.layout]);
        ci.attach_info(vec![frame_context.color_format()], Some(frame_context.depth_format()), None);
        ci.color_blend_attach_states(vec![vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)]);

        let d3_pipe = RhiGraphicsPipeline::new(rhi.device.clone(), &ci, "phong-d3-pipe");

        Self {
            pipeline: d3_pipe,
            bindless_manager,
        }
    }

    fn bind(&self, cmd: &RhiCommandBuffer, viewport: &vk::Rect2D, push_constant: &shader::DrawData) {
        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline);
        cmd.cmd_set_viewport(
            0,
            &[vk::Viewport {
                x: viewport.offset.x as f32,
                y: viewport.offset.y as f32 + viewport.extent.height as f32,
                width: viewport.extent.width as f32,
                height: -(viewport.extent.height as f32),
                min_depth: 0.0,
                max_depth: 1.0,
            }],
        );
        cmd.cmd_set_scissor(0, &[*viewport]);
        cmd.cmd_push_constants(
            self.pipeline.pipeline_layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            bytemuck::bytes_of(push_constant),
        );

        // TODO 这里多帧暂时使用同一个 descriptor set
        cmd.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline.pipeline_layout,
            0,
            &[self.bindless_manager.bindless_set.handle],
            &[],
        );
    }

    pub fn draw(
        &self,
        cmd: &RhiCommandBuffer,
        app_ctx: &AppCtx,
        push_constant: &shader::DrawData,
        scene_data: &FrameScene,
    ) {
        self.bind(cmd, &app_ctx.render_context.swapchain_extent().into(), push_constant);

        scene_data.draw(cmd, &mut |ins_idx| {
            cmd.cmd_push_constants(
                self.pipeline.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                offset_of!(shader::DrawData, instance_id) as u32,
                bytemuck::bytes_of(&ins_idx),
            );
        });
    }
}

struct PhongApp {
    bindless_mgr: Rc<BindlessManager>,
    frame_data_buffers: Vec<RhiBDABuffer<shader::FrameData>>,
    frame_data_stage_buffers: Vec<RhiStageBuffer<shader::FrameData>>,

    scene_mgr: Rc<SceneManager>,

    main_pass: Simple3DMainPass,
    frame_scene: FrameScene,

    /// BOX
    cube: SimpleMesh,

    camera: Camera,
}

impl PhongApp {}

impl OuterApp for PhongApp {
    fn init(rhi: &Rhi, render_context: &mut FrameContext) -> Self {
        let bindless_mgr = Rc::new(BindlessManager::new(rhi));
        let main_pass = Simple3DMainPass::new(rhi, render_context, bindless_mgr.clone());

        let cube = VertexLayoutAosPosNormalUv::cube(rhi);

        let frame_data_buffers = (0..render_context.frames_cnt)
            .into_iter()
            .map(|idx| RhiBDABuffer::<shader::FrameData>::new_ubo(rhi, format!("frame-data-buffer-{idx}")))
            .collect_vec();
        let frame_data_stage_buffers = (0..render_context.frames_cnt)
            .into_iter()
            .map(|idx| RhiStageBuffer::<shader::FrameData>::new(rhi, format!("frame-data-buffer-{idx}-stage-buffer")))
            .collect_vec();

        let mut scene_mgr = SceneManager::new(bindless_mgr.clone());
        // 复制多个 instance
        let ins_id = scene_mgr.register_model(rhi, std::path::Path::new("assets/obj/spot.obj"), &glam::Mat4::IDENTITY);
        let ins = scene_mgr.instance_map.get(&ins_id[0]).unwrap().clone();
        let ins_1 = SimpleInstance {
            transform: glam::Mat4::from_translation(glam::vec3(5.0, 0.0, 0.0)),
            ..ins.clone()
        };
        scene_mgr.register_instance(ins_1);
        let ins_2 = SimpleInstance {
            transform: glam::Mat4::from_translation(glam::vec3(0.0, 5.0, 0.0)),
            ..ins.clone()
        };
        scene_mgr.register_instance(ins_2);
        scene_mgr.register_point_light(shader::PointLight {
            pos: glam::vec3(-20.0, 40.0, 0.0).into(),
            color: (glam::vec3(5.0, 6.0, 1.0) * 2.0).into(),
            ..Default::default()
        });
        scene_mgr.register_point_light(shader::PointLight {
            pos: glam::vec3(40.0, 40.0, -30.0).into(),
            color: (glam::vec3(1.0, 6.0, 7.0) * 3.0).into(),
            ..Default::default()
        });
        scene_mgr.register_point_light(shader::PointLight {
            pos: glam::vec3(40.0, 40.0, 30.0).into(),
            color: (glam::vec3(5.0, 1.0, 8.0) * 3.0).into(),
            ..Default::default()
        });
        let scene_mgr = Rc::new(scene_mgr);

        let rot =
            glam::Mat4::from_euler(glam::EulerRot::XYZ, 30f32.to_radians(), 40f32.to_radians(), 50f32.to_radians());
        let _mesh_trans = [
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

        let frame_scene = FrameScene::new(scene_mgr.clone());

        Self {
            bindless_mgr,
            frame_data_buffers,
            frame_data_stage_buffers,
            frame_scene,
            scene_mgr,
            main_pass,
            cube,
            camera,
        }
    }

    fn draw_ui(&mut self, ui: &mut Ui) {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }

    fn update(&mut self, app_ctx: &mut AppCtx) {
        let frame_idx = app_ctx.render_context.current_frame_index();

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

        self.frame_scene.prepare_render_data();
        let frame_data_stage_buffer = &mut self.frame_data_stage_buffers[frame_idx];
        frame_data_stage_buffer.transfer(&|data: &mut shader::FrameData| {
            let mouse_pos = app_ctx.input_state.crt_mouse_pos;
            let render_extent = app_ctx.render_context.swapchain_extent();

            data.projection = self.camera.get_projection_matrix().into();
            data.view = self.camera.get_view_matrix().into();
            data.camera_pos = self.camera.position.into();
            data.camera_forward = self.camera.camera_forward().into();
            data.time_ms = app_ctx.timer.duration.as_millis() as f32;
            data.delta_time_ms = app_ctx.timer.delta_time_s * 1000.0;
            data.frame_id = app_ctx.render_context.current_frame_num() as u64;
            data.mouse_pos = shader::float2 {
                x: mouse_pos.x as f32,
                y: mouse_pos.y as f32,
            };
            data.resolution = shader::float2 {
                x: render_extent.width as f32,
                y: render_extent.height as f32,
            };

            self.frame_scene.write_to_buffer(data);
        });

        // 将数据从 stage buffe 传输到 uniform buffer
        let cmd = app_ctx.render_context.alloc_command_buffer("update-draw-buffer");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[update-draw-buffer]stage-to-ubo");
        cmd.cmd_copy_buffer(
            &self.frame_data_stage_buffers[frame_idx],
            &mut self.frame_data_buffers[frame_idx],
            &[vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size: self.frame_data_stage_buffers[frame_idx].size(),
            }],
        );
        cmd.buffer_memory_barrier(
            vk::DependencyFlags::empty(),
            &[RhiBufferBarrier::default()
                .buffer(self.frame_data_buffers[frame_idx].handle(), 0, vk::WHOLE_SIZE)
                .src_mask(vk::PipelineStageFlags2::TRANSFER, vk::AccessFlags2::TRANSFER_WRITE)
                .dst_mask(
                    vk::PipelineStageFlags2::VERTEX_SHADER | vk::PipelineStageFlags2::FRAGMENT_SHADER,
                    vk::AccessFlags2::SHADER_READ,
                )],
        );
        cmd.end();
        app_ctx.render_context.graphics_queue().submit(vec![RhiSubmitInfo::new(std::slice::from_ref(&cmd))], None);
    }

    fn draw(&self, app_ctx: &mut AppCtx) {
        let frame_id = app_ctx.render_context.current_frame_index();

        let color_attach = FrameBuffer::get_color_attachment(app_ctx.render_context.current_present_image_view());
        let depth_attach = FrameBuffer::get_depth_attachment(app_ctx.render_context.depth_view.handle());
        let render_info = FrameBuffer::get_render_info(
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
            cmd.cmd_begin_rendering(&render_info);

            // cmd.begin_label("[phong-pass]simple-draw", LabelColor::COLOR_PASS);
            // self.draw_simple_obj(&cmd, app_ctx);
            // cmd.end_label();

            cmd.begin_label("[phong-pass]3d-draw", LabelColor::COLOR_PASS);
            self.main_pass.draw(
                &cmd,
                &app_ctx,
                &shader::DrawData {
                    frame_data: self.frame_data_buffers[frame_id].device_address(),
                    ..Default::default()
                },
                &self.frame_scene,
            );
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
