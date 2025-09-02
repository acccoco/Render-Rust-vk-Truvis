use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use ash::{vk, vk::Handle};

use crate::{
    foundation::{debug_messenger::DebugType, device::DeviceFunctions, mem_allocator::MemAllocator},
    impl_derive_buffer,
    resources::{buffer::Buffer, buffer_creator::BufferCreateInfo},
};

/// buffer 内存放的是结构体或者结构体的数组
pub struct StructuredBuffer<T: bytemuck::Pod>
{
    inner: Buffer,
    /// 结构体的数量
    len: usize,
    _phantom: PhantomData<T>,
}

impl_derive_buffer!(StructuredBuffer<T: bytemuck::Pod>, Buffer, inner);

impl<T: bytemuck::Pod> StructuredBuffer<T>
{
    #[inline]
    pub fn new_ubo(
        device_functions: Rc<DeviceFunctions>,
        allocator: Rc<MemAllocator>,
        len: usize,
        debug_name: impl AsRef<str>,
    ) -> Self
    {
        Self::new(
            device_functions,
            allocator,
            debug_name,
            len,
            vk::BufferUsageFlags::UNIFORM_BUFFER |
                vk::BufferUsageFlags::TRANSFER_DST |
                vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            false,
        )
    }

    #[inline]
    pub fn new_stage_buffer(
        device_functions: Rc<DeviceFunctions>,
        allocator: Rc<MemAllocator>,
        len: usize,
        debug_name: impl AsRef<str>,
    ) -> Self
    {
        Self::new(device_functions, allocator, debug_name, len, vk::BufferUsageFlags::TRANSFER_SRC, true)
    }

    #[inline]
    pub fn new(
        device_functions: Rc<DeviceFunctions>,
        allocator: Rc<MemAllocator>,
        debug_name: impl AsRef<str>,
        len: usize,
        buffer_usage_flags: vk::BufferUsageFlags,
        mapped: bool,
    ) -> Self
    {
        let allocation_create_flags = if mapped {
            // TODO 或许可以优化这个 flag
            vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM
        } else {
            vk_mem::AllocationCreateFlags::empty()
        };

        Self {
            inner: Buffer::new(
                device_functions,
                allocator,
                Rc::new(BufferCreateInfo::new((len * size_of::<T>()) as vk::DeviceSize, buffer_usage_flags)),
                Rc::new(vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    flags: allocation_create_flags,
                    ..Default::default()
                }),
                None,
                debug_name,
            ),
            len,
            _phantom: PhantomData,
        }
    }

    pub fn mapped_slice(&mut self) -> &mut [T]
    {
        let mapped_ptr = self.inner.mapped_ptr();
        unsafe { std::slice::from_raw_parts_mut(mapped_ptr as *mut T, self.len) }
    }
}

impl<T: bytemuck::Pod> DebugType for StructuredBuffer<T>
{
    #[inline]
    fn debug_type_name() -> &'static str
    {
        "StructuredBuffer"
    }

    #[inline]
    fn vk_handle(&self) -> impl Handle
    {
        self.inner.handle
    }
}
