use ash::vk;
use itertools::Itertools;

use crate::core::debug_utils::{RhiDebugType, RhiDebugUtils};
use crate::core::rendering_info::RhiRenderingInfo;
use crate::core::resources::buffer::RhiBuffer;
use crate::core::synchronize::RhiBufferBarrier;
use crate::resources::managed_buffer::RhiManagedBuffer;
use crate::{
    basic::color::LabelColor,
    core::{
        command_pool::RhiCommandPool,
        command_queue::{RhiQueue, RhiSubmitInfo},
        device::RhiDevice,
        query_pool::RhiQueryPool,
        synchronize::RhiImageBarrier,
    },
    rhi::Rhi,
};

#[derive(Debug, Copy, Clone)]
pub struct RhiCommandBuffer {
    vk_handle: vk::CommandBuffer,
    command_pool_handle: vk::CommandPool,
}

/// 创建与销毁
impl RhiCommandBuffer {
    pub fn new(
        device: &RhiDevice,
        debug_utils: &RhiDebugUtils,
        command_pool: &RhiCommandPool,
        debug_name: &str,
    ) -> Self {
        let info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool.handle())
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = unsafe { device.allocate_command_buffers(&info).unwrap()[0] };
        let cmd_buffer = RhiCommandBuffer {
            vk_handle: command_buffer,
            command_pool_handle: command_pool.handle(),
        };
        debug_utils.set_debug_name(&cmd_buffer, debug_name);
        cmd_buffer
    }

    /// 释放 command buffer 在 command pool 中所占用的内存
    ///
    /// 释放之后 command buffer 就不存在了
    #[inline]
    pub fn free(self, device: &RhiDevice) {
        unsafe {
            device.free_command_buffers(self.command_pool_handle, std::slice::from_ref(&self.vk_handle));
        }
    }
}

/// Basic 命令
impl RhiCommandBuffer {
    /// 立即执行某个 command，并同步等待执行结果
    pub fn one_time_exec<F, R>(
        rhi: &Rhi,
        command_pool: &RhiCommandPool,
        queue: &RhiQueue,
        func: F,
        name: impl AsRef<str>,
    ) -> R
    where
        F: FnOnce(&RhiCommandBuffer) -> R,
    {
        let command_buffer = RhiCommandBuffer::new(
            rhi.device(),
            rhi.debug_utils(),
            command_pool,
            &format!("one-time-{}", name.as_ref()),
        );

        command_buffer.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, name.as_ref());
        let result = func(&command_buffer);
        command_buffer.end();

        queue.submit(rhi.device(), vec![RhiSubmitInfo::new(&[command_buffer])], None);
        queue.wait_idle(rhi.device());
        command_buffer.free(rhi.device());

        result
    }

    /// 开始录制 command
    ///
    /// 自动设置 debug label
    #[inline]
    pub fn begin(&self, device: &RhiDevice, usage_flag: vk::CommandBufferUsageFlags, debug_label_name: &str) {
        unsafe {
            device
                .begin_command_buffer(self.vk_handle, &vk::CommandBufferBeginInfo::default().flags(usage_flag))
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
        unsafe { self.device.end_command_buffer(self.vk_handle).unwrap() }
    }
}

/// getters
impl RhiCommandBuffer {
    /// getter
    #[inline]
    pub fn vk_handle(&self) -> vk::CommandBuffer {
        self.vk_handle
    }
}

/// 数据传输类型
impl RhiCommandBuffer {
    /// - command type: action
    /// - 支持的 queue：transfer，graphics，compute
    #[inline]
    pub fn cmd_copy_buffer_1(&self, src: &RhiBuffer, dst: &mut RhiBuffer, regions: &[vk::BufferCopy]) {
        unsafe {
            self.device.cmd_copy_buffer(self.vk_handle, src.handle(), dst.handle(), regions);
        }
    }

    /// - command type: action
    /// - 支持的 queue：transfer，graphics，compute
    #[inline]
    pub fn cmd_copy_buffer(&self, src: &RhiManagedBuffer, dst: &mut RhiManagedBuffer, regions: &[vk::BufferCopy]) {
        unsafe {
            self.device.cmd_copy_buffer(self.vk_handle, src.handle(), dst.handle(), regions);
        }
    }

    /// - command type: action
    /// - 支持的 queue：transfer，graphics，compute
    #[inline]
    pub fn cmd_copy_buffer_to_image(&self, copy_info: &vk::CopyBufferToImageInfo2) {
        unsafe { self.device.cmd_copy_buffer_to_image2(self.vk_handle, copy_info) }
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
        unsafe { self.device.cmd_update_buffer(self.vk_handle, buffer, offset, data) }
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
            self.device.cmd_push_constants(self.vk_handle, pipeline_layout, stage, offset, data);
        }
    }
}

/// 绘制类型的命令
impl RhiCommandBuffer {
    /// - command type: action, state
    /// - supported queue types: graphics
    #[inline]
    pub fn cmd_begin_rendering(&self, render_info: &vk::RenderingInfo) {
        unsafe {
            self.device.dynamic_rendering_pf().cmd_begin_rendering(self.vk_handle, render_info);
        }
    }

    pub fn cmd_begin_rendering2(&self, rendering_info: &RhiRenderingInfo) {
        let rendering_info = rendering_info.rendering_info();
        unsafe {
            self.device.dynamic_rendering_pf().cmd_begin_rendering(self.vk_handle, &rendering_info);
        }
    }

