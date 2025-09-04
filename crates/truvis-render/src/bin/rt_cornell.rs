use imgui::Ui;
use shader_binding::shader;
use truvis_render::{
    app::TruvisApp, outer_app::OuterApp, platform::camera::DrsCamera,
    render_pipeline::pipeline_context::PipelineContext, render_pipeline::rt_pipeline::RtPipeline,
    renderer::renderer::Renderer,
};

struct PhongApp {
    rt_pipeline: RtPipeline,
}

impl PhongApp {
    fn create_scene(renderer: &mut Renderer, camera: &mut DrsCamera) {
        camera.position = glam::vec3(-400.0, 1000.0, 1000.0);
        camera.euler_yaw_deg = 330.0;
        camera.euler_pitch_deg = -27.0;

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
        //     &renderer.render_context,
        //     std::path::Path::new("assets/fbx/sponza/Sponza.fbx"),
        //     &glam::Mat4::from_translation(glam::vec3(10.0, 10.0, 10.0)),
        // );
        scene_mgr.load_scene(std::path::Path::new("assets/blender/coord.fbx"), &glam::Mat4::IDENTITY);
    }
}

impl OuterApp for PhongApp {
    fn init(renderer: &mut Renderer, camera: &mut DrsCamera) -> Self {
        let rt_pipeline = RtPipeline::new(renderer.bindless_mgr.clone());

        Self::create_scene(renderer, camera);

        Self { rt_pipeline }
    }

    fn draw_ui(&mut self, _ui: &Ui) {}

    fn draw(&self, pipeline_ctx: PipelineContext) {
        self.rt_pipeline.render(pipeline_ctx);
    }
}

fn main() {
    TruvisApp::<PhongApp>::run();
}
