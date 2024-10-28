use ash::vk;

use crate::framework::{
    core::{command_pool::RhiCommandPool, queue::RhiSubmitBatch},
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

        let command_buffer =
            unsafe { Rhi::instance().device().allocate_command_buffers(&info).unwrap()[0] };
        Rhi::instance().set_debug_name(command_buffer, debug_name);
        Self {
            command_buffer,
            command_pool: pool.command_pool,
        }
    }

    /// 从 Rhi 中的 command pool 分配 command buffer 进行执行
    pub fn one_time_exec<F>(ty: vk::QueueFlags, f: F)
    where
        F: FnOnce(&mut RhiCommandBuffer),
    {
        let rhi = Rhi::instance();

        let pool;
        let queue;
        match ty {
            vk::QueueFlags::COMPUTE => {
                pool = rhi.compute_command_pool();
                queue = rhi.compute_queue();
            }
            vk::QueueFlags::TRANSFER => {
                pool = rhi.transfer_command_pool();
                queue = rhi.transfer_queue();
            }
            vk::QueueFlags::GRAPHICS => {
                pool = rhi.graphics_command_pool();
                queue = rhi.graphics_queue();
            }
            other => panic!("not supported queue type: SPARSE_BINDING, {:?}", other),
        }

        let mut command_buffer = Self::new(pool, "one-time-command-buffer");

        command_buffer.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        f(&mut command_buffer);
        command_buffer.end();

        queue.submit(
            vec![RhiSubmitBatch {
                command_buffers: vec![command_buffer.clone()],
                ..Default::default()
            }],
            None,
        );
        queue.wait_idle();
        command_buffer.free();
    }

    pub fn free(self)
    {
        unsafe {
            Rhi::instance().device().free_command_buffers(
                self.command_pool,
                std::slice::from_ref(&self.command_buffer),
            );
        }
    }

    pub fn begin(&mut self, usage_flag: vk::CommandBufferUsageFlags)
    {
        unsafe {
            Rhi::instance()
                .device()
                .begin_command_buffer(
                    self.command_buffer,
                    &vk::CommandBufferBeginInfo::builder().flags(usage_flag),
                )
                .unwrap();
        }
    }

    #[inline]
    pub fn end(&mut self)
    {
        unsafe { Rhi::instance().device().end_command_buffer(self.command_buffer).unwrap() }
    }
}

mod _transfer_cmd
{
    use ash::vk;

    use crate::framework::{
        core::{buffer::RhiBuffer, command_buffer::RhiCommandBuffer},
        rhi::Rhi,
    };

    // transfer 类型的命令
    impl RhiCommandBuffer
    {
        #[inline]
        pub fn copy_buffer(
            &mut self,
            src: &RhiBuffer,
            dst: &mut RhiBuffer,
            regions: &[vk::BufferCopy],
        )
        {
            unsafe {
                Rhi::instance().device().cmd_copy_buffer(
                    self.command_buffer,
                    src.buffer,
                    dst.buffer,
                    regions,
                );
            }
        }

        /// 注：仅支持 compute queue
        #[inline]
        pub fn copy_acceleration_structure(
            &mut self,
            copy_info: &vk::CopyAccelerationStructureInfoKHR,
        )
        {
            unsafe {
                Rhi::instance()
                    .acceleration_structure_pf()
                    .cmd_copy_acceleration_structure(self.command_buffer, copy_info);
            }
        }
    }
}

mod _draw_cmd
{
    use ash::vk;

    use crate::framework::{core::command_buffer::RhiCommandBuffer, rhi::Rhi};

