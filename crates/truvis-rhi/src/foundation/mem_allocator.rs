use std::ops::Deref;

use ash::vk;

pub struct MemAllocator {
    inner: vk_mem::Allocator,
}

impl MemAllocator {
    /// 由于 vma 的生命周期设定：需要引用 Instance 以及
    /// Device，并确保在其声明周期之内这两个的引用是有效的.
    /// 因此需要在 Rhi 的其他部分都初始化完成后再初始化 vma，并确保 Instance 和
    /// Device 是 Pin 的
    pub fn new(instance: &ash::Instance, pdevice: vk::PhysicalDevice, device: &ash::Device) -> Self {
        let mut vma_ci = vk_mem::AllocatorCreateInfo::new(instance, device, pdevice);
        vma_ci.vulkan_api_version = vk::API_VERSION_1_3;
        vma_ci.flags = vk_mem::AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS;

        let vma = unsafe { vk_mem::Allocator::new(vma_ci).unwrap() };

        Self { inner: vma }
    }

    // TODO 外部改用 Rc，因此这里只能是 &self 了
    pub fn destroy(&self) {
        // 通过 drop 触发销毁
    }
}

impl Deref for MemAllocator {
    type Target = vk_mem::Allocator;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
