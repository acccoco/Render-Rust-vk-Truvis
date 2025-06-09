use crate::renderer::pipeline_settings::{DefaultRendererSettings, FifLabel, FrameSettings, PipelineSettings};
use crate::renderer::bindless::BindlessManager;
use crate::renderer::swapchain::RenderSwapchain;
use crate::renderer::window_system::MainWindow;
use ash::vk;
use itertools::Itertools;
use shader_binding::shader;
use std::rc::Rc;
use truvis_rhi::{
    core::{
        command_buffer::RhiCommandBuffer,
        command_pool::RhiCommandPool,
        device::RhiDevice,
        image::{RhiImage2D, RhiImage2DView, RhiImageCreateInfo, RhiImageViewCreateInfo},
        synchronize::{RhiFence, RhiImageBarrier, RhiSemaphore},
    },
    rhi::Rhi,
};

pub struct FrameContext {
    render_swapchain: RenderSwapchain,

    frame_settings: FrameSettings,

    /// 当前处在 in-flight 的第几帧：A, B, C
    fif_label: FifLabel,

    /// 当前的帧序号，一直累加
    frame_id: usize,

    /// 为每个 frame 分配一个 command pool
    graphics_command_pools: Vec<Rc<RhiCommandPool>>,

    /// 每个 command pool 已经分配出去的 command buffer，用于集中 free 或其他操作
    allocated_command_buffers: Vec<Vec<RhiCommandBuffer>>,

    /// 来自于 swpachain，生命周期跟随 swapchain
    present_images: Vec<vk::Image>,
    present_image_views: Vec<Rc<RhiImage2DView>>,
    present_image_bindless_keys: Vec<String>,

    /// FIXME 听说可以只需要一个 depth view，因为不需要同时渲染两帧
    _depth_image: Rc<RhiImage2D>,
    _depth_view: Rc<RhiImage2DView>,

    present_complete_semaphores: Vec<RhiSemaphore>,
    render_complete_semaphores: Vec<RhiSemaphore>,
    fence_frame_in_flight: Vec<RhiFence>,

    rt_image: Rc<RhiImage2D>,
    _rt_image_view: Rc<RhiImage2DView>,
    rt_bindless_key: String,

    device: Rc<RhiDevice>,
}
// init
impl FrameContext {
    pub fn new(
        rhi: &Rhi,
        window: &MainWindow,
        pipeline_settings: &PipelineSettings,
        bindless_mgr: &mut BindlessManager,
    ) -> Self {
        let swapchain = RenderSwapchain::new(
            rhi,
            window,
            DefaultRendererSettings::DEFAULT_PRESENT_MODE,
            DefaultRendererSettings::DEFAULT_SURFACE_FORMAT,
        );
        let present_images = swapchain.present_images();
        let present_image_views = present_images
            .iter()
            .enumerate()
            .map(|(idx, image)| {
                Rc::new(RhiImage2DView::new_with_raw_image(
                    rhi,
                    *image,
                    RhiImageViewCreateInfo::new_image_view_2d_info(
                        pipeline_settings.color_format,
                        vk::ImageAspectFlags::COLOR,
                    ),
                    format!("swapchain-present-{idx}"),
                ))
            })
            .collect_vec();
        let present_image_bindless_keys =
            present_image_views.iter().enumerate().map(|(idx, _)| format!("present-image-{}", idx)).collect_vec();
        for (key, view) in present_image_bindless_keys.iter().zip(present_image_views.iter()) {
            bindless_mgr.register_image(key.clone(), view.clone());
        }

        let frame_settings = FrameSettings {
            viewport_extent: swapchain.extent(),
            rt_extent: swapchain.extent(),
            rt_offset: vk::Offset2D { x: 0, y: 0 },
        };

        let (depth_image, depth_image_view) =
            Self::create_depth_image_and_view(rhi, frame_settings.viewport_extent, pipeline_settings.depth_format);

        let create_semaphore = |name: &str| {
            (0..pipeline_settings.frames_in_flight)
                .map(|i| FifLabel::from_usize(i))
                .map(|frame_label| RhiSemaphore::new(rhi, &format!("{name}_{frame_label}")))
                .collect_vec()
        };
        let present_complete_semaphores = create_semaphore("present_complete_semaphore");
        let render_complete_semaphores = create_semaphore("render_complete_semaphores");

        let fence_frame_in_flight = (0..pipeline_settings.frames_in_flight)
            .map(|i| FifLabel::from_usize(i))
            .map(|frame_label| RhiFence::new(rhi, true, &format!("frame_in_flight_fence_{frame_label}")))
            .collect();

        let graphics_command_pools = Self::init_command_pool(rhi, pipeline_settings.frames_in_flight);
        let (rt_image, rt_image_view) =
            Self::create_rt_images(rhi, pipeline_settings.color_format, frame_settings.rt_extent);
        let rt_keyword = "rt-image".to_string();
        bindless_mgr.register_image(rt_keyword.clone(), rt_image_view.clone());

        Self {
            render_swapchain: swapchain,
            present_images,
            present_image_views,
            present_image_bindless_keys,

            frame_settings,

            fif_label: FifLabel::A, // 初始为 A
            frame_id: 0,

            graphics_command_pools,
            allocated_command_buffers: vec![Vec::new(); pipeline_settings.frames_in_flight],

            _depth_image: depth_image,
            _depth_view: depth_image_view,

            rt_image,
            _rt_image_view: rt_image_view,
            rt_bindless_key: rt_keyword,

            present_complete_semaphores,
            render_complete_semaphores,
            fence_frame_in_flight,

            device: rhi.device.clone(),
        }
    }

