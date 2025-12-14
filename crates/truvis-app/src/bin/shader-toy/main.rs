use imgui::Ui;
use truvis_app::app::TruvisApp;
use truvis_app::outer_app::OuterApp;
use truvis_model_manager::components::geometry::RtGeometry;
use truvis_render::core::frame_context::FrameContext;
use truvis_render::platform::camera::Camera;

mod shader_toy_pass;

use shader_toy_pass::ShaderToyPass;
use truvis_model_manager::shapes::rect::RectSoA;
use truvis_render::core::renderer::{RenderContext, RenderContextMut, Renderer};

struct ShaderToy {
    rectangle: RtGeometry,
    pipeline: ShaderToyPass,
}
impl OuterApp for ShaderToy {
    fn init(_renderer: &mut Renderer, _camera: &mut Camera) -> Self {
        log::info!("shader toy.");
        Self {
            pipeline: ShaderToyPass::new(FrameContext::get().frame_settings().color_format),
            rectangle: RectSoA::create_mesh(),
        }
    }

    fn draw_ui(&mut self, ui: &Ui) {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }

    fn draw(&self, render_context: &RenderContext, render_context_mut: &mut RenderContextMut) {
        self.pipeline.render(render_context, render_context_mut, &self.rectangle);
    }
}

fn main() {
    TruvisApp::<ShaderToy>::run();
}
