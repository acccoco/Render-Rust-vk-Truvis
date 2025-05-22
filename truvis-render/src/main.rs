use ash::vk;
use imgui::Ui;
use itertools::Itertools;
use model_manager::component::instance::SimpleInstance;
use model_manager::component::mesh::SimpleMesh;
use model_manager::vertex::vertex_pnu::VertexLayoutAosPosNormalUv;
use shader_binding::shader;
use std::cell::RefCell;
use std::rc::Rc;
use truvis_render::app::{AppCtx, OuterApp, TruvisApp};
use truvis_render::frame_context::FrameContext;
use truvis_render::platform::camera_controller::CameraController;
use truvis_render::render_pass::phong::Simple3DMainPass;
use truvis_render::renderer::bindless::BindlessManager;
use truvis_render::renderer::frame_scene::GpuScene;
use truvis_render::renderer::framebuffer::FrameBuffer;
use truvis_render::renderer::scene_manager::SceneManager;
use truvis_rhi::core::buffer::{RhiBDABuffer, RhiStageBuffer};
use truvis_rhi::core::synchronize::RhiBufferBarrier;
use truvis_rhi::{basic::color::LabelColor, core::command_queue::RhiSubmitInfo, rhi::Rhi};

struct PhongApp {
    _bindless_mgr: Rc<RefCell<BindlessManager>>,
    _scene_mgr: Rc<RefCell<SceneManager>>,

    frame_data_buffers: Vec<RhiBDABuffer<shader::FrameData>>,
    frame_data_stage_buffers: Vec<RhiStageBuffer<shader::FrameData>>,

    main_pass: Simple3DMainPass,
    frame_scene: GpuScene,

    /// BOX
    _cube: SimpleMesh,

    // 保存共享的相机控制器
    camera_controller: Rc<RefCell<CameraController>>,
}

impl PhongApp {}

impl OuterApp for PhongApp {
    fn init(rhi: &Rhi, render_context: &mut FrameContext, camera_controller: Rc<RefCell<CameraController>>) -> Self {
        let bindless_mgr = Rc::new(RefCell::new(BindlessManager::new(rhi, render_context.frame_cnt_in_flight)));
        bindless_mgr.borrow_mut().register_texture(rhi, "assets/uv_checker.png".to_string());

        let main_pass = Simple3DMainPass::new(rhi, render_context, bindless_mgr.clone());

        let cube = VertexLayoutAosPosNormalUv::cube(rhi);

        let frame_data_buffers = (0..render_context.frame_cnt_in_flight)
            .into_iter()
            .map(|idx| RhiBDABuffer::<shader::FrameData>::new_ubo(rhi, format!("frame-data-buffer-{idx}")))
            .collect_vec();
        let frame_data_stage_buffers = (0..render_context.frame_cnt_in_flight)
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
        scene_mgr.register_model(
            rhi,
            std::path::Path::new("assets/fbx/sponza/Sponza.fbx"),
            &glam::Mat4::from_translation(glam::vec3(10.0, 10.0, 10.0)),
        );

        let scene_mgr = Rc::new(RefCell::new(scene_mgr));

        let rot =
            glam::Mat4::from_euler(glam::EulerRot::XYZ, 30f32.to_radians(), 40f32.to_radians(), 50f32.to_radians());
        let _mesh_trans = [
            glam::Mat4::from_translation(glam::vec3(10.0, 0.0, 0.0)) * rot,
            glam::Mat4::from_translation(glam::vec3(0.0, 10.0, 0.0)) * rot,
            glam::Mat4::from_translation(glam::vec3(0.0, 15.0, 10.0)),
            glam::Mat4::from_translation(glam::vec3(0.0, 0.0, 0.0)) * rot,
            glam::Mat4::from_translation(glam::vec3(0.0, -10.0, 0.0)) * rot,
        ];

        let frame_scene = GpuScene::new(scene_mgr.clone(), bindless_mgr.clone());

        // 更新相机的初始状态
        {
            let mut camera_controller = camera_controller.borrow_mut();
            let camera = camera_controller.camera_mut();
            camera.position = glam::vec3(20.0, 0.0, 0.0);
            camera.euler_yaw_deg = 90.0;
        }

        Self {
            _bindless_mgr: bindless_mgr,
            frame_data_buffers,
            frame_data_stage_buffers,
            frame_scene,
            _scene_mgr: scene_mgr,
            main_pass,
            _cube: cube,
            camera_controller,
        }
    }

    fn draw_ui(&mut self, ui: &mut Ui) {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }
    fn update(&mut self, app_ctx: &mut AppCtx) {
        let frame_idx = app_ctx.render_context.current_frame_label();

        // 直接使用 TruvisApp 中的 camera_controller，无需再创建新的实例

        // 将场景数据写入到帧缓冲区
        self.frame_scene.prepare_render_data(app_ctx.render_context.current_frame_label());
        let frame_data_stage_buffer = &mut self.frame_data_stage_buffers[frame_idx];
        frame_data_stage_buffer.transfer(&|data: &mut shader::FrameData| {
            let mouse_pos = app_ctx.input_state.crt_mouse_pos;
            let extent = app_ctx.render_context.swapchain_extent();

            // 从共享的相机控制器获取相机数据
            let camera_controller = self.camera_controller.borrow();
            let camera = camera_controller.camera();
            data.projection = camera.get_projection_matrix().into();
            data.view = camera.get_view_matrix().into();
            data.camera_pos = camera.position.into();
            data.camera_forward = camera.camera_forward().into();
            data.time_ms = app_ctx.timer.duration.as_millis() as f32;
            data.delta_time_ms = app_ctx.timer.delta_time_s * 1000.0;
            data.frame_id = app_ctx.render_context.current_frame_num() as u64;
            data.mouse_pos = shader::Float2 {
                x: mouse_pos.x as f32,
                y: mouse_pos.y as f32,
            };
            data.resolution = shader::Float2 {
                x: extent.width as f32,
                y: extent.height as f32,
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
        let frame_id = app_ctx.render_context.current_frame_label();

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
                app_ctx.render_context.current_frame_label(),
            );
            cmd.end_label();

            cmd.end_rendering();
        }
        cmd.end();

        app_ctx.rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[cmd])], None);
    }
    fn rebuild(&mut self, _rhi: &Rhi, render_context: &mut FrameContext) {
        // 更新相机的宽高比
        let mut controller = self.camera_controller.borrow_mut();
        controller.camera_mut().asp =
            render_context.swapchain_extent().width as f32 / render_context.swapchain_extent().height as f32;
    }
}

fn main() {
    TruvisApp::<PhongApp>::run();
}
