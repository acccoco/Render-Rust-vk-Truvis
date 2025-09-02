use ash::vk;
use itertools::Itertools;

use crate::commands::{command_buffer::CommandBuffer, semaphore::Semaphore};

/// Rhi 关于 submitInfo 的封装，更易用
#[derive(Default)]
pub struct SubmitInfo
{
    inner: vk::SubmitInfo2<'static>,

    _command_buffers: Vec<vk::CommandBufferSubmitInfo<'static>>,
    wait_infos: Vec<vk::SemaphoreSubmitInfo<'static>>,
    signal_infos: Vec<vk::SemaphoreSubmitInfo<'static>>,
}

impl SubmitInfo
{
    pub fn new(commands: &[CommandBuffer]) -> Self
    {
        let command_buffers = commands
            .iter()
            .map(|cmd| vk::CommandBufferSubmitInfo::default().command_buffer(cmd.vk_handle()))
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
    pub fn submit_info(&self) -> vk::SubmitInfo2<'_>
    {
        self.inner
            .command_buffer_infos(&self._command_buffers)
            .wait_semaphore_infos(&self.wait_infos)
            .signal_semaphore_infos(&self.signal_infos)
    }

    #[inline]
    pub fn wait(mut self, semaphore: &Semaphore, stage: vk::PipelineStageFlags2, value: Option<u64>) -> Self
    {
        self.wait_infos.push(
            vk::SemaphoreSubmitInfo::default()
                .semaphore(semaphore.handle())
                .stage_mask(stage)
                .value(value.unwrap_or_default()),
        );
        self
    }

    #[inline]
    pub fn signal(mut self, semaphore: &Semaphore, stage: vk::PipelineStageFlags2, value: Option<u64>) -> Self
    {
        self.signal_infos.push(
            vk::SemaphoreSubmitInfo::default()
                .semaphore(semaphore.handle())
                .stage_mask(stage)
                .value(value.unwrap_or_default()),
        );
        self
    }
}
