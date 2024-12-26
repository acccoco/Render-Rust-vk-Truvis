pub(crate) use _ctx_init::RenderContextInitInfo;
use ash::vk;

use crate::framework::{
    core::{
        command_buffer::RhiCommandBuffer,
        command_pool::RhiCommandPool,
        queue::RhiSubmitBatch,
        swapchain::RenderSwapchain,
        synchronize::{RhiFence, RhiSemaphore},
    },
    rhi::Rhi,
};


pub struct RenderContext
{
    render_swapchain: RenderSwapchain,

    swapchain_image_index: usize,

    frame_index: usize,
    frames_cnt: usize,

    /// 为每个 frame 分配一个 command pool
    graphics_command_pools: Vec<RhiCommandPool>,

    /// 每个 command pool 已经分配出去的 command buffer，用于集中 free 或其他操作
    allocated_command_buffers: Vec<Vec<RhiCommandBuffer>>,

    depth_format: Option<vk::Format>,
    depth_image: Option<vk::Image>,
    depth_image_allcation: Option<vk_mem::Allocation>,
    depth_image_view: Option<vk::ImageView>,
    depth_attach_info: vk::RenderingAttachmentInfo,

    semaphores_swapchain_available: Vec<RhiSemaphore>,
    semaphores_image_render_finish: Vec<RhiSemaphore>,
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
        let current_fence = &mut self.fence_frame_in_flight[self.frame_index];
        current_fence.wait();
        self.graphics_command_pools[self.frame_index].reset(self.rhi);
        std::mem::take(&mut self.allocated_command_buffers[self.frame_index]).into_iter().for_each(|c| c.free());
        current_fence.reset();

        self.swapchain_image_index =
            self.render_swapchain.acquire_next_frame(&self.semaphores_swapchain_available[self.frame_index], None)
                as usize;

