use ash::vk;

use crate::framework::render_core::Core;

pub struct CommandPool
{
    pub command_pool: vk::CommandPool,
    pub queue_family_index: u32,
}

impl CommandPool
{
    pub fn reset(&mut self, rhi: &Core)
    {
        unsafe {
            rhi.vk_device().reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::RELEASE_RESOURCES).unwrap();
        }
    }
}
