use std::{
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

pub struct SBTBuffer {
    _inner: Buffer,
}

impl_derive_buffer!(SBTBuffer, Buffer, _inner);
impl SBTBuffer {
    pub fn new(
        size: vk::DeviceSize,
        align: vk::DeviceSize,
        name: impl AsRef<str>,
    ) -> Self {
        let buffer = Self {
            _inner: Buffer::new(
                Rc::new(BufferCreateInfo::new(
                    size,
                    vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR
                        | vk::BufferUsageFlags::TRANSFER_SRC
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                )),
                Rc::new(vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
                    ..Default::default()
                }),
                Some(align),
                format!("SBTBuffer::{}", name.as_ref()),
            ),
        };
        let device_functions = RenderContext::get().device_functions();
        device_functions.set_debug_name(&buffer, name.as_ref());
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