    /// 需要手动调用该函数释放资源
    pub fn destroy(mut self, bindless_mgr: &mut BindlessManager) {
        for semaphore in std::mem::take(&mut self.present_complete_semaphores).into_iter() {
            semaphore.destroy();
        }
        for semaphore in std::mem::take(&mut self.render_complete_semaphores).into_iter() {
            semaphore.destroy();
        }
        for fence in std::mem::take(&mut self.fence_frame_in_flight).into_iter() {
            fence.destroy();
        }

        bindless_mgr.unregister_image(&self.rt_bindless_key);

        for present_image_view in std::mem::take(&mut self.present_image_views).into_iter() {
            drop(present_image_view);
        }
        for present_image_bindless_key in std::mem::take(&mut self.present_image_bindless_keys).into_iter() {
            bindless_mgr.unregister_image(&present_image_bindless_key);
        }
    }

    fn create_depth_image_and_view(
        rhi: &Rhi,
        extent: vk::Extent2D,
        depth_format: vk::Format,
    ) -> (Rc<RhiImage2D>, Rc<RhiImage2DView>) {
        let depth_image = Rc::new(RhiImage2D::new(
            rhi,
            Rc::new(RhiImageCreateInfo::new_image_2d_info(
                extent,
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

        (depth_image, Rc::new(depth_image_view))
    }

    /// rt_extent: 表示的是 rt 渲染的分辨率，并不是最终 framebuffer 的分辨率
    fn create_rt_images(
        rhi: &Rhi,
        color_format: vk::Format,
        rt_extent: vk::Extent2D,
    ) -> (Rc<RhiImage2D>, Rc<RhiImage2DView>) {
        let rt_image = Rc::new(RhiImage2D::new(
            rhi,
            Rc::new(RhiImageCreateInfo::new_image_2d_info(
                rt_extent,
                color_format,
                vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED,
            )),
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            "rt-image",
        ));

        let rt_image_view = Rc::new(RhiImage2DView::new(
            rhi,
            rt_image.clone(),
            RhiImageViewCreateInfo::new_image_view_2d_info(color_format, vk::ImageAspectFlags::COLOR),
            "rt-image-view".to_string(),
        ));

        // layout transfer
        RhiCommandBuffer::one_time_exec(
            rhi,
            rhi.graphics_command_pool.clone(),
            &rhi.graphics_queue,
            |cmd| {
                let barrier = RhiImageBarrier::new()
                    .image(rt_image.handle())
                    .src_mask(vk::PipelineStageFlags2::TOP_OF_PIPE, vk::AccessFlags2::empty())
                    .dst_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                    .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL)
                    .image_aspect_flag(vk::ImageAspectFlags::COLOR);

                cmd.image_memory_barrier(vk::DependencyFlags::empty(), std::slice::from_ref(&barrier));
            },
            "transfer-rt-image-layout",
        );

        (rt_image, rt_image_view)
    }

    fn init_command_pool(rhi: &Rhi, frames_in_flight: usize) -> Vec<Rc<RhiCommandPool>> {
        (0..frames_in_flight)
            .map(|i| {
                Rc::new(RhiCommandPool::new(
                    rhi.device.clone(),
                    rhi.graphics_queue_family(),
                    vk::CommandPoolCreateFlags::TRANSIENT,
                    &format!("render_context_graphics_command_pool_{}", i),
                ))
            })
            .collect_vec()
    }
}
// getter
impl FrameContext {
    #[inline]
    pub fn crt_fence(&self) -> &RhiFence {
        &self.fence_frame_in_flight[*self.fif_label]
    }

    /// 当前处在第几帧：A, B, C
    #[inline]
    pub fn crt_frame_label(&self) -> FifLabel {
        self.fif_label
    }

    /// 当前帧的编号，一直增加
    #[inline]
    pub fn crt_frame_id(&self) -> usize {
        self.frame_id
    }

    /// 当前帧的 debug prefix，例如：`[frame-A-113]`
    #[inline]
    pub fn crt_frame_prefix(&self) -> String {
        format!("[F{}{}]", self.frame_id, self.fif_label)
    }

    #[inline]
    pub fn crt_render_complete_semaphore(&self) -> RhiSemaphore {
        self.render_complete_semaphores[*self.fif_label].clone()
    }

    #[inline]
    pub fn current_present_complete_semaphore(&self) -> RhiSemaphore {
        self.present_complete_semaphores[*self.fif_label].clone()
    }

    #[inline]
    pub fn depth_view(&self) -> &RhiImage2DView {
        &self._depth_view
    }

    #[inline]
    pub fn crt_present_image_view(&self) -> &RhiImage2DView {
        let present_image_idx = self.render_swapchain.current_present_image_index();
        &self.present_image_views[present_image_idx]
    }

    #[inline]
    pub fn crt_present_image(&self) -> vk::Image {
        let present_image_idx = self.render_swapchain.current_present_image_index();
        self.present_images[present_image_idx]
    }

    #[inline]
    pub fn crt_present_image_bindless_handle(&self, bindless_manager: &BindlessManager) -> shader::ImageHandle {
        let present_image_idx = self.render_swapchain.current_present_image_index();
        bindless_manager.get_image_idx(&self.present_image_bindless_keys[present_image_idx]).unwrap()
    }

    #[inline]
    pub fn crt_rt_bindless_handle(&self, bindless_manager: &BindlessManager) -> shader::ImageHandle {
        bindless_manager.get_image_idx(&self.rt_bindless_key).unwrap()
    }

    #[inline]
    pub fn crt_rt_image_view(&self) -> &RhiImage2D {
        &self.rt_image
    }

    #[inline]
    pub fn crt_rt_image(&self) -> &RhiImage2D {
        &self.rt_image
    }

    #[inline]
    pub fn frame_settings(&self) -> FrameSettings {
        self.frame_settings
    }
}
// tools
impl FrameContext {
    /// 分配 command buffer，在当前 frame 使用
    pub fn alloc_command_buffer(&mut self, debug_name: &str) -> RhiCommandBuffer {
        let name = format!("[frame-{}-{}]{}", self.fif_label, self.frame_id, debug_name);
        let cmd =
            RhiCommandBuffer::new(self.device.clone(), self.graphics_command_pools[*self.fif_label].clone(), &name);

        self.allocated_command_buffers[*self.fif_label].push(cmd.clone());
        cmd
    }
}
// phase methods
impl FrameContext {
    pub fn begin_frame(&mut self) {
        {
            let current_fence = &self.fence_frame_in_flight[*self.fif_label];
            current_fence.wait();
            current_fence.reset();

            // 释放当前 frame 的 command buffer 的资源
            std::mem::take(&mut self.allocated_command_buffers[*self.fif_label]) //
                .into_iter()
                .for_each(|cmd| cmd.free());

            // 这个调用并不会释放资源，而是将 pool 内的 command buffer 设置到初始状态
            self.graphics_command_pools[*self.fif_label].reset_all_buffers();
        }

        let crt_present_complete_semaphore = self.current_present_complete_semaphore();
        self.render_swapchain.acquire(&crt_present_complete_semaphore, None);
    }

    pub fn before_render(&mut self) {}

    pub fn end_frame(&mut self, rhi: &Rhi) {
        self.render_swapchain.submit(&rhi.graphics_queue, &[self.crt_render_complete_semaphore()]);

        self.fif_label.next_frame();
        self.frame_id += 1;
    }

    pub fn after_render(&mut self) {}
}
impl Drop for FrameContext {
    fn drop(&mut self) {
        assert!(self.present_complete_semaphores.is_empty(), "need destroy render context manually");
        assert!(self.render_complete_semaphores.is_empty(), "need destroy render context manually");
        assert!(self.fence_frame_in_flight.is_empty(), "need destroy render context manually");
    }
}
