use crate::pipeline_settings::{FrameLabel, FrameSettings};
use ash::vk;
use itertools::Itertools;
use std::rc::Rc;
use truvis_rhi::core::command_queue::RhiSubmitInfo;
use truvis_rhi::core::synchronize::RhiSemaphore;
use truvis_rhi::{
    core::{command_buffer::RhiCommandBuffer, command_pool::RhiCommandPool, device::RhiDevice},
    rhi::Rhi,
};

pub enum RenderTimelinePhase {
    RenderComplete = 0,
    FrameEnd,

    Count,
}

pub struct FrameController {
    /// 当前处在 in-flight 的第几帧：A, B, C
    fif_label: FrameLabel,
    /// 当前的帧序号，一直累加，初始序号是 1
    frame_id: usize,
    fif_count: usize,

    /// 为每个 frame 分配一个 command pool
    graphics_command_pools: Vec<Rc<RhiCommandPool>>,
    /// 每个 command pool 已经分配出去的 command buffer，用于集中 free 或其他操作
    allocated_command_buffers: Vec<Vec<RhiCommandBuffer>>,

    /// 帧渲染完成的 timeline，value 就等于 frame_id
    render_timeline_semaphore: RhiSemaphore,

    device: Rc<RhiDevice>,
}
impl Drop for FrameController {
    fn drop(&mut self) {}
}
// ctor
impl FrameController {
    pub fn new(rhi: &Rhi, frame_settings: &FrameSettings) -> Self {
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

        let render_timeline_semaphore = RhiSemaphore::new_timeline(rhi, 0, "render-timeline");

        // 初始值应该是 1，因为 timeline semaphore 初始值是 0
        let init_frame_id = 1;
        Self {
            frame_id: init_frame_id,
            fif_label: FrameLabel::from_usize(init_frame_id),
            fif_count: frame_settings.fif_num,
            render_timeline_semaphore,
            graphics_command_pools,
            allocated_command_buffers: vec![Vec::new(); frame_settings.fif_num],
            device: rhi.device.clone(),
        }
    }
}

// getter
impl FrameController {
    #[inline]
    pub fn frame_label(&self) -> FrameLabel {
        self.fif_label
    }

    #[inline]
    pub fn frame_id(&self) -> usize {
        self.frame_id
    }

    #[inline]
    pub fn frame_name(&self) -> String {
        format!("[F{}{}]", self.frame_id, self.fif_label)
    }

    #[inline]
    pub fn render_timeline_semaphore(&self, phase: RenderTimelinePhase) -> (&RhiSemaphore, u64) {
        (
            &self.render_timeline_semaphore, //
            self.frame_id as u64 * RenderTimelinePhase::Count as u64 + phase as u64,
        )
    }
}

// phase methods
impl FrameController {
    /// 分配 command buffer，在当前 frame 使用
    pub fn alloc_command_buffer(&mut self, debug_name: &str) -> RhiCommandBuffer {
        let name = format!("[{}]{}", self.frame_name(), debug_name);
        let cmd =
            RhiCommandBuffer::new(self.device.clone(), self.graphics_command_pools[*self.fif_label].clone(), &name);

        self.allocated_command_buffers[*self.fif_label].push(cmd.clone());
        cmd
    }

    pub fn begin_frame(&mut self) {
        // 等待 command buffer 之类的资源复用
        {
            let wait_frame = if self.frame_id > 3 { self.frame_id as u64 - 3 } else { 0 };
            let wait_timeline_value = if wait_frame == 0 {
                0
            } else {
                wait_frame as u64 * RenderTimelinePhase::Count as u64 + RenderTimelinePhase::FrameEnd as u64
            };
            let timeout_ns = 30 * 1000 * 1000 * 1000;
            self.render_timeline_semaphore.wait_timeline(wait_timeline_value, timeout_ns);
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

    pub fn end_render(&self, rhi: &Rhi) {
        // 设置渲染帧结束的 semaphore
        let submit_info = RhiSubmitInfo::new(&[]).signal(
            &self.render_timeline_semaphore,
            vk::PipelineStageFlags2::NONE,
            Some(self.render_timeline_semaphore(RenderTimelinePhase::RenderComplete).1),
        );
        rhi.graphics_queue.submit(vec![submit_info], None);
    }

    pub fn end_frame(&mut self, rhi: &Rhi) {
        // 设置当前帧结束的 semaphore，用于保护当前帧的资源
        {
            let submit_info = RhiSubmitInfo::new(&[]).signal(
                &self.render_timeline_semaphore,
                vk::PipelineStageFlags2::NONE,
                Some(self.render_timeline_semaphore(RenderTimelinePhase::FrameEnd).1),
            );
            rhi.graphics_queue.submit(vec![submit_info], None);
        }

        self.frame_id += 1;
        self.fif_label = FrameLabel::from_usize(self.frame_id % self.fif_count);
    }
}
