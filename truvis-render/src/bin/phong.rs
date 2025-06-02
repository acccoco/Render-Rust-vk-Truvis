use ash::vk;
use imgui::Ui;
use model_manager::component::DrsInstance;
use shader_binding::shader;
use truvis_render::app::{OuterApp, TruvisApp};
use truvis_render::platform::camera::DrsCamera;
use truvis_render::platform::timer::Timer;
use truvis_render::render::Renderer;
use truvis_render::render_pass::phong::PhongPass;
use truvis_render::render_pass::simple_rt::SimlpeRtPass;
use truvis_render::renderer::framebuffer::FrameBuffer;
use truvis_rhi::core::command_queue::RhiSubmitInfo;

struct PhongApp {
    phong_pass: PhongPass,
    rt_pass: SimlpeRtPass,
}

impl PhongApp {
    fn create_scene(renderer: &mut Renderer, camera: &mut DrsCamera) {
        camera.position = glam::vec3(270.0, 194.0, -64.0);
        camera.euler_yaw_deg = 90.0;
        camera.euler_pitch_deg = 0.0;

        let mut scene_mgr = renderer.scene_mgr.borrow_mut();

        scene_mgr.register_point_light(shader::PointLight {
            pos: glam::vec3(-20.0, 40.0, 0.0).into(),
            color: (glam::vec3(5.0, 6.0, 1.0) * 2.0).into(),

            _pos_padding: Default::default(),
            _color_padding: Default::default(),
        });
        scene_mgr.register_point_light(shader::PointLight {
            pos: glam::vec3(40.0, 40.0, -30.0).into(),
            color: (glam::vec3(1.0, 6.0, 7.0) * 3.0).into(),

            _pos_padding: Default::default(),
            _color_padding: Default::default(),
        });
        scene_mgr.register_point_light(shader::PointLight {
            pos: glam::vec3(40.0, 40.0, 30.0).into(),
            color: (glam::vec3(5.0, 1.0, 8.0) * 3.0).into(),

            _pos_padding: Default::default(),
            _color_padding: Default::default(),
        });
        // scene_mgr.load_scene(
        //     &renderer.rhi,
        //     std::path::Path::new("assets/fbx/sponza/Sponza.fbx"),
        //     &glam::Mat4::from_translation(glam::vec3(10.0, 10.0, 10.0)),
        // );
        scene_mgr.load_scene(&renderer.rhi, std::path::Path::new("assets/blender/Sponza.fbx"), &glam::Mat4::IDENTITY);
    }
}

impl OuterApp for PhongApp {
    fn init(renderer: &mut Renderer, camera: &mut DrsCamera) -> Self {
        let rt_pass = SimlpeRtPass::new(&renderer.rhi, renderer.bindless_mgr.clone());
        let phong_pass = PhongPass::new(&renderer.rhi, &renderer.frame_settings(), renderer.bindless_mgr.clone());

        Self::create_scene(renderer, camera);

        Self { phong_pass, rt_pass }
    }

    fn draw_ui(&mut self, ui: &mut Ui) {
        // ui.text_wrapped("Hello world!");
        // ui.text_wrapped("こんにちは世界！");
    }

    fn draw(&self, renderer: &mut Renderer, _timer: &Timer) {
        let crt_frame_label = renderer.crt_frame_label();
        let frame_settings = renderer.frame_settings();

        let render_context = renderer.render_context.as_mut().unwrap();
        let swapchian = renderer.render_swapchain.as_mut().unwrap();
        let rhi = &renderer.rhi;

        let color_attach = FrameBuffer::get_color_attachment(swapchian.current_present_image_view());
        let depth_attach = FrameBuffer::get_depth_attachment(render_context.depth_view().handle());
        let render_info = FrameBuffer::get_render_info(
            vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: frame_settings.extent,
            },
            std::slice::from_ref(&color_attach),
            &depth_attach,
        );

        let per_frame_data_buffer = &renderer.per_frame_data_buffers[crt_frame_label];

        if false {
            let phong_cmd = render_context.alloc_command_buffer("[main-pass]render");
            phong_cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[phong-pass]draw");
            self.phong_pass.draw(
                &phong_cmd,
                &render_info,
                frame_settings.extent,
                per_frame_data_buffer,
                &renderer.gpu_scene,
                crt_frame_label,
            );
            phong_cmd.end();
        }

        let rt_cmd = render_context.alloc_command_buffer("[rt-pass]ray-trace");
        rt_cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[rt-pass]ray-trace");
        self.rt_pass.ray_trace(&rt_cmd, render_context, &frame_settings, per_frame_data_buffer, &renderer.gpu_scene);
        rt_cmd.end();

        rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[rt_cmd])], None);
    }
}

fn main() {
    TruvisApp::<PhongApp>::run();
}
