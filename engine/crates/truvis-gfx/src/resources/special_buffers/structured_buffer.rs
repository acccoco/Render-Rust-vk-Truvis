use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use ash::vk;

use crate::{foundation::debug_messenger::DebugType, impl_derive_buffer, resources::buffer::GfxBuffer};

/// buffer 内存放的是结构体或者结构体的数组
pub struct GfxStructuredBuffer<T: bytemuck::Pod> {
    inner: GfxBuffer,
    /// 结构体的数量
    ele_num: usize,
    _phantom: PhantomData<T>,
}
impl_derive_buffer!(GfxStructuredBuffer<T: bytemuck::Pod>, GfxBuffer, inner);
impl<T: bytemuck::Pod> GfxStructuredBuffer<T> {
    #[inline]
    pub fn new_ssbo(len: usize, debug_name: impl AsRef<str>) -> Self {
        Self::new(
            debug_name,
            len,
            vk::BufferUsageFlags::STORAGE_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            false,
        )
    }

    #[inline]
    pub fn new_ubo(len: usize, debug_name: impl AsRef<str>) -> Self {
        Self::new(debug_name, len, vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST, false)
    }

    #[inline]
    pub fn new_stage_buffer(len: usize, debug_name: impl AsRef<str>) -> Self {
        Self::new(debug_name, len, vk::BufferUsageFlags::TRANSFER_SRC, true)
    }

    #[inline]
    pub fn new(
        debug_name: impl AsRef<str>,
        len: usize,
        buffer_usage_flags: vk::BufferUsageFlags,
        mapped: bool,
    ) -> Self {
        let buffer =
            GfxBuffer::new((len * size_of::<T>()) as vk::DeviceSize, buffer_usage_flags, None, mapped, debug_name);

        Self {
            inner: buffer,
            ele_num: len,
            _phantom: PhantomData,
        }
    }

    pub fn mapped_slice(&mut self) -> &mut [T] {
        let mapped_ptr = self.inner.mapped_ptr();
        unsafe { std::slice::from_raw_parts_mut(mapped_ptr as *mut T, self.ele_num) }
    }
}
impl<T: bytemuck::Pod> DebugType for GfxStructuredBuffer<T> {
    #[inline]
    fn debug_type_name() -> &'static str {
        "StructuredBuffer"
    }

    #[inline]
    fn vk_handle(&self) -> impl vk::Handle {
        self.vk_buffer()
    }
}
