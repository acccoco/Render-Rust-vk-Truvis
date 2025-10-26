use imgui::Ui;

use truvis_model_manager::components::geometry::Geometry;
use truvis_model_manager::vertex::aos_pos_color::VertexLayoutAoSPosColor;
use truvis_render::{
    app::TruvisApp, outer_app::OuterApp, platform::camera::Camera, render_pipeline::pipeline_context::PipelineContext,
    renderer::renderer::Renderer,
};

mod triangle_pass;
mod triangle_pipeline;

use crate::triangle_pipeline::TrianglePipeline;

struct HelloTriangle {
    triangle_pipeline: TrianglePipeline,
    triangle: Geometry<VertexLayoutAoSPosColor>,
}
impl OuterApp for HelloTriangle {
    fn init(renderer: &mut Renderer, _camera: &mut Camera) -> Self {
        log::info!("hello triangle init.");

        Self {
            triangle_pipeline: TrianglePipeline::new(&renderer.frame_settings()),
            triangle: VertexLayoutAoSPosColor::triangle(),
        }
    }

    fn draw_ui(&mut self, _ui: &Ui) {
        static mut _UI_VALUE: usize = 0;
    }

    fn draw(&self, pipeline_ctx: PipelineContext) {
        self.triangle_pipeline.render(pipeline_ctx, &self.triangle);
    }
}

fn main() {
    TruvisApp::<HelloTriangle>::run();
}
