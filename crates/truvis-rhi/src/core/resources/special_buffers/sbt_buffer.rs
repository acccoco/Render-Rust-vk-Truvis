use crate::core::debug_utils::RhiDebugType;
use crate::core::resources::buffer::RhiBuffer;
use crate::core::resources::buffer_creator::RhiBufferCreateInfo;
use crate::impl_derive_buffer;
use crate::rhi::Rhi;
use ash::vk;
use std::ops::Deref;
use std::ops::DerefMut;
use std::rc::Rc;

pub struct RhiSBTBuffer {
    _inner: RhiBuffer,
}

impl_derive_buffer!(RhiSBTBuffer, RhiBuffer, _inner);
impl RhiSBTBuffer {
    pub fn new(rhi: &Rhi, size: vk::DeviceSize, align: vk::DeviceSize, name: impl AsRef<str>) -> Self {
        let buffer = Self {
            _inner: RhiBuffer::new(
                rhi,
                Rc::new(RhiBufferCreateInfo::new(
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
        rhi.device.debug_utils().set_debug_name(&buffer, name.as_ref());
        buffer
    }

    #[inline]
    pub fn handle(&self) -> vk::Buffer {
        self._inner.handle
    }
}
