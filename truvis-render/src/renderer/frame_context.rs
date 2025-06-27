use crate::pipeline_settings::{FrameLabel, FrameSettings};
use crate::platform::timer::Timer;
use crate::renderer::bindless::BindlessManager;
use ash::vk;
use itertools::Itertools;
use shader_binding::shader;
use std::rc::Rc;
use truvis_rhi::core::synchronize::RhiSemaphore;
use truvis_rhi::{
    core::{
        command_buffer::RhiCommandBuffer,
        command_pool::RhiCommandPool,
        device::RhiDevice,
        image::{RhiImage2D, RhiImage2DView, RhiImageCreateInfo, RhiImageViewCreateInfo},
        synchronize::RhiImageBarrier,
    },
    rhi::Rhi,
};

/// 各种各样和 frame 以及 viewport 相关的 buffers
struct FrameBuffers {
    rt_images: Vec<Rc<RhiImage2D>>,
    rt_image_views: Vec<Rc<RhiImage2DView>>,
    rt_bindless_keys: Vec<String>,

    _depth_image: Rc<RhiImage2D>,
    _depth_view: Rc<RhiImage2DView>,
}
impl Drop for FrameBuffers {
    fn drop(&mut self) {
        assert_eq!(Rc::strong_count(&self._depth_view), 1);
        assert_eq!(Rc::strong_count(&self._depth_image), 2); // 1 for self, 1 for image view
        for rt_image in &self.rt_images {
            assert_eq!(Rc::strong_count(rt_image), 2); // 1 for self, 1 for image view
        }
        for rt_view in &self.rt_image_views {
            assert_eq!(Rc::strong_count(rt_view), 1);
        }
    }
}
impl FrameBuffers {
    pub fn new(rhi: &Rhi, frame_settings: &FrameSettings, bindless_mgr: &mut BindlessManager) -> Self {
        let (depth_image, depth_image_view) =
            Self::create_depth_image(rhi, frame_settings.depth_format, frame_settings.frame_extent);
        let (rt_images, rt_image_views) = Self::create_rt_image(rhi, frame_settings);

        // 将相关的 image 注册到 bindless manager 中
        let rt_bindless_keys = rt_images
            .iter()
            .enumerate()
            .map(|(i, _)| format!("Renderer::RtImageView_{}", FrameLabel::from_usize(i)))
            .collect_vec();
        for (rt_bindless_key, rt_image_view) in rt_bindless_keys.iter().zip(rt_image_views.iter()) {
            bindless_mgr.register_image(rt_bindless_key.clone(), rt_image_view.clone());
        }

        Self {
            rt_images,
            rt_image_views,
            rt_bindless_keys,

            _depth_image: depth_image,
            _depth_view: depth_image_view,
        }
    }

    pub fn unregister_bindless(&self, bindless_mgr: &mut BindlessManager) {
        for rt_bindless_key in &self.rt_bindless_keys {
            bindless_mgr.unregister_image(rt_bindless_key);
        }
    }

    /// 创建深度图像和视图
    fn create_depth_image(
        rhi: &Rhi,
        depth_format: vk::Format,
        frame_extent: vk::Extent2D,
    ) -> (Rc<RhiImage2D>, Rc<RhiImage2DView>) {
        let depth_image = Rc::new(RhiImage2D::new(
            rhi,
            Rc::new(RhiImageCreateInfo::new_image_2d_info(
                frame_extent,
                depth_format,
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
            RhiImageViewCreateInfo::new_image_view_2d_info(depth_format, vk::ImageAspectFlags::DEPTH),
            "depth".to_string(),
        );

        (depth_image, Rc::new(depth_image_view))
    }

    /// 创建 RayTracing 需要的 image
    fn create_rt_image(rhi: &Rhi, frame_settings: &FrameSettings) -> (Vec<Rc<RhiImage2D>>, Vec<Rc<RhiImage2DView>>) {
        let rt_images = (0..frame_settings.fif_num)
            .map(|i| {
                Rc::new(RhiImage2D::new(
                    rhi,
                    Rc::new(RhiImageCreateInfo::new_image_2d_info(
                        frame_settings.frame_extent,
                        frame_settings.color_format,
                        vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED,
                    )),
                    &vk_mem::AllocationCreateInfo {
                        usage: vk_mem::MemoryUsage::AutoPreferDevice,
                        ..Default::default()
                    },
                    &format!("rt-{}", FrameLabel::from_usize(i)),
                ))
            })
            .collect_vec();

        let rt_image_views = rt_images
            .iter()
            .enumerate()
            .map(|(i, rt_image)| {
                Rc::new(RhiImage2DView::new(
                    rhi,
                    rt_image.clone(),
                    RhiImageViewCreateInfo::new_image_view_2d_info(
                        frame_settings.color_format,
                        vk::ImageAspectFlags::COLOR,
                    ),
                    format!("rt-{}", FrameLabel::from_usize(i)),
                ))
            })
            .collect_vec();

        // layout transfer
        RhiCommandBuffer::one_time_exec(
            rhi,
            rhi.graphics_command_pool.clone(),
            &rhi.graphics_queue,
            |cmd| {
                let barriers = rt_images
                    .iter()
                    .map(|rt_image| {
                        RhiImageBarrier::new()
                            .image(rt_image.handle())
                            .src_mask(vk::PipelineStageFlags2::TOP_OF_PIPE, vk::AccessFlags2::empty())
                            .dst_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                            .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL)
                            .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                    })
                    .collect_vec();

                cmd.image_memory_barrier(vk::DependencyFlags::empty(), &barriers);
            },
            "transfer-rt-image-layout",
        );

        (rt_images, rt_image_views)
    }
}

pub struct RendererData<'a> {
    pub image: &'a RhiImage2DView,
    pub image_bindless_key: String,
    pub wait_timeline_value: u64,
    pub wait_timeline_semaphore: &'a RhiSemaphore,
    pub signal_timeline_semaphore: &'a RhiSemaphore,
}

pub struct FrameContext {
    /// 当前处在 in-flight 的第几帧：A, B, C
    fif_label: FrameLabel,

