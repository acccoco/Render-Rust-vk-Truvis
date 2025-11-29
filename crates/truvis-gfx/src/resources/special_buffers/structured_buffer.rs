use std::marker::PhantomData;

use ash::{vk, vk::Handle};

use crate::{
    foundation::debug_messenger::DebugType,
    gfx::Gfx,
    resources::{
        handles::{BufferHandle, StructuredBufferHandle},
        resource_data::BufferType,
    },
};

/// buffer 内存放的是结构体或者结构体的数组
pub struct GfxStructuredBuffer<T: bytemuck::Pod> {
    pub handle: StructuredBufferHandle<T>,
    /// 结构体的数量
    ele_num: usize,
    _phantom: PhantomData<T>,
}

impl<T: bytemuck::Pod> GfxStructuredBuffer<T> {
    #[inline]
    pub fn new_ubo(len: usize, debug_name: impl AsRef<str>) -> Self {
        let mut rm = Gfx::get().resource_manager();
        let handle = rm.create_structured_buffer(
            len,
            vk::BufferUsageFlags::UNIFORM_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            false,
            BufferType::Uniform,
            debug_name,
        );

        Self {
            handle,
            ele_num: len,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn new_stage_buffer(len: usize, debug_name: impl AsRef<str>) -> Self {
        let mut rm = Gfx::get().resource_manager();
        let handle =
            rm.create_structured_buffer(len, vk::BufferUsageFlags::TRANSFER_SRC, true, BufferType::Stage, debug_name);

        Self {
            handle,
            ele_num: len,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn new(
        debug_name: impl AsRef<str>,
        len: usize,
        buffer_usage_flags: vk::BufferUsageFlags,
        mapped: bool,
    ) -> Self {
        let mut rm = Gfx::get().resource_manager();
        let handle = rm.create_structured_buffer(len, buffer_usage_flags, mapped, BufferType::Raw, debug_name);

        Self {
            handle,
            ele_num: len,
            _phantom: PhantomData,
        }
    }

    pub fn mapped_slice(&mut self) -> &mut [T] {
        let mut rm = Gfx::get().resource_manager();
        let buffer_handle = BufferHandle {
            inner: self.handle.inner,
        };
        let resource = rm.get_buffer_mut(buffer_handle).expect("Buffer not found");

        let mapped_ptr = resource.mapped_ptr.expect("Buffer is not mapped");
        unsafe { std::slice::from_raw_parts_mut(mapped_ptr as *mut T, self.ele_num) }
    }

    pub fn transfer_data_by_mmap(&mut self, data: &[T]) {
        let slice = self.mapped_slice();
        slice.copy_from_slice(data);

        let rm = Gfx::get().resource_manager();
        let buffer_handle = BufferHandle {
            inner: self.handle.inner,
        };
        rm.flush_buffer(buffer_handle, 0, std::mem::size_of_val(data) as vk::DeviceSize);
    }

    pub fn flush(&self, offset: vk::DeviceSize, size: vk::DeviceSize) {
        let rm = Gfx::get().resource_manager();
        let buffer_handle = BufferHandle {
            inner: self.handle.inner,
        };
        rm.flush_buffer(buffer_handle, offset, size);
    }

    pub fn vk_buffer(&self) -> vk::Buffer {
        let rm = Gfx::get().resource_manager();
        let buffer_handle = BufferHandle {
            inner: self.handle.inner,
        };
        rm.get_buffer(buffer_handle).expect("Buffer not found").buffer
    }

    pub fn device_address(&self) -> vk::DeviceAddress {
        let rm = Gfx::get().resource_manager();
        let buffer_handle = BufferHandle {
            inner: self.handle.inner,
        };
        rm.get_buffer(buffer_handle).expect("Buffer not found").device_addr.unwrap_or(0)
    }

    pub fn size(&self) -> vk::DeviceSize {
        let rm = Gfx::get().resource_manager();
        let buffer_handle = BufferHandle {
            inner: self.handle.inner,
        };
        rm.get_buffer(buffer_handle).expect("Buffer not found").size
    }
}

impl<T: bytemuck::Pod> DebugType for GfxStructuredBuffer<T> {
    #[inline]
    fn debug_type_name() -> &'static str {
        "StructuredBuffer"
    }

    #[inline]
    fn vk_handle(&self) -> impl Handle {
        self.vk_buffer()
    }
}

impl<T: bytemuck::Pod> Drop for GfxStructuredBuffer<T> {
    fn drop(&mut self) {
        let mut rm = Gfx::get().resource_manager();
        let buffer_handle = BufferHandle {
            inner: self.handle.inner,
        };
        rm.destroy_buffer_auto(buffer_handle);
    }
}
