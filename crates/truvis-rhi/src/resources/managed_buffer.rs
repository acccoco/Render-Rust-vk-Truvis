use ash::vk;

pub struct ManagedBuffer {
    handle: vk::Buffer,
    allocation: vk_mem::Allocation,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    debug_name: String,
}