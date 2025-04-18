use std::rc::Rc;

use ash::vk;
use itertools::Itertools;
use truvis_rhi::core::descriptor_pool::{RhiDescriptorPool, RhiDescriptorPoolCreateInfo};
use truvis_rhi::{
    basic::{color::LabelColor, FRAME_ID_MAP},
    core::{
        command_buffer::RhiCommandBuffer,
        command_pool::RhiCommandPool,
        command_queue::{RhiQueue, RhiSubmitInfo},
        device::RhiDevice,
        image::{RhiImage2D, RhiImage2DView, RhiImageCreateInfo, RhiImageViewCreateInfo},
        swapchain::{RhiSwapchain, RhiSwapchainInitInfo},
        synchronize::{RhiFence, RhiImageBarrier, RhiSemaphore},
    },
    rhi::Rhi,
};

pub struct RenderContext {
    pub render_swapchain: RhiSwapchain,

    swapchain_image_index: usize,

    current_frame: usize,
    pub frames_cnt: usize,

    pub frame_id: u64,

    /// 为每个 frame 分配一个 command pool
    graphics_command_pools: Vec<Rc<RhiCommandPool>>,

    /// 每个 command pool 已经分配出去的 command buffer，用于集中 free 或其他操作
    allocated_command_buffers: Vec<Vec<RhiCommandBuffer>>,

    descriptor_pool: RhiDescriptorPool,

    pub depth_format: vk::Format,
    pub depth_image: Rc<RhiImage2D>,
    pub depth_view: Rc<RhiImage2DView>,

    present_complete_semaphores: Vec<RhiSemaphore>,
    render_complete_semaphores: Vec<RhiSemaphore>,
    fence_frame_in_flight: Vec<RhiFence>,

    device: Rc<RhiDevice>,
    graphics_queue: Rc<RhiQueue>,
    _command_queue: Rc<RhiQueue>,
    _transfer_queue: Rc<RhiQueue>,
}

const DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT: u32 = 256;
const DESCRIPTOR_POOL_MAX_MATERIAL_CNT: u32 = 256;
const DESCRIPTOR_POOL_MAX_BINDLESS_TEXTURE_CNT: u32 = 128;

impl RenderContext {
    pub fn new(rhi: &Rhi, init_info: &RenderContextInitInfo, render_swapchain_init_info: RhiSwapchainInitInfo) -> Self {
        let render_swapchain = RhiSwapchain::new(rhi, &render_swapchain_init_info);
        let (depth_format, depth_image, depth_image_view) =
            Self::create_depth_image_and_view(rhi, &render_swapchain, &init_info.depth_format_dedicate);

        let create_semaphore = |name: &str| {
            (0..init_info.frames_in_flight)
                .map(|i| FRAME_ID_MAP[i])
                .map(|tag| RhiSemaphore::new(rhi, &format!("{name}_{tag}")))
                .collect_vec()
        };
        let present_complete_semaphores = create_semaphore("present_complete_semaphore");
        let render_complete_semaphores = create_semaphore("render_complete_semaphores");

        let fence_frame_in_flight = (0..init_info.frames_in_flight)
            .map(|i| FRAME_ID_MAP[i])
            .map(|tag| RhiFence::new(rhi, true, &format!("frame_in_flight_fence_{tag}")))
            .collect();

        let graphics_command_pools = Self::init_command_pool(rhi, init_info);
        let descriptor_pool = Self::init_descriptor_pool(&rhi.device);

        Self {
            render_swapchain,

            swapchain_image_index: 0,
            current_frame: 0,
            frame_id: 0,
            frames_cnt: init_info.frames_in_flight,

            graphics_command_pools,
            allocated_command_buffers: vec![Vec::new(); init_info.frames_in_flight],

            descriptor_pool,

            depth_format,
            depth_image,
            depth_view: depth_image_view,

            present_complete_semaphores,
            render_complete_semaphores,
            fence_frame_in_flight,

            device: rhi.device.clone(),
            graphics_queue: rhi.graphics_queue.clone(),
            _command_queue: rhi.compute_queue.clone(),
            _transfer_queue: rhi.transfer_queue.clone(),
        }
    }

