use truvis_render_core::core::renderer::Renderer;
use truvis_render_core::platform::camera::Camera;
use truvis_render_graph::render_context::RenderContext;

pub mod async_load_test;
pub mod cornell_app;
pub mod shader_toy;
pub mod sponza_app;
pub mod triangle;

/// 外部应用接口 trait
///
/// 定义应用生命周期的关键钩子函数。所有自定义应用需实现此 trait。
///
/// # 开发模式
/// ```ignore
/// struct MyApp { pipeline: MyPipeline }
///
/// impl OuterApp for MyApp {
///     fn init(renderer: &mut Renderer, camera: &mut Camera) -> Self {
///         Self { pipeline: MyPipeline::new() }
///     }
///
///     fn draw(&self) {
///         self.pipeline.render();
///     }
/// }
///
/// fn main() {
///     TruvisApp::<MyApp>::run();
/// }
/// ```
pub trait OuterApp {
    fn init(&mut self, _renderer: &mut Renderer, _camera: &mut Camera) {}

    /// 绘制 GUI（可选）
    fn draw_ui(&mut self, _ui: &imgui::Ui) {}

    /// 每帧更新逻辑（可选）
    fn update(&mut self, _renderer: &mut Renderer) {}

    /// 渲染主逻辑（发生于 acquire_frame 之后，submit_frame 之前）
    fn draw(&self, _render_context: &RenderContext) {}

    /// 窗口大小改变后重建资源（可选）
    fn on_window_resized(&mut self, _renderer: &mut Renderer) {}
}
