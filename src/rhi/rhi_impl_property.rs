use ash::{vk, Device, Entry, Instance};

use crate::{
    rhi::{physical_device::RhiPhysicalDevice, Rhi, RHI},
    rhi_type::{command_pool::RhiCommandPool, queue::RhiQueue},
};


// 属性访问
impl Rhi
{
    #[inline]
    pub fn graphics_command_pool(&self) -> &RhiCommandPool { self.graphics_command_pool.as_ref().unwrap() }
    #[inline]
    pub fn compute_command_pool(&self) -> &RhiCommandPool { self.compute_command_pool.as_ref().unwrap() }
    #[inline]
    pub fn transfer_command_pool(&self) -> &RhiCommandPool { self.transfer_command_pool.as_ref().unwrap() }
    #[inline]
    pub fn instance() -> &'static Self { unsafe { RHI.as_ref().unwrap_unchecked() } }
    #[inline]
    pub(crate) fn vk_instance(&self) -> &Instance { unsafe { self.instance.as_ref().unwrap_unchecked() } }
    #[inline]
    pub(crate) fn device(&self) -> &Device { unsafe { self.device.as_ref().unwrap_unchecked() } }
    #[inline]
    pub(crate) fn physical_device(&self) -> &RhiPhysicalDevice
    {
        unsafe { self.physical_device.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub fn compute_queue(&self) -> &RhiQueue { unsafe { self.compute_queue.as_ref().unwrap_unchecked() } }
    #[inline]
    pub fn graphics_queue(&self) -> &RhiQueue { unsafe { self.graphics_queue.as_ref().unwrap_unchecked() } }
    #[inline]
    pub fn transfer_queue(&self) -> &RhiQueue { unsafe { self.transfer_queue.as_ref().unwrap_unchecked() } }
    #[inline]
    pub fn descriptor_pool(&self) -> vk::DescriptorPool { unsafe { self.descriptor_pool.unwrap_unchecked() } }
    #[inline]
    pub(crate) fn vma(&self) -> &vk_mem::Allocator { unsafe { self.vma.as_ref().unwrap_unchecked() } }
    #[inline]
    pub(crate) fn vk_pf(&self) -> &Entry { unsafe { self.vk_pf.as_ref().unwrap_unchecked() } }
    #[inline]
    pub(crate) fn dynamic_render_pf(&self) -> &ash::extensions::khr::DynamicRendering
    {
        unsafe { self.dynamic_render_pf.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub(crate) fn acceleration_structure_pf(&self) -> &ash::extensions::khr::AccelerationStructure
    {
        unsafe { self.acc_pf.as_ref().unwrap_unchecked() }
    }
}