    fn create_depth_image_and_view(
        rhi: &Rhi,
        swapchain: &RhiSwapchain,
        depth_format_dedicate: &[vk::Format],
    ) -> (vk::Format, Rc<RhiImage2D>, Rc<RhiImage2DView>) {
        let depth_format = rhi
            .find_supported_format(
                depth_format_dedicate,
                vk::ImageTiling::OPTIMAL,
                vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
            )
            .first()
            .copied()
            .unwrap();

        let depth_image = Rc::new(RhiImage2D::new(
            rhi,
            Rc::new(RhiImageCreateInfo::new_image_2d_info(
                swapchain.extent,
                depth_format,
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            )),
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            "depth-image",
        ));

        let depth_image_view = RhiImage2DView::new(
            rhi,
            depth_image.clone(),
            RhiImageViewCreateInfo::new_image_view_2d_info(depth_format, vk::ImageAspectFlags::DEPTH),
            "depth-image-view".to_string(),
        );

        (depth_format, depth_image, Rc::new(depth_image_view))
    }

    fn init_command_pool(rhi: &Rhi, init_info: &RenderContextInitInfo) -> Vec<Rc<RhiCommandPool>> {
        let graphics_command_pools = (0..init_info.frames_in_flight)
            .map(|i| {
                Rc::new(RhiCommandPool::new(
                    rhi,
                    rhi.device.graphics_queue_family_index,
                    vk::CommandPoolCreateFlags::TRANSIENT,
                    &format!("render_context_graphics_command_pool_{}", i),
                ))
            })
            .collect();

        graphics_command_pools
    }

    fn init_descriptor_pool(device: &RhiDevice) -> RhiDescriptorPool {
        let pool_size = vec![
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
                descriptor_count: 128,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: DESCRIPTOR_POOL_MAX_MATERIAL_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: DESCRIPTOR_POOL_MAX_MATERIAL_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::INPUT_ATTACHMENT,
                descriptor_count: 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
                descriptor_count: 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: DESCRIPTOR_POOL_MAX_BINDLESS_TEXTURE_CNT + 32,
            },
        ];

        let pool_ci = Rc::new(RhiDescriptorPoolCreateInfo::new(
            vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET | vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
            DESCRIPTOR_POOL_MAX_MATERIAL_CNT + DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT + 32,
            pool_size,
        ));

