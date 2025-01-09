use ash::vk;

use crate::framework::{
    core::{
        command_buffer::RhiCommandBuffer,
        command_pool::RhiCommandPool,
        queue::RhiSubmitBatch,
        swapchain::RenderSwapchain,
        synchronize::{RhiFence, RhiSemaphore},
    },
    rhi::{Rhi, RHI},
};

pub struct RenderContext
{
    pub render_swapchain: RenderSwapchain,

    swapchain_image_index: usize,

    current_frame: usize,
    pub frames_cnt: usize,

    /// 为每个 frame 分配一个 command pool
    graphics_command_pools: Vec<RhiCommandPool>,

    /// 每个 command pool 已经分配出去的 command buffer，用于集中 free 或其他操作
    allocated_command_buffers: Vec<Vec<RhiCommandBuffer>>,

    pub depth_format: vk::Format,
    pub depth_image: vk::Image,
    depth_image_allcation: vk_mem::Allocation,
    pub depth_image_view: vk::ImageView,

    present_complete_semaphores: Vec<RhiSemaphore>,
    render_complete_semaphores: Vec<RhiSemaphore>,
    fence_frame_in_flight: Vec<RhiFence>,

    rhi: &'static Rhi,
}

impl RenderContext
{
    /// 准备好渲染当前frame 所需的资源
    ///
    /// * 通过 fence 等待当前 frame 资源释放
    /// * 为 image 进行 layout transition 的操作
    pub fn acquire_frame(&mut self)
    {
        let rhi = RHI.get().unwrap();
        let current_fence = &mut self.fence_frame_in_flight[self.current_frame];
        rhi.wait_for_fence(&current_fence);
        rhi.reset_fence(&current_fence);
        rhi.reset_command_pool(&mut self.graphics_command_pools[self.current_frame]);
        std::mem::take(&mut self.allocated_command_buffers[self.current_frame]).into_iter().for_each(|c| c.free());

        self.swapchain_image_index =
            self.render_swapchain.acquire_next_frame(&self.present_complete_semaphores[self.current_frame], None)
                as usize;

        {
            let mut cmd = self.alloc_command_buffer(format!("ctx-1st-color-layout-trans-frame-{}", self.current_frame));
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            // 只需要建立起执行依赖即可，确保 present 完成后，再进行 layout trans
            // COLOR_ATTACHMENT_READ 对应 blend 等操作
            cmd.image_barrier(
                (vk::PipelineStageFlags::BOTTOM_OF_PIPE, vk::AccessFlags::empty()),
                (
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::COLOR_ATTACHMENT_READ,
                ),
                self.current_present_image(),
                vk::ImageAspectFlags::COLOR,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            );
            cmd.end();
            rhi.queue_submit(
                rhi.graphics_queue(),
                vec![RhiSubmitBatch {
                    command_buffers: vec![cmd],
                    wait_info: vec![(
                        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                        self.current_present_complete_semaphore(),
                    )],
                    signal_info: vec![],
                }],
                None,
            );
        }
    }


    /// 提交当前 frame
    ///
    /// * 在提交之前，为 image 进行 layout transition
    pub fn submit_frame(&mut self)
    {
        {
            let mut cmd = self.alloc_command_buffer(format!("ctx-2nd-color-layout-trans-frame-{}", self.current_frame));
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            cmd.image_barrier(
                (vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, vk::AccessFlags::COLOR_ATTACHMENT_WRITE),
                (vk::PipelineStageFlags::BOTTOM_OF_PIPE, vk::AccessFlags::empty()),
                self.current_present_image(),
                vk::ImageAspectFlags::COLOR,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk::ImageLayout::PRESENT_SRC_KHR,
            );
            cmd.end();
            self.rhi.graphics_queue().submit(
                self.rhi,
                vec![RhiSubmitBatch {
                    command_buffers: vec![cmd],
                    wait_info: vec![],
                    signal_info: vec![self.current_render_complete_semaphore()],
                }],
                Some(self.current_fence().clone()),
            );
        }

        self.render_swapchain.submit_frame(
            self.rhi,
            self.swapchain_image_index as u32,
            &[self.current_render_complete_semaphore().semaphore],
        );

        self.current_frame = (self.current_frame + 1) % self.frames_cnt;
    }


    /// 分配 command buffer，在当前 frame 使用
    pub fn alloc_command_buffer<S: AsRef<str>>(&mut self, debug_name: S) -> RhiCommandBuffer
    {
        let name = format!("frame-{}-command-buffer-{}", self.current_frame, debug_name.as_ref());
        let cmd = RhiCommandBuffer::new(self.rhi, &self.graphics_command_pools[self.current_frame], name);

        self.allocated_command_buffers[self.current_frame].push(cmd.clone());

        cmd
    }
}

pub use _impl_init::RenderContextInitInfo;

mod _impl_init
{
    use ash::vk;
    use itertools::Itertools;

