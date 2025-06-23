use crate::pipeline_settings::{FrameLabel, FrameSettings, PipelineSettings};
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
    pub fn new(
        rhi: &Rhi,
        pipeline_settings: &PipelineSettings,
        frame_settings: &FrameSettings,
        bindless_mgr: &mut BindlessManager,
    ) -> Self {
        let (depth_image, depth_image_view) = Self::create_depth_image(rhi, pipeline_settings, frame_settings);
        let (rt_images, rt_image_views) = Self::create_rt_image(rhi, pipeline_settings, frame_settings);

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
        pipeline_settings: &PipelineSettings,
        frame_settings: &FrameSettings,
    ) -> (Rc<RhiImage2D>, Rc<RhiImage2DView>) {
        let depth_image = Rc::new(RhiImage2D::new(
            rhi,
            Rc::new(RhiImageCreateInfo::new_image_2d_info(
                frame_settings.frame_extent,
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
    ) -> (Vec<Rc<RhiImage2D>>, Vec<Rc<RhiImage2DView>>) {
        let rt_images = (0..pipeline_settings.frames_in_flight)
            .map(|i| {
                Rc::new(RhiImage2D::new(
                    rhi,
                    Rc::new(RhiImageCreateInfo::new_image_2d_info(
                        frame_settings.frame_extent,
                        pipeline_settings.color_format,
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
                        pipeline_settings.color_format,
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

pub struct PresentData {
    pub image: Rc<RhiImage2DView>,
    pub wait_timeline_value: u64,
    pub wait_semaphore: RhiSemaphore,
    pub signal_timeline_semaphore: RhiSemaphore,
}

pub struct FrameContext {
    /// 当前处在 in-flight 的第几帧：A, B, C
    fif_label: FrameLabel,

    /// 当前的帧序号，一直累加
    frame_id: usize,
    rebuild_frame_id: usize,
    fif_count: usize,

    frame_settings: FrameSettings,

    /// 为每个 frame 分配一个 command pool
    graphics_command_pools: Vec<Rc<RhiCommandPool>>,

    /// 每个 command pool 已经分配出去的 command buffer，用于集中 free 或其他操作
    allocated_command_buffers: Vec<Vec<RhiCommandBuffer>>,

    frame_buffers: FrameBuffers,

    /// 帧渲染完成的 timeline，value 就等于 frame_id
    render_timeline: vk::Semaphore,

    /// 渲染帧依赖的外部时间线
    present_timeline: vk::Semaphore,

    /// 每一帧依赖的外部时间线的 value
    frame_present_time_value: Vec<u64>,

    device: Rc<RhiDevice>,
}
impl Drop for FrameContext {
    fn drop(&mut self) {}
}
impl FrameContext {
    // region ctor

    pub fn new(rhi: &Rhi, pipeline_settings: &PipelineSettings, bindless_mgr: &mut BindlessManager) -> Self {
        let frame_settings = FrameSettings {
            frame_extent: vk::Extent2D {
                width: 400,
                height: 400,
            },
        };

        let graphics_command_pools = (0..pipeline_settings.frames_in_flight)
            .map(|i| {
                Rc::new(RhiCommandPool::new(
                    rhi.device.clone(),
                    rhi.graphics_queue_family(),
                    vk::CommandPoolCreateFlags::TRANSIENT,
                    &format!("render_context_graphics_command_pool_{}", i),
                ))
            })
            .collect_vec();

        let frame_buffers = FrameBuffers::new(rhi, pipeline_settings, &frame_settings, bindless_mgr);

        let render_timeline = {
            let mut timeline_type_ci =
                vk::SemaphoreTypeCreateInfo::default().semaphore_type(vk::SemaphoreType::TIMELINE).initial_value(0);
            let timeline_semaphore_ci = vk::SemaphoreCreateInfo::default().push_next(&mut timeline_type_ci);
            unsafe { rhi.device.create_semaphore(&timeline_semaphore_ci, None).unwrap() }
        };
        let external_timeline = {
            let mut timeline_type_ci =
                vk::SemaphoreTypeCreateInfo::default().semaphore_type(vk::SemaphoreType::TIMELINE).initial_value(0);
            let timeline_semaphore_ci = vk::SemaphoreCreateInfo::default().push_next(&mut timeline_type_ci);
            unsafe { rhi.device.create_semaphore(&timeline_semaphore_ci, None).unwrap() }
        };

        Self {
            frame_settings,
            // 初始值应该是 1，因为 timeline semaphore 初始值是 0
            frame_id: 1,
            // 由于初始的 frame_id 是 1，所以初始的 fif_label 是 B
            fif_label: FrameLabel::B,
            // 在此之前的帧都被销毁了
            rebuild_frame_id: 1,
            fif_count: pipeline_settings.frames_in_flight,
            frame_buffers,
            render_timeline,
            present_timeline: external_timeline,
            frame_present_time_value: vec![0; pipeline_settings.frames_in_flight],
            graphics_command_pools,
            allocated_command_buffers: vec![Vec::new(); pipeline_settings.frames_in_flight],
            device: rhi.device.clone(),
        }
    }

    // endregion

    // region getter

    /// 当前处在第几帧：A, B, C
    #[inline]
    pub fn crt_frame_label(&self) -> FrameLabel {
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
    pub fn depth_view(&self) -> &RhiImage2DView {
        &self.frame_buffers._depth_view
    }

    /// 获取用于 present 的 image
    ///
    /// 一起返回的还有 timeline value，表示该 image 渲染完成的时间点
    #[inline]
    pub fn get_present_image(&self) -> Option<(&RhiImage2DView, u64)> {
        // 使用刚渲染好的一帧进行 present
        let frame_id_to_present = self.frame_id - 1; // 注意溢出
        if frame_id_to_present > 0 && frame_id_to_present >= self.rebuild_frame_id {
            Some((
                &self.frame_buffers.rt_image_views[frame_id_to_present % self.fif_count], //
                frame_id_to_present as u64,
            ))
        } else {
            None
        }
    }

    /// 当前需要渲染的帧，等待的 present 的 timeline
    #[inline]
    fn crt_frame_wait_semaphore_value(&self) -> u64 {
        self.frame_present_time_value[*self.fif_label]
    }

    #[inline]
    pub fn crt_frame_bindless_handle(&self, bindless_manager: &BindlessManager) -> shader::ImageHandle {
        bindless_manager.get_image_idx(&self.frame_buffers.rt_bindless_keys[*self.fif_label]).unwrap()
    }

    #[inline]
    pub fn frame_settings(&self) -> FrameSettings {
        self.frame_settings
    }
    // endregion

    // region tools

    /// 分配 command buffer，在当前 frame 使用
    pub fn alloc_command_buffer(&mut self, debug_name: &str) -> RhiCommandBuffer {
        let name = format!("[frame-{}-{}]{}", self.fif_label, self.frame_id, debug_name);
        let cmd =
            RhiCommandBuffer::new(self.device.clone(), self.graphics_command_pools[*self.fif_label].clone(), &name);

        self.allocated_command_buffers[*self.fif_label].push(cmd.clone());
        cmd
    }

    // endregion

    // region phase methods

    pub fn begin_frame(&mut self) {
        // wait semaphore 的 timeout 设为 0，表示 assert
        // TODO 改成 wait time == 0， 引入 time_to_render()
        unsafe {
            let wait_value = if self.frame_id > 3 { self.frame_id as u64 - 3 } else { 0 };
            let wait_info = vk::SemaphoreWaitInfo::default()
                .semaphores(std::slice::from_ref(&self.render_timeline))
                .values(std::slice::from_ref(&wait_value));
            self.device.wait_semaphores(&wait_info, u64::MAX).unwrap();
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

    pub fn before_render(&mut self) {}

    pub fn end_frame(&mut self, _rhi: &Rhi) {
        self.fif_label.next_frame();
        self.frame_id += 1;
    }

    pub fn after_render(&mut self) {}

    pub fn rebuild_framebuffers(
        &mut self,
        rhi: &Rhi,
        pipeline_settings: &PipelineSettings,
        new_extent: vk::Extent2D,
        bindless_mgr: &mut BindlessManager,
    ) {
        self.frame_settings.frame_extent = new_extent;

        self.frame_buffers.unregister_bindless(bindless_mgr);
        self.frame_buffers = FrameBuffers::new(rhi, pipeline_settings, &self.frame_settings, bindless_mgr);

        self.rebuild_frame_id = self.frame_id;
    }

    // endregion
}
