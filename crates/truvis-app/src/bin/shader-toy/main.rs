use imgui::Ui;
use truvis_app::app::TruvisApp;
use truvis_app::outer_app::OuterApp;
use truvis_model::components::geometry::RtGeometry;
use truvis_render::platform::camera::Camera;

mod shader_toy_pass;

use shader_toy_pass::ShaderToyPass;
use truvis_model::shapes::rect::RectSoA;
use truvis_render::core::renderer::Renderer;
use truvis_render_graph::render_context::RenderContext;

struct ShaderToy {
    rectangle: RtGeometry,
    pipeline: ShaderToyPass,
}
impl OuterApp for ShaderToy {
    fn init(renderer: &mut Renderer, _camera: &mut Camera) -> Self {
        log::info!("shader toy.");
        Self {
            pipeline: ShaderToyPass::new(
                renderer.render_context.frame_settings.color_format,
                &mut renderer.cmd_allocator,
            ),
            rectangle: RectSoA::create_mesh(),
        }
    }

    fn draw_ui(&mut self, ui: &Ui) {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }

    fn draw(&self, render_context: &RenderContext) {
        self.pipeline.render(render_context, &self.rectangle);
    }
}

fn main() {
    TruvisApp::<ShaderToy>::run();
}
