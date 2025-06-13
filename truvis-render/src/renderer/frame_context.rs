use crate::renderer::bindless::BindlessManager;
use crate::renderer::pipeline_settings::{DefaultRendererSettings, FifLabel, FrameSettings, PipelineSettings};
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

/// 各种各样和 frame 以及 viewport 相关的 buffers
struct FrameBuffers {
    rt_image: Rc<RhiImage2D>,
    _rt_image_view: Rc<RhiImage2DView>,
    rt_bindless_key: String,

    /// 来自于 swpachain，生命周期跟随 swapchain
    present_images: Vec<vk::Image>,
    present_image_views: Vec<Rc<RhiImage2DView>>,
    present_image_bindless_keys: Vec<String>,

    _depth_image: Rc<RhiImage2D>,
    _depth_view: Rc<RhiImage2DView>,
}
impl Drop for FrameBuffers {
    fn drop(&mut self) {
        assert_eq!(Rc::strong_count(&self._rt_image_view), 1);
        assert_eq!(Rc::strong_count(&self.rt_image), 2); // 1 for self, 1 for image view
        assert_eq!(Rc::strong_count(&self._depth_view), 1);
        assert_eq!(Rc::strong_count(&self._depth_image), 2); // 1 for self, 1 for image view
        for present_image_view in &self.present_image_views {
            assert_eq!(Rc::strong_count(present_image_view), 1);
        }
    }
}
impl FrameBuffers {
    pub fn new(
        rhi: &Rhi,
        pipeline_settings: &PipelineSettings,
        frame_settings: &FrameSettings,
        bindless_mgr: &mut BindlessManager,
        swapchain: &RenderSwapchain,
    ) -> Self {
        let (present_images, present_image_views) = Self::create_present_images(rhi, swapchain, pipeline_settings);
        let (depth_image, depth_image_view) = Self::create_depth_image(rhi, pipeline_settings, frame_settings);
        let (rt_image, rt_image_view) = Self::create_rt_image(rhi, pipeline_settings, frame_settings);

        // 将相关的 image 注册到 bindless manager 中
        let rt_bindless_key = "Renderer::RtImageView".to_string();
        let present_image_bindless_keys = present_image_views
            .iter()
            .enumerate()
            .map(|(idx, _)| format!("Renderer::PresentImageView-{idx}"))
            .collect_vec();
        bindless_mgr.register_image(rt_bindless_key.clone(), rt_image_view.clone());
        for (key, view) in present_image_bindless_keys.iter().zip(present_image_views.iter()) {
            bindless_mgr.register_image(key.clone(), view.clone());
        }

        Self {
            rt_image,
            _rt_image_view: rt_image_view,
            rt_bindless_key,

            present_images,
            present_image_views,
            present_image_bindless_keys,

            _depth_image: depth_image,
            _depth_view: depth_image_view,
        }
    }

    pub fn unregister_bindless_images(&self, bindless_mgr: &mut BindlessManager) {
        bindless_mgr.unregister_image(&self.rt_bindless_key);
        for key in &self.present_image_bindless_keys {
            bindless_mgr.unregister_image(key);
        }
    }

    /// 渲染区域发生变化时，重建一些 buffers
    pub fn on_draw_area_resized(
        &mut self,
        rhi: &Rhi,
        pipeline_settings: &PipelineSettings,
        frame_settings: &FrameSettings,
        bindless_mgr: &mut BindlessManager,
    ) {
        log::info!("rebuild rt image: {:?}", frame_settings.rt_extent);

        // 移除 bindless 中记录的 rt images
        bindless_mgr.unregister_image(&self.rt_bindless_key);
        assert_eq!(Rc::strong_count(&self._rt_image_view), 1);
        assert_eq!(Rc::strong_count(&self.rt_image), 2); // 1 for self, 1 for image view

        let (rt_image, rt_image_view) = Self::create_rt_image(rhi, pipeline_settings, frame_settings);
        self.rt_image = rt_image;
        self._rt_image_view = rt_image_view;
        bindless_mgr.register_image(self.rt_bindless_key.clone(), self._rt_image_view.clone());
    }

    /// 创建用于 present 的 image 和 view
    fn create_present_images(
        rhi: &Rhi,
        swapchain: &RenderSwapchain,
        pipeline_settings: &PipelineSettings,
    ) -> (Vec<vk::Image>, Vec<Rc<RhiImage2DView>>) {
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

        (present_images, present_image_views)
    }

    /// 创建深度图像和视图
    fn create_depth_image(
        rhi: &Rhi,
        pipeline_settings: &PipelineSettings,
        frame_settings: &FrameSettings,
    ) -> (Rc<RhiImage2D>, Rc<RhiImage2DView>) {
        let depth_image = Rc::new(RhiImage2D::new(
            rhi,
            Rc::new(RhiImageCreateInfo::new_image_2d_info(
                frame_settings.viewport_extent,
                pipeline_settings.depth_format,
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            )),
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            "depth",
        ));

        let depth_image_view = RhiImage2DView::new(
            rhi,
            depth_image.clone(),
            RhiImageViewCreateInfo::new_image_view_2d_info(pipeline_settings.depth_format, vk::ImageAspectFlags::DEPTH),
            "depth".to_string(),
        );

        (depth_image, Rc::new(depth_image_view))
    }

