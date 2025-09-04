#[macro_use]
extern crate truvis_crate_tools;

mod triangle_pass;
mod triangle_pipeline;

use imgui::Ui;
use model_manager::{
    component::DrsGeometry,
    vertex::vertex_pc::{VertexAosLayoutPosColor, VertexPosColor},
};
use truvis_render::{
    app::TruvisApp, outer_app::OuterApp, platform::camera::DrsCamera,
    render_pipeline::pipeline_context::PipelineContext, renderer::renderer::Renderer,
};
use truvis_rhi::render_context::RenderContext;

use crate::triangle_pipeline::TrianglePipeline;

struct HelloTriangle {
    triangle_pipeline: TrianglePipeline,
    triangle: DrsGeometry<VertexPosColor>,
}
impl OuterApp for HelloTriangle {
    fn init(renderer: &mut Renderer, _camera: &mut DrsCamera) -> Self {
        log::info!("hello triangle init.");

        Self {
            triangle_pipeline: TrianglePipeline::new(&renderer.frame_settings()),
            triangle: VertexAosLayoutPosColor::triangle(RenderContext::get()),
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
