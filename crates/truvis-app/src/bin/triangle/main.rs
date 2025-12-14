use imgui::Ui;
use truvis_app::app::TruvisApp;
use truvis_app::outer_app::OuterApp;
use truvis_model_manager::components::geometry::RtGeometry;
use truvis_render::platform::camera::Camera;

mod triangle_pass;

use triangle_pass::TrianglePass;
use truvis_model_manager::shapes::triangle::TriangleSoA;
use truvis_render::core::renderer::Renderer;
use truvis_render_base::frame_context::FrameContext;
use truvis_render_graph::render_context::{RenderContext, RenderContextMut};

struct HelloTriangle {
    triangle_pipeline: TrianglePass,
    triangle: RtGeometry,
}
impl OuterApp for HelloTriangle {
    fn init(_renderer: &mut Renderer, _camera: &mut Camera) -> Self {
        log::info!("hello triangle init.");

        Self {
            triangle_pipeline: TrianglePass::new(&FrameContext::get().frame_settings()),
            triangle: TriangleSoA::create_mesh(),
        }
    }

    fn draw_ui(&mut self, _ui: &Ui) {
        static mut _UI_VALUE: usize = 0;
    }

    fn draw(&self, render_context: &RenderContext, render_context_mut: &mut RenderContextMut) {
        self.triangle_pipeline.render(render_context, render_context_mut, &self.triangle);
    }
}

fn main() {
    TruvisApp::<HelloTriangle>::run();
}
