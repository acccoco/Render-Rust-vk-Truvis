use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use ash::{vk, vk::Handle};

use crate::{
    foundation::debug_messenger::DebugType,
    impl_derive_buffer,
    render_context::RenderContext,
    resources::{buffer::Buffer, buffer_creator::BufferCreateInfo},
};

pub struct StageBuffer<T: bytemuck::Pod> {
    inner: Buffer,
    _phantom: PhantomData<T>,
}

impl_derive_buffer!(StageBuffer<T: bytemuck::Pod>, Buffer, inner);
impl<T: bytemuck::Pod> StageBuffer<T> {
    pub fn new(debug_name: impl AsRef<str>) -> Self {
        let buffer = Self {
            inner: Buffer::new(
                Rc::new(BufferCreateInfo::new(size_of::<T>() as vk::DeviceSize, vk::BufferUsageFlags::TRANSFER_SRC)),
                Rc::new(vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
                    ..Default::default()
                }),
                None,
                debug_name,
            ),
            _phantom: PhantomData,
        };
        let device_functions = RenderContext::get().device_functions();
        device_functions.set_debug_name(&buffer, &buffer.inner.debug_name);
        buffer
    }

    // BUG 可能需要考虑内存对齐
    pub fn transfer(&mut self, trans_func: &dyn Fn(&mut T)) {
        self.inner.map();
        unsafe {
            let ptr = self.inner.map_ptr.unwrap() as *mut T;

            trans_func(&mut *ptr);
        }
        let allocator = RenderContext::get().allocator();
        allocator.flush_allocation(&self.inner.allocation, 0, size_of::<T>() as vk::DeviceSize).unwrap();
        self.inner.unmap();
    }
}

impl<T: bytemuck::Pod> DebugType for StageBuffer<T> {
    fn debug_type_name() -> &'static str {
        "StageBuffer"
    }

    fn vk_handle(&self) -> impl Handle {
        self.inner.handle
    }
}
