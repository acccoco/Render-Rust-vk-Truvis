use ash::vk;
use itertools::Itertools;

use crate::framework::{
    basic::{color::LabelColor, FRAME_ID_MAP},
    core::{
        command_buffer::CommandBuffer,
        command_pool::CommandPool,
        create_utils::CreateUtils,
        queue::SubmitInfo,
        swapchain::{Swapchain, SwapchainInitInfo},
        synchronize::{Fence, Semaphore},
    },
    render_core::{Core, CORE},
};

pub struct RenderContext
{
    pub render_swapchain: Swapchain,

    swapchain_image_index: usize,

    current_frame: usize,
    pub frames_cnt: usize,

    pub frame_id: u64,

    /// 为每个 frame 分配一个 command pool
    graphics_command_pools: Vec<CommandPool>,

    /// 每个 command pool 已经分配出去的 command buffer，用于集中 free 或其他操作
    allocated_command_buffers: Vec<Vec<CommandBuffer>>,

    pub depth_format: vk::Format,
    pub depth_image: vk::Image,
    depth_image_allcation: vk_mem::Allocation,
    pub depth_image_view: vk::ImageView,

    present_complete_semaphores: Vec<Semaphore>,
    render_complete_semaphores: Vec<Semaphore>,
    fence_frame_in_flight: Vec<Fence>,

    rhi: &'static Core,
}

impl RenderContext
{
    pub fn new(
        rhi: &'static Core,
        init_info: &RenderContextInitInfo,
        render_swapchain_init_info: SwapchainInitInfo,
    ) -> Self
    {
        let render_swapchain = Swapchain::new(rhi, &render_swapchain_init_info);
        let (depth_format, depth_image, depth_image_allcation, depth_image_view) =
            Self::init_depth_image_and_view(rhi, &render_swapchain, &init_info.depth_format_dedicate);

        let create_semaphore = |name: &str| {
            (0..init_info.frames_in_flight)
                .map(|i| FRAME_ID_MAP[i])
                .map(|tag| Semaphore::new(rhi, format!("{name}_{tag}")))
                .collect_vec()
        };
        let present_complete_semaphores = create_semaphore("present_complete_semaphore");
        let render_complete_semaphores = create_semaphore("render_complete_semaphores");

        let fence_frame_in_flight = (0..init_info.frames_in_flight)
            .map(|i| FRAME_ID_MAP[i])
            .map(|tag| Fence::new(rhi, true, format!("frame_in_flight_fence_{tag}")))
            .collect();

        let graphics_command_pools = Self::init_command_pool(rhi, init_info);

        let ctx = Self {
            render_swapchain,

            swapchain_image_index: 0,
            current_frame: 0,
            frame_id: 0,
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
        rhi: &Core,
        swapchain: &Swapchain,
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
            let create_info = CreateUtils::make_image2d_create_info(
                swapchain.extent,
                depth_format,
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            );
            rhi.create_image(&create_info, "depth-image")
        };

        let depth_image_view = {
            let create_info = CreateUtils::make_image_view_2d_create_info(
                depth_image,
                depth_format,
                vk::ImageAspectFlags::DEPTH,
            );
            rhi.create_image_view(&create_info, "depth-image-view")
        };

        (depth_format, depth_image, depth_image_allocation, depth_image_view)
    }

    fn init_command_pool(rhi: &Core, init_info: &RenderContextInitInfo) -> Vec<CommandPool>
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

