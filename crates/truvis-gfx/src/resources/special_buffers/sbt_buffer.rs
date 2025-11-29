use ash::{vk, vk::Handle};

use crate::{
    foundation::debug_messenger::DebugType,
    gfx::Gfx,
    resources::{handles::BufferHandle, resource_data::BufferType},
};

pub struct GfxSBTBuffer {
    handle: BufferHandle,
}

// init & destroy
impl GfxSBTBuffer {
    pub fn new(size: vk::DeviceSize, _align: vk::DeviceSize, name: impl AsRef<str>) -> Self {
        let mut rm = Gfx::get().resource_manager();
        let handle = rm.create_buffer(
            size,
            vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR
                | vk::BufferUsageFlags::TRANSFER_SRC
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            true,
            BufferType::Raw,
            format!("SBTBuffer::{}", name.as_ref()),
        );
        Self { handle }
    }

    #[inline]
    pub fn handle(&self) -> vk::Buffer {
        let rm = Gfx::get().resource_manager();
        rm.get_buffer(self.handle).unwrap().buffer
    }

    #[inline]
    pub fn device_address(&self) -> vk::DeviceAddress {
        let rm = Gfx::get().resource_manager();
        rm.get_buffer(self.handle).unwrap().device_addr.unwrap_or(0)
    }

    #[inline]
    pub fn mapped_ptr(&self) -> *mut u8 {
        let rm = Gfx::get().resource_manager();
        rm.get_buffer(self.handle).unwrap().mapped_ptr.unwrap()
    }

    #[inline]
    pub fn size(&self) -> vk::DeviceSize {
        let rm = Gfx::get().resource_manager();
        rm.get_buffer(self.handle).unwrap().size
    }

    pub fn flush(&self, offset: vk::DeviceSize, size: vk::DeviceSize) {
        let rm = Gfx::get().resource_manager();
        rm.flush_buffer(self.handle, offset, size);
    }
}

impl DebugType for GfxSBTBuffer {
    fn debug_type_name() -> &'static str {
        "SBTBuffer"
    }

    fn vk_handle(&self) -> impl Handle {
        self.handle()
    }
}
