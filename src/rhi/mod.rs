use ash::{vk, Device, Entry, Instance};

use crate::{
    rhi::physical_device::RhiPhysicalDevice,
    rhi_type::{command_pool::RhiCommandPool, queue::RhiQueue},
};


pub(crate) mod create_utils;
pub(crate) mod physical_device;
mod rhi_impl_init;
mod rhi_impl_property;
mod rhi_impl_tools;
pub(crate) mod rhi_init_info;


static mut RHI: Option<Rhi> = None;


/// Rhi 只需要做到能够创建各种资源的程度就行了
pub struct Rhi
{
    vk_pf: Option<Entry>,
    instance: Option<Instance>,

    debug_util_pf: Option<ash::extensions::ext::DebugUtils>,
    dynamic_render_pf: Option<ash::extensions::khr::DynamicRendering>,
    acc_pf: Option<ash::extensions::khr::AccelerationStructure>,

    debug_util_messenger: Option<vk::DebugUtilsMessengerEXT>,

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
