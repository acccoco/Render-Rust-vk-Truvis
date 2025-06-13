mod triangle_pass;
mod triangle_pipeline;

use crate::triangle_pipeline::TrianglePipeline;
use imgui::{StyleColor, TextureId, Ui};
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
        static mut UI_VALUE: usize = 0;

        ui.dockspace_over_main_viewport();

        ui.window("test windwo").size([100.0, 100.0], imgui::Condition::FirstUseEver).build(|| {
            ui.text_wrapped("test Hello world!");
        });
        ui.window("hello world").size([100.0, 100.0], imgui::Condition::FirstUseEver).build(|| {
            ui.text_wrapped("Hello world!");
            ui.text_wrapped("こんにちは世界！");
            ui.text_wrapped(format!("Frame ID: {}", self.frame_id));
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
        });
        ui.window("render")
            .size([400.0, 400.0], imgui::Condition::FirstUseEver)
            .title_bar(false)
            .resizable(false)
            // .bg_alpha(0.0)
            // .draw_background(false)
            .build(|| {
                ui.text("render window");
                imgui::Image::new(TextureId::new(114), [400.0, 400.0]).build(ui);

                let window_size = ui.window_size();
                let window_pos = ui.window_pos();
                ui.text(format!("Window Size: ({:.1},{:.1})", window_size[0], window_size[1]));
                ui.text(format!("Window Position: ({:.1},{:.1})", window_pos[0], window_pos[1]));
            });
    }

    fn draw(&self, pipeline_ctx: PipelineContext) {
        self.triangle_pipeline.render(pipeline_ctx, &self.triangle);
    }
}

fn main() {
    TruvisApp::<HelloTriangle>::run();
}
