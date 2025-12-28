use crate::outer_app::OuterApp;
use crate::outer_app::shader_toy::shader_toy_pass::ShaderToyPass;
use imgui::Ui;
use truvis_render_graph::render_context::RenderContext;
use truvis_render_interface::geometry::RtGeometry;
use truvis_renderer::platform::camera::Camera;
use truvis_renderer::renderer::Renderer;
use truvis_scene::shapes::rect::RectSoA;

#[derive(Default)]
pub struct ShaderToy {
    rectangle: Option<RtGeometry>,
    pipeline: Option<ShaderToyPass>,
}
impl OuterApp for ShaderToy {
    fn init(&mut self, renderer: &mut Renderer, _camera: &mut Camera) {
        log::info!("shader toy.");

        self.pipeline =
            Some(ShaderToyPass::new(renderer.render_context.frame_settings.color_format, &mut renderer.cmd_allocator));
        self.rectangle = Some(RectSoA::create_mesh());
    }

    fn draw_ui(&mut self, ui: &Ui) {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }

    fn draw(&self, render_context: &RenderContext) {
        self.pipeline.as_ref().unwrap().render(render_context, self.rectangle.as_ref().unwrap());
    }
}
