use imgui::Ui;
use truvis_app::app::TruvisApp;
use truvis_app::outer_app::OuterApp;
use truvis_render::core::frame_context::FrameContext;
use truvis_render::core::renderer::Renderer;
use truvis_render::{platform::camera::Camera, render_pipeline::rt_pass::RtRenderPass};
use truvis_shader_binding::truvisl;

struct CornellApp {
    rt_pipeline: RtRenderPass,
}

impl CornellApp {
    fn create_scene(_renderer: &mut Renderer, camera: &mut Camera) {
        camera.position = glam::vec3(-400.0, 1000.0, 1000.0);
        camera.euler_yaw_deg = 330.0;
        camera.euler_pitch_deg = -27.0;

        let mut scene_manager = FrameContext::scene_manager_mut();

        scene_manager.register_point_light(truvisl::PointLight {
            pos: glam::vec3(-20.0, 40.0, 0.0).into(),
            color: (glam::vec3(5.0, 6.0, 1.0) * 2.0).into(),

            _pos_padding: Default::default(),
            _color_padding: Default::default(),
        });
        scene_manager.register_point_light(truvisl::PointLight {
            pos: glam::vec3(40.0, 40.0, -30.0).into(),
            color: (glam::vec3(1.0, 6.0, 7.0) * 3.0).into(),

            _pos_padding: Default::default(),
            _color_padding: Default::default(),
        });
        scene_manager.register_point_light(truvisl::PointLight {
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
        log::info!("Loading scene...");
        scene_manager.load_scene(std::path::Path::new("assets/blender/coord.fbx"), &glam::Mat4::IDENTITY);
        log::info!("Scene loaded.");
    }
}

impl OuterApp for CornellApp {
    fn init(renderer: &mut Renderer, camera: &mut Camera) -> Self {
        let rt_pipeline = RtRenderPass::new();

        Self::create_scene(renderer, camera);

        Self { rt_pipeline }
    }

    fn draw_ui(&mut self, _ui: &Ui) {}

    fn draw(&self) {
        self.rt_pipeline.render();
    }
}

fn main() {
    TruvisApp::<CornellApp>::run();
}