    /// 创建 RayTracing 需要的 image
    fn create_rt_image(
        rhi: &Rhi,
        pipeline_settings: &PipelineSettings,
        frame_settings: &FrameSettings,
    ) -> (Rc<RhiImage2D>, Rc<RhiImage2DView>) {
        let rt_image = Rc::new(RhiImage2D::new(
            rhi,
            Rc::new(RhiImageCreateInfo::new_image_2d_info(
                frame_settings.rt_extent,
                pipeline_settings.color_format,
                vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED,
            )),
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            "rt",
        ));

        let rt_image_view = Rc::new(RhiImage2DView::new(
            rhi,
            rt_image.clone(),
            RhiImageViewCreateInfo::new_image_view_2d_info(pipeline_settings.color_format, vk::ImageAspectFlags::COLOR),
            "rt".to_string(),
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
}

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

    frame_buffers: FrameBuffers,

    present_complete_semaphores: Vec<RhiSemaphore>,
    render_complete_semaphores: Vec<RhiSemaphore>,
    fence_frame_in_flight: Vec<RhiFence>,

    device: Rc<RhiDevice>,
}
impl Drop for FrameContext {
    fn drop(&mut self) {
        assert!(self.present_complete_semaphores.is_empty(), "need destroy render context manually");
        assert!(self.render_complete_semaphores.is_empty(), "need destroy render context manually");
        assert!(self.fence_frame_in_flight.is_empty(), "need destroy render context manually");
    }
}
// region init
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
        let frame_settings = FrameSettings {
            viewport_extent: swapchain.extent(),
            rt_extent: swapchain.extent(),
            rt_offset: vk::Offset2D { x: 0, y: 0 },
        };

        let present_complete_semaphores = (0..pipeline_settings.frames_in_flight)
            .map(|i| RhiSemaphore::new(rhi, &format!("present_complete_{}", FifLabel::from_usize(i))))
            .collect_vec();
        let render_complete_semaphores = (0..pipeline_settings.frames_in_flight)
            .map(|i| RhiSemaphore::new(rhi, &format!("render_complete_{}", FifLabel::from_usize(i))))
            .collect_vec();
        let fence_frame_in_flight = (0..pipeline_settings.frames_in_flight)
            .map(|i| RhiFence::new(rhi, true, &format!("frame_in_flight_fence_{}", FifLabel::from_usize(i))))
            .collect();

        let graphics_command_pools = Self::init_command_pool(rhi, pipeline_settings.frames_in_flight);

        let frame_buffers = FrameBuffers::new(rhi, pipeline_settings, &frame_settings, bindless_mgr, &swapchain);

        Self {
            render_swapchain: swapchain,
            frame_settings,
            fif_label: FifLabel::A,
            frame_id: 0,
            frame_buffers,
            graphics_command_pools,
            allocated_command_buffers: vec![Vec::new(); pipeline_settings.frames_in_flight],
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

        self.frame_buffers.unregister_bindless_images(bindless_mgr);
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
// endregion
// region getter
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
        &self.frame_buffers._depth_view
    }

    #[inline]
    pub fn crt_present_image_view(&self) -> &RhiImage2DView {
        let present_image_idx = self.render_swapchain.current_present_image_index();
        &self.frame_buffers.present_image_views[present_image_idx]
    }

    #[inline]
    pub fn crt_present_image(&self) -> vk::Image {
        let present_image_idx = self.render_swapchain.current_present_image_index();
        self.frame_buffers.present_images[present_image_idx]
    }

    #[inline]
    pub fn crt_present_image_bindless_handle(&self, bindless_manager: &BindlessManager) -> shader::ImageHandle {
        let present_image_idx = self.render_swapchain.current_present_image_index();
        bindless_manager.get_image_idx(&self.frame_buffers.present_image_bindless_keys[present_image_idx]).unwrap()
    }

    #[inline]
    pub fn crt_rt_bindless_handle(&self, bindless_manager: &BindlessManager) -> shader::ImageHandle {
        bindless_manager.get_image_idx(&self.frame_buffers.rt_bindless_key).unwrap()
    }

    #[inline]
    pub fn crt_rt_image_view(&self) -> &RhiImage2DView {
        &self.frame_buffers._rt_image_view
    }

    #[inline]
    pub fn crt_rt_image(&self) -> &RhiImage2D {
        &self.frame_buffers.rt_image
    }

    #[inline]
    pub fn frame_settings(&self) -> FrameSettings {
        self.frame_settings
    }
}
// endregion
// region tools
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
// endregion
// region phase methods
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

    /// 在渲染区域发生变化后，调整 rt buffers 的大小
    pub fn on_render_area_changed(
        &mut self,
        rhi: &Rhi,
        pipeline_settings: &PipelineSettings,
        new_rect: vk::Rect2D,
        bindless_manager: &mut BindlessManager,
    ) {
        self.frame_settings.rt_offset = new_rect.offset;

        // 仅有 size 发生改变时，才需要重建 rt-image
        if self.frame_settings.rt_extent.width != new_rect.extent.width
            || self.frame_settings.rt_extent.height != new_rect.extent.height
        {
            self.frame_settings.rt_extent = new_rect.extent;
            // 重新创建 rt image
            self.frame_buffers.on_draw_area_resized(rhi, pipeline_settings, &self.frame_settings, bindless_manager);
        }
    }
}
// endregion
