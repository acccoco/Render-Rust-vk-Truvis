use std::rc::Rc;

use ash::vk;
use itertools::Itertools;

use crate::core::synchronize::RhiBufferBarrier;
use crate::{
    basic::color::LabelColor,
    core::{
        buffer::RhiBuffer,
        command_pool::RhiCommandPool,
        command_queue::{RhiQueue, RhiSubmitInfo},
        device::RhiDevice,
        query_pool::QueryPool,
        synchronize::RhiImageBarrier,
    },
    rhi::Rhi,
};

/// 不能实现 Drop，因为需要手动去 free；cmd 支持 clone，不应该在意外的地方 free
/// impl Drop for RhiCommandBuffer {}
#[derive(Clone)]
pub struct RhiCommandBuffer {
    handle: vk::CommandBuffer,

    /// command buffer 在需要通过 command pool 进行 free，因此需要保存 command pool 的引用
    pub command_pool: Rc<RhiCommandPool>,

    pub device: Rc<RhiDevice>,
}

// basic 命令
impl RhiCommandBuffer {
    pub fn new(device: Rc<RhiDevice>, command_pool: Rc<RhiCommandPool>, debug_name: &str) -> Self {
        let info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool.handle())
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = unsafe { device.allocate_command_buffers(&info).unwrap()[0] };
        device.debug_utils.set_object_debug_name(command_buffer, debug_name);
        RhiCommandBuffer {
            handle: command_buffer,
            command_pool,

            device,
        }
    }

    /// getter
    #[inline]
    pub fn handle(&self) -> vk::CommandBuffer {
        self.handle
    }

    /// 立即执行某个 command，并同步等待执行结果
    pub fn one_time_exec<F, R>(rhi: &Rhi, command_pool: Rc<RhiCommandPool>, queue: &RhiQueue, func: F, name: &str) -> R
    where
        F: FnOnce(&RhiCommandBuffer) -> R,
    {
        let command_buffer = RhiCommandBuffer::new(rhi.device.clone(), command_pool, &format!("one-time-{}", name));

        command_buffer.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, name);
        let result = func(&command_buffer);
        command_buffer.end();

        queue.submit(vec![RhiSubmitInfo::new(&[command_buffer.clone()])], None);
        queue.wait_idle();
        command_buffer.free();

        result
    }

    /// 释放 command buffer 在 command pool 中所占用的内存
    ///
    /// 释放之后 command buffer 就不存在了
    #[inline]
    pub fn free(self) {
        unsafe {
            self.device.free_command_buffers(self.command_pool.handle(), std::slice::from_ref(&self.handle));
        }
    }

    /// 开始录制 command
    ///
    /// 自动设置 debug label
    #[inline]
    pub fn begin(&self, usage_flag: vk::CommandBufferUsageFlags, debug_label_name: &str) {
        unsafe {
            self.device
                .begin_command_buffer(self.handle, &vk::CommandBufferBeginInfo::default().flags(usage_flag))
                .unwrap();
        }
        self.begin_label(debug_label_name, LabelColor::COLOR_CMD);
    }

    /// 结束录制 command
    ///
    /// 结束 debug label
    #[inline]
    pub fn end(&self) {
        self.end_label();
        unsafe { self.device.end_command_buffer(self.handle).unwrap() }
    }
}

// transfer 类型的命令
impl RhiCommandBuffer {
    /// - command type: action
    /// - 支持的 queue：transfer，graphics，compute
    #[inline]
    pub fn cmd_copy_buffer(&self, src: &RhiBuffer, dst: &mut RhiBuffer, regions: &[vk::BufferCopy]) {
        unsafe {
            self.device.cmd_copy_buffer(self.handle, src.handle(), dst.handle(), regions);
        }
    }

    /// - command type: action
    /// - 支持的 queue：transfer，graphics，compute
    #[inline]
    pub fn cmd_copy_buffer_to_image(&self, copy_info: &vk::CopyBufferToImageInfo2) {
        unsafe { self.device.cmd_copy_buffer_to_image2(self.handle, copy_info) }
    }

    /// 将 data 传输到 buffer 中，大小限制：65536Bytes=64KB
    ///
    /// 首先将 data copy 到 cmd buffer 中，然后再 transfer 到指定 buffer 中，这是一个  transfer op
    ///
    /// 需要在 render pass 之外进行，注意同步
    ///
    ///
    /// - command type: action
    /// - supported queue types: transfer, graphics, compute
    #[inline]
    pub fn cmd_update_buffer(&self, buffer: vk::Buffer, offset: vk::DeviceSize, data: &[u8]) {
        unsafe { self.device.cmd_update_buffer(self.handle, buffer, offset, data) }
    }

