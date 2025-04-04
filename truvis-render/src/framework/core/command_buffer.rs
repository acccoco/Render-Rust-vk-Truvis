use ash::vk;
use itertools::Itertools;

use crate::framework::{
    basic::color::LabelColor,
    core::{buffer::Buffer, command_pool::CommandPool, pipeline::Pipeline, query_pool::QueryPool, queue::SubmitInfo},
    render_core::Core,
};

#[derive(Clone)]
pub struct CommandBuffer
{
    pub(crate) command_buffer: vk::CommandBuffer,
    pub(crate) command_pool: vk::CommandPool,

    rhi: &'static Core,
}

impl CommandBuffer
{
    pub fn new<S: AsRef<str>>(rhi: &'static Core, pool: &CommandPool, debug_name: S) -> Self
    {
        let info = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = unsafe { rhi.vk_device().allocate_command_buffers(&info).unwrap()[0] };
        rhi.set_debug_name(command_buffer, debug_name);
        Self {
            command_buffer,
            command_pool: pool.command_pool,
            rhi,
        }
    }

    /// 从 Rhi 中的 command pool 分配 command buffer 进行执行
    pub fn one_time_exec<F, R>(rhi: &'static Core, ty: vk::QueueFlags, f: F, name: &str) -> R
    where
        F: FnOnce(&mut CommandBuffer) -> R,
    {
        let pool;
        let queue;
        match ty {
            vk::QueueFlags::COMPUTE => {
                pool = &rhi.compute_command_pool;
                queue = rhi.compute_queue();
            }
            vk::QueueFlags::TRANSFER => {
                pool = &rhi.transfer_command_pool;
                queue = rhi.transfer_queue();
            }
            vk::QueueFlags::GRAPHICS => {
                pool = &rhi.graphics_command_pool;
                queue = rhi.graphics_queue();
            }
            other => panic!("not supported queue type: SPARSE_BINDING, {:?}", other),
        }

        let mut command_buffer = Self::new(rhi, pool, format!("one-time-{}", name));

        command_buffer.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, name);
        let result = f(&mut command_buffer);
        command_buffer.end();

        queue.submit(
            rhi,
            vec![SubmitInfo {
                command_buffers: vec![command_buffer.clone()],
                ..Default::default()
            }],
            None,
        );
        queue.wait_idle(rhi);
        command_buffer.free();

        result
    }

    #[inline]
    pub fn free(self)
    {
        unsafe {
            self.rhi.vk_device().free_command_buffers(self.command_pool, std::slice::from_ref(&self.command_buffer));
        }
    }

    #[inline]
    pub fn begin(&mut self, usage_flag: vk::CommandBufferUsageFlags, label_name: &str)
    {
        unsafe {
            self.rhi
                .vk_device()
                .begin_command_buffer(self.command_buffer, &vk::CommandBufferBeginInfo::default().flags(usage_flag))
                .unwrap();
        }
        self.begin_label(label_name, LabelColor::COLOR_CMD);
    }

    #[inline]
    pub fn end(&mut self)
    {
        self.end_label();
        unsafe { self.rhi.vk_device().end_command_buffer(self.command_buffer).unwrap() }
    }
}


// transfer 类型的命令
impl CommandBuffer
{
    #[inline]
    pub fn copy_buffer(&mut self, src: &Buffer, dst: &mut Buffer, regions: &[vk::BufferCopy])
    {
        unsafe {
            self.rhi.vk_device().cmd_copy_buffer(self.command_buffer, src.handle, dst.handle, regions);
        }
    }

    /// 注：仅支持 compute queue
    #[inline]
    pub fn copy_acceleration_structure(&mut self, copy_info: &vk::CopyAccelerationStructureInfoKHR)
    {
        unsafe {
            self.rhi.vk_acceleration_struct_pf.cmd_copy_acceleration_structure(self.command_buffer, copy_info);
        }
    }