    /// 当前的帧序号，一直累加，初始序号是 1
    frame_id: usize,

    /// 发生重建时，当时的帧序号
    rebuild_frame_id: usize,
    fif_count: usize,

    timer: Timer,
    fps_limit: f32,

    /// 为每个 frame 分配一个 command pool
    graphics_command_pools: Vec<Rc<RhiCommandPool>>,

    /// 每个 command pool 已经分配出去的 command buffer，用于集中 free 或其他操作
    allocated_command_buffers: Vec<Vec<RhiCommandBuffer>>,

    frame_buffers: FrameBuffers,

    /// 帧渲染完成的 timeline，value 就等于 frame_id
    render_timeline_semaphore: RhiSemaphore,

    /// 渲染帧依赖的外部时间线
    present_timeline_semaphore: RhiSemaphore,

    /// 每一帧依赖的外部时间线的 value
    frame_present_time_value: Vec<Vec<u64>>,

    device: Rc<RhiDevice>,
}
impl Drop for FrameContext {
    fn drop(&mut self) {}
}
impl FrameContext {
    // region ctor

    pub fn new(rhi: &Rhi, frame_settings: &FrameSettings, bindless_mgr: &mut BindlessManager) -> Self {
        let graphics_command_pools = (0..frame_settings.fif_num)
            .map(|i| {
                Rc::new(RhiCommandPool::new(
                    rhi.device.clone(),
                    rhi.graphics_queue_family(),
                    vk::CommandPoolCreateFlags::TRANSIENT,
                    &format!("render_context_graphics_command_pool_{}", i),
                ))
            })
            .collect_vec();

        let frame_buffers = FrameBuffers::new(rhi, frame_settings, bindless_mgr);

        let render_timeline_semaphore = RhiSemaphore::new_timeline(rhi, 0, "frame-render-complete");
        let present_timeline_semaphore = RhiSemaphore::new_timeline(rhi, 0, "frame-present-complete");

        // 初始值应该是 1，因为 timeline semaphore 初始值是 0
        let init_frame_id = 1;
        Self {
            frame_id: init_frame_id,
            fif_label: FrameLabel::from_usize(init_frame_id),
            rebuild_frame_id: init_frame_id,
            fif_count: frame_settings.fif_num,
            timer: Timer::default(),
            fps_limit: 59.9,
            frame_buffers,
            render_timeline_semaphore,
            present_timeline_semaphore,
            frame_present_time_value: vec![vec![]; frame_settings.fif_num],
            graphics_command_pools,
            allocated_command_buffers: vec![Vec::new(); frame_settings.fif_num],
            device: rhi.device.clone(),
        }
    }

    // endregion

    // region getter

    #[inline]
    pub fn crt_frame_label(&self) -> FrameLabel {
        self.fif_label
    }

    #[inline]
    pub fn crt_frame_id(&self) -> usize {
        self.frame_id
    }

    #[inline]
    pub fn crt_frame_name(&self) -> String {
        format!("[F{}{}]", self.frame_id, self.fif_label)
    }

    #[inline]
    pub fn depth_view(&self) -> &RhiImage2DView {
        &self.frame_buffers._depth_view
    }

