use ash::vk;
use imgui::Ui;
use model_manager::component::TruInstance;
use shader_binding::shader;
use std::cell::RefCell;
use std::rc::Rc;
use truvis_render::app::{AppCtx, OuterApp, TruvisApp};
use truvis_render::frame_context::FrameContext;
use truvis_render::render_pass::phong::PhongPass;
use truvis_render::renderer::bindless::BindlessManager;
use truvis_render::renderer::frame_scene::GpuScene;
use truvis_render::renderer::framebuffer::FrameBuffer;
use truvis_render::renderer::scene_manager::TheWorld;
use truvis_rhi::core::buffer::RhiStructuredBuffer;
use truvis_rhi::{core::command_queue::RhiSubmitInfo, rhi::Rhi};

struct PhongApp {
    phong_pass: PhongPass,
}

impl PhongApp {}

impl OuterApp for PhongApp {
    fn init(
        rhi: &Rhi,
        render_context: &mut FrameContext,
        scene_mgr: Rc<RefCell<TheWorld>>,
        bindless_mgr: Rc<RefCell<BindlessManager>>,
    ) -> Self {
        bindless_mgr.borrow_mut().register_texture(rhi, "assets/uv_checker.png".to_string());

        let main_pass = PhongPass::new(rhi, render_context, bindless_mgr.clone());

        let mut scene_mgr = scene_mgr.borrow_mut();
        // 复制多个 instance
        let ins_id = scene_mgr.load_scene(rhi, std::path::Path::new("assets/obj/spot.obj"), &glam::Mat4::IDENTITY);
        let ins = scene_mgr.get_instance(&ins_id[0]).unwrap().clone();
        let ins_1 = TruInstance {
            transform: glam::Mat4::from_translation(glam::vec3(5.0, 0.0, 0.0)),
            ..ins.clone()
        };
        scene_mgr.register_instance(ins_1);
        let ins_2 = TruInstance {
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
        scene_mgr.load_scene(
            rhi,
            std::path::Path::new("assets/fbx/sponza/Sponza.fbx"),
            &glam::Mat4::from_translation(glam::vec3(10.0, 10.0, 10.0)),
        );

        let rot =
            glam::Mat4::from_euler(glam::EulerRot::XYZ, 30f32.to_radians(), 40f32.to_radians(), 50f32.to_radians());
        let _mesh_trans = [
            glam::Mat4::from_translation(glam::vec3(10.0, 0.0, 0.0)) * rot,
            glam::Mat4::from_translation(glam::vec3(0.0, 10.0, 0.0)) * rot,
            glam::Mat4::from_translation(glam::vec3(0.0, 15.0, 10.0)),
            glam::Mat4::from_translation(glam::vec3(0.0, 0.0, 0.0)) * rot,
            glam::Mat4::from_translation(glam::vec3(0.0, -10.0, 0.0)) * rot,
        ];

        Self { phong_pass: main_pass }
    }

    fn draw_ui(&mut self, ui: &mut Ui) {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }

    fn draw(
        &self,
        app_ctx: &mut AppCtx,
        per_frame_data_buffer: &RhiStructuredBuffer<shader::PerFrameData>,
        gpu_scene: &GpuScene,
    ) {
        let crt_frame_label = app_ctx.render_context.current_frame_label();

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
        self.phong_pass.draw(
            &cmd,
            &render_info,
            app_ctx.render_context.swapchain_extent(),
            per_frame_data_buffer,
            gpu_scene,
            crt_frame_label,
        );
        cmd.end();

        app_ctx.rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[cmd])], None);
    }
}

fn main() {
    TruvisApp::<PhongApp>::run();
}
