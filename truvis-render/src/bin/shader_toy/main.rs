mod shader_toy_pass;
mod shader_toy_pipeline;

use crate::shader_toy_pipeline::ShaderToyPipeline;
use imgui::Ui;
use model_manager::component::DrsGeometry;
use model_manager::vertex::vertex_pc::{VertexAosLayoutPosColor, VertexPosColor};
use truvis_render::app::TruvisApp;
use truvis_render::outer_app::OuterApp;
use truvis_render::platform::camera::DrsCamera;
use truvis_render::render_pipeline::pipeline_context::PipelineContext;
use truvis_render::renderer::renderer::Renderer;

struct ShaderToy {
    rectangle: DrsGeometry<VertexPosColor>,
    pipeline: ShaderToyPipeline,
}
impl OuterApp for ShaderToy {
    fn init(renderer: &mut Renderer, _camera: &mut DrsCamera) -> Self {
        log::info!("shader toy.");
        Self {
            rectangle: VertexAosLayoutPosColor::rectangle(&renderer.rhi),
            pipeline: ShaderToyPipeline::new(&renderer.rhi, &renderer.frame_settings()),
        }
    }

    fn draw_ui(&mut self, ui: &mut Ui) {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }

    fn draw(&self, pipeline_ctx: PipelineContext) {
        self.pipeline.render(pipeline_ctx, &self.rectangle);
    }
}

fn main() {
    TruvisApp::<ShaderToy>::run();
}
