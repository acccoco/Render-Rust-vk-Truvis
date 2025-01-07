use ash::vk;

use crate::framework::rhi::Rhi;

pub struct RhiCommandPool
{
    pub command_pool: vk::CommandPool,
    pub queue_family_index: u32,
}

impl RhiCommandPool
{
    pub fn reset(&mut self, rhi: &Rhi)
    {
        unsafe {
            rhi.vk_device().reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::RELEASE_RESOURCES).unwrap();
        }
    }
}
