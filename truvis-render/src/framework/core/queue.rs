use ash::vk;
use itertools::Itertools;

use crate::framework::{
    core::{
        command_buffer::CommandBuffer,
        synchronize::{Fence, Semaphore},
    },
    render_core::Core,
};

pub struct Queue
{
    pub vk_queue: vk::Queue,
    pub queue_family_index: u32,
}

impl Queue
{
    // TODO 使用 queue_submit2
    pub fn submit(&self, rhi: &Core, batches: Vec<SubmitInfo>, fence: Option<Fence>)
    {
        unsafe {
            // batches 的存在是有必要的，submit_infos 引用的 batches 的内存
            let batches = batches.iter().map(|b| b.to_vk_batch()).collect_vec();
            let submit_infos = batches.iter().map(|b| b.submit_info()).collect_vec();

            rhi.vk_device()
                .queue_submit(self.vk_queue, &submit_infos, fence.map_or(vk::Fence::null(), |f| f.fence))
                .unwrap();
        }
    }

    /// 根据 specification，vkQueueWaitIdle 应该和 Fence 效率相同
    #[inline]
    pub fn wait_idle(&self, rhi: &Core)
    {
        unsafe { rhi.vk_device().queue_wait_idle(self.vk_queue).unwrap() }
    }
}


// TODO 这个封装的不怎么样
/// RHi 关于 submitInfo 的封装，更易用
#[derive(Default)]
pub struct SubmitInfo
{
    pub command_buffers: Vec<CommandBuffer>,
    pub wait_info: Vec<(vk::PipelineStageFlags, Semaphore)>,
    pub signal_info: Vec<Semaphore>,
}


/// 兼容 VkSubmitInfo 的内存模式
pub struct SubmitInfoTemp
{
    command_buffers: Vec<vk::CommandBuffer>,
    wait_stages: Vec<vk::PipelineStageFlags>,
    wait_semaphores: Vec<vk::Semaphore>,
    signal_semaphores: Vec<vk::Semaphore>,
}


impl SubmitInfoTemp
{
    pub fn submit_info(&self) -> vk::SubmitInfo
    {
        let mut info = vk::SubmitInfo::default()
            .command_buffers(&self.command_buffers)
            .wait_semaphores(&self.wait_semaphores)
            .wait_dst_stage_mask(&self.wait_stages)
            .signal_semaphores(&self.signal_semaphores);

        if self.wait_semaphores.is_empty() {
            info.p_wait_semaphores = std::ptr::null();
            info.p_wait_dst_stage_mask = std::ptr::null();
        }

        if self.signal_semaphores.is_empty() {
            info.p_signal_semaphores = std::ptr::null();
        }

        info
    }
}

impl SubmitInfo
{
    #[inline]
    pub fn to_vk_batch(&self) -> SubmitInfoTemp
    {
        SubmitInfoTemp {
            command_buffers: self.commands(),
            wait_stages: self.wait_stages(),
            wait_semaphores: self.wait_semaphores(),
            signal_semaphores: self.signal_semaphores(),
        }
    }

    #[inline]
    fn commands(&self) -> Vec<vk::CommandBuffer>
    {
        self.command_buffers.iter().map(|c| c.command_buffer).collect()
    }

    #[inline]
    fn wait_semaphores(&self) -> Vec<vk::Semaphore>
    {
        self.wait_info.iter().map(|(_, s)| s.semaphore).collect()
    }

    #[inline]
    fn signal_semaphores(&self) -> Vec<vk::Semaphore>
    {
        self.signal_info.iter().map(|s| s.semaphore).collect()
    }

    #[inline]
    fn wait_stages(&self) -> Vec<vk::PipelineStageFlags>
    {
        self.wait_info.iter().map(|(s, _)| *s).collect()
    }
}
