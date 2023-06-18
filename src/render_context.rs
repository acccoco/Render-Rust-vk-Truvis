use ash::{extensions::khr::Swapchain, vk};
use itertools::Itertools;

use crate::{
    resource_type::{
        command_pool::RhiCommandPool,
        sync_primitives::{RhiFence, RhiSemaphore},
    },
    rhi::{create_utils::RhiCreateInfoUtil, Rhi},
    swapchain::RenderSwapchain,
};

pub(crate) struct RenderContextInitInfo
{
    frames_in_flight: usize,
    depth_format_dedicate: Vec<vk::Format>,
}

impl Default for RenderContextInitInfo
{
    fn default() -> Self
    {
        Self {
            depth_format_dedicate: vec![
                vk::Format::D32_SFLOAT,
                vk::Format::D32_SFLOAT_S8_UINT,
                vk::Format::D24_UNORM_S8_UINT,
            ],
            frames_in_flight: 3,
        }
    }
}

static mut RENDER_CONTEXT: Option<RenderContext> = None;

pub struct RenderContext
{
    swapchain_image_index: usize,

    frame_index: usize,
    frames_cnt: usize,

    // 为每个 frame 分配一个 command pool
    graphics_command_pools: Vec<RhiCommandPool>,

    depth_format: Option<vk::Format>,
    depth_image: Option<vk::Image>,
    depth_image_allcation: Option<vk_mem::Allocation>,
    depth_image_view: Option<vk::ImageView>,
    depth_attach_info: vk::RenderingAttachmentInfo,

    semaphores_swapchain_available: Vec<RhiSemaphore>,
    semaphores_image_render_finish: Vec<RhiSemaphore>,
    fence_frame_in_flight: Vec<RhiFence>,
}

impl RenderContext
{
    pub(crate) fn init(init_info: &RenderContextInitInfo)
    {
        let mut ctx = RenderContext {
            swapchain_image_index: 0,
            frame_index: 0,
            frames_cnt: init_info.frames_in_flight,

            graphics_command_pools: vec![],

            depth_format: None,
            depth_image: None,
            depth_image_allcation: None,
            depth_image_view: None,

            depth_attach_info: Default::default(),
            semaphores_swapchain_available: vec![],
            semaphores_image_render_finish: vec![],
            fence_frame_in_flight: vec![],
        };

        ctx.init_depth_image_and_view(&init_info.depth_format_dedicate);
        ctx.init_synchronous_primitives();
        ctx.init_command_pool();
        ctx.init_depth_attach();

        unsafe { RENDER_CONTEXT = Some(ctx) }
    }

    #[inline]
    pub fn acquire_frame()
    {
        unsafe {
            let mut ctx = RENDER_CONTEXT.as_mut().unwrap_unchecked();

            let current_fence = &mut ctx.fence_frame_in_flight[ctx.frame_index];
            ctx.swapchain_image_index = RenderSwapchain::instance()
                .acquire_next_frame(&ctx.semaphores_swapchain_available[ctx.frame_index], Some(current_fence))
                as usize;

            current_fence.wait();
            current_fence.reset();
        }
    }

    #[inline]
    pub fn submit_frame()
    {
        unsafe {
            let mut ctx = RENDER_CONTEXT.as_mut().unwrap_unchecked();

            RenderSwapchain::instance().submit_frame(
                ctx.swapchain_image_index as u32,
                &[RenderContext::current_image_render_finish_semaphore().semaphore],
            );

            ctx.frame_index = (ctx.frame_index + 1) % ctx.frames_cnt;
        }
    }

    #[inline]
    pub fn extent() -> vk::Extent2D { RenderSwapchain::instance().extent() }

    #[inline]
    pub fn current_image_render_finish_semaphore() -> RhiSemaphore
    {
        let ctx = Self::instance();
        ctx.semaphores_image_render_finish[ctx.frame_index]
    }

    #[inline]
    pub fn current_swapchain_available_semaphore() -> RhiSemaphore
    {
        let ctx = Self::instance();
        ctx.semaphores_swapchain_available[ctx.frame_index]
    }

    #[inline]
    pub fn color_attach_info() -> &'static vk::RenderingAttachmentInfo
    {
        &RenderSwapchain::instance().color_attach_infos[Self::instance().swapchain_image_index]
    }

    #[inline]
    pub fn depth_attach_info() -> &'static vk::RenderingAttachmentInfo { &Self::instance().depth_attach_info }

    #[inline]
    pub fn current_image() -> vk::Image { RenderSwapchain::instance().images[Self::instance().swapchain_image_index] }

    #[inline]
    pub fn render_info() -> vk::RenderingInfo
    {
        vk::RenderingInfo::builder()
            .layer_count(1)
            .render_area(Self::extent().into())
            .color_attachments(std::slice::from_ref(Self::color_attach_info()))
            .depth_attachment(Self::depth_attach_info())
            .build()
    }

    #[inline]
    pub fn instance() -> &'static Self { unsafe { RENDER_CONTEXT.as_ref().unwrap_unchecked() } }

    #[inline]
    pub fn color_format(&self) -> vk::Format { RenderSwapchain::instance().color_format() }

    #[inline]
    pub fn depth_format() -> vk::Format { unsafe { Self::instance().depth_format.unwrap_unchecked() } }

    fn init_depth_image_and_view(&mut self, depth_format_dedicate: &[vk::Format])
    {
        let rhi = Rhi::instance();

        let depth_format = rhi
            .find_supported_format(
                depth_format_dedicate,
                vk::ImageTiling::OPTIMAL,
                vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
            )
            .first()
            .copied()
            .unwrap();

        let (depth_image, depth_image_allocation) = {
            let create_info = RhiCreateInfoUtil::make_image2d_create_info(
                RenderSwapchain::instance().extent.unwrap(),
                depth_format,
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            );
            rhi.create_image(&create_info, Some("depth-image"))
        };

        let depth_image_view = {
            let create_info = RhiCreateInfoUtil::make_image_view_2d_create_info(
                depth_image,
                depth_format,
                vk::ImageAspectFlags::DEPTH,
            );
            rhi.create_image_view(&create_info, Some("depth-image-view"))
        };

        self.depth_format = Some(depth_format);
        self.depth_image = Some(depth_image);
        self.depth_image_allcation = Some(depth_image_allocation);
        self.depth_image_view = Some(depth_image_view);
    }

    fn init_synchronous_primitives(&mut self)
    {
        let create_semaphore =
            |name: &str| (0..self.frames_cnt).map(|i| RhiSemaphore::new(Some(&format!("{name}-{i}")))).collect_vec();
        self.semaphores_swapchain_available = create_semaphore("image-available-for-render");
        self.semaphores_image_render_finish = create_semaphore("image-finished-for-present");

        self.fence_frame_in_flight =
            (0..self.frames_cnt).map(|i| RhiFence::new(false, Some(&format!("frame-in-flight-{i}")))).collect();
    }

    fn init_command_pool(&mut self)
    {
        let rhi = Rhi::instance();
        self.graphics_command_pools = (0..self.frames_cnt)
            .map(|i| {
                rhi.create_command_pool(
                    vk::QueueFlags::GRAPHICS,
                    vk::CommandPoolCreateFlags::TRANSIENT,
                    Some(&format!("context-graphics-{}", i)),
                )
                .unwrap()
            })
            .collect();
    }

    fn init_depth_attach(&mut self)
    {
        self.depth_attach_info = vk::RenderingAttachmentInfo::builder()
            .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .image_view(self.depth_image_view.unwrap())
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .clear_value(vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1_f32, stencil: 0 } })
            .build();
    }
}
