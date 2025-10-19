#[macro_use]
extern crate truvis_crate_tools;

mod triangle_pass;
mod triangle_pipeline;

use imgui::Ui;
use model_manager::{
    component::Geometry,
    vertex::vertex_pc::{VertexAosLayoutPosColor, VertexPosColor},
};
use truvis_render::{
    app::TruvisApp, outer_app::OuterApp, platform::camera::Camera,
    render_pipeline::pipeline_context::PipelineContext, renderer::renderer::Renderer,
};

use crate::triangle_pipeline::TrianglePipeline;

struct HelloTriangle {
    triangle_pipeline: TrianglePipeline,
    triangle: Geometry<VertexPosColor>,
}
impl OuterApp for HelloTriangle {
    fn init(renderer: &mut Renderer, _camera: &mut Camera) -> Self {
        log::info!("hello triangle init.");

        Self {
            triangle_pipeline: TrianglePipeline::new(&renderer.frame_settings()),
            triangle: VertexAosLayoutPosColor::triangle(),
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
