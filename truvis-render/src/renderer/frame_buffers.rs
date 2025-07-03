use crate::renderer::bindless::BindlessManager;
use ash::vk;
use std::rc::Rc;
use truvis_rhi::core::image::{RhiImage2D, RhiImage2DView};
use truvis_rhi::rhi::Rhi;

/// 所有帧会用到的 buffers
pub struct FrameBuffers {
    color_image: RhiImage2D,
    color_image_view: RhiImage2DView,
    color_bindless_key: String,

    depth_image: RhiImage2D,
    depth_image_view: RhiImage2DView,

    /// fif 每一帧的渲染结果
    frame_rt: Vec<RhiImage2D>,
    frame_rt_views: Vec<RhiImage2DView>,
}

impl FrameBuffers {
    pub fn new(rhi: &Rhi, extent: vk::Extent2D, bindless_mgr: &mut BindlessManager) -> Self {}
}
