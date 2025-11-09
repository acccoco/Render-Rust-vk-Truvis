use std::ops::{Deref, DerefMut};

use ash::{vk, vk::Handle};

use crate::{foundation::debug_messenger::DebugType, gfx::Gfx, impl_derive_buffer, resources::buffer::Buffer};

pub struct SBTBuffer {
    _inner: Buffer,
}

impl_derive_buffer!(SBTBuffer, Buffer, _inner);

// init & destroy
impl SBTBuffer {
    pub fn new(size: vk::DeviceSize, align: vk::DeviceSize, name: impl AsRef<str>) -> Self {
        let inner = Buffer::new(
            size,
            vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR
                | vk::BufferUsageFlags::TRANSFER_SRC
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            Some(align),
            true,
            format!("SBTBuffer::{}", name.as_ref()),
        );
        let buffer = Self { _inner: inner };
        let gfx_device = Gfx::get().gfx_device();
        gfx_device.set_debug_name(&buffer, name.as_ref());
        buffer
    }

    #[inline]
    pub fn handle(&self) -> vk::Buffer {
        self._inner.handle
    }
}

impl DebugType for SBTBuffer {
    fn debug_type_name() -> &'static str {
        "SBTBuffer"
    }

    fn vk_handle(&self) -> impl Handle {
        self.handle()
    }
}