        RhiDescriptorPool::new(device, pool_ci, "ctx-descriptor-pool")
    }

    /// getter
    #[inline]
    pub fn graphics_queue(&self) -> &RhiQueue {
        &self.graphics_queue
    }

    /// 准备好渲染当前frame 所需的资源
    ///
    /// * 通过 fence 等待当前 frame 资源释放
    /// * 为 image 进行 layout transition 的操作
    pub fn acquire_frame(&mut self) {
        self.device.debug_utils.begin_queue_label(
            self.graphics_queue.handle,
            "[acquire-frame]",
            LabelColor::COLOR_STAGE,
        );
        {
            let current_fence = &self.fence_frame_in_flight[self.current_frame];
            current_fence.wait();
            current_fence.reset();

            // 释放当前 frame 的 command buffer 的资源
            std::mem::take(&mut self.allocated_command_buffers[self.current_frame]) //
                .into_iter()
                .for_each(|c| c.free());

            // 这个调用并不会释放资源，而是将 pool 内的 command buffer 设置到初始状态
            self.graphics_command_pools[self.current_frame].reset();
        }
        self.device.debug_utils.end_queue_label(self.graphics_queue.handle);

        self.swapchain_image_index =
            self.render_swapchain.acquire_next_frame(&self.present_complete_semaphores[self.current_frame], None)
                as usize;

        self.device.debug_utils.begin_queue_label(
            self.graphics_queue.handle,
            "[acquire-frame]color-attach-transfer",
            LabelColor::COLOR_STAGE,
        );
        {
            let cmd = self.alloc_command_buffer(&format!(
                "{}-[acquire-frame]color-attach-layout-transfer",
                self.current_frame_prefix()
            ));
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[acquire]color-attach-layout-transfer");
            {
                // 只需要建立起执行依赖即可，确保 present 完成后，再进行 layout trans
                // COLOR_ATTACHMENT_READ 对应 blend 等操作
                let image_barrier = RhiImageBarrier::new()
                    .src_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                    .dst_mask(
                        vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        vk::AccessFlags2::COLOR_ATTACHMENT_WRITE | vk::AccessFlags2::COLOR_ATTACHMENT_READ,
                    )
                    .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                    .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .image(self.current_present_image());
                cmd.image_memory_barrier(vk::DependencyFlags::empty(), std::slice::from_ref(&image_barrier));
            }
            cmd.end();

            self.graphics_queue.submit(
                vec![RhiSubmitInfo::new(&[cmd]).wait_infos(&[(
                    self.current_present_complete_semaphore(),
                    vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                )])],
                None,
            );
        }
        self.device.debug_utils.end_queue_label(self.graphics_queue.handle);
    }

    /// 提交当前 frame
    ///
    /// * 在提交之前，为 image 进行 layout transition
    pub fn submit_frame(&mut self) {
        self.device.debug_utils.begin_queue_label(self.graphics_queue.handle, "[submit-frame]", LabelColor::COLOR_PASS);
        {
            let cmd = self.alloc_command_buffer(&format!(
                "{}-[submit-frame]color-attach-layout-transfer",
                self.current_frame_prefix()
            ));
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "color-attach-layout-transfer");
            {
                let image_barrier = RhiImageBarrier::new()
                    .src_mask(
                        vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                    )
                    .dst_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                    .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                    .layout_transfer(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR)
                    .image(self.current_present_image());
                cmd.image_memory_barrier(vk::DependencyFlags::empty(), std::slice::from_ref(&image_barrier));
            }
            cmd.end();

            self.graphics_queue.submit(
                vec![RhiSubmitInfo::new(&[cmd]).signal_infos(&[(
                    self.current_render_complete_semaphore(),
                    vk::PipelineStageFlags2::BOTTOM_OF_PIPE, /*TODO 需要确认 signal 的 stage*/
                )])],
                Some(self.fence_frame_in_flight[self.current_frame].clone()),
            );
        }
        // queue label 不能跨过 submit，否则会导致 Nsight mismatch label
        self.device.debug_utils.end_queue_label(self.graphics_queue.handle);

        self.render_swapchain.submit_frame(
            &self.graphics_queue,
            self.swapchain_image_index as u32,
            &[self.current_render_complete_semaphore()],
        );

        self.current_frame = (self.current_frame + 1) % self.frames_cnt;
        self.frame_id += 1;
    }

    /// 分配 command buffer，在当前 frame 使用
    pub fn alloc_command_buffer(&mut self, debug_name: &str) -> RhiCommandBuffer {
        let name = format!("[frame-{}-{}]{}", FRAME_ID_MAP[self.current_frame], self.frame_id, debug_name);
        let cmd =
            RhiCommandBuffer::new(self.device.clone(), self.graphics_command_pools[self.current_frame].clone(), &name);

        self.allocated_command_buffers[self.current_frame].push(cmd.clone());

        cmd
    }

    /// getter
    #[inline]
    pub fn descriptor_pool(&self) -> &RhiDescriptorPool {
        &self.descriptor_pool
    }

    /// 直接从 swapchain 获取 extent
    #[inline]
    pub fn swapchain_extent(&self) -> vk::Extent2D {
        self.render_swapchain.extent
    }

    #[inline]
    pub fn current_fence(&self) -> &RhiFence {
        &self.fence_frame_in_flight[self.current_frame]
    }

    #[inline]
    pub fn color_format(&self) -> vk::Format {
        self.render_swapchain.color_format
    }

    #[inline]
    pub fn current_frame_index(&self) -> usize {
        self.current_frame
    }

    /// 当前帧的 debug prefix，例如：`[frame-A-113]`
    #[inline]
    pub fn current_frame_prefix(&self) -> String {
        format!("[frame-{}-{}]", FRAME_ID_MAP[self.current_frame], self.frame_id)
    }

    #[inline]
    pub fn depth_format(&self) -> vk::Format {
        self.depth_format
    }

    #[inline]
    pub fn current_render_complete_semaphore(&self) -> RhiSemaphore {
        self.render_complete_semaphores[self.current_frame].clone()
    }

    #[inline]
    pub fn current_present_complete_semaphore(&self) -> RhiSemaphore {
        self.present_complete_semaphores[self.current_frame].clone()
    }

    /// 当前帧从 swapchain 获取到的用于 present 的 image
    #[inline]
    pub fn current_present_image(&self) -> vk::Image {
        self.render_swapchain.images[self.swapchain_image_index]
    }

    #[inline]
    pub fn current_present_image_view(&self) -> vk::ImageView {
        self.render_swapchain.image_views[self.swapchain_image_index]
    }
}

pub struct RenderContextInitInfo {
    frames_in_flight: usize,
    depth_format_dedicate: Vec<vk::Format>,
}

impl Default for RenderContextInitInfo {
    fn default() -> Self {
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
