//! Ray Tracing 所需的加速结构

use std::rc::Rc;

use ash::vk;
use itertools::Itertools;

use crate::{
    foundation::{debug_messenger::DebugType, device::DeviceFunctions, mem_allocator::MemAllocator},
    query::query_pool::QueryPool,
    render_context::RenderContext,
    resources::buffer::Buffer,
};

pub struct BlasInputInfo<'a> {
    pub geometry: vk::AccelerationStructureGeometryKHR<'a>,
    pub range: vk::AccelerationStructureBuildRangeInfoKHR,
}

pub struct Acceleration {
    acceleration_structure: vk::AccelerationStructureKHR,
    _buffer: Buffer,
    device_functions: Rc<DeviceFunctions>,
}
impl DebugType for Acceleration {
    fn debug_type_name() -> &'static str {
        "RhiAcceleration"
    }

    fn vk_handle(&self) -> impl vk::Handle {
        self.acceleration_structure
    }
}
impl Acceleration {
    /// 同步构建 blas
    ///
    /// 需要指定每个 geometry 的信息，以及每个 geometry 拥有的 max primitives
    /// 数量 会自动添加 compact 和 trace 的 flag
    ///
    /// # 构建过程
    ///
    /// 1. 查询构建 blas 所需的尺寸
    /// 2. 构建 blas
    /// 3. 查询 blas 的 compact size
    /// 4. 将 blas copy 到 compact 的 blas
    ///
    /// # params
    /// - primitives 每个 geometry 的 max primitives 数量
    pub fn build_blas_sync(
        render_context: &RenderContext,
        blas_inputs: &[BlasInputInfo],
        build_flags: vk::BuildAccelerationStructureFlagsKHR,
        debug_name: impl AsRef<str>,
    ) -> Self {
        let geometries = blas_inputs.iter().map(|blas_input| blas_input.geometry).collect_vec();
        let range_infos = blas_inputs.iter().map(|blas_input| blas_input.range).collect_vec();
        let max_primitives = blas_inputs.iter().map(|blas_input| blas_input.range.primitive_count).collect_vec();

        // 使用部分完整的 AccelerationStructureBuildGeometryInfo 来查询所需的资源大小
        let mut build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::default()
            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(
                build_flags
                    | vk::BuildAccelerationStructureFlagsKHR::ALLOW_COMPACTION
                    | vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE,
            )
            .geometries(&geometries)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD);

        // blas 所需的尺寸信息
        let size_info = unsafe {
            let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
            render_context.device_functions().acceleration_structure.get_acceleration_structure_build_sizes(
                vk::AccelerationStructureBuildTypeKHR::DEVICE,
                &build_geometry_info,
                &max_primitives, // 每一个 geometry 里面的最大 primitive 数量
                &mut size_info,
            );
            size_info
        };

        let uncompact_acceleration = Self::new(
            render_context.device_functions(),
            render_context.allocator(),
            size_info.acceleration_structure_size,
            vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
            format!("{}-uncompact-blas", debug_name.as_ref()),
        );

        let scratch_buffer = Buffer::new_accleration_scratch_buffer(
            size_info.build_scratch_size,
            format!("{}-blas-scratch-buffer", debug_name.as_ref()),
        );

        // 填充 build geometry info 的剩余部分以 build blas
        build_geometry_info.dst_acceleration_structure = uncompact_acceleration.acceleration_structure;
        build_geometry_info.scratch_data = vk::DeviceOrHostAddressKHR {
            device_address: scratch_buffer.device_address(),
        };

        // 创建一个 QueryPool，用于查询 compact size
        let mut query_pool = QueryPool::new(vk::QueryType::ACCELERATION_STRUCTURE_COMPACTED_SIZE_KHR, 1, "");
        query_pool.reset(0, 1);

        // 等待初步 build 完成
        render_context.one_time_exec(
            |cmd| {
                cmd.build_acceleration_structure(&build_geometry_info, &range_infos);
                // 查询 compact size 属于 read 操作，需要同步
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
            render_context.device_functions(),
            render_context.allocator(),
            compact_size[0],
            vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
            format!("{}-compact-blas", debug_name.as_ref()),
        );

        render_context.one_time_exec(
            |cmd| {
                cmd.cmd_copy_acceleration_structure(
                    &vk::CopyAccelerationStructureInfoKHR::default()
                        .src(uncompact_acceleration.acceleration_structure)
                        .dst(compact_acceleration.acceleration_structure)
                        .mode(vk::CopyAccelerationStructureModeKHR::COMPACT),
                );
            },
            "compact-blas",
        );

        // 回收临时资源
        {
            uncompact_acceleration.destroy();
            query_pool.destroy();
        }

        compact_acceleration
    }