    // 绘制类型命令
    impl RhiCommandBuffer
    {
        #[inline]
        pub fn begin_rendering(&mut self, render_info: &vk::RenderingInfo)
        {
            unsafe {
                Rhi::instance()
                    .dynamic_render_pf()
                    .cmd_begin_rendering(self.command_buffer, render_info);
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
        pub fn draw_indexed(
            &mut self,
            index_info: (u32, u32),
            instance_info: (u32, u32),
            vertex_offset: i32,
        )
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
}


mod _status_cmd
{
    use ash::vk;
    use itertools::Itertools;

    use crate::framework::{
        core::{buffer::RhiBuffer, command_buffer::RhiCommandBuffer, pipeline::RhiPipeline},
        rhi::Rhi,
    };

    // 状态设置命令
    impl RhiCommandBuffer
    {
        #[inline]
        pub fn bind_pipeline(&mut self, bind_point: vk::PipelineBindPoint, pipeline: &RhiPipeline)
        {
            unsafe {
                Rhi::instance().device().cmd_bind_pipeline(
                    self.command_buffer,
                    bind_point,
                    pipeline.pipeline,
                );
            }
        }

        /// buffers 每个 vertex buffer 以及 offset
        #[inline]
        pub fn bind_vertex_buffer(
            &mut self,
            first_bind: u32,
            buffers: &[RhiBuffer],
            offsets: &[vk::DeviceSize],
        )
        {
            unsafe {
                let buffers = buffers.iter().map(|b| b.buffer).collect_vec();
                Rhi::instance().device().cmd_bind_vertex_buffers(
                    self.command_buffer,
                    first_bind,
                    &buffers,
                    offsets,
                );
            }
        }

        #[inline]
        pub fn bind_index_buffer(
            &mut self,
            buffer: &RhiBuffer,
            offset: vk::DeviceSize,
            index_type: vk::IndexType,
        )
        {
            unsafe {
                Rhi::instance().device().cmd_bind_index_buffer(
                    self.command_buffer,
                    buffer.buffer,
                    offset,
                    index_type,
                );
            }
        }
    }
}

mod _sync_cmd
{
    use ash::vk;

    use crate::framework::{core::command_buffer::RhiCommandBuffer, rhi::Rhi};

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
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(image_aspect)
                        .layer_count(1)
                        .level_count(1)
                        .build(),
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

        #[inline]
        pub fn memory_barrier(&mut self, barriers: &[vk::MemoryBarrier2])
        {
            let dependency_info = vk::DependencyInfo::builder().memory_barriers(barriers);
            unsafe {
                Rhi::instance()
                    .device()
                    .cmd_pipeline_barrier2(self.command_buffer, &dependency_info);
            }
        }
    }
}


mod _ray_tracing_cmd
{
    use ash::vk;

    use crate::framework::{
        core::{command_buffer::RhiCommandBuffer, query_pool::RhiQueryPool},
        rhi::Rhi,
    };

    // RayTracing 相关的命令
    impl RhiCommandBuffer
    {
        /// 注：仅支持 compute queue
        #[inline]
        pub fn build_acceleration_structure(
            &mut self,
            geometry: &vk::AccelerationStructureBuildGeometryInfoKHR,
            ranges: &[vk::AccelerationStructureBuildRangeInfoKHR],
        )
        {
            unsafe {
                // 该函数可以一次构建多个 AccelerationStructure，这里只构建了 1 个
                Rhi::instance().acceleration_structure_pf().cmd_build_acceleration_structures(
                    self.command_buffer,
                    std::slice::from_ref(geometry),
                    &[ranges],
                )
            }
        }

        /// 注：仅支持 compute queue
        #[inline]
        pub fn write_acceleration_structure_properties(
            &mut self,
            query_pool: &mut RhiQueryPool,
            first_query: u32,
            acceleration_structures: &[vk::AccelerationStructureKHR],
        )
        {
            unsafe {
                Rhi::instance()
                    .acceleration_structure_pf()
                    .cmd_write_acceleration_structures_properties(
                        self.command_buffer,
                        acceleration_structures,
                        query_pool.query_type,
                        query_pool.handle,
                        first_query,
                    )
            }
        }
    }
}

mod _other_cmd
{
    use ash::vk;

    use crate::framework::{
        core::{command_buffer::RhiCommandBuffer, pipeline::RhiPipeline},
        rhi::Rhi,
    };

    // 其他命令
    impl RhiCommandBuffer
    {
        pub fn push_constants(
            &mut self,
            pipeline: &RhiPipeline,
            stage: vk::ShaderStageFlags,
            offset: u32,
            data: &[u8],
        )
        {
            unsafe {
                Rhi::instance().device().cmd_push_constants(
                    self.command_buffer,
                    pipeline.pipeline_layout,
                    stage,
                    offset,
                    data,
                );
            }
        }
    }
}