    /// - command type: action, state
    /// - supported queue types: graphics
    #[inline]
    pub fn end_rendering(&self) {
        unsafe {
            self.device.dynamic_rendering_pf().cmd_end_rendering(self.vk_handle);
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
                self.vk_handle,
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
    ///
    /// 不使用 index buffer 的绘制
    #[inline]
    pub fn cmd_draw(&self, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32) {
        unsafe {
            self.device.cmd_draw(self.vk_handle, vertex_count, instance_count, first_vertex, first_instance);
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
        dynamic_offsets: Option<&[u32]>,
    ) {
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                self.vk_handle,
                bind_point,
                pipeline_layout,
                first_set,
                descriptor_sets,
                dynamic_offsets.unwrap_or(&[]),
            );
        }
    }

    /// - command type: state
    /// - supported queue types: graphics, compute
    #[inline]
    pub fn cmd_bind_pipeline(&self, bind_point: vk::PipelineBindPoint, pipeline: vk::Pipeline) {
        unsafe {
            self.device.cmd_bind_pipeline(self.vk_handle, bind_point, pipeline);
        }
    }

    /// buffers 每个 vertex buffer 以及 offset
    /// - command type: state
    /// - supported queue types: graphics
    #[inline]
    pub fn cmd_bind_vertex_buffers(&self, first_bind: u32, buffers: &[RhiBuffer], offsets: &[vk::DeviceSize]) {
        unsafe {
            let buffers = buffers.iter().map(|b| b.handle()).collect_vec();
            self.device.cmd_bind_vertex_buffers(self.vk_handle, first_bind, &buffers, offsets);
        }
    }

    /// - command type: state
    /// - supported queue types: graphics
    #[inline]
    pub fn cmd_bind_index_buffer(&self, buffer: &RhiBuffer, offset: vk::DeviceSize, index_type: vk::IndexType) {
        unsafe {
            self.device.cmd_bind_index_buffer(self.vk_handle, buffer.handle(), offset, index_type);
        }
    }

    /// - command type: state
    /// - supported queue types: graphics
    #[inline]
    pub fn cmd_set_viewport(&self, first_viewport: u32, viewports: &[vk::Viewport]) {
        unsafe {
            self.device.cmd_set_viewport(self.vk_handle, first_viewport, viewports);
        }
    }

    /// - command type: state
    /// - supported queue types: graphics
    #[inline]
    pub fn cmd_set_scissor(&self, first_scissor: u32, scissors: &[vk::Rect2D]) {
        unsafe {
            self.device.cmd_set_scissor(self.vk_handle, first_scissor, scissors);
        }
    }
}

/// 光追相关
impl RhiCommandBuffer {
    /// - command type: action
    /// - supported queue types: compute
    #[inline]
    pub fn cmd_copy_acceleration_structure(&self, copy_info: &vk::CopyAccelerationStructureInfoKHR) {
        unsafe {
            self.device.acceleration_structure_pf().cmd_copy_acceleration_structure(self.vk_handle, copy_info);
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
            self.device.acceleration_structure_pf().cmd_build_acceleration_structures(
                self.vk_handle,
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
        query_pool: &mut RhiQueryPool,
        first_query: u32,
        acceleration_structures: &[vk::AccelerationStructureKHR],
    ) {
        unsafe {
            self.device.acceleration_structure_pf().cmd_write_acceleration_structures_properties(
                self.vk_handle,
                acceleration_structures,
                query_pool.query_type(),
                query_pool.handle(),
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
            self.device.rt_pipeline_pf().cmd_trace_rays(
                self.vk_handle,
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

/// 计算着色器相关命令
impl RhiCommandBuffer {
    #[inline]
    pub fn cmd_dispatch(&self, group_cnt: glam::UVec3) {
        unsafe {
            self.device.cmd_dispatch(self.vk_handle, group_cnt.x, group_cnt.y, group_cnt.z);
        }
    }
}

/// 同步相关命令
impl RhiCommandBuffer {
    /// - command type: synchronize
    /// - supported queue types: graphics, compute, transfer
    #[inline]
    pub fn memory_barrier(&self, barriers: &[vk::MemoryBarrier2]) {
        let dependency_info = vk::DependencyInfo::default().memory_barriers(barriers);
        unsafe {
            self.device.cmd_pipeline_barrier2(self.vk_handle, &dependency_info);
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
            self.device.cmd_pipeline_barrier2(self.vk_handle, &dependency_info);
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
            self.device.cmd_pipeline_barrier2(self.vk_handle, &dependency_info);
        }
    }
}

/// debug 相关命令
impl RhiCommandBuffer {
    /// - command type: state, action
    /// - supported queue type: graphics, compute
    #[inline]
    pub fn begin_label(&self, label_name: &str, label_color: glam::Vec4) {
        self.device.debug_utils().cmd_begin_debug_label(self.vk_handle, label_name, label_color);
    }

    /// - command type: state, action
    /// - supported queue type: graphics, compute
    #[inline]
    pub fn end_label(&self) {
        self.device.debug_utils().cmd_end_debug_label(self.vk_handle);
    }

    /// - command type: action
    /// - supported queue type: graphics, compute
    #[inline]
    pub fn insert_label(&self, label_name: &str, label_color: glam::Vec4) {
        self.device.debug_utils().cmd_insert_debug_label(self.vk_handle, label_name, label_color);
    }
}

impl RhiDebugType for RhiCommandBuffer {
    fn debug_type_name() -> &'static str {
        "RhiCommandBuffer"
    }

    fn vk_handle(&self) -> impl vk::Handle {
        self.vk_handle
    }
}
