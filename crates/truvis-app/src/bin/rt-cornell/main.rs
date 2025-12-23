use imgui::Ui;
use truvis_app::app::TruvisApp;
use truvis_app::outer_app::OuterApp;
use truvis_crate_tools::resource::TruvisPath;
use truvis_render::core::renderer::Renderer;
use truvis_render::model_loader::assimp_loader::AssimpSceneLoader;
use truvis_render::platform::camera::Camera;
use truvis_render_graph::render_context::RenderContext;
use truvis_render_graph::render_pipeline::rt_pass::RtRenderPass;
use truvis_shader_binding::truvisl;

struct CornellApp {
    rt_pipeline: RtRenderPass,
}

impl CornellApp {
    fn create_scene(renderer: &mut Renderer, camera: &mut Camera) {
        camera.position = glam::vec3(-400.0, 1000.0, 1000.0);
        camera.euler_yaw_deg = 330.0;
        camera.euler_pitch_deg = -27.0;

        renderer.render_context.scene_manager.register_point_light(truvisl::PointLight {
            pos: glam::vec3(-20.0, 40.0, 0.0).into(),
            color: (glam::vec3(5.0, 6.0, 1.0) * 2.0).into(),

            _pos_padding: Default::default(),
            _color_padding: Default::default(),
        });
        renderer.render_context.scene_manager.register_point_light(truvisl::PointLight {
            pos: glam::vec3(40.0, 40.0, -30.0).into(),
            color: (glam::vec3(1.0, 6.0, 7.0) * 3.0).into(),

            _pos_padding: Default::default(),
            _color_padding: Default::default(),
        });
        renderer.render_context.scene_manager.register_point_light(truvisl::PointLight {
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
        AssimpSceneLoader::load_scene(
            TruvisPath::assets_path("fbx/cube-coord.fbx").as_ref(),
            &mut renderer.render_context.scene_manager,
            &mut renderer.asset_hub,
        );
        log::info!("Scene loaded.");
    }
}

impl OuterApp for CornellApp {
    fn init(renderer: &mut Renderer, camera: &mut Camera) -> Self {
        let rt_pipeline =
            RtRenderPass::new(&renderer.render_context.global_descriptor_sets, &mut renderer.cmd_allocator);

        Self::create_scene(renderer, camera);

        Self { rt_pipeline }
    }

    fn draw_ui(&mut self, _ui: &Ui) {}

    fn draw(&self, render_context: &RenderContext) {
        self.rt_pipeline.render(render_context);
    }
}

fn main() {
    TruvisApp::<CornellApp>::run();
}
