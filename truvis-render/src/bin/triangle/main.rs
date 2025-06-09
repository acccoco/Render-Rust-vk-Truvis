mod triangle_pass;
mod triangle_pipeline;

use crate::triangle_pipeline::TrianglePipeline;
use imgui::Ui;
use model_manager::component::DrsGeometry;
use model_manager::vertex::vertex_pc::{VertexAosLayoutPosColor, VertexPosColor};
use truvis_render::app::{OuterApp, TruvisApp};
use truvis_render::platform::camera::DrsCamera;
use truvis_render::render::Renderer;
use truvis_render::render_pipeline::pipeline_context::PipelineContext;

struct HelloTriangle {
    triangle_pipeline: TrianglePipeline,
    triangle: DrsGeometry<VertexPosColor>,
    frame_id: usize,
}
impl OuterApp for HelloTriangle {
    fn init(renderer: &mut Renderer, _camera: &mut DrsCamera) -> Self {
        log::info!("hello triangle init.");

        Self {
            triangle_pipeline: TrianglePipeline::new(
                &renderer.rhi,
                &renderer.renderer_settings().pipeline_settings,
                renderer.bindless_mgr.clone(),
            ),
            triangle: VertexAosLayoutPosColor::triangle(&renderer.rhi),
            frame_id: 0,
        }
    }

    fn draw_ui(&mut self, ui: &mut Ui) {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
        ui.text_wrapped(format!("Frame ID: {}", self.frame_id));
        static mut UI_VALUE: usize = 0;
        let choices = ["test test this is 1", "test test this is 2"];
        unsafe {
            if ui.button(choices[UI_VALUE]) {
                UI_VALUE += 1;
                UI_VALUE %= 2;
            }
        }

        ui.button("This...is...imgui-rs!");
        ui.separator();
        let mouse_pos = ui.io().mouse_pos;
        ui.text(format!("Mouse Position: ({:.1},{:.1})", mouse_pos[0], mouse_pos[1]));
    }

    fn draw(&self, pipeline_ctx: PipelineContext) {
        self.triangle_pipeline.render(pipeline_ctx, &self.triangle);
    }
}

fn main() {
    TruvisApp::<HelloTriangle>::run();
}
