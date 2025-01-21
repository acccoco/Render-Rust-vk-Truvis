use std::ffi::c_void;

use ash::vk;
use vk_mem::Alloc;

use crate::framework::{core::command_buffer::RhiCommandBuffer, rhi::Rhi};

pub struct RhiBuffer
{
    pub handle: vk::Buffer,
    allocation: vk_mem::Allocation,

    map_ptr: Option<*mut u8>,
    size: vk::DeviceSize,

    debug_name: String,

    rhi: &'static Rhi,
}

impl RhiBuffer
{
    pub fn new2(
        rhi: &'static Rhi,
        buffer_ci: &vk::BufferCreateInfo,
        alloc_ci: &vk_mem::AllocationCreateInfo,
        align: Option<vk::DeviceSize>,
        debug_name: &str,
    ) -> Self
    {
        unsafe {
            let (buffer, allocation) = if let Some(offset_align) = align {
                rhi.vma().create_buffer_with_alignment(buffer_ci, alloc_ci, offset_align).unwrap()
            } else {
                rhi.vma().create_buffer(buffer_ci, alloc_ci).unwrap()
            };

            rhi.set_debug_name(buffer, debug_name);
            Self {
                rhi,
                handle: buffer,
                allocation,
                map_ptr: None,
                size: buffer_ci.size,
                debug_name: debug_name.to_string(),
            }
        }
    }

    /// @param min_align 对 memory 的 offset align 限制
    pub fn new(
        rhi: &'static Rhi,
        size: vk::DeviceSize,
        buffer_usage: vk::BufferUsageFlags,
        mem_usage: vk_mem::MemoryUsage,
        alloc_flags: vk_mem::AllocationCreateFlags,
        min_align: Option<vk::DeviceSize>,
        debug_name: String,
    ) -> Self
    {
        let buffer_info = vk::BufferCreateInfo {
            size,
            usage: buffer_usage,
            ..Default::default()
        };
        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: mem_usage,
            flags: alloc_flags,
            ..Default::default()
        };

        Self::new2(rhi, &buffer_info, &alloc_info, min_align, debug_name.as_str())
    }

    #[inline]
    pub fn new_device_buffer(
        rhi: &'static Rhi,
        size: vk::DeviceSize,
        flags: vk::BufferUsageFlags,
        debug_name: &str,
    ) -> Self
    {
        Self::new(
            rhi,
            size as vk::DeviceSize,
            flags,
            vk_mem::MemoryUsage::AutoPreferDevice,
            vk_mem::AllocationCreateFlags::empty(),
            None,
            debug_name.to_string(),
        )
    }

    #[inline]
    pub fn new_acceleration_instance_buffer<S>(rhi: &'static Rhi, size: vk::DeviceSize, debug_name: S) -> Self
    where
        S: AsRef<str>,
    {
        Self::new_device_buffer(
            rhi,
            size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS |
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR |
                vk::BufferUsageFlags::TRANSFER_DST,
            debug_name.as_ref(),
        )
    }

    #[inline]
    pub fn new_stage_buffer<S>(rhi: &'static Rhi, size: vk::DeviceSize, debug_name: S) -> Self
    where
        S: AsRef<str>,
    {
        Self::new(
            rhi,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk_mem::MemoryUsage::Auto,
            vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
            None,
            debug_name.as_ref().to_string(),
        )
    }

    #[inline]
    pub fn new_index_buffer(rhi: &'static Rhi, size: usize, debug_name: &str) -> Self
    {
        Self::new_device_buffer(
            rhi,
            size as vk::DeviceSize,
            vk::BufferUsageFlags::INDEX_BUFFER |
                vk::BufferUsageFlags::TRANSFER_DST |
                vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS |
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            debug_name.as_ref(),
        )
    }

    #[inline]
    pub fn new_vertex_buffer(rhi: &'static Rhi, size: usize, debug_name: &str) -> Self
    {
        Self::new_device_buffer(
            rhi,
            size as vk::DeviceSize,
            vk::BufferUsageFlags::VERTEX_BUFFER |
                vk::BufferUsageFlags::TRANSFER_DST |
                vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS |
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            debug_name.as_ref(),
        )
    }

    #[inline]
    pub fn new_accleration_buffer<S: AsRef<str>>(rhi: &'static Rhi, size: usize, debug_name: S) -> Self
    {
        Self::new_device_buffer(
            rhi,
            size as vk::DeviceSize,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            debug_name.as_ref(),
        )
    }

    #[inline]
    pub fn new_accleration_scratch_buffer<S: AsRef<str>>(rhi: &'static Rhi, size: vk::DeviceSize, debug_name: S)
        -> Self
    {
        Self::new_device_buffer(
            rhi,
            size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            debug_name.as_ref(),
        )
    }

    #[inline]
    pub fn map(&mut self)
    {
        if self.map_ptr.is_some() {
            return;
        }
        unsafe {
            self.map_ptr = Some(self.rhi.vma().map_memory(&mut self.allocation).unwrap());
        }
    }

    #[inline]
    pub fn unmap(&mut self)
    {
        if self.map_ptr.is_none() {
            return;
        }
        unsafe {
            self.rhi.vma().unmap_memory(&mut self.allocation);
            self.map_ptr = None;
        }
    }

    #[inline]
    pub fn destroy(mut self)
    {
        unsafe {
            self.rhi.vma().destroy_buffer(self.handle, &mut self.allocation);
        }
    }


    /// 通过 mem map 的方式将 data 传入到 buffer 中
    pub fn transfer_data_by_mem_map<T>(&mut self, data: &[T])
    where
        T: Sized + Copy,
    {
        self.map();
        unsafe {
            // 这里的 size 是目标内存的最大 size
            // align 表示目标内存位置额外的内存对齐要求，这里使用 align_of 表示和 rust 中 [T; n] 的保持一致
            let mut slice =
                ash::util::Align::new(self.map_ptr.unwrap() as *mut c_void, align_of::<T>() as u64, self.size);
            slice.copy_from_slice(data);
            self.rhi.vma().flush_allocation(&self.allocation, 0, size_of_val(data) as vk::DeviceSize).unwrap();
        }
        self.unmap();
    }

    // FIXME 这个隐含同步等待，需要特殊标注一下
    /// 创建一个临时的 stage buffer，先将数据放入 stage buffer，再 transfer 到 self
    pub fn transfer_data_by_stage_buffer<T>(&mut self, data: &[T], name: &str)
    where
        T: Sized + Copy,
    {
        let mut stage_buffer = Self::new_stage_buffer(
            self.rhi,
            std::mem::size_of_val(data) as vk::DeviceSize,
            format!("{}-stage-buffer", self.debug_name),
        );

        stage_buffer.transfer_data_by_mem_map(data);

        RhiCommandBuffer::one_time_exec(
            self.rhi,
            vk::QueueFlags::TRANSFER,
            |cmd| {
                cmd.copy_buffer(
                    &stage_buffer,
                    self,
                    &[vk::BufferCopy {
                        size: size_of_val(data) as vk::DeviceSize,
                        ..Default::default()
                    }],
                );
            },
            name,
        );

        stage_buffer.destroy();
    }

    pub fn get_device_address(&self) -> vk::DeviceAddress
    {
        unsafe {
            self.rhi.vk_device().get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(self.handle))
        }
    }
}
