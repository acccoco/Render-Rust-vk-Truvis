use ash::vk;

use crate::gfx::Gfx;
use crate::resources::handles::BufferHandle;
use crate::resources::resource_data::BufferType;

pub struct GfxAccelerationScratchBuffer {
    handle: BufferHandle,
}

impl GfxAccelerationScratchBuffer {
    pub fn new(size: vk::DeviceSize, name: impl AsRef<str>) -> Self {
        let mut rm = Gfx::get().resource_manager();
        let handle = rm.create_buffer(
            size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            false,
            BufferType::Raw,
            name.as_ref(),
        );

        Self { handle }
    }

    pub fn device_address(&self) -> vk::DeviceAddress {
        let rm = Gfx::get().resource_manager();
        rm.get_buffer(self.handle).unwrap().device_addr.unwrap_or(0)
    }

    pub fn vk_buffer(&self) -> vk::Buffer {
        let rm = Gfx::get().resource_manager();
        rm.get_buffer(self.handle).unwrap().buffer
    }
}

pub struct GfxAccelerationStructureBuffer {
    handle: BufferHandle,
}

impl GfxAccelerationStructureBuffer {
    pub fn new(size: vk::DeviceSize, name: impl AsRef<str>) -> Self {
        let mut rm = Gfx::get().resource_manager();
        let handle = rm.create_buffer(
            size,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            false,
            BufferType::Raw,
            name.as_ref(),
        );

        Self { handle }
    }

    pub fn device_address(&self) -> vk::DeviceAddress {
        let rm = Gfx::get().resource_manager();
        rm.get_buffer(self.handle).unwrap().device_addr.unwrap_or(0)
    }

    pub fn vk_buffer(&self) -> vk::Buffer {
        let rm = Gfx::get().resource_manager();
        rm.get_buffer(self.handle).unwrap().buffer
    }
}

pub struct GfxAccelerationInstanceBuffer {
    handle: BufferHandle,
}

impl GfxAccelerationInstanceBuffer {
    pub fn new(size: vk::DeviceSize, name: impl AsRef<str>) -> Self {
        let mut rm = Gfx::get().resource_manager();
        let handle = rm.create_buffer(
            size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::TRANSFER_DST,
            false,
            BufferType::Raw,
            name.as_ref(),
        );

        Self { handle }
    }

    pub fn device_address(&self) -> vk::DeviceAddress {
        let rm = Gfx::get().resource_manager();
        rm.get_buffer(self.handle).unwrap().device_addr.unwrap_or(0)
    }

    pub fn vk_buffer(&self) -> vk::Buffer {
        let rm = Gfx::get().resource_manager();
        rm.get_buffer(self.handle).unwrap().buffer
    }

    pub fn transfer_data_sync<T: Copy>(&self, data: &[T]) {
        let size_bytes = std::mem::size_of_val(data);
        let mut rm = Gfx::get().resource_manager();

        // Create staging buffer
        let staging_handle = rm.create_buffer(
            size_bytes as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            true,
            BufferType::Stage,
            "accel-instance-staging",
        );

        // Copy to staging
        {
            let buffer_res = rm.get_buffer_mut(staging_handle).unwrap();
            if let Some(ptr) = buffer_res.mapped_ptr {
                unsafe {
                    std::ptr::copy_nonoverlapping(data.as_ptr() as *const u8, ptr, size_bytes);
                    // Flush
                    let allocator = Gfx::get().allocator();
                    allocator.flush_allocation(&buffer_res.allocation, 0, size_bytes as vk::DeviceSize).unwrap();
                }
            }
        }

        let staging_vk = rm.get_buffer(staging_handle).unwrap().buffer;
        let dst_vk = rm.get_buffer(self.handle).unwrap().buffer;

        // Copy command
        Gfx::get().one_time_exec(
            |cmd| {
                let region = vk::BufferCopy {
                    src_offset: 0,
                    dst_offset: 0,
                    size: size_bytes as vk::DeviceSize,
                };
                unsafe {
                    Gfx::get().gfx_device().cmd_copy_buffer(cmd.vk_handle(), staging_vk, dst_vk, &[region]);
                }
            },
            "upload-accel-instance",
        );

        // Destroy staging
        rm.destroy_buffer_immediate(staging_handle);
    }
}
