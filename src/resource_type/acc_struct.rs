//! Ray Tracing 所需的加速结构

use ash::vk;
use itertools::Itertools;

use crate::{
    resource_type::{
        buffer::RhiBuffer, command_buffer::RhiCommandBuffer, query_pool::RhiQueryPool, queue::RhiSubmitBatch,
        sync_primitives::RhiFence,
    },
    rhi::Rhi,
};

pub struct RhiAcceleration
{
    handle: vk::AccelerationStructureKHR,
    buffer: RhiBuffer,
}


impl RhiAcceleration
{
    /// 需要指定每个 geometry 的信息，以及每个 geometry 拥有的 max primitives 数量
    /// 会自动添加 compact 和 trace 的 flag
    pub fn build_acceleration(
        flags: vk::BuildAccelerationStructureFlagsKHR,
        geometries: &[vk::AccelerationStructureGeometryKHR],
        range_info: &[vk::AccelerationStructureBuildRangeInfoKHR],
    ) -> Self
    {
        let rhi = Rhi::instance();

        let mut build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR {
            ty: vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
            flags: flags |
                vk::BuildAccelerationStructureFlagsKHR::ALLOW_COMPACTION |
                vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE,
            geometry_count: geometries.len() as u32,
            p_geometries: geometries.as_ptr(),

            // 在查询 size 时，其他字段暂时会被忽略
            ..Default::default()
        };

        let size_info = unsafe {
            let primitive_cnts = range_info.iter().map(|r| r.primitive_count).collect_vec();

            rhi.acc_struct_pf().get_acceleration_structure_build_sizes(
                vk::AccelerationStructureBuildTypeKHR::DEVICE,
                &build_geometry_info,
                &primitive_cnts,
            )
        };

        let (raw_acceleration_structure, raw_buffer) =
            Self::create_blas(size_info.acceleration_structure_size, vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL);

        let scratch_buffer =
            RhiBuffer::new_accleration_scratch_buffer(size_info.build_scratch_size as usize, "scratch_buffer");
        let scratch_buffer_addr = scratch_buffer.get_device_address();

        // 填充 build geometry info 的剩余部分
        build_geometry_info.mode = vk::BuildAccelerationStructureModeKHR::BUILD;
        build_geometry_info.dst_acceleration_structure = raw_acceleration_structure;
        build_geometry_info.scratch_data = vk::DeviceOrHostAddressKHR {
            device_address: scratch_buffer_addr,
        };

        // 创建一个 QueryPool，用于查询 compact size
        let mut query_pool = RhiQueryPool::new(vk::QueryType::ACCELERATION_STRUCTURE_COMPACTED_SIZE_KHR, 1, "");
        query_pool.reset(0, 1);

        // 等待初步 build 完成
        RhiCommandBuffer::one_time_exec(vk::QueueFlags::GRAPHICS, |cmd| {
            cmd.build_blas(&build_geometry_info, range_info);
            cmd.write_acceleration_structure_properties(
                &mut query_pool,
                0,
                std::slice::from_ref(&build_geometry_info.dst_acceleration_structure),
            );
        });

        // 提供更紧凑的 acceleration
        let compact_size: Vec<vk::DeviceSize> = query_pool.get_query_result(0, 1);
        let (compact_acceleration_structure, buffer) =
            Self::create_blas(compact_size[0], vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL);

        RhiCommandBuffer::one_time_exec(vk::QueueFlags::GRAPHICS, |cmd| {
            let copy_info = vk::CopyAccelerationStructureInfoKHR {
                src: raw_acceleration_structure,
                dst: compact_acceleration_structure,
                mode: vk::CopyAccelerationStructureModeKHR::COMPACT,
                ..Default::default()
            };
            cmd.copy_acceleration_structure(&copy_info);
        });

        // 回收临时资源
        unsafe {
            rhi.acc_struct_pf().destroy_acceleration_structure(raw_acceleration_structure, None);
            raw_buffer.destroy();
            scratch_buffer.destroy();
            query_pool.destroy();
        }

        Self {
            handle: compact_acceleration_structure,
            buffer,
        }
    }


    /// 创建 AccelerationStructure 以及 buffer    
    fn create_blas(
        size: vk::DeviceSize,
        ty: vk::AccelerationStructureTypeKHR,
    ) -> (vk::AccelerationStructureKHR, RhiBuffer)
    {
        let buffer = RhiBuffer::new_accleration_buffer(size as usize, "");

        let create_info = vk::AccelerationStructureCreateInfoKHR {
            ty,
            size,
            buffer: buffer.buffer,
            ..Default::default()
        };

        let acc_structure =
            unsafe { Rhi::instance().acc_struct_pf().create_acceleration_structure(&create_info, None).unwrap() };

        (acc_structure, buffer)
    }
}
