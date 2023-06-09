use std::{
    any::Any,
    collections::HashSet,
    ffi::{c_char, CStr, CString},
};

use ash::{extensions::khr::Swapchain, vk, Device, Entry, Instance};
use itertools::Itertools;
use queue::RhiQueueFamilyPresentProps;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::{
    rhi::{physical_device::RhiPhysicalDevice, queue::RhiQueueType, render_ctx::RenderCtx},
    window_system::WindowSystem,
};

mod init;
mod physical_device;
mod queue;
mod render_ctx;
mod swapchain;

/// Rhi 只需要做到能够创建各种资源的程度就行了
#[derive(Default)]
pub struct RhiCore
{
    entry: Option<Entry>,
    instance: Option<Instance>,

    debug_util_loader: Option<ash::extensions::ext::DebugUtils>,
    debug_util_messenger: Option<vk::DebugUtilsMessengerEXT>,

    surface_loader: Option<ash::extensions::khr::Surface>,
    /// 这个字段是可空的
    surface: Option<vk::SurfaceKHR>,

    physical_device: Option<RhiPhysicalDevice>,

    queue_family_index_compute: Option<u32>,
    queue_family_index_graphics: Option<u32>,
    queue_family_index_present: Option<u32>,

    device: Option<Device>,
    queue_compute: Option<vk::Queue>,
    queue_graphics: Option<vk::Queue>,
    queue_present: Option<vk::Queue>,

    dynamic_render_loader: Option<ash::extensions::khr::DynamicRendering>,

    swapchain_loader: Option<ash::extensions::khr::Swapchain>,
}

pub struct Rhi<'a>
{
    window_system: &'a WindowSystem,
    core: RhiCore,
    swapchain: Swapchain,
    context: RenderCtx,
}

// 工具方法
impl RhiCore
{
    pub fn set_debug_name<T>(&self, handle: T, name: &str)
    where
        T: vk::Handle + Copy,
    {
        let name = CString::new(name).unwrap();
        unsafe {
            self.debug_util_loader
                .as_ref()
                .unwrap()
                .set_debug_utils_object_name(
                    self.device.as_ref().unwrap().handle(),
                    &vk::DebugUtilsObjectNameInfoEXT::builder()
                        .object_name(name.as_c_str())
                        .object_type(T::TYPE)
                        .object_handle(handle.as_raw()),
                )
                .unwrap();
        }
    }
}
