static mut VMA: Option<vk_mem::Allocator> = None;

pub struct AllocatedBase
{
    alloc_create_info: vk_mem::AllocationCreateInfo,
    allocation: vk_mem::Allocation,
    mapped_data: Option<*mut u8>,
    
    /// vk::memory_property_host_coherent
    coherent: bool,
    
    /// 是否是 persistently mapped
    persistent: bool,
}

pub trait IAllcated {}