    /// 准备好渲染当前frame 所需的资源
    ///
    /// * 通过 fence 等待当前 frame 资源释放
    /// * 为 image 进行 layout transition 的操作
    pub fn acquire_frame(&mut self)
    {
        let rhi = CORE.get().unwrap();

        rhi.graphics_queue_begin_label("[acquire-frame]reset", LabelColor::COLOR_STAGE);
        {
            let current_fence = &self.fence_frame_in_flight[self.current_frame];
            rhi.wait_for_fence(current_fence);
            rhi.reset_fence(current_fence);

            // 释放当前 frame 的 command buffer 的资源
            std::mem::take(&mut self.allocated_command_buffers[self.current_frame]).into_iter().for_each(|c| c.free());

            // 这个调用并不会释放资源，而是将 pool 内的 command buffer 设置到初始状态
            rhi.reset_command_pool(&mut self.graphics_command_pools[self.current_frame]);
        }
        rhi.graphics_queue_end_label();

        self.swapchain_image_index =
            self.render_swapchain.acquire_next_frame(&self.present_complete_semaphores[self.current_frame], None)
                as usize;

        rhi.graphics_queue_begin_label("[acquire-frame]color-attach-transfer", LabelColor::COLOR_STAGE);
        {
            let mut cmd = self.alloc_command_buffer(format!(
                "{}-[acquire-frame]color-attach-layout-transfer",
                self.current_frame_prefix()
            ));
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[acquire]color-attach-layout-transfer");
            {
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
            }
            cmd.end();

            self.rhi.graphics_queue_submit(
                vec![SubmitInfo {
                    command_buffers: vec![cmd],
                    wait_info: vec![(
                        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                        self.current_present_complete_semaphore(),
                    )],
                    ..Default::default()
                }],
                None,
            );
        }
        rhi.graphics_queue_end_label();
    }


    /// 提交当前 frame
    ///
    /// * 在提交之前，为 image 进行 layout transition
    pub fn submit_frame(&mut self)
    {
        self.rhi.graphics_queue_begin_label("[submit-frame]", LabelColor::COLOR_PASS);
        {
            let mut cmd = self.alloc_command_buffer(format!(
                "{}-[submit-frame]color-attach-layout-transfer",
                self.current_frame_prefix()
            ));
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "color-attach-layout-transfer");
            {
                cmd.image_barrier(
                    (vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, vk::AccessFlags::COLOR_ATTACHMENT_WRITE),
                    (vk::PipelineStageFlags::BOTTOM_OF_PIPE, vk::AccessFlags::empty()),
                    self.current_present_image(),
                    vk::ImageAspectFlags::COLOR,
                    vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    vk::ImageLayout::PRESENT_SRC_KHR,
                );
            }
            cmd.end();

            self.rhi.graphics_queue_submit(
                vec![SubmitInfo {
                    command_buffers: vec![cmd],
                    signal_info: vec![self.current_render_complete_semaphore()],
                    ..Default::default()
                }],
                Some(self.fence_frame_in_flight[self.current_frame].clone()),
            );
        }
        // queue label 不能跨过 submit，否则会导致 Nsight mismatch label
        self.rhi.graphics_queue_end_label();

        self.render_swapchain.submit_frame(
            self.rhi,
            self.swapchain_image_index as u32,
            &[self.current_render_complete_semaphore().semaphore],
        );

        self.current_frame = (self.current_frame + 1) % self.frames_cnt;
        self.frame_id += 1;
    }


    /// 分配 command buffer，在当前 frame 使用
    pub fn alloc_command_buffer<S: AsRef<str>>(&mut self, debug_name: S) -> CommandBuffer
    {
        let name = format!("[frame-{}-{}]{}", FRAME_ID_MAP[self.current_frame], self.frame_id, debug_name.as_ref());
        let cmd = CommandBuffer::new(self.rhi, &self.graphics_command_pools[self.current_frame], name);

        self.allocated_command_buffers[self.current_frame].push(cmd.clone());

        cmd
    }

    /// 直接从 swapchain 获取 extent
    #[inline]
    pub fn swapchain_extent(&self) -> vk::Extent2D
    {
        self.render_swapchain.extent
    }

    #[inline]
    pub fn current_fence(&self) -> &Fence
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

    /// 当前帧的 debug prefix，例如：`[frame-A-113]`
    #[inline]
    pub fn current_frame_prefix(&self) -> String
    {
        format!("[frame-{}-{}]", FRAME_ID_MAP[self.current_frame], self.frame_id)
    }

    #[inline]
    pub fn depth_format(&self) -> vk::Format
    {
        self.depth_format
    }

    #[inline]
    pub fn current_render_complete_semaphore(&self) -> Semaphore
    {
        self.render_complete_semaphores[self.current_frame]
    }

    #[inline]
    pub fn current_present_complete_semaphore(&self) -> Semaphore
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
