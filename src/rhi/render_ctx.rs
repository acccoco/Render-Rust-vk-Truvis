// TODO 包括各种资源


use ash::extensions::khr::Swapchain;

use crate::{rhi::RhiCore, window_system::WindowSystem};

pub struct RenderCtx
{
    swapchain_image_index: u32,
}
