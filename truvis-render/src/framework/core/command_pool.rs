use ash::vk;

use crate::framework::rhi::Rhi;

pub struct RhiCommandPool
{
    pub(crate) command_pool: vk::CommandPool,
    pub(crate) queue_family_index: u32,
}

impl RhiCommandPool
{
    #[inline]
    pub(crate) fn reset(&mut self, rhi: &Rhi)
    {
        unsafe {
            rhi.device().reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::RELEASE_RESOURCES).unwrap();
        }
    }
}