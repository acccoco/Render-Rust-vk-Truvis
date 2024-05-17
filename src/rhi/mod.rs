use ash::{vk, Device, Entry, Instance};

use crate::{
    rhi::physical_device::RhiPhysicalDevice,
    rhi_type::{command_pool::RhiCommandPool, queue::RhiQueue},
};


pub(crate) mod create_utils;
mod init;
pub(crate) mod physical_device;
mod props;
pub(crate) mod rhi_init_info;
mod tools;


static mut RHI: Option<Rhi> = None;


/// Rhi 只需要做到能够创建各种资源的程度就行了
pub struct Rhi
{
    /// vk 基础函数的接口
    vk_pf: Option<Entry>,
    vk_instance: Option<Instance>,

    vk_debug_util_pf: Option<ash::extensions::ext::DebugUtils>,
    vk_dynamic_render_pf: Option<ash::extensions::khr::DynamicRendering>,
    vk_acceleration_pf: Option<ash::extensions::khr::AccelerationStructure>,

    vk_debug_util_messenger: Option<vk::DebugUtilsMessengerEXT>,

    physical_device: Option<RhiPhysicalDevice>,
    device: Option<Device>,

    /// 可以提交 graphics 命令，也可以进行 present 操作
    graphics_queue: Option<RhiQueue>,
    transfer_queue: Option<RhiQueue>,
    compute_queue: Option<RhiQueue>,

    vma: Option<vk_mem::Allocator>,

    descriptor_pool: Option<vk::DescriptorPool>,

    graphics_command_pool: Option<RhiCommandPool>,
    transfer_command_pool: Option<RhiCommandPool>,
    compute_command_pool: Option<RhiCommandPool>,
}
