use crate::resources_new::managed_buffer::Buffer2;
use crate::resources_new::resource_handles::BufferHandle;
use crate::resources_new::resource_manager::ResourceManager;
use ash::vk;

pub struct AccelerationBufferHandle {
    buffer: BufferHandle,
}

impl AccelerationBufferHandle {
    pub fn new(resource_mgr: &mut ResourceManager, size: vk::DeviceSize, debug_name: impl AsRef<str>) -> Self {
        Self {
            buffer: resource_mgr.register_buffer(Self::new_managed(size, debug_name)),
        }
    }

    fn new_managed(buffer_size: vk::DeviceSize, debug_name: impl AsRef<str>) -> Buffer2 {
        Buffer2::new(
            buffer_size,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            None,
            false,
            debug_name,
        )
    }
}

pub struct AccelerationScratchBufferHandle {
    buffer: BufferHandle,
}

impl AccelerationScratchBufferHandle {
    pub fn new(resource_mgr: &mut ResourceManager, size: vk::DeviceSize, debug_name: impl AsRef<str>) -> Self {
        Self {
            buffer: resource_mgr.register_buffer(Self::new_managed(size, debug_name)),
        }
    }

    fn new_managed(buffer_size: vk::DeviceSize, debug_name: impl AsRef<str>) -> Buffer2 {
        Buffer2::new(
            buffer_size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            None,
            false,
            debug_name,
        )
    }
}