    // TODO 为每个 image 分配一个 uuid，bindless 就使用这个 uuid 即可
    #[inline]
    pub fn crt_frame_bindless_handle(&self, bindless_manager: &BindlessManager) -> shader::ImageHandle {
        bindless_manager.get_image_idx(&self.frame_buffers.rt_bindless_keys[*self.fif_label]).unwrap()
    }

    // endregion

    // region phase methods

    /// 分配 command buffer，在当前 frame 使用
    pub fn alloc_command_buffer(&mut self, debug_name: &str) -> RhiCommandBuffer {
        let name = format!("[{}]{}", self.crt_frame_name(), debug_name);
        let cmd =
            RhiCommandBuffer::new(self.device.clone(), self.graphics_command_pools[*self.fif_label].clone(), &name);

        self.allocated_command_buffers[*self.fif_label].push(cmd.clone());
        cmd
    }

    /// 获取用于 present 的 image
    ///
    /// 一起返回的还有 timeline value，表示该 image 渲染完成的时间点
    pub fn get_renderer_data(&self) -> Option<RendererData> {
        // TODO 需要确认一下时机
        // 使用前 2 帧去进行 present
        if self.frame_id <= 2 {
            return None;
        }

        let frame_id_to_present = self.frame_id - 2; // 注意溢出
        if frame_id_to_present < self.rebuild_frame_id {
            return None; // 还没有重建过
        }

        let frame_label_to_present = FrameLabel::from_usize(frame_id_to_present % self.fif_count);
        Some(RendererData {
            image: &self.frame_buffers.rt_image_views[*frame_label_to_present],
            image_bindless_key: self.frame_buffers.rt_bindless_keys[*frame_label_to_present].clone(),
            wait_timeline_semaphore: &self.render_timeline_semaphore,
            signal_timeline_semaphore: &self.present_timeline_semaphore,
            wait_timeline_value: frame_id_to_present as u64,
        })
    }

    pub fn begin_frame(&mut self) {
        // 等待 command buffer 之类的资源复用
        unsafe {
            let wait_timeline_value = if self.frame_id > 3 { self.frame_id as u64 - 3 } else { 0 };
            let wait_semaphore = [self.render_timeline_semaphore.handle()];
            let wait_info = vk::SemaphoreWaitInfo::default()
                .semaphores(&wait_semaphore)
                .values(std::slice::from_ref(&wait_timeline_value));
            // timeout: 5 * 1000 * 1000 * 1000 ns
            self.device.wait_semaphores(&wait_info, 5 * 1000 * 1000 * 1000).unwrap();
        }

        // 释放当前 frame 的 command buffer 的资源
        {
            std::mem::take(&mut self.allocated_command_buffers[*self.fif_label]) //
                .into_iter()
                .for_each(|cmd| cmd.free());

            // 这个调用并不会释放资源，而是将 pool 内的 command buffer 设置到初始状态
            self.graphics_command_pools[*self.fif_label].reset_all_buffers();
        }
    }

    // TODO 需要确认一下时机
    pub fn time_to_render(&self) -> bool {
        // 时间未到时，直接返回 false
        let limit_elapsed_us = 1000.0 * 1000.0 / self.fps_limit;
        if limit_elapsed_us > self.timer.toc().as_micros() as f32 {
            return false;
        }

        // framebuffer 未填满时，尽快填满
        if (self.frame_id - self.rebuild_frame_id) <= self.fif_count {
            return true;
        }

        // 确保即将被 present 的 frame 已经渲染好了即可
        let wait_result = unsafe {
            self.device.wait_semaphores(
                &vk::SemaphoreWaitInfo::default()
                    .semaphores(&[self.render_timeline_semaphore.handle()])
                    .values(&[self.frame_id as u64 - 2]),
                1,
            )
        };
        match wait_result {
            Ok(_) => true,
            Err(err) => {
                match err {
                    vk::Result::TIMEOUT => {
                        // 如果等待超时，说明当前帧还没有渲染完成
                        false
                    }
                    vk::Result::SUCCESS => true,
                    _ => {
                        // 其他错误
                        panic!("wait render timeline failed: {:?}", err);
                    }
                }
            }
        }
    }

    pub fn before_render(&mut self) {}

    pub fn end_frame(&mut self, _rhi: &Rhi) {
        self.frame_id += 1;
        self.fif_label = FrameLabel::from_usize(self.frame_id % self.fif_count);
    }

    pub fn after_render(&mut self) {}

    pub fn rebuild_framebuffers(
        &mut self,
        rhi: &Rhi,
        frame_settings: &FrameSettings,
        bindless_mgr: &mut BindlessManager,
    ) {
        self.frame_buffers.unregister_bindless(bindless_mgr);
        self.frame_buffers = FrameBuffers::new(rhi, frame_settings, bindless_mgr);

        self.rebuild_frame_id = self.frame_id;
    }

    // endregion
}
