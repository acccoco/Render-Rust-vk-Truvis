use imgui::Ui;
use truvis_model_manager::components::geometry::Geometry;
use truvis_model_manager::vertex::aos_pos_color::VertexLayoutAoSPosColor;
use truvis_render::{
    app::TruvisApp, outer_app::OuterApp, platform::camera::Camera, render_pipeline::pipeline_context::PipelineContext,
    renderer::renderer::Renderer,
};

mod shader_toy_pass;
mod shader_toy_pipeline;

use crate::shader_toy_pipeline::ShaderToyPipeline;

struct ShaderToy {
    rectangle: Geometry<VertexLayoutAoSPosColor>,
    pipeline: ShaderToyPipeline,
}
impl OuterApp for ShaderToy {
    fn init(renderer: &mut Renderer, _camera: &mut Camera) -> Self {
        log::info!("shader toy.");
        Self {
            rectangle: VertexLayoutAoSPosColor::rectangle(),
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
