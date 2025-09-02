use crate::{
    platform::camera::DrsCamera, render_pipeline::pipeline_context::PipelineContext, renderer::renderer::Renderer,
};

pub trait OuterApp {
    fn init(renderer: &mut Renderer, camera: &mut DrsCamera) -> Self;

    fn draw_ui(&mut self, _ui: &imgui::Ui) {}

    fn update(&mut self, _renderer: &mut Renderer) {}

    /// 发生于 acquire_frame 之后，submit_frame 之前
    fn draw(&self, _pipeline_ctx: PipelineContext) {}

    /// window 发生改变后，重建
    fn rebuild(&mut self, _renderer: &mut Renderer) {}
}
