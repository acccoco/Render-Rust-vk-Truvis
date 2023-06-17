use ash::vk;

pub struct RhiQueue
{
    pub(crate) queue: vk::Queue,
    pub(crate) queue_family_index: u32,
}
