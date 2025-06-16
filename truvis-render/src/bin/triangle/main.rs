mod triangle_pass;
mod triangle_pipeline;

use crate::triangle_pipeline::TrianglePipeline;
use imgui::Ui;
use model_manager::component::DrsGeometry;
use model_manager::vertex::vertex_pc::{VertexAosLayoutPosColor, VertexPosColor};
use truvis_render::app::TruvisApp;
use truvis_render::outer_app::OuterApp;
use truvis_render::platform::camera::DrsCamera;
use truvis_render::render_pipeline::pipeline_context::PipelineContext;
use truvis_render::renderer::renderer::Renderer;

struct HelloTriangle {
    triangle_pipeline: TrianglePipeline,
    triangle: DrsGeometry<VertexPosColor>,
}
impl OuterApp for HelloTriangle {
    fn init(renderer: &mut Renderer, _camera: &mut DrsCamera) -> Self {
        log::info!("hello triangle init.");

        Self {
            triangle_pipeline: TrianglePipeline::new(&renderer.rhi, &renderer.frame_settings()),
            triangle: VertexAosLayoutPosColor::triangle(&renderer.rhi),
        }
    }

    fn draw_ui(&mut self, _ui: &mut Ui) {
        static mut _UI_VALUE: usize = 0;
    }

    fn draw(&self, pipeline_ctx: PipelineContext) {
        self.triangle_pipeline.render(pipeline_ctx, &self.triangle);
    }
}

fn main() {
    TruvisApp::<HelloTriangle>::run();
}
