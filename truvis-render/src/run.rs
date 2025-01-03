use crate::{
    framework::{rendering::render_context::RenderContext, rhi::Rhi},
    render::{RenderInitInfo, Renderer, Timer},
};

pub trait App
{
    fn new(rhi: &'static Rhi, render_context: &mut RenderContext) -> Self;
    fn init_info() -> RenderInitInfo;
    fn get_init_info(&self) -> RenderInitInfo;
    fn prepare(&mut self, rhi: &'static Rhi, render_context: &mut RenderContext);
    fn update(&self, rhi: &'static Rhi, render_context: &mut RenderContext, timer: &Timer);
}


pub fn panic_handler(info: &std::panic::PanicInfo)
{
    log::error!("{}", info);
    // std::thread::sleep(std::time::Duration::from_secs(3));
}


pub fn run(mut app: impl App)
{
    std::panic::set_hook(Box::new(panic_handler));

    let render_init_info = app.get_init_info();
    let mut renderer = Renderer::new(&render_init_info);
    app.prepare(Renderer::get_rhi(), &mut renderer.render_context);

    renderer.timer.reset();
    // 由于 Rust 的借用检查器，这里不能直接调用 Renderer 的 render_loop()，而是需要调用 window 的 render_loop()
    renderer.window.render_loop(|| {
        renderer.timer.update();
        app.update(Renderer::get_rhi(), &mut renderer.render_context, &renderer.timer);
    });
}

pub fn run2<T: App>(render_init_info: RenderInitInfo)
{
    std::panic::set_hook(Box::new(panic_handler));

    let mut renderer = Renderer::new(&render_init_info);
    let app = T::new(Renderer::get_rhi(), &mut renderer.render_context);

    renderer.timer.reset();
    // 由于 Rust 的借用检查器，这里不能直接调用 Renderer 的 render_loop()，而是需要调用 window 的 render_loop()
    renderer.window.render_loop(|| {
        renderer.timer.update();
        app.update(Renderer::get_rhi(), &mut renderer.render_context, &renderer.timer);
    });
}
