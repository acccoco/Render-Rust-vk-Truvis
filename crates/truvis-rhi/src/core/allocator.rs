use std::{ops::Deref, rc::Rc};

use ash::vk;

use crate::core::{device::RhiDevice, instance::RhiInstance, physical_device::RhiPhysicalDevice};

pub struct RhiAllocator {
    inner: vk_mem::Allocator,

    _instance: Rc<RhiInstance>,
    _pdevice: Rc<RhiPhysicalDevice>,
    _device: Rc<RhiDevice>,
}

impl Deref for RhiAllocator {
    type Target = vk_mem::Allocator;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for RhiAllocator {
    fn drop(&mut self) {
        log::info!("Destroying RhiAllocator");
        // vk_mem 是 RAII 的
    }
}

impl RhiAllocator {
    /// 由于 vma 恶心的生命周期设定：需要引用 Instance 以及 Device，并确保在其声明周期之内这两个的引用是有效的.
    /// 因此需要在 Rhi 的其他部分都初始化完成后再初始化 vma，确保 Instance 和 Device 是 pin 的
    pub fn new(instance: Rc<RhiInstance>, pdevice: Rc<RhiPhysicalDevice>, device: Rc<RhiDevice>) -> Self {
        let mut vma_ci = vk_mem::AllocatorCreateInfo::new(instance.handle(), device.handle(), pdevice.handle);
        vma_ci.vulkan_api_version = vk::API_VERSION_1_3;
        vma_ci.flags = vk_mem::AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS;

        let vma = unsafe { vk_mem::Allocator::new(vma_ci).unwrap() };

        Self {
            inner: vma,
            _instance: instance,
            _pdevice: pdevice,
            _device: device,
        }
    }
}