    use crate::framework::{
        core::{
            command_pool::RhiCommandPool,
            create_utils::RhiCreateInfoUtil,
            swapchain::{RenderSwapchain, RenderSwapchainInitInfo},
            synchronize::{RhiFence, RhiSemaphore},
        },
        rendering::render_context::RenderContext,
        rhi::Rhi,
    };

    pub struct RenderContextInitInfo
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
                    vk::Format::D32_SFLOAT_S8_UINT,
                    vk::Format::D32_SFLOAT,
                    vk::Format::D24_UNORM_S8_UINT,
                    vk::Format::D16_UNORM_S8_UINT,
                    vk::Format::D16_UNORM,
                ],
                frames_in_flight: 3,
            }
        }
    }

    impl RenderContext
    {
        pub fn new(
            rhi: &'static Rhi,
            init_info: &RenderContextInitInfo,
            render_swapchain_init_info: RenderSwapchainInitInfo,
        ) -> Self
        {
            let render_swapchain = RenderSwapchain::new(rhi, &render_swapchain_init_info);
            let (depth_format, depth_image, depth_image_allcation, depth_image_view) =
                Self::init_depth_image_and_view(rhi, &render_swapchain, &init_info.depth_format_dedicate);

            let create_semaphore = |name: &str| {
                (0..init_info.frames_in_flight).map(|i| RhiSemaphore::new(rhi, format!("{name}_{i}"))).collect_vec()
            };
            let present_complete_semaphores = create_semaphore("present_complete_semaphore");
            let render_complete_semaphores = create_semaphore("render_complete_semaphores");

            let fence_frame_in_flight = (0..init_info.frames_in_flight)
                .map(|i| RhiFence::new(rhi, true, format!("frame_in_flight_fence_{i}")))
                .collect();

            let graphics_command_pools = Self::init_command_pool(rhi, init_info);

            let ctx = Self {
                render_swapchain,

                swapchain_image_index: 0,
                current_frame: 0,
                frames_cnt: init_info.frames_in_flight,

                graphics_command_pools,
                allocated_command_buffers: vec![Vec::new(); init_info.frames_in_flight],

                depth_format,
                depth_image,
                depth_image_allcation,
                depth_image_view,

                present_complete_semaphores,
                render_complete_semaphores,
                fence_frame_in_flight,

                rhi,
            };

            ctx
        }


        fn init_depth_image_and_view(
            rhi: &Rhi,
            swapchain: &RenderSwapchain,
            depth_format_dedicate: &[vk::Format],
        ) -> (vk::Format, vk::Image, vk_mem::Allocation, vk::ImageView)
        {
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
                    swapchain.extent,
                    depth_format,
                    vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                );
                rhi.create_image(&create_info, "depth-image")
            };

            let depth_image_view = {
                let create_info = RhiCreateInfoUtil::make_image_view_2d_create_info(
                    depth_image,
                    depth_format,
                    vk::ImageAspectFlags::DEPTH,
                );
                rhi.create_image_view(&create_info, "depth-image-view")
            };

            (depth_format, depth_image, depth_image_allocation, depth_image_view)
        }

        fn init_command_pool(rhi: &Rhi, init_info: &RenderContextInitInfo) -> Vec<RhiCommandPool>
        {
            let graphics_command_pools = (0..init_info.frames_in_flight)
                .map(|i| {
                    rhi.create_command_pool(
                        vk::QueueFlags::GRAPHICS,
                        vk::CommandPoolCreateFlags::TRANSIENT,
                        format!("render_context_graphics_command_pool_{}", i),
                    )
                })
                .collect();

            graphics_command_pools
        }
    }
}

mod _impl_property
{
    use ash::vk;

    use crate::framework::{
        core::synchronize::{RhiFence, RhiSemaphore},
        rendering::render_context::RenderContext,
    };

    impl RenderContext
    {
        /// 直接从 swapchain 获取 extent
        #[inline]
        pub fn swapchain_extent(&self) -> vk::Extent2D
        {
            self.render_swapchain.extent
        }

        #[inline]
        pub fn current_fence(&self) -> &RhiFence
        {
            &self.fence_frame_in_flight[self.current_frame]
        }

        #[inline]
        pub fn color_format(&self) -> vk::Format
        {
            self.render_swapchain.color_format
        }

        #[inline]
        pub fn current_frame_index(&self) -> usize
        {
            self.current_frame
        }

        #[inline]
        pub fn depth_format(&self) -> vk::Format
        {
            self.depth_format
        }

        #[inline]
        pub fn current_render_complete_semaphore(&self) -> RhiSemaphore
        {
            self.render_complete_semaphores[self.current_frame]
        }

        #[inline]
        pub fn current_present_complete_semaphore(&self) -> RhiSemaphore
        {
            self.present_complete_semaphores[self.current_frame]
        }

        /// 当前帧从 swapchain 获取到的用于 present 的 image
        #[inline]
        pub fn current_present_image(&self) -> vk::Image
        {
            self.render_swapchain.images[self.swapchain_image_index]
        }

        #[inline]
        pub fn current_present_image_view(&self) -> vk::ImageView
        {
            self.render_swapchain.image_views[self.swapchain_image_index]
        }
    }
}
