use ash::vk;

use crate::rhi::{rhi_struct::RhiQueue, Rhi};


pub struct RhiCommandBuffer
{
    command_buffer: vk::CommandBuffer,
    command_pool: vk::CommandPool,
}

impl RhiCommandBuffer
{
    pub fn new(pool: vk::CommandPool) -> Self
    {
        let info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = unsafe { Rhi::instance().device().allocate_command_buffers(&info).unwrap()[0] };
        Self {
            command_buffer,
            command_pool: pool,
        }
    }

    pub fn one_time_exec<F>(f: F)
    where
        F: FnOnce(vk::CommandBuffer),
    {
        unsafe {
            let rhi = Rhi::instance();
            let command_buffer = Self::new(rhi.graphics_command_pool().command_pool);
            let fence = rhi.core().create_fence(false, Some("one-time-command-fence"));

            rhi.device()
                .begin_command_buffer(
                    command_buffer.command_buffer,
                    &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap();
            f(command_buffer.command_buffer);
            rhi.device().end_command_buffer(command_buffer.command_buffer).unwrap();

            // TODO 可以考虑增加一个 get queue 的方法，动态决定使用哪个 queue
            //       是否需要动态决定呢？可以静态决定吗？
            command_buffer.submit_command(rhi.core().graphics_queue(), &[], &[], &[], Some(fence));
            rhi.device().wait_for_fences(&[fence], true, u64::MAX).unwrap();

            rhi.device().destroy_fence(fence, None);
        }
    }

    fn submit_command(
        &self,
        queue: &RhiQueue,
        wait_stages: &[vk::PipelineStageFlags],
        wait_semaphores: &[vk::Semaphore],
        signal_semaphore: &[vk::Semaphore],
        signal_fence: Option<vk::Fence>,
    )
    {
        let command_buffers = [self.command_buffer];
        let submit_info = vk::SubmitInfo::builder()
            .wait_dst_stage_mask(wait_stages)
            .wait_semaphores(wait_semaphores)
            .signal_semaphores(signal_semaphore)
            .command_buffers(&command_buffers);

        let rhi = Rhi::instance();
        unsafe {
            if let Some(signal_fence) = signal_fence {
                rhi.device().queue_submit(queue.queue, &[submit_info.build()], signal_fence)
            } else {
                rhi.device().queue_submit(queue.queue, &[submit_info.build()], vk::Fence::null())
            }
            .expect("queue submit failed")
        }
    }
}


impl Drop for RhiCommandBuffer
{
    fn drop(&mut self)
    {
        unsafe {
            Rhi::instance()
                .device()
                .free_command_buffers(self.command_pool, std::slice::from_ref(&self.command_buffer));
        }
    }
}
