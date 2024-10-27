use ash::vk;

use crate::core::{
    command_pool::CommandPool,
    debug::DebugUtils,
    fence_pool::FencePool,
    physical_device::PhysicalDevice,
    queue::Queue,
    vulkan_resource::{IVulkanResource, VulkanResource},
};

pub struct Device
{
    inner_resource: VulkanResource<vk::Device>,

    gpu: PhysicalDevice,

    surface: vk::SurfaceKHR,

    debug_utils: DebugUtils,

    enabled_extensions: Vec<String>,

    queues: Vec<Vec<Queue>>,

    command_pool: CommandPool,

    fence_pool: FencePool,
}

impl Device
{
    pub fn get_debug_utils(&self) -> &DebugUtils
    {
        &self.debug_utils
    }
}

impl IVulkanResource for Device
{
    type Handle = vk::Device;

    fn get_inner_resource(&self) -> &VulkanResource<Self::Handle>
    {
        &self.inner_resource
    }
    fn get_inner_resource_mut(&mut self) -> &mut VulkanResource<vk::Device>
    {
        &mut self.inner_resource
    }
}
