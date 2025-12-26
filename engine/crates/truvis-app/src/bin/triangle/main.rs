use imgui::Ui;
use truvis_app::app::WinitApp;
use truvis_app::outer_app::OuterApp;
use truvis_model::components::geometry::RtGeometry;
use truvis_render_core::platform::camera::Camera;

mod triangle_pass;

use triangle_pass::TrianglePass;
use truvis_model::shapes::triangle::TriangleSoA;
use truvis_render_core::core::renderer::Renderer;
use truvis_render_graph::render_context::RenderContext;

#[derive(Default)]
struct HelloTriangle {
    triangle_pipeline: Option<TrianglePass>,
    triangle: Option<RtGeometry>,
}
impl OuterApp for HelloTriangle {
    fn init(&mut self, renderer: &mut Renderer, _camera: &mut Camera) {
        log::info!("hello triangle init.");

        self.triangle_pipeline =
            Some(TrianglePass::new(&renderer.render_context.frame_settings, &mut renderer.cmd_allocator));
        self.triangle = Some(TriangleSoA::create_mesh());
    }

    fn draw_ui(&mut self, _ui: &Ui) {
        static mut _UI_VALUE: usize = 0;
    }

    fn draw(&self, render_context: &RenderContext) {
        self.triangle_pipeline.as_ref().unwrap().render(render_context, self.triangle.as_ref().unwrap());
    }
}

fn main() {
    let outer_app = Box::new(HelloTriangle::default());
    WinitApp::run(outer_app);
}
