use imgui::Ui;
use truvis_app::app::TruvisApp;
use truvis_app::outer_app::OuterApp;
use truvis_model_manager::components::geometry::Geometry;
use truvis_model_manager::vertex::aos_pos_color::VertexLayoutAoSPosColor;
use truvis_render::renderer::frame_context::FrameContext;
use truvis_render::{platform::camera::Camera, renderer::renderer::Renderer};

mod shader_toy_pass;
mod shader_toy_pipeline;

use crate::shader_toy_pipeline::ShaderToyPipeline;

struct ShaderToy {
    rectangle: Geometry<VertexLayoutAoSPosColor>,
    pipeline: ShaderToyPipeline,
}
impl OuterApp for ShaderToy {
    fn init(_renderer: &mut Renderer, _camera: &mut Camera) -> Self {
        log::info!("shader toy.");
        Self {
            rectangle: VertexLayoutAoSPosColor::rectangle(),
            pipeline: ShaderToyPipeline::new(FrameContext::get().frame_settings().color_format),
        }
    }

    fn draw_ui(&mut self, ui: &Ui) {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }

    fn draw(&self) {
        self.pipeline.render(&self.rectangle);
    }
}

fn main() {
    TruvisApp::<ShaderToy>::run();
}