    /// - command type: state
    /// - 支持的 queue: graphics, compute
    #[inline]
    pub fn cmd_push_constants(
        &self,
        pipeline_layout: vk::PipelineLayout,
        stage: vk::ShaderStageFlags,
        offset: u32,
        data: &[u8],
    ) {
        unsafe {
            self.device.cmd_push_constants(self.handle, pipeline_layout, stage, offset, data);
        }
    }
}

// 绘制类型命令
impl RhiCommandBuffer {
    /// - command type: action, state
    /// - supported queue types: graphics
    #[inline]
    pub fn cmd_begin_rendering(&self, render_info: &vk::RenderingInfo) {
        unsafe {
            self.device.vk_dynamic_render_pf.cmd_begin_rendering(self.handle, render_info);
        }
    }

    /// - command type: action, state
    /// - supported queue types: graphics
    #[inline]
    pub fn end_rendering(&self) {
        unsafe {
            self.device.vk_dynamic_render_pf.cmd_end_rendering(self.handle);
        }
    }

    /// - command type: action
    /// - supported queue types: graphics
    #[inline]
    pub fn draw_indexed(
        &self,
        index_cnt: u32,
        first_index: u32,
        instance_cnt: u32,
        first_instance: u32,
        vertex_offset: i32,
    ) {
        unsafe {
            self.device.cmd_draw_indexed(
                self.handle,
                index_cnt,
                instance_cnt,
                first_index,
                vertex_offset,
                first_instance,
            );
        }
    }

    /// - command type: action
    /// - supported queue types: graphics
    /// 不使用 index buffer 的绘制
    #[inline]
    pub fn cmd_draw(&self, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32) {
        unsafe {
            self.device.cmd_draw(self.handle, vertex_count, instance_count, first_vertex, first_instance);
        }
    }

    /// - command type: state
    /// - supported queue types: graphics, compute
    #[inline]
    pub fn bind_descriptor_sets(
        &self,
        bind_point: vk::PipelineBindPoint,
        pipeline_layout: vk::PipelineLayout,
        first_set: u32,
        descriptor_sets: &[vk::DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                self.handle,
                bind_point,
                pipeline_layout,
                first_set,
                descriptor_sets,
                dynamic_offsets,
            );
        }
    }

    /// - command type: state
    /// - supported queue types: graphics, compute
    #[inline]
    pub fn cmd_bind_pipeline(&self, bind_point: vk::PipelineBindPoint, pipeline: vk::Pipeline) {
        unsafe {
            self.device.cmd_bind_pipeline(self.handle, bind_point, pipeline);
        }
    }

    /// buffers 每个 vertex buffer 以及 offset
    /// - command type: state
    /// - supported queue types: graphics
    #[inline]
    pub fn cmd_bind_vertex_buffers(&self, first_bind: u32, buffers: &[RhiBuffer], offsets: &[vk::DeviceSize]) {
        unsafe {
            let buffers = buffers.iter().map(|b| b.handle()).collect_vec();
            self.device.cmd_bind_vertex_buffers(self.handle, first_bind, &buffers, offsets);
        }
    }

    /// - command type: state
    /// - supported queue types: graphics
    #[inline]
    pub fn cmd_bind_index_buffer(&self, buffer: &RhiBuffer, offset: vk::DeviceSize, index_type: vk::IndexType) {
        unsafe {
            self.device.cmd_bind_index_buffer(self.handle, buffer.handle(), offset, index_type);
        }
    }

    /// - command type: state
    /// - supported queue types: graphics
    #[inline]
    pub fn cmd_set_viewport(&self, first_viewport: u32, viewports: &[vk::Viewport]) {
        unsafe {
            self.device.cmd_set_viewport(self.handle, first_viewport, viewports);
        }
    }

    /// - command type: state
    /// - supported queue types: graphics
    #[inline]
    pub fn cmd_set_scissor(&self, first_scissor: u32, scissors: &[vk::Rect2D]) {
        unsafe {
            self.device.cmd_set_scissor(self.handle, first_scissor, scissors);
        }
    }
}

// 同步命令
impl RhiCommandBuffer {
    /// - command type: synchronize
    /// - supported queue types: graphics, compute, transfer
    #[inline]
    pub fn memory_barrier(&self, barriers: &[vk::MemoryBarrier2]) {
        let dependency_info = vk::DependencyInfo::default().memory_barriers(barriers);
        unsafe {
            self.device.cmd_pipeline_barrier2(self.handle, &dependency_info);
        }
    }

