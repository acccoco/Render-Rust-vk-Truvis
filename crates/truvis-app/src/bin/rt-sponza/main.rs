use imgui::Ui;
use truvis_app::app::TruvisApp;
use truvis_app::outer_app::OuterApp;
use truvis_render::renderer::frame_context::FrameContext;
use truvis_render::{platform::camera::Camera, render_pipeline::rt_pipeline::RtPipeline, renderer::renderer::Renderer};
use truvis_shader_binding::shader;

struct RtApp {
    rt_pipeline: RtPipeline,
}

impl RtApp {
    fn create_scene(_renderer: &mut Renderer, camera: &mut Camera) {
        camera.position = glam::vec3(270.0, 194.0, -64.0);
        camera.euler_yaw_deg = 90.0;
        camera.euler_pitch_deg = 0.0;

        let mut scene_mgr = FrameContext::scene_manager_mut();

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
        //     &renderer.render_context,
        //     std::path::Path::new("assets/fbx/sponza/Sponza.fbx"),
        //     &glam::Mat4::from_translation(glam::vec3(10.0, 10.0, 10.0)),
        // );
        scene_mgr.load_scene(std::path::Path::new("assets/blender/sponza.fbx"), &glam::Mat4::IDENTITY);
    }
}

impl OuterApp for RtApp {
    fn init(renderer: &mut Renderer, camera: &mut Camera) -> Self {
        let rt_pipeline = RtPipeline::new();

        Self::create_scene(renderer, camera);

        Self { rt_pipeline }
    }

    fn draw_ui(&mut self, _ui: &Ui) {}

    fn draw(&self) {
        self.rt_pipeline.render();
    }
}

fn main() {
    TruvisApp::<RtApp>::run();
}
