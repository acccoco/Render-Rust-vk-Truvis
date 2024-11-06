use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use ash::vk;

use crate::framework::core::device::{Device, RhiDevice};


pub struct VulkanResource<Handle: vk::Handle + Copy>
{
    debug_name: String,
    handle: Handle,
}


impl<Handle: vk::Handle + Copy> VulkanResource<Handle>
{
    pub fn new(handle: Handle) -> Self
    {
        Self {
            debug_name: String::new(),
            handle,
        }
    }

    pub fn set_debug_name(&mut self, device: &RhiDevice, name: String)
    {
        self.debug_name = name;

        device.debug_utils.set_debug_name(device.device().handle(), self.handle, &self.debug_name);
    }

    pub fn get_handle(&self) -> Handle
    {
        self.handle
    }
}


pub trait IVulkanResource
{
    /// vk 内部的 handle
    type Handle: vk::Handle + Copy;

    fn get_inner_resource(&self) -> &VulkanResource<Self::Handle>;
    fn get_inner_resource_mut(&mut self) -> &mut VulkanResource<Self::Handle>;


    fn get_handle(&self) -> Self::Handle
    {
        self.get_inner_resource().get_handle()
    }

    fn set_debug_name(&mut self, device: &RhiDevice, name: String)
    {
        self.get_inner_resource_mut().set_debug_name(device, name)
    }

    fn get_object_type(&self) -> vk::ObjectType
    {
        <Self::Handle as vk::Handle>::TYPE
    }
}
