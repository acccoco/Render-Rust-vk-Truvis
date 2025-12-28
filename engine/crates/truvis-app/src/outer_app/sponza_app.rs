use crate::outer_app::OuterApp;
use imgui::Ui;
use truvis_crate_tools::resource::TruvisPath;
use truvis_render_graph::render_context::RenderContext;
use truvis_render_graph::render_pipeline::rt_pass::RtRenderPass;
use truvis_renderer::model_loader::assimp_loader::AssimpSceneLoader;
use truvis_renderer::platform::camera::Camera;
use truvis_renderer::renderer::Renderer;
use truvis_shader_binding::truvisl;

#[derive(Default)]
pub struct SponzaApp {
    rt_pipeline: Option<RtRenderPass>,
}

impl SponzaApp {
    fn create_scene(renderer: &mut Renderer, camera: &mut Camera) {
        camera.position = glam::vec3(270.0, 194.0, -64.0);
        camera.euler_yaw_deg = 90.0;
        camera.euler_pitch_deg = 0.0;

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
        log::info!("start load sponza scene");
        AssimpSceneLoader::load_scene(
            &TruvisPath::assets_path("blender/sponza.fbx"),
            &mut renderer.render_context.scene_manager,
            &mut renderer.render_context.asset_hub,
        );
        log::info!("finished load sponza scene");
    }
}

impl OuterApp for SponzaApp {
    fn init(&mut self, renderer: &mut Renderer, camera: &mut Camera) {
        let rt_pipeline =
            RtRenderPass::new(&renderer.render_context.global_descriptor_sets, &mut renderer.cmd_allocator);

        Self::create_scene(renderer, camera);

        self.rt_pipeline = Some(rt_pipeline);
    }

    fn draw_ui(&mut self, _ui: &Ui) {}

    fn draw(&self, render_context: &RenderContext) {
        self.rt_pipeline.as_ref().unwrap().render(render_context);
    }
}
