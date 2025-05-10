use ash::vk;

/// 等价于 framebuffer
pub struct FrameBuffer {}

impl FrameBuffer {
    pub fn get_depth_attachment(depth_image_view: vk::ImageView) -> vk::RenderingAttachmentInfo<'static> {
        vk::RenderingAttachmentInfo::default()
            .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .image_view(depth_image_view)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1_f32, // 1 表示无限远
                    stencil: 0,
                },
            })
    }

    pub fn get_color_attachment(image_view: vk::ImageView) -> vk::RenderingAttachmentInfo<'static> {
        vk::RenderingAttachmentInfo::default()
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image_view(image_view)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0_f32, 0_f32, 0_f32, 1_f32],
                },
            })
    }

    pub fn get_render_info<'a, 'b, 'c>(
        area: vk::Rect2D,
        color_attachs: &'a [vk::RenderingAttachmentInfo],
        depth_attach: &'b vk::RenderingAttachmentInfo,
    ) -> vk::RenderingInfo<'c>
    where
        'b: 'c,
        'a: 'c,
    {
        vk::RenderingInfo::default()
            .layer_count(1)
            .render_area(area)
            .color_attachments(color_attachs)
            .depth_attachment(depth_attach)
    }
}