    /// 同步构建 tlas
    /// # 构建过程
    /// 1. 查询构建 tlas 所需的尺寸
    /// 2. 构建 tlas
    pub fn build_tlas_sync(
        render_context: &RenderContext,
        instances: &[vk::AccelerationStructureInstanceKHR],
        build_flags: vk::BuildAccelerationStructureFlagsKHR,
        debug_name: impl AsRef<str>,
    ) -> Self {
        let mut acceleration_instance_buffer = Buffer::new_acceleration_instance_buffer(
            size_of_val(instances) as vk::DeviceSize,
            format!("{}-acceleration-instance-buffer", debug_name.as_ref()),
        );
        acceleration_instance_buffer.transfer_data_sync(render_context, instances);

        let geometry = vk::AccelerationStructureGeometryKHR::default()
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                instances: vk::AccelerationStructureGeometryInstancesDataKHR::default()
                    // true: data 是 &[vk::AccelerationStructureInstanceKHR]
                    // false: data 是 &[&vk::AccelerationStructureInstanceKHR]
                    .array_of_pointers(false)
                    .data(vk::DeviceOrHostAddressConstKHR {
                        device_address: acceleration_instance_buffer.device_address(),
                    }),
            });
        let range_info = vk::AccelerationStructureBuildRangeInfoKHR::default().primitive_count(instances.len() as u32);

        let mut build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::default()
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .flags(build_flags | vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .geometries(std::slice::from_ref(&geometry));

        // 获得 AccelerationStructure 所需的尺寸
        let size_info = unsafe {
            let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
            render_context.device_functions().acceleration_structure.get_acceleration_structure_build_sizes(
                vk::AccelerationStructureBuildTypeKHR::DEVICE,
                &build_geometry_info,
                &[instances.len() as u32],
                &mut size_info,
            );

            size_info
        };

        let acceleration = Self::new(
            render_context.device_functions(),
            render_context.allocator(),
            size_info.acceleration_structure_size,
            vk::AccelerationStructureTypeKHR::TOP_LEVEL,
            format!("{}-tlas", debug_name.as_ref()),
        );

        let scratch_buffer = Buffer::new_accleration_scratch_buffer(
            size_info.build_scratch_size,
            format!("{}-tlas-scratch-buffer", debug_name.as_ref()),
        );

        // 补全剩下的 build info
        build_geometry_info.dst_acceleration_structure = acceleration.acceleration_structure;
        build_geometry_info.scratch_data.device_address = scratch_buffer.device_address();

        // 正式构建 TLAS
        render_context.one_time_exec(
            |cmd| {
                cmd.build_acceleration_structure(&build_geometry_info, std::slice::from_ref(&range_info));
            },
            "build-tlas",
        );

        acceleration
    }

    /// 创建 AccelerationStructure 以及 buffer    
    fn new(
        device_functions: Rc<DeviceFunctions>,
        allocator: Rc<MemAllocator>,
        size: vk::DeviceSize,
        ty: vk::AccelerationStructureTypeKHR,
        debug_name: impl AsRef<str>,
    ) -> Self {
        let buffer = Buffer::new_accleration_buffer(size as usize, debug_name.as_ref());

        let create_info = vk::AccelerationStructureCreateInfoKHR::default() //
            .ty(ty)
            .size(size)
            .buffer(buffer.handle());

        let acceleration_structure = unsafe {
            device_functions.acceleration_structure.create_acceleration_structure(&create_info, None).unwrap()
        };

        let acc = Self {
            device_functions: device_functions.clone(),
            acceleration_structure,
            _buffer: buffer,
        };
        device_functions.set_debug_name(&acc, debug_name);
        acc
    }

    #[inline]
    pub fn handle(&self) -> vk::AccelerationStructureKHR {
        self.acceleration_structure
    }

    #[inline]
    pub fn get_device_address(&self) -> vk::DeviceAddress {
        unsafe {
            self.device_functions.acceleration_structure.get_acceleration_structure_device_address(
                &vk::AccelerationStructureDeviceAddressInfoKHR::default()
                    .acceleration_structure(self.acceleration_structure),
            )
        }
    }

    #[inline]
    pub fn destroy(self) {
        drop(self)
    }
}
impl Drop for Acceleration {
    fn drop(&mut self) {
        unsafe {
            self.device_functions
                .acceleration_structure
                .destroy_acceleration_structure(self.acceleration_structure, None);
        }
    }
}
