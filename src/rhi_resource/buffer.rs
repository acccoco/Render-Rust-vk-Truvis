use ash::vk;
use vk_mem::Alloc;

use crate::rhi::Rhi;

pub struct RhiBuffer
{
    buffer: vk::Buffer,
    allocation: vk_mem::Allocation,

    map_ptr: Option<*mut u8>,
    size: vk::DeviceSize,
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
    pub fn new_index_buffer(size: vk::DeviceSize, debug_name: Option<&str>) -> Self
    {
        Self::new(
            size,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
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
}
