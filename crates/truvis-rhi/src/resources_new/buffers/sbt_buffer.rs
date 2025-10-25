use crate::resources_new::managed_buffer::Buffer2;
use crate::resources_new::resource_handles::BufferHandle;
use crate::resources_new::resource_manager::ResourceManager;
use ash::vk;

#[derive(Copy, Clone)]
pub struct SBTBufferHandle {
    buffer: BufferHandle,
}

// init & destroy
impl SBTBufferHandle {
    /// align: shader group 的对齐要求
    pub fn new(
        resource_manager: &mut ResourceManager,
        size: vk::DeviceSize,
        align: vk::DeviceSize,
        name: impl AsRef<str>,
    ) -> Self {
        let buffer = Self::new_managed(size, align, name.as_ref());
        Self {
            buffer: resource_manager.register_buffer(buffer),
        }
    }

    fn new_managed(size: vk::DeviceSize, align: vk::DeviceSize, name: impl AsRef<str>) -> Buffer2 {
        let buffer_size = (size + align - 1) / align * align;
        Buffer2::new(
            buffer_size,
            vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            Some(align),
            true,
            format!("SBTBuffer::{}", name.as_ref()),
        )
    }
}
