use ash::vk;
use itertools::Itertools;

use crate::{
    resource_type::{
        buffer::RhiBuffer, command_pool::RhiCommandPool, pipeline::RhiPipeline, queue::RhiSubmitBatch,
        sync_primitives::RhiFence,
    },
    rhi::Rhi,
};


#[derive(Clone)]
pub struct RhiCommandBuffer
{
    pub(crate) command_buffer: vk::CommandBuffer,
    pub(crate) command_pool: vk::CommandPool,
}

impl RhiCommandBuffer
{
    pub fn new<S>(pool: &RhiCommandPool, debug_name: S) -> Self
    where
        S: AsRef<str>,
    {
        let info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(pool.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = unsafe { Rhi::instance().device().allocate_command_buffers(&info).unwrap()[0] };
        Rhi::instance().set_debug_name(command_buffer, debug_name);
        Self {
            command_buffer,
            command_pool: pool.command_pool,
        }
    }

    /// 专用于 transfer 的仅一次使用的 command buffer
    pub fn one_time_transfer<F>(f: F)
    where
        F: FnOnce(&mut RhiCommandBuffer),
    {
        unsafe {
            let rhi = Rhi::instance();
            let mut command_buffer = Self::new(rhi.transfer_command_pool(), "one-time-transfer-command-buffer");

            rhi.device()
                .begin_command_buffer(
                    command_buffer.command_buffer,
                    &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap();
            f(&mut command_buffer);
            rhi.device().end_command_buffer(command_buffer.command_buffer).unwrap();

            let fence = RhiFence::new(false, "one-time-command-fence");
            rhi.transfer_queue().submit(
                vec![RhiSubmitBatch {
                    command_buffers: vec![command_buffer.clone()],
                    ..Default::default()
                }],
                Some(fence.clone()),
            );
            fence.wait();
            fence.drop();
            command_buffer.drop();
        }
    }

    pub fn drop(self)
    {
        unsafe {
            Rhi::instance()
                .device()
                .free_command_buffers(self.command_pool, std::slice::from_ref(&self.command_buffer));
        }
    }

    pub fn begin(&mut self, usage_flag: vk::CommandBufferUsageFlags)
    {
        unsafe {
            Rhi::instance()
                .device()
                .begin_command_buffer(self.command_buffer, &vk::CommandBufferBeginInfo::builder().flags(usage_flag))
                .unwrap();
        }
    }

    #[inline]
    pub fn end(&mut self) { unsafe { Rhi::instance().device().end_command_buffer(self.command_buffer).unwrap() } }
}

// transfer 类型的命令
impl RhiCommandBuffer
{
    #[inline]
    pub fn copy_buffer(&mut self, src: &RhiBuffer, dst: &mut RhiBuffer, regions: &[vk::BufferCopy])
    {
        unsafe {
            Rhi::instance().device().cmd_copy_buffer(self.command_buffer, src.buffer, dst.buffer, regions);
        }
    }
}

// 绘制类型命令
impl RhiCommandBuffer
{
    #[inline]
    pub fn begin_rendering(&mut self, render_info: &vk::RenderingInfo)
    {
        unsafe {
            Rhi::instance().dynamic_render_pf().cmd_begin_rendering(self.command_buffer, render_info);
        }
    }

    #[inline]
    pub fn end_rendering(&mut self)
    {
        unsafe {
            Rhi::instance().dynamic_render_pf().cmd_end_rendering(self.command_buffer);
        }
    }

    /// index_info: (index_count, first_index)
    /// instance_info: (instance_count, first_instance)
    #[inline]
    pub fn draw_indexed(&mut self, index_info: (u32, u32), instance_info: (u32, u32), vertex_offset: i32)
    {
        unsafe {
            Rhi::instance().device().cmd_draw_indexed(
                self.command_buffer,
                index_info.0,
                instance_info.0,
                index_info.1,
                vertex_offset,
                instance_info.1,
            );
        }
    }
}

// 状态设置命令
impl RhiCommandBuffer
{
    #[inline]
    pub fn bind_pipeline(&mut self, bind_point: vk::PipelineBindPoint, pipeline: &RhiPipeline)
    {
        unsafe {
            Rhi::instance().device().cmd_bind_pipeline(self.command_buffer, bind_point, pipeline.pipeline);
        }
    }

    /// buffers 每个 vertex buffer 以及 offset
    #[inline]
    pub fn bind_vertex_buffer(&mut self, first_bind: u32, buffers: &[RhiBuffer], offsets: &[vk::DeviceSize])
    {
        unsafe {
            let buffers = buffers.iter().map(|b| b.buffer).collect_vec();
            Rhi::instance().device().cmd_bind_vertex_buffers(self.command_buffer, first_bind, &buffers, offsets);
        }
    }

    #[inline]
    pub fn bind_index_buffer(&mut self, buffer: &RhiBuffer, offset: vk::DeviceSize, index_type: vk::IndexType)
    {
        unsafe {
            Rhi::instance()
                .device()
                .cmd_bind_index_buffer(self.command_buffer, buffer.buffer, offset, index_type);
        }
    }
}

// 同步命令
impl RhiCommandBuffer
{
    // TODO 临时的，修改下
    #[inline]
    pub fn image_barrier(
        &mut self,
        src: (vk::PipelineStageFlags, vk::AccessFlags),
        dst: (vk::PipelineStageFlags, vk::AccessFlags),
        image: vk::Image,
        image_aspect: vk::ImageAspectFlags,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    )
    {
        let barrier = vk::ImageMemoryBarrier::builder()
            .src_access_mask(src.1)
            .dst_access_mask(dst.1)
            .old_layout(old_layout)
            .new_layout(new_layout)
            .image(image)
            .subresource_range(
                vk::ImageSubresourceRange::builder().aspect_mask(image_aspect).layer_count(1).level_count(1).build(),
            );

        unsafe {
            Rhi::instance().device().cmd_pipeline_barrier(
                self.command_buffer,
                src.0,
                dst.0,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier.build()],
            );
        }
    }
}
