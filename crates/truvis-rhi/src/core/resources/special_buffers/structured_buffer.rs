use crate::core::debug_utils::RhiDebugType;
use crate::core::resources::buffer::RhiBuffer;
use crate::core::resources::buffer_creator::RhiBufferCreateInfo;
use crate::impl_derive_buffer;
use crate::rhi::Rhi;
use ash::vk;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;
use std::rc::Rc;

/// buffer 内存放的是结构体或者结构体的数组
pub struct RhiStructuredBuffer<T: bytemuck::Pod> {
    inner: RhiBuffer,
    /// 结构体的数量
    len: usize,
    _phantom: PhantomData<T>,
}
impl_derive_buffer!(RhiStructuredBuffer<T: bytemuck::Pod>, RhiBuffer, inner);
impl<T: bytemuck::Pod> RhiStructuredBuffer<T> {
    #[inline]
    pub fn new_ubo(rhi: &Rhi, len: usize, debug_name: impl AsRef<str>) -> Self {
        Self::new(
            rhi,
            debug_name,
            len,
            vk::BufferUsageFlags::UNIFORM_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            false,
        )
    }

    #[inline]
    pub fn new_stage_buffer(rhi: &Rhi, len: usize, debug_name: impl AsRef<str>) -> Self {
        Self::new(rhi, debug_name, len, vk::BufferUsageFlags::TRANSFER_SRC, true)
    }

    #[inline]
    pub fn new(
        rhi: &Rhi,
        debug_name: impl AsRef<str>,
        len: usize,
        buffer_usage_flags: vk::BufferUsageFlags,
        mapped: bool,
    ) -> Self {
        let allocation_create_flags = if mapped {
            // TODO 或许可以优化这个 flag
            vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM
        } else {
            vk_mem::AllocationCreateFlags::empty()
        };

        Self {
            inner: RhiBuffer::new(
                rhi,
                Rc::new(RhiBufferCreateInfo::new((len * size_of::<T>()) as vk::DeviceSize, buffer_usage_flags)),
                Rc::new(vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    flags: allocation_create_flags,
                    ..Default::default()
                }),
                // TODO 可能不需要这个 align
                Some(rhi.device.min_ubo_offset_align()),
                debug_name,
            ),
            len,
            _phantom: PhantomData,
        }
    }

    pub fn mapped_slice(&mut self) -> &mut [T] {
        let mapped_ptr = self.inner.mapped_ptr();
        unsafe { std::slice::from_raw_parts_mut(mapped_ptr as *mut T, self.len) }
    }
}
