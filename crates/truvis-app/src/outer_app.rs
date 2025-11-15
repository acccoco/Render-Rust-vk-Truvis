use truvis_render::core::renderer::Renderer;
use truvis_render::platform::camera::Camera;

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
    /// 初始化应用（窗口创建后调用）
    fn init(renderer: &mut Renderer, camera: &mut Camera) -> Self;

    /// 绘制 GUI（可选）
    fn draw_ui(&mut self, _ui: &imgui::Ui) {}

    /// 每帧更新逻辑（可选）
    fn update(&mut self, _renderer: &mut Renderer) {}

    /// 渲染主逻辑（发生于 acquire_frame 之后，submit_frame 之前）
    fn draw(&self) {}

    /// 窗口大小改变后重建资源（可选）
    fn rebuild(&mut self, _renderer: &mut Renderer) {}
}
