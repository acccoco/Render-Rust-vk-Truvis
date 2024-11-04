use crate::{
    framework::{rendering::render_context::RenderContext, rhi::Rhi},
    render::{RenderInitInfo, Renderer, Timer},
};

pub trait App
{
    fn get_init_info(&self) -> RenderInitInfo;
    fn init(&mut self, rhi: &'static Rhi, render_context: &mut RenderContext);
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
    app.init(Renderer::get_rhi(), &mut renderer.render_context);

    renderer.timer.reset();
    // 由于 Rust 的借用检查器，这里不能直接调用 Renderer 的 render_loop()，而是需要调用 window 的 render_loop()
    renderer.window.render_loop(|| {
        renderer.timer.update();
        app.update(Renderer::get_rhi(), &mut renderer.render_context, &renderer.timer);
    });
}
