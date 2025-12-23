use imgui::Ui;
use truvis_app::app::TruvisApp;
use truvis_app::outer_app::OuterApp;
use truvis_model::components::geometry::RtGeometry;
use truvis_render::platform::camera::Camera;

mod triangle_pass;

use triangle_pass::TrianglePass;
use truvis_model::shapes::triangle::TriangleSoA;
use truvis_render::core::renderer::Renderer;
use truvis_render_graph::render_context::RenderContext;

struct HelloTriangle {
    triangle_pipeline: TrianglePass,
    triangle: RtGeometry,
}
impl OuterApp for HelloTriangle {
    fn init(renderer: &mut Renderer, _camera: &mut Camera) -> Self {
        log::info!("hello triangle init.");

        Self {
            triangle_pipeline: TrianglePass::new(&renderer.render_context.frame_settings, &mut renderer.cmd_allocator),
            triangle: TriangleSoA::create_mesh(),
        }
    }

    fn draw_ui(&mut self, _ui: &Ui) {
        static mut _UI_VALUE: usize = 0;
    }

    fn draw(&self, render_context: &RenderContext) {
        self.triangle_pipeline.render(render_context, &self.triangle);
    }
}

fn main() {
    TruvisApp::<HelloTriangle>::run();
}
