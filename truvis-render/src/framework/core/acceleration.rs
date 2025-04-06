//! Ray Tracing 所需的加速结构

use std::rc::Rc;

use ash::vk;
use itertools::Itertools;

use crate::framework::{
    core::{buffer::RhiBuffer, command_buffer::RhiCommandBuffer, device::RhiDevice, query_pool::QueryPool},
    render_core::Rhi,
};

pub struct Acceleration
{
    acceleration_structure: vk::AccelerationStructureKHR,
    buffer: RhiBuffer,

    device: Rc<RhiDevice>,
}


impl Acceleration
{
    /// 需要指定每个 geometry 的信息，以及每个 geometry 拥有的 max primitives 数量
    /// 会自动添加 compact 和 trace 的 flag
    pub fn build_blas(
        rhi: &Rhi,
        data: Vec<(vk::AccelerationStructureGeometryTrianglesDataKHR, u32)>,
        flags: vk::BuildAccelerationStructureFlagsKHR,
        debug_name: &str,
    ) -> Self
    {
        let mut geometries = Vec::with_capacity(data.len());
        let mut range_infos = Vec::with_capacity(data.len());
        data.into_iter().for_each(|(triangle_data, primitive_cnt)| {
            geometries.push(vk::AccelerationStructureGeometryKHR {
                geometry_type: vk::GeometryTypeKHR::TRIANGLES,
                flags: vk::GeometryFlagsKHR::OPAQUE,
                geometry: vk::AccelerationStructureGeometryDataKHR {
                    triangles: triangle_data,
                },
                ..Default::default()
            });

            range_infos.push(vk::AccelerationStructureBuildRangeInfoKHR {
                first_vertex: 0,
                primitive_count: primitive_cnt,
                primitive_offset: 0,
                transform_offset: 0,
            });
        });

        // 使用部分完整的 AccelerationStructureBuildGeometryInfo 来查询所需的资源大小
        let mut build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR {
            ty: vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
            flags: flags |
                vk::BuildAccelerationStructureFlagsKHR::ALLOW_COMPACTION |
                vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE,
            geometry_count: geometries.len() as u32,
            p_geometries: geometries.as_ptr(),
            mode: vk::BuildAccelerationStructureModeKHR::BUILD,

            // 在查询 size 时，其他字段暂时会被忽略
            ..Default::default()
        };

        let size_info = unsafe {
            let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
            rhi.device.vk_acceleration_struct_pf.get_acceleration_structure_build_sizes(
                vk::AccelerationStructureBuildTypeKHR::DEVICE,
                &build_geometry_info,
                &range_infos.iter().map(|r| r.primitive_count).collect_vec(),
                &mut size_info,
            );
            size_info
        };

        let uncompact_acceleration = Self::new(
            rhi,
            size_info.acceleration_structure_size,
            vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
            &format!("{}-uncompact-blas", debug_name),
        );

        let scratch_buffer = RhiBuffer::new_accleration_scratch_buffer(
            rhi,
            size_info.build_scratch_size,
            &format!("{}-blas-scratch-buffer", debug_name),
        );

        // 填充 build geometry info 的剩余部分以 build AccelerationStructure
        build_geometry_info.dst_acceleration_structure = uncompact_acceleration.acceleration_structure;
        build_geometry_info.scratch_data = vk::DeviceOrHostAddressKHR {
            device_address: scratch_buffer.get_device_address(),
        };

        // 创建一个 QueryPool，用于查询 compact size
        let mut query_pool = QueryPool::new(rhi, vk::QueryType::ACCELERATION_STRUCTURE_COMPACTED_SIZE_KHR, 1, "");
        query_pool.reset(0, 1);

        // 等待初步 build 完成
        RhiCommandBuffer::one_time_exec(
            rhi,
            rhi.compute_command_pool.clone(),
            &rhi.compute_queue,
            |cmd| {
                cmd.build_acceleration_structure(&build_geometry_info, &range_infos);
                cmd.memory_barrier(std::slice::from_ref(&vk::MemoryBarrier2 {
                    src_stage_mask: vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR,
                    dst_stage_mask: vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR,
                    src_access_mask: vk::AccessFlags2::ACCELERATION_STRUCTURE_WRITE_KHR,
                    dst_access_mask: vk::AccessFlags2::ACCELERATION_STRUCTURE_READ_KHR,
                    ..Default::default()
                }));
                cmd.write_acceleration_structure_properties(
                    &mut query_pool,
                    0,
                    std::slice::from_ref(&build_geometry_info.dst_acceleration_structure),
                );
            },
            "build-blas",
        );

        // 提供更紧凑的 acceleration
        let compact_size: Vec<vk::DeviceSize> = query_pool.get_query_result(0, 1);
        let compact_acceleration = Self::new(
            rhi,
            compact_size[0],
            vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
            &format!("{}-compact-blas", debug_name),
        );

        RhiCommandBuffer::one_time_exec(
            rhi,
            rhi.compute_command_pool.clone(),
            &rhi.compute_queue,
            |cmd| {
                let copy_info = vk::CopyAccelerationStructureInfoKHR {
                    src: uncompact_acceleration.acceleration_structure,
                    dst: compact_acceleration.acceleration_structure,
                    mode: vk::CopyAccelerationStructureModeKHR::COMPACT,
                    ..Default::default()
                };
                cmd.cmd_copy_acceleration_structure(&copy_info);
            },
            "compact-blas",
        );

        // 回收临时资源
        {
            uncompact_acceleration.destroy();
            scratch_buffer.destroy();
            query_pool.destroy();
        }

        compact_acceleration
    }