    #[inline]
    pub fn cmd_copy_buffer_to_image(&mut self, copy_info: &vk::CopyBufferToImageInfo2)
    {
        unsafe { self.rhi.vk_device().cmd_copy_buffer_to_image2(self.command_buffer, copy_info) }
    }

    /// 将 data 传输到 buffer 中，大小限制：65536Bytes
    ///
    /// 首先将 data copy 到 cmd buffer 中，然后再 transfer 到指定 buffer 中，这是一个  transfer op
    ///
    /// 需要在 render pass 之外进行，注意同步
    #[inline]
    pub fn cmd_update_buffer(&mut self, buffer: vk::Buffer, offset: vk::DeviceSize, data: &[u8])
    {
        unsafe { self.rhi.vk_device().cmd_update_buffer(self.command_buffer, buffer, offset, data) }
    }
}

// 绘制类型命令
impl CommandBuffer
{
    #[inline]
    pub fn cmd_begin_rendering(&mut self, render_info: &vk::RenderingInfo)
    {
        unsafe {
            self.rhi.vk_dynamic_render_pf.cmd_begin_rendering(self.command_buffer, render_info);
        }
    }

    #[inline]
    pub fn end_rendering(&mut self)
    {
        unsafe {
            self.rhi.vk_dynamic_render_pf.cmd_end_rendering(self.command_buffer);
        }
    }

    /// index_info: (index_count, first_index)
    ///
    /// instance_info: (instance_count, first_instance)
    #[inline]
    pub fn draw_indexed(&mut self, index_info: (u32, u32), instance_info: (u32, u32), vertex_offset: i32)
    {
        unsafe {
            self.rhi.vk_device().cmd_draw_indexed(
                self.command_buffer,
                index_info.0,
                instance_info.0,
                index_info.1,
                vertex_offset,
                instance_info.1,
            );
        }
    }

    #[inline]
    pub fn draw(&mut self, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32)
    {
        unsafe {
            self.rhi.vk_device().cmd_draw(
                self.command_buffer,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            );
        }
    }

    #[inline]
    pub fn draw_indexed2(
        &mut self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    )
    {
        unsafe {
            self.rhi.vk_device().cmd_draw_indexed(
                self.command_buffer,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            );
        }
    }

    #[inline]
    pub fn cmd_push_constants(
        &mut self,
        pipeline_layout: vk::PipelineLayout,
        stage: vk::ShaderStageFlags,
        offset: u32,
        data: &[u8],
    )
    {
        unsafe {
            self.rhi.vk_device().cmd_push_constants(self.command_buffer, pipeline_layout, stage, offset, data);
        }
    }

    #[inline]
    pub fn bind_descriptor_sets(
        &mut self,
        bind_point: vk::PipelineBindPoint,
        pipeline_layout: vk::PipelineLayout,
        first_set: u32,
        descriptor_sets: &[vk::DescriptorSet],
        dynamic_offsets: &[u32],
    )
    {
        unsafe {
            self.rhi.vk_device().cmd_bind_descriptor_sets(
                self.command_buffer,
                bind_point,
                pipeline_layout,
                first_set,
                descriptor_sets,
                dynamic_offsets,
            );
        }
    }


    #[inline]
    pub fn bind_pipeline(&mut self, bind_point: vk::PipelineBindPoint, pipeline: vk::Pipeline)
    {
        unsafe {
            self.rhi.vk_device().cmd_bind_pipeline(self.command_buffer, bind_point, pipeline);
        }
    }

    /// buffers 每个 vertex buffer 以及 offset
    #[inline]
    pub fn bind_vertex_buffer(&mut self, first_bind: u32, buffers: &[Buffer], offsets: &[vk::DeviceSize])
    {
        unsafe {
            let buffers = buffers.iter().map(|b| b.handle).collect_vec();
            self.rhi.vk_device().cmd_bind_vertex_buffers(self.command_buffer, first_bind, &buffers, offsets);
        }
    }

