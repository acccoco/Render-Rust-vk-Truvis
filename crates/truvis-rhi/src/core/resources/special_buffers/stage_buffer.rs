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

pub struct RhiStageBuffer<T: bytemuck::Pod> {
    inner: RhiBuffer,
    _phantom: PhantomData<T>,
}

impl_derive_buffer!(RhiStageBuffer<T: bytemuck::Pod>, RhiBuffer, inner);
impl<T: bytemuck::Pod> RhiStageBuffer<T> {
    pub fn new(rhi: &Rhi, debug_name: impl AsRef<str>) -> Self {
        let buffer = Self {
            inner: RhiBuffer::new(
                rhi,
                Rc::new(RhiBufferCreateInfo::new(size_of::<T>() as vk::DeviceSize, vk::BufferUsageFlags::TRANSFER_SRC)),
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
        rhi.device.debug_utils().set_debug_name(&buffer, &buffer.inner.debug_name);
        buffer
    }

    // BUG 可能需要考虑内存对齐
    pub fn transfer(&mut self, trans_func: &dyn Fn(&mut T)) {
        self.inner.map();
        unsafe {
            let ptr = self.inner.map_ptr.unwrap() as *mut T;

            trans_func(&mut *ptr);
        }
        self.inner.allocator.flush_allocation(&self.inner.allocation, 0, size_of::<T>() as vk::DeviceSize).unwrap();
        self.inner.unmap();
    }
}
