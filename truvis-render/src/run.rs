use ash::vk;

use crate::{
    framework::{rendering::render_context::RenderContext, rhi::Rhi},
    render::{RenderInitInfo, Renderer, Timer},
};

pub trait App
{
    fn update(&self, rhi: &'static Rhi, render_context: &mut RenderContext, timer: &Timer);


    fn init(rhi: &'static Rhi, render_context: &mut RenderContext) -> Self;

    /// 由 App 提供的，用于初始化 Rhi
    fn get_render_init_info() -> RenderInitInfo;


    fn get_depth_attachment(depth_image_view: vk::ImageView) -> vk::RenderingAttachmentInfo
    {
        vk::RenderingAttachmentInfo::builder()
            .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .image_view(depth_image_view)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .clear_value(vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1_f32,
                    stencil: 0,
                },
            })
            .build()
    }

    fn get_color_attachment(image_view: vk::ImageView) -> vk::RenderingAttachmentInfo
    {
        vk::RenderingAttachmentInfo::builder()
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image_view(image_view)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0_f32, 0_f32, 0_f32, 1_f32],
                },
            })
            .build()
    }

    fn get_render_info(
        area: vk::Rect2D,
        color_attachs: &[vk::RenderingAttachmentInfo],
        depth_attach: &vk::RenderingAttachmentInfo,
    ) -> vk::RenderingInfo
    {
        vk::RenderingInfo::builder()
            .layer_count(1)
            .render_area(area)
            .color_attachments(color_attachs)
            .depth_attachment(depth_attach)
            .build()
    }
}


pub fn panic_handler(info: &std::panic::PanicInfo)
{
    log::error!("{}", info);
    // std::thread::sleep(std::time::Duration::from_secs(3));
}


pub fn run<T: App>()
{
    let render_init_info = T::get_render_init_info();
    std::panic::set_hook(Box::new(panic_handler));

    let mut renderer = Renderer::new(&render_init_info);
    let app = T::init(Renderer::get_rhi(), &mut renderer.render_context);

    renderer.timer.reset();
    // 由于 Rust 的借用检查器，这里不能直接调用 Renderer 的 render_loop()，而是需要调用 window 的 render_loop()
    renderer.window.render_loop(|| {
        renderer.timer.update();
        app.update(Renderer::get_rhi(), &mut renderer.render_context, &renderer.timer);
    });
}
