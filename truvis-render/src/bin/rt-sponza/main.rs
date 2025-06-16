use imgui::Ui;
use shader_binding::shader;
use truvis_render::app::TruvisApp;
use truvis_render::outer_app::OuterApp;
use truvis_render::platform::camera::DrsCamera;
use truvis_render::render_pipeline::pipeline_context::PipelineContext;
use truvis_render::render_pipeline::rt_pipeline::RtPipeline;
use truvis_render::renderer::renderer::Renderer;

struct PhongApp {
    rt_pipeline: RtPipeline,
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
        scene_mgr.load_scene(&renderer.rhi, std::path::Path::new("assets/blender/sponza.fbx"), &glam::Mat4::IDENTITY);
    }
}

impl OuterApp for PhongApp {
    fn init(renderer: &mut Renderer, camera: &mut DrsCamera) -> Self {
        let rt_pipeline = RtPipeline::new(&renderer.rhi, renderer.bindless_mgr.clone());

        Self::create_scene(renderer, camera);

        Self { rt_pipeline }
    }

    fn draw_ui(&mut self, _ui: &mut Ui) {}

    fn draw(&self, pipeline_ctx: PipelineContext) {
        self.rt_pipeline.render(pipeline_ctx);
    }
}

fn main() {
    TruvisApp::<PhongApp>::run();
}
