use std::rc::Rc;

use ash::vk;
use itertools::Itertools;

use crate::core::debug_utils::RhiDebugType;
use crate::core::{
    command_buffer::RhiCommandBuffer,
    device::RhiDevice,
    synchronize::{RhiFence, RhiSemaphore},
};

#[derive(Clone, Debug)]
pub struct RhiQueueFamily {
    pub name: String,
    pub queue_family_index: u32,
    pub queue_flags: vk::QueueFlags,
    pub queue_count: u32,
}

/// # destroy
///
/// RhiQueueFamily 在 RhiDevice 销毁时会被销毁
pub struct RhiQueue {
    pub(crate) handle: vk::Queue,
    pub(crate) queue_family: RhiQueueFamily,

    pub(crate) device: Rc<RhiDevice>,
}
impl RhiDebugType for RhiQueue {
    fn debug_type_name() -> &'static str {
        "RhiQueue"
    }
    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
}

impl RhiQueue {
    #[inline]
    pub fn handle(&self) -> vk::Queue {
        self.handle
    }

    pub fn submit(&self, batches: Vec<RhiSubmitInfo>, fence: Option<RhiFence>) {
        unsafe {
            // batches 的存在是有必要的，submit_infos 引用的 batches 的内存
            let batches = batches.iter().map(|b| *b.inner()).collect_vec();
            self.device.queue_submit2(self.handle, &batches, fence.map_or(vk::Fence::null(), |f| f.handle())).unwrap()
        }
    }

    /// 根据 specification，vkQueueWaitIdle 应该和 Fence 效率相同
    #[inline]
    pub fn wait_idle(&self) {
        unsafe { self.device.queue_wait_idle(self.handle).unwrap() }
    }
}

/// RHi 关于 submitInfo 的封装，更易用
#[derive(Default)]
pub struct RhiSubmitInfo {
    inner: vk::SubmitInfo2<'static>,

    _command_buffers: Vec<vk::CommandBufferSubmitInfo<'static>>,
    wait_infos: Vec<vk::SemaphoreSubmitInfo<'static>>,
    signal_infos: Vec<vk::SemaphoreSubmitInfo<'static>>,
}

impl RhiSubmitInfo {
    pub fn new(commands: &[RhiCommandBuffer]) -> Self {
        let command_buffers = commands
            .iter()
            .map(|cmd| vk::CommandBufferSubmitInfo::default().command_buffer(cmd.handle()))
            .collect_vec();

        let inner = vk::SubmitInfo2 {
            // 暂时不使用该 flag
            flags: vk::SubmitFlags::empty(),

            command_buffer_info_count: command_buffers.len() as u32,
            p_command_buffer_infos: command_buffers.as_ptr(),
            ..Default::default()
        };

        Self {
            inner,
            _command_buffers: command_buffers,
            wait_infos: vec![],
            signal_infos: vec![],
        }
    }

    #[inline]
    pub fn inner(&self) -> &vk::SubmitInfo2 {
        &self.inner
    }

    /// builder
    #[inline]
    pub fn wait_infos(mut self, wait_semaphores: &[(RhiSemaphore, vk::PipelineStageFlags2)]) -> Self {
        self.wait_infos = wait_semaphores
            .iter()
            .map(|(s, stage)| vk::SemaphoreSubmitInfo::default().semaphore(s.handle()).stage_mask(*stage))
            .collect_vec();
        self.inner.wait_semaphore_info_count = self.wait_infos.len() as u32;
        self.inner.p_wait_semaphore_infos = self.wait_infos.as_ptr();
        self
    }

    /// builder
    #[inline]
    pub fn signal_infos(mut self, signal_semaphores: &[(RhiSemaphore, vk::PipelineStageFlags2)]) -> Self {
        self.signal_infos = signal_semaphores
            .iter()
            .map(|(s, stage)| vk::SemaphoreSubmitInfo::default().semaphore(s.handle()).stage_mask(*stage))
            .collect_vec();
        self.inner.signal_semaphore_info_count = self.signal_infos.len() as u32;
        self.inner.p_signal_semaphore_infos = self.signal_infos.as_ptr();
        self
    }
}
