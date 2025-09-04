#[macro_use]
extern crate truvis_crate_tools;

mod shader_toy_pass;
mod shader_toy_pipeline;

use imgui::Ui;
use model_manager::{
    component::DrsGeometry,
    vertex::vertex_pc::{VertexAosLayoutPosColor, VertexPosColor},
};
use truvis_render::{
    app::TruvisApp, outer_app::OuterApp, platform::camera::DrsCamera,
    render_pipeline::pipeline_context::PipelineContext, renderer::renderer::Renderer,
};

use crate::shader_toy_pipeline::ShaderToyPipeline;

struct ShaderToy {
    rectangle: DrsGeometry<VertexPosColor>,
    pipeline: ShaderToyPipeline,
}
impl OuterApp for ShaderToy {
    fn init(renderer: &mut Renderer, _camera: &mut DrsCamera) -> Self {
        log::info!("shader toy.");
        Self {
            rectangle: VertexAosLayoutPosColor::rectangle(),
            pipeline: ShaderToyPipeline::new(renderer.frame_settings().color_format),
        }
    }

    fn draw_ui(&mut self, ui: &Ui) {
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
