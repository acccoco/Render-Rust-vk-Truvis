use std::ops::Deref;
use std::ops::DerefMut;

use ash::vk;

use crate::impl_derive_buffer;
use crate::resources::buffer::Buffer;

pub struct AccelerationScratchBuffer {
    inner: Buffer,
}
impl_derive_buffer!(AccelerationScratchBuffer, Buffer, inner);
impl AccelerationScratchBuffer {
    pub fn new(size: vk::DeviceSize, name: impl AsRef<str>) -> Self {
        let buffer = Buffer::new(
            size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            None,
            false,
            name,
        );

        Self { inner: buffer }
    }
}

pub struct AccelerationStructureBuffer {
    inner: Buffer,
}
impl_derive_buffer!(AccelerationStructureBuffer, Buffer, inner);
impl AccelerationStructureBuffer {
    pub fn new(size: vk::DeviceSize, name: impl AsRef<str>) -> Self {
        let buffer = Buffer::new(
            size,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            None,
            false,
            name,
        );

        Self { inner: buffer }
    }
}

pub struct AccelerationInstanceBuffer {
    inner: Buffer,
}
impl_derive_buffer!(AccelerationInstanceBuffer, Buffer, inner);
impl AccelerationInstanceBuffer {
    pub fn new(size: vk::DeviceSize, name: impl AsRef<str>) -> Self {
        let buffer = Buffer::new(
            size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::TRANSFER_DST,
            None,
            false,
            name,
        );

        Self { inner: buffer }
    }
}
