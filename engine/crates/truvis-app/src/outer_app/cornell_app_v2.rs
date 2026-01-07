//! Cornell Box 使用 RenderGraph V2 的渲染应用
//!
//! 这是 Cornell Box 示例的 RenderGraph V2 版本，演示如何使用
//! 声明式的 RenderGraph 来组织 RT 渲染管线。

use crate::outer_app::OuterApp;
use imgui::Ui;
use truvis_crate_tools::resource::TruvisPath;
use truvis_render_graph::render_context::RenderContext;
use truvis_render_graph::render_graph_v2::RgPassContext;
use crate::render_pipeline::rt_render_graph::RtPipeline;
use truvis_renderer::model_loader::assimp_loader::AssimpSceneLoader;
use truvis_renderer::platform::camera::Camera;
use truvis_renderer::renderer::Renderer;
use truvis_shader_binding::truvisl;

/// 使用 RenderGraph V2 的 Cornell Box 应用
#[derive(Default)]
pub struct CornellAppV2 {
    rt_pipeline: Option<RtPipeline>,
}

impl CornellAppV2 {
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

        log::info!("Loading scene...");
        AssimpSceneLoader::load_scene(
            TruvisPath::assets_path_str("fbx/cube-coord.fbx").as_ref(),
            &mut renderer.render_context.scene_manager,
            &mut renderer.render_context.asset_hub,
        );
        log::info!("Scene loaded.");
    }
}

impl OuterApp for CornellAppV2 {
    fn init(&mut self, renderer: &mut Renderer, camera: &mut Camera) {
        let rt_pipeline =
            RtPipeline::new(&renderer.render_context.global_descriptor_sets, &mut renderer.cmd_allocator);

        Self::create_scene(renderer, camera);

        self.rt_pipeline = Some(rt_pipeline);
    }

    fn draw_ui(&mut self, _ui: &Ui) {}

    fn draw(&self, render_context: &RenderContext) {
        // 方式 1: 直接渲染（不含 UI）
        // self.rt_pipeline.as_ref().unwrap().render(render_context);

        // 方式 2: 使用闭包添加 UI Pass
        // 注意：这里演示如何通过闘包注入 UI 绘制逻辑
        // 实际使用时需要在 Renderer 层面集成 gui_pass
        self.rt_pipeline.as_ref().unwrap().render_with_ui_lambda(
            render_context,
            None::<fn(&RgPassContext<'_>)>, // 暂不添加 UI
        );

        // 方式 3: 使用 build_graph 手动构建并扩展
        // let (mut builder, render_target) = self.rt_pipeline.as_ref().unwrap().build_graph(render_context);
        // // 添加自定义 pass...
        // builder.add_pass_lambda("custom-pass", |b| {
        //     b.read_image(render_target, RgImageState::SHADER_READ_FRAGMENT);
        // }, |ctx| {
        //     // 自定义渲染逻辑
        // });
        // let compiled = builder.compile();
        // self.rt_pipeline.as_ref().unwrap().execute_graph(render_context, &compiled);
    }
}
