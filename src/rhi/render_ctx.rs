use ash::vk;
use itertools::Itertools;

use crate::{
    rhi::{create_info_uitls::RhiCreateInfoUtil, queue::RhiQueueType, rhi_core::RhiCore, swapchain::RHISwapchain},
    rhi_init_info::RhiInitInfo,
    window_system::WindowSystem,
};

pub struct RenderCtx
{
    swapchain_image_index: usize,
    frame_index: usize,
    frames_cnt: usize,

    // 为每个 frame 分配一个 command pool
    graphics_command_pools: Vec<vk::CommandPool>,

    depth_format: Option<vk::Format>,
    depth_image: Option<vk::Image>,
    depth_image_allcation: Option<vk_mem::Allocation>,
    depth_image_view: Option<vk::ImageView>,

    semaphore_image_available_for_render: Vec<vk::Semaphore>,
    semaphore_image_finished_for_present: Vec<vk::Semaphore>,
    fence_frame_in_flight: Vec<vk::Fence>,
}

impl RenderCtx
{
    pub fn init(rhi_core: &RhiCore, swapchain: &RHISwapchain, init_info: &RhiInitInfo) -> Self
    {
        let mut ctx = RenderCtx {
            swapchain_image_index: 0,
            frame_index: 0,
            frames_cnt: init_info.frames_in_flight as usize,

            graphics_command_pools: vec![],

            depth_format: None,
            depth_image: None,
            depth_image_allcation: None,
            depth_image_view: None,

            semaphore_image_available_for_render: vec![],
            semaphore_image_finished_for_present: vec![],
            fence_frame_in_flight: vec![],
        };

        ctx.init_depth_image_and_view(rhi_core, swapchain, init_info);
        ctx.init_synchronous_primitives(rhi_core);
        ctx.init_command_pool(rhi_core);

        ctx
    }

    fn init_depth_image_and_view(&mut self, rhi_core: &RhiCore, swapchain: &RHISwapchain, init_info: &RhiInitInfo)
    {
        let depth_format = rhi_core
            .physical_device()
            .find_supported_format(
                &init_info.depth_format_dedicate,
                vk::ImageTiling::OPTIMAL,
                vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
            )
            .first()
            .copied()
            .unwrap();

        let (depth_image, depth_image_allocation) = {
            let create_info = RhiCreateInfoUtil::make_image2d_create_info(
                swapchain.extent.unwrap(),
                depth_format,
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            );
            rhi_core.create_image(&create_info, Some("depth-image"))
        };

        let depth_image_view = {
            let create_info = RhiCreateInfoUtil::make_image_view_2d_create_info(
                depth_image,
                depth_format,
                vk::ImageAspectFlags::DEPTH,
            );
            rhi_core.create_image_view(&create_info, Some("depth-image-view"))
        };

        self.depth_format = Some(depth_format);
        self.depth_image = Some(depth_image);
        self.depth_image_allcation = Some(depth_image_allocation);
        self.depth_image_view = Some(depth_image_view);
    }

    fn init_synchronous_primitives(&mut self, rhi_core: &RhiCore)
    {
        let create_semaphore = |name: &str| {
            (0..self.frames_cnt).map(|i| rhi_core.create_semaphore(Some(&format!("{name}-{i}")))).collect_vec()
        };
        self.semaphore_image_available_for_render = create_semaphore("image-available-for-render");
        self.semaphore_image_finished_for_present = create_semaphore("image-finished-for-present");

        self.fence_frame_in_flight = (0..self.frames_cnt)
            .map(|i| rhi_core.create_fence(true, Some(&format!("frame-in-flight-{i}"))))
            .collect();
    }

    fn init_command_pool(&mut self, rhi_core: &RhiCore)
    {
        self.graphics_command_pools = (0..self.frames_cnt)
            .map(|i| {
                rhi_core.create_command_pool(
                    RhiQueueType::Graphics,
                    vk::CommandPoolCreateFlags::TRANSIENT,
                    Some("context-graphics-pool"),
                )
            })
            .collect();
    }
}