    #[inline]
    pub fn bind_index_buffer(&mut self, buffer: &Buffer, offset: vk::DeviceSize, index_type: vk::IndexType)
    {
        unsafe {
            self.rhi.vk_device().cmd_bind_index_buffer(self.command_buffer, buffer.handle, offset, index_type);
        }
    }

    #[inline]
    pub fn cmd_set_viewport(&mut self, first_viewport: u32, viewports: &[vk::Viewport])
    {
        unsafe {
            self.rhi.vk_device().cmd_set_viewport(self.command_buffer, first_viewport, viewports);
        }
    }

    #[inline]
    pub fn cmd_set_scissor(&mut self, first_scissor: u32, scissors: &[vk::Rect2D])
    {
        unsafe {
            self.rhi.vk_device().cmd_set_scissor(self.command_buffer, first_scissor, scissors);
        }
    }
}

// 同步命令
impl CommandBuffer
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
        let barrier = vk::ImageMemoryBarrier::default()
            .src_access_mask(src.1)
            .dst_access_mask(dst.1)
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(
                vk::ImageSubresourceRange::default().aspect_mask(image_aspect).layer_count(1).level_count(1),
            );

        unsafe {
            self.rhi.vk_device().cmd_pipeline_barrier(
                self.command_buffer,
                src.0,
                dst.0,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    }

    #[inline]
    pub fn memory_barrier(&mut self, barriers: &[vk::MemoryBarrier2])
    {
        let dependency_info = vk::DependencyInfo::default().memory_barriers(barriers);
        unsafe {
            self.rhi.vk_device().cmd_pipeline_barrier2(self.command_buffer, &dependency_info);
        }
    }

    #[inline]
    pub fn image_memory_barrier(&mut self, dependency_flags: vk::DependencyFlags, barriers: &[vk::ImageMemoryBarrier2])
    {
        let dependency_info =
            vk::DependencyInfo::default().image_memory_barriers(barriers).dependency_flags(dependency_flags);
        unsafe {
            self.rhi.vk_device().cmd_pipeline_barrier2(self.command_buffer, &dependency_info);
        }
    }

    #[inline]
    pub fn buffer_memory_barrier(
        &mut self,
        dependency_flags: vk::DependencyFlags,
        barriers: &[vk::BufferMemoryBarrier2],
    )
    {
        let dependency_info =
            vk::DependencyInfo::default().buffer_memory_barriers(barriers).dependency_flags(dependency_flags);
        unsafe {
            self.rhi.vk_device().cmd_pipeline_barrier2(self.command_buffer, &dependency_info);
        }
    }
}


// RayTracing 相关的命令
impl CommandBuffer
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
            self.rhi.vk_acceleration_struct_pf.cmd_build_acceleration_structures(
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
        query_pool: &mut QueryPool,
        first_query: u32,
        acceleration_structures: &[vk::AccelerationStructureKHR],
    )
    {
        unsafe {
            self.rhi.vk_acceleration_struct_pf.cmd_write_acceleration_structures_properties(
                self.command_buffer,
                acceleration_structures,
                query_pool.query_type,
                query_pool.handle,
                first_query,
            )
        }
    }
}

// 其他命令
impl CommandBuffer
{
    pub fn push_constants(&mut self, pipeline: &Pipeline, stage: vk::ShaderStageFlags, offset: u32, data: &[u8])
    {
        unsafe {
            self.rhi.vk_device().cmd_push_constants(self.command_buffer, pipeline.pipeline_layout, stage, offset, data);
        }
    }

    #[inline]
    pub fn begin_label(&mut self, label_name: &str, label_color: glam::Vec4)
    {
        self.rhi.debug_utils.cmd_begin_debug_label(self.command_buffer, label_name, label_color);
    }

    #[inline]
    pub fn end_label(&mut self)
    {
        self.rhi.debug_utils.cmd_end_debug_label(self.command_buffer);
    }

    #[inline]
    pub fn insert_label(&mut self, label_name: &str, label_color: glam::Vec4)
    {
        self.rhi.debug_utils.cmd_insert_debug_label(self.command_buffer, label_name, label_color);
    }
}
