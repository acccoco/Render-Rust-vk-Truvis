use std::ffi::c_void;

use ash::vk;
use vk_mem::Alloc;

use crate::{resource_type::command_buffer::RhiCommandBuffer, rhi::Rhi};

pub struct RhiBuffer
{
    pub(crate) buffer: vk::Buffer,
    allocation: vk_mem::Allocation,

    map_ptr: Option<*mut u8>,
    size: vk::DeviceSize,

    debug_name: String,
}

impl RhiBuffer
{
    /// @param min_align 对 memory 的 offset align 限制
    pub fn new(
        size: vk::DeviceSize,
        buffer_usage: vk::BufferUsageFlags,
        mem_usage: vk_mem::MemoryUsage,
        alloc_flags: vk_mem::AllocationCreateFlags,
        min_align: Option<vk::DeviceSize>,
        debug_name: Option<&str>,
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

        unsafe {
            let rhi = Rhi::instance();
            let (buffer, allocation) = if let Some(offset_align) = min_align {
                rhi.vma().create_buffer_with_alignment(&buffer_info, &alloc_info, offset_align).unwrap()
            } else {
                rhi.vma().create_buffer(&buffer_info, &alloc_info).unwrap()
            };

            rhi.try_set_debug_name(buffer, debug_name);
            Self {
                buffer,
                allocation,
                map_ptr: None,
                size,
                debug_name: debug_name.unwrap_or("").into(),
            }
        }
    }

    #[inline]
    pub fn new_stage_buffer(size: vk::DeviceSize, debug_name: Option<&str>) -> Self
    {
        Self::new(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk_mem::MemoryUsage::Auto,
            vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
            None,
            debug_name,
        )
    }

    #[inline]
    pub fn new_index_buffer(size: usize, debug_name: Option<&str>) -> Self
    {
        Self::new(
            size as vk::DeviceSize,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk_mem::MemoryUsage::AutoPreferDevice,
            vk_mem::AllocationCreateFlags::empty(),
            None,
            debug_name,
        )
    }

    #[inline]
    pub fn new_vertex_buffer(size: usize, debug_name: Option<&str>) -> Self
    {
        Self::new(
            size as vk::DeviceSize,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk_mem::MemoryUsage::AutoPreferDevice,
            vk_mem::AllocationCreateFlags::empty(),
            None,
            debug_name,
        )
    }


    pub fn map(&mut self)
    {
        if self.map_ptr.is_some() {
            return;
        }
        unsafe {
            self.map_ptr = Some(Rhi::instance().vma().map_memory(&mut self.allocation).unwrap());
        }
    }

    pub fn unmap(&mut self)
    {
        if self.map_ptr.is_none() {
            return;
        }
        unsafe {
            Rhi::instance().vma().unmap_memory(&mut self.allocation);
            self.map_ptr = None;
        }
    }

    pub fn drop(self)
    {
        unsafe {
            Rhi::instance().vma().destroy_buffer(self.buffer, self.allocation);
        }
    }

    /// 创建一个临时的 stage buffer，先将数据放入 stage buffer，再 transfer 到 self
    pub fn transfer_data<T>(&mut self, data: &[T])
    where
        T: Sized + Copy,
    {
        let mut stage_buffer = Self::new_stage_buffer(
            std::mem::size_of_val(data) as vk::DeviceSize,
            Some(&format!("{}-stage-buffer", self.debug_name)),
        );
        stage_buffer.map();

        unsafe {
            // 这里的 size 是目标内存的最大 size
            // align 表示目标内存位置额外的内存对齐要求，这里使用 size_of，表示和 rust 中 [T; n] 的保持一致
            let mut slice = ash::util::Align::new(
                stage_buffer.map_ptr.unwrap() as *mut c_void,
                std::mem::size_of::<T>() as u64,
                self.size,
            );
            slice.copy_from_slice(data);
        }

        stage_buffer.unmap();

        RhiCommandBuffer::one_time_transfer(|cmd| {
            cmd.copy_buffer(
                &stage_buffer,
                self,
                &[vk::BufferCopy {
                    size: std::mem::size_of_val(data) as vk::DeviceSize,
                    ..Default::default()
                }],
            );
        });

        stage_buffer.drop();
    }
}
