use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use ash::vk;

use crate::framework::core::device::Device;


pub struct VulkanResource<Handle: vk::Handle + Copy>
{
    debug_name: String,
    device: Option<Weak<RefCell<Device>>>,
    handle: Handle,
}


pub trait IVulkanResource
{
    /// vk 内部的 handle
    type Handle: vk::Handle + Copy;

    fn get_inner_resource(&self) -> &VulkanResource<Self::Handle>;
    fn get_inner_resource_mut(&mut self) -> &mut VulkanResource<Self::Handle>;

    fn get_device(&self) -> Rc<RefCell<Device>>
    {
        self.get_inner_resource()
            .device
            .as_ref()
            .expect("Device handle not set")
            .upgrade()
            .expect("Device dropped")
    }


    fn get_handle(&self) -> Self::Handle
    {
        self.get_inner_resource().handle
    }

    fn set_debug_name(&mut self, name: String)
    {
        self.get_inner_resource_mut().debug_name = name;
        let inner = self.get_inner_resource();

        if !inner.debug_name.is_empty() && inner.device.is_some() {
            let device = self.get_device();
            let device = device.borrow();
            device.get_debug_utils().set_debug_name(
                device.get_handle(),
                inner.handle,
                &inner.debug_name,
            );
        }
    }

    fn get_object_type(&self) -> vk::ObjectType
    {
        <Self::Handle as vk::Handle>::TYPE
    }
}