    pub fn build_tlas(
        rhi: &Rhi,
        instances: &[vk::AccelerationStructureInstanceKHR],
        flags: vk::BuildAccelerationStructureFlagsKHR,
        debug_name: &str,
    ) -> Self
    {
        let mut acceleration_instance_buffer = RhiBuffer::new_acceleration_instance_buffer(
            rhi,
            size_of_val(instances) as vk::DeviceSize,
            format!("{}-acceleration-instance-buffer", debug_name),
        );
        acceleration_instance_buffer.transfer_data_by_stage_buffer(rhi, instances);

        let geometry = vk::AccelerationStructureGeometryKHR {
            geometry_type: vk::GeometryTypeKHR::INSTANCES,
            geometry: vk::AccelerationStructureGeometryDataKHR {
                instances: vk::AccelerationStructureGeometryInstancesDataKHR {
                    // true: data 是 &[vk::AccelerationStructureInstanceKHR]
                    // false: data 是 &[&vk::AccelerationStructureInstanceKHR]
                    array_of_pointers: vk::FALSE,
                    data: vk::DeviceOrHostAddressConstKHR {
                        device_address: acceleration_instance_buffer.get_device_address(),
                    },
                    ..Default::default()
                },
            },
            ..Default::default()
        };

        let mut geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR {
            flags: flags | vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE,
            geometry_count: 1,
            p_geometries: (&geometry) as _,
            mode: vk::BuildAccelerationStructureModeKHR::BUILD,
            ty: vk::AccelerationStructureTypeKHR::TOP_LEVEL,
            ..Default::default()
        };

        // 获得 AccelerationStructure 所需的尺寸
        let size_info = unsafe {
            let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
            rhi.device.vk_acceleration_struct_pf.get_acceleration_structure_build_sizes(
                vk::AccelerationStructureBuildTypeKHR::DEVICE,
                &geometry_info,
                &[instances.len() as u32],
                &mut size_info,
            );

            size_info
        };

        let acceleration = Self::new(
            rhi,
            size_info.acceleration_structure_size,
            vk::AccelerationStructureTypeKHR::TOP_LEVEL,
            &format!("{}-tlas", debug_name),
        );

        let scratch_buffer = RhiBuffer::new_accleration_scratch_buffer(
            rhi,
            size_info.build_scratch_size,
            format!("{}-tlas-scratch-buffer", debug_name),
        );

        // 补全剩下的 build info
        geometry_info.dst_acceleration_structure = acceleration.acceleration_structure;
        geometry_info.scratch_data.device_address = scratch_buffer.get_device_address();

        // range info
        let range_info = vk::AccelerationStructureBuildRangeInfoKHR {
            primitive_count: instances.len() as u32,
            ..Default::default()
        };

        // 正式构建 TLAS
        RhiCommandBuffer::one_time_exec(
            rhi,
            rhi.compute_command_pool.clone(),
            &rhi.compute_queue,
            |cmd| {
                cmd.build_acceleration_structure(&geometry_info, std::slice::from_ref(&range_info));
            },
            "build-tlas",
        );

        // 回收资源
        {
            acceleration_instance_buffer.destroy();
            scratch_buffer.destroy();
        }

        acceleration
    }

    /// 创建 AccelerationStructure 以及 buffer    
    fn new(rhi: &Rhi, size: vk::DeviceSize, ty: vk::AccelerationStructureTypeKHR, debug_name: &str) -> Self
    {
        let buffer = RhiBuffer::new_accleration_buffer(rhi, size as usize, debug_name);

        let create_info = vk::AccelerationStructureCreateInfoKHR {
            ty,
            size,
            buffer: buffer.handle(),
            ..Default::default()
        };

        let acceleration_structure =
            unsafe { rhi.device.vk_acceleration_struct_pf.create_acceleration_structure(&create_info, None).unwrap() };
        rhi.set_debug_name(acceleration_structure, debug_name);

        Self {
            device: rhi.device.clone(),
            acceleration_structure,
            buffer,
        }
    }


    #[inline]
    pub fn get_device_address(&self) -> vk::DeviceAddress
    {
        unsafe {
            self.device.vk_acceleration_struct_pf.get_acceleration_structure_device_address(
                &vk::AccelerationStructureDeviceAddressInfoKHR::default()
                    .acceleration_structure(self.acceleration_structure),
            )
        }
    }


    #[inline]
    pub fn destroy(self)
    {
        unsafe {
            self.device.vk_acceleration_struct_pf.destroy_acceleration_structure(self.acceleration_structure, None);
            self.buffer.destroy();
        }
    }
}