        {
            let mut cmd = self.alloc_command_buffer(format!("ctx-1st-color-layout-trans-frame-{}", self.frame_index));
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            // 只需要建立起执行依赖即可，确保 present 完成后，再进行 layout trans
            // COLOR_ATTACHMENT_READ 对应 blend 等操作
            cmd.image_barrier(
                (vk::PipelineStageFlags::BOTTOM_OF_PIPE, vk::AccessFlags::empty()),
                (
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::COLOR_ATTACHMENT_READ,
                ),
                self.current_image(),
                vk::ImageAspectFlags::COLOR,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            );
            cmd.end();
            self.rhi.graphics_queue().submit(
                self.rhi,
                vec![RhiSubmitBatch {
                    command_buffers: vec![cmd],
                    wait_info: vec![(
                        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                        self.current_swapchain_available_semaphore(),
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
            let mut cmd = self.alloc_command_buffer(format!("ctx-2nd-color-layout-trans-frame-{}", self.frame_index));
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            cmd.image_barrier(
                (vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, vk::AccessFlags::COLOR_ATTACHMENT_WRITE),
                (vk::PipelineStageFlags::BOTTOM_OF_PIPE, vk::AccessFlags::empty()),
                self.current_image(),
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
                    signal_info: vec![self.current_image_render_finish_semaphore()],
                }],
                Some(self.current_fence().clone()),
            );
        }

        self.render_swapchain.submit_frame(
            self.rhi,
            self.swapchain_image_index as u32,
            &[self.current_image_render_finish_semaphore().semaphore],
        );

        self.frame_index = (self.frame_index + 1) % self.frames_cnt;
    }


    #[inline]
    pub fn render_info(&self) -> vk::RenderingInfo
    {
        vk::RenderingInfo::builder()
            .layer_count(1)
            .render_area(self.extent().into())
            .color_attachments(std::slice::from_ref(self.color_attach_info()))
            .depth_attachment(self.depth_attach_info())
            .build()
    }


    /// 分配 command buffer，在当前 frame 使用
    #[inline]
    pub fn alloc_command_buffer<S: AsRef<str>>(&mut self, debug_name: S) -> RhiCommandBuffer
    {
        let name = format!("frame-{}-command-buffer-{}", self.frame_index, debug_name.as_ref());
        let cmd = RhiCommandBuffer::new(self.rhi, &self.graphics_command_pools[self.frame_index], name);

        self.allocated_command_buffers[self.frame_index].push(cmd.clone());

        cmd
    }
}


mod _ctx_init
{
    use ash::vk;
    use itertools::Itertools;

    use crate::framework::{
        core::{
            create_utils::RhiCreateInfoUtil,
            swapchain::{RenderSwapchain, RenderSwapchainInitInfo},
            synchronize::{RhiFence, RhiSemaphore},
        },
        rendering::render_context::RenderContext,
        rhi::Rhi,
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

    impl RenderContext
    {
        pub fn new(
            rhi: &'static Rhi,
            init_info: &RenderContextInitInfo,
            render_swapchain_init_info: RenderSwapchainInitInfo,
        ) -> Self
        {
            let render_swapchain = RenderSwapchain::new(rhi, &render_swapchain_init_info);

            let mut ctx = RenderContext {
                render_swapchain,

                swapchain_image_index: 0,
                frame_index: 0,
                frames_cnt: init_info.frames_in_flight,

                graphics_command_pools: vec![],
                allocated_command_buffers: vec![Vec::new(); init_info.frames_in_flight],

                depth_format: None,
                depth_image: None,
                depth_image_allcation: None,
                depth_image_view: None,

                depth_attach_info: Default::default(),
                semaphores_swapchain_available: vec![],
                semaphores_image_render_finish: vec![],
                fence_frame_in_flight: vec![],

                rhi,
            };

            ctx.init_depth_image_and_view(&init_info.depth_format_dedicate);
            ctx.init_synchronous_primitives();
            ctx.init_command_pool();
            ctx.init_depth_attach();

            ctx
        }


        fn init_depth_image_and_view(&mut self, depth_format_dedicate: &[vk::Format])
        {
            let depth_format = self
                .rhi
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
                    self.render_swapchain.extent.unwrap(),
                    depth_format,
                    vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                );
                self.rhi.create_image(&create_info, "depth-image")
            };

            let depth_image_view = {
                let create_info = RhiCreateInfoUtil::make_image_view_2d_create_info(
                    depth_image,
                    depth_format,
                    vk::ImageAspectFlags::DEPTH,
                );
                self.rhi.create_image_view(&create_info, "depth-image-view")
            };

            self.depth_format = Some(depth_format);
            self.depth_image = Some(depth_image);
            self.depth_image_allcation = Some(depth_image_allocation);
            self.depth_image_view = Some(depth_image_view);
        }

        fn init_synchronous_primitives(&mut self)
        {
            let create_semaphore = |name: &str| {
                (0..self.frames_cnt).map(|i| RhiSemaphore::new(self.rhi, format!("{name}-{i}"))).collect_vec()
            };
            self.semaphores_swapchain_available = create_semaphore("image-available-for-render-semaphore");
            self.semaphores_image_render_finish = create_semaphore("image-finished-for-present-semaphore");

            self.fence_frame_in_flight = (0..self.frames_cnt)
                .map(|i| RhiFence::new(self.rhi, true, format!("frame-in-flight-fence-{i}")))
                .collect();
        }

        fn init_command_pool(&mut self)
        {
            self.graphics_command_pools = (0..self.frames_cnt)
                .map(|i| {
                    self.rhi
                        .create_command_pool(
                            vk::QueueFlags::GRAPHICS,
                            vk::CommandPoolCreateFlags::TRANSIENT,
                            format!("context-graphics-command-pool-{}", i),
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
                .clear_value(vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1_f32,
                        stencil: 0,
                    },
                })
                .build();
        }
    }
}

mod _ctx_property
{
    use ash::vk;

    use crate::framework::{
        core::synchronize::{RhiFence, RhiSemaphore},
        rendering::render_context::RenderContext,
    };

    impl RenderContext
    {
        #[inline]
        pub fn extent(&self) -> vk::Extent2D
        {
            self.render_swapchain.extent()
        }

        #[inline]
        pub fn current_fence(&self) -> &RhiFence
        {
            &self.fence_frame_in_flight[self.frame_index]
        }

        #[inline]
        pub fn color_format(&self) -> vk::Format
        {
            self.render_swapchain.color_format()
        }

        #[inline]
        pub fn current_frame_index(&self) -> usize
        {
            self.frame_index
        }


        #[inline]
        pub fn depth_format(&self) -> vk::Format
        {
            unsafe { self.depth_format.unwrap_unchecked() }
        }


        #[inline]
        pub fn current_image_render_finish_semaphore(&self) -> RhiSemaphore
        {
            self.semaphores_image_render_finish[self.frame_index]
        }

        #[inline]
        pub fn current_swapchain_available_semaphore(&self) -> RhiSemaphore
        {
            self.semaphores_swapchain_available[self.frame_index]
        }

        #[inline]
        pub fn color_attach_info(&self) -> &vk::RenderingAttachmentInfo
        {
            &self.render_swapchain.color_attach_infos[self.swapchain_image_index]
        }

        #[inline]
        pub fn depth_attach_info(&self) -> &vk::RenderingAttachmentInfo
        {
            &self.depth_attach_info
        }

        #[inline]
        pub fn current_image(&self) -> vk::Image
        {
            self.render_swapchain.images[self.swapchain_image_index]
        }
    }
}
