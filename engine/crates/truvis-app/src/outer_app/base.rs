use truvis_gfx::commands::semaphore::GfxSemaphore;
use truvis_renderer::platform::camera::Camera;
use truvis_renderer::renderer::Renderer;

/// 外部应用接口 trait
///
/// 定义应用生命周期的关键钩子函数。所有自定义应用需实现此 trait。
pub trait OuterApp {
    fn init(&mut self, renderer: &mut Renderer, camera: &mut Camera);

    /// 绘制 GUI（可选）
    fn draw_ui(&mut self, ui: &imgui::Ui);

    /// 每帧更新逻辑（可选）
    fn update(&mut self, renderer: &mut Renderer);

    /// 渲染主逻辑（发生于 acquire_frame 之后，submit_frame 之前）
    fn draw(&self, renderer: &Renderer, gui_draw_data: &imgui::DrawData, fence: &GfxSemaphore);

    /// 窗口大小改变后重建资源（可选）
    fn on_window_resized(&mut self, _renderer: &mut Renderer) {}
}
