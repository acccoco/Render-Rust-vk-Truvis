use imgui::Ui;
use truvis_app::app::TruvisApp;
use truvis_app::outer_app::OuterApp;
use truvis_model_manager::components::geometry::Geometry;
use truvis_model_manager::vertex::aos_pos_color::VertexLayoutAoSPosColor;
use truvis_render::renderer::frame_context::FrameContext;
use truvis_render::{platform::camera::Camera, renderer::renderer::Renderer};

mod triangle_pass;
mod triangle_pipeline;

use crate::triangle_pipeline::TrianglePipeline;

struct HelloTriangle {
    triangle_pipeline: TrianglePipeline,
    triangle: Geometry<VertexLayoutAoSPosColor>,
}
impl OuterApp for HelloTriangle {
    fn init(_renderer: &mut Renderer, _camera: &mut Camera) -> Self {
        log::info!("hello triangle init.");

        Self {
            triangle_pipeline: TrianglePipeline::new(&FrameContext::get().frame_settings()),
            triangle: VertexLayoutAoSPosColor::triangle(),
        }
    }

    fn draw_ui(&mut self, _ui: &Ui) {
        static mut _UI_VALUE: usize = 0;
    }

    fn draw(&self) {
        self.triangle_pipeline.render(&self.triangle);
    }
}

fn main() {
    TruvisApp::<HelloTriangle>::run();
}
