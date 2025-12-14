use imgui::Ui;
use truvis_app::app::TruvisApp;
use truvis_app::outer_app::OuterApp;
use truvis_render::core::renderer::{FrameContext2, FrameContext3, Renderer};
use truvis_render::{platform::camera::Camera, render_pipeline::rt_pass::RtRenderPass};
use truvis_shader_binding::truvisl;

struct SponzaApp {
    rt_pipeline: RtRenderPass,
}

impl SponzaApp {
    fn create_scene(renderer: &mut Renderer, camera: &mut Camera) {
        camera.position = glam::vec3(270.0, 194.0, -64.0);
        camera.euler_yaw_deg = 90.0;
        camera.euler_pitch_deg = 0.0;

        renderer.frame_context2.scene_manager.register_point_light(truvisl::PointLight {
            pos: glam::vec3(-20.0, 40.0, 0.0).into(),
            color: (glam::vec3(5.0, 6.0, 1.0) * 2.0).into(),

            _pos_padding: Default::default(),
            _color_padding: Default::default(),
        });
        renderer.frame_context2.scene_manager.register_point_light(truvisl::PointLight {
            pos: glam::vec3(40.0, 40.0, -30.0).into(),
            color: (glam::vec3(1.0, 6.0, 7.0) * 3.0).into(),

            _pos_padding: Default::default(),
            _color_padding: Default::default(),
        });
        renderer.frame_context2.scene_manager.register_point_light(truvisl::PointLight {
            pos: glam::vec3(40.0, 40.0, 30.0).into(),
            color: (glam::vec3(5.0, 1.0, 8.0) * 3.0).into(),

            _pos_padding: Default::default(),
            _color_padding: Default::default(),
        });
        // scene_manager.load_scene(
        //     &renderer.render_context,
        //     std::path::Path::new("assets/fbx/sponza/Sponza.fbx"),
        //     &glam::Mat4::from_translation(glam::vec3(10.0, 10.0, 10.0)),
        // );
        renderer.frame_context2.scene_manager.load_scene(
            &mut renderer.frame_context2.gfx_resource_manager,
            &mut renderer.frame_context2.bindless_manager,
            std::path::Path::new("assets/blender/sponza.fbx"),
            &glam::Mat4::IDENTITY,
        );
    }
}

impl OuterApp for SponzaApp {
    fn init(renderer: &mut Renderer, camera: &mut Camera) -> Self {
        let rt_pipeline = RtRenderPass::new(&renderer.frame_context2.bindless_manager);

        Self::create_scene(renderer, camera);

        Self { rt_pipeline }
    }

    fn draw_ui(&mut self, _ui: &Ui) {}

    fn draw(&self, frame_context2: &FrameContext2, frame_context3: &mut FrameContext3) {
        self.rt_pipeline.render(frame_context2, frame_context3);
    }
}

fn main() {
    TruvisApp::<SponzaApp>::run();
}