    /// - command type: synchronize
    /// - supported queue types: graphics, compute, transfer
    #[inline]
    pub fn image_memory_barrier(&self, dependency_flags: vk::DependencyFlags, barriers: &[RhiImageBarrier]) {
        let barriers = barriers.iter().map(|b| *b.inner()).collect_vec();
        let dependency_info =
            vk::DependencyInfo::default().image_memory_barriers(&barriers).dependency_flags(dependency_flags);
        unsafe {
            self.device.cmd_pipeline_barrier2(self.handle, &dependency_info);
        }
    }

    /// - command type: synchronize
    /// - supported queue types: graphics, compute, transfer
    #[inline]
    pub fn buffer_memory_barrier(&self, dependency_flags: vk::DependencyFlags, barriers: &[RhiBufferBarrier]) {
        let barriers = barriers.iter().map(|b| *b.inner()).collect_vec();
        let dependency_info =
            vk::DependencyInfo::default().buffer_memory_barriers(&barriers).dependency_flags(dependency_flags);
        unsafe {
            self.device.cmd_pipeline_barrier2(self.handle, &dependency_info);
        }
    }
}

// RayTracing 相关的命令
impl RhiCommandBuffer {
    /// - command type: action
    /// - supported queue types: compute
    #[inline]
    pub fn cmd_copy_acceleration_structure(&self, copy_info: &vk::CopyAccelerationStructureInfoKHR) {
        unsafe {
            self.device.vk_acceleration_struct_pf.cmd_copy_acceleration_structure(self.handle, copy_info);
        }
    }

    /// - command type: action
    /// - supported queue types: compute
    #[inline]
    pub fn build_acceleration_structure(
        &self,
        geometry: &vk::AccelerationStructureBuildGeometryInfoKHR,
        ranges: &[vk::AccelerationStructureBuildRangeInfoKHR],
    ) {
        unsafe {
            // 该函数可以一次构建多个 AccelerationStructure，这里只构建了 1 个
            self.device.vk_acceleration_struct_pf.cmd_build_acceleration_structures(
                self.handle,
                std::slice::from_ref(geometry),
                &[ranges],
            )
        }
    }

    /// 这里涉及到对加速结构的 read，需要同步
    /// - command type: action
    /// - supported queue types: compute
    #[inline]
    pub fn write_acceleration_structure_properties(
        &self,
        query_pool: &mut QueryPool,
        first_query: u32,
        acceleration_structures: &[vk::AccelerationStructureKHR],
    ) {
        unsafe {
            self.device.vk_acceleration_struct_pf.cmd_write_acceleration_structures_properties(
                self.handle,
                acceleration_structures,
                query_pool.query_type,
                query_pool.handle,
                first_query,
            )
        }
    }

    /// 光追的入口
    /// - command type: action
    /// - supported queue types: compute
    #[inline]
    pub fn trace_rays(
        &self,
        raygen_table: &vk::StridedDeviceAddressRegionKHR,
        miss_table: &vk::StridedDeviceAddressRegionKHR,
        hit_table: &vk::StridedDeviceAddressRegionKHR,
        callable_table: &vk::StridedDeviceAddressRegionKHR,
        thread_size: [u32; 3],
    ) {
        unsafe {
            self.device.vk_rt_pipeline_pf.cmd_trace_rays(
                self.handle,
                raygen_table,
                miss_table,
                hit_table,
                callable_table,
                thread_size[0],
                thread_size[1],
                thread_size[2],
            );
        }
    }
}

// debug 相关的指令
impl RhiCommandBuffer {
    /// - command type: state, action
    /// - supported queue type: graphics, compute
    #[inline]
    pub fn begin_label(&self, label_name: &str, label_color: glam::Vec4) {
        self.device.debug_utils.cmd_begin_debug_label(self.handle, label_name, label_color);
    }

    /// - command type: state, action
    /// - supported queue type: graphics, compute
    #[inline]
    pub fn end_label(&self) {
        self.device.debug_utils.cmd_end_debug_label(self.handle);
    }

    /// - command type: action
    /// - supported queue type: graphics, compute
    #[inline]
    pub fn insert_label(&self, label_name: &str, label_color: glam::Vec4) {
        self.device.debug_utils.cmd_insert_debug_label(self.handle, label_name, label_color);
    }
}
