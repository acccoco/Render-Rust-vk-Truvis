use ash::vk;

pub struct RhiCommandPool
{
    pub(crate) command_pool: vk::CommandPool,
    pub(crate) queue_family_index: u32,
}

pub struct RhiQueue {
    pub(crate) queue: vk::Queue,
    pub(crate) queue_family_index: u32,
}
