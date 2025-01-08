use std::ffi::CString;

use ash::vk;

use crate::framework::rhi::RhiInitInfo;

pub struct RhiDebugUtils
{
    pub vk_debug_utils_instance: ash::ext::debug_utils::Instance,
    pub vk_debug_utils_device: ash::ext::debug_utils::Device,
    pub vk_debug_utils_messenger: vk::DebugUtilsMessengerEXT,
}

impl RhiDebugUtils
{
    pub fn new(vk_pf: &ash::Entry, instance: &ash::Instance, device: &ash::Device, init_info: &RhiInitInfo) -> Self
    {
        let loader = ash::ext::debug_utils::Instance::new(vk_pf, instance);

        let create_info = init_info.get_debug_utils_messenger_ci();
        let debug_messenger = unsafe { loader.create_debug_utils_messenger(&create_info, None).unwrap() };

        let debug_utils = ash::ext::debug_utils::Device::new(instance, device);

        Self {
            vk_debug_utils_instance: loader,
            vk_debug_utils_messenger: debug_messenger,
            vk_debug_utils_device: debug_utils,
        }
    }

    pub fn set_debug_name<T, S>(&self, handle: T, name: S)
    where
        T: vk::Handle + Copy,
        S: AsRef<str>,
    {
        let name = if name.as_ref().is_empty() { "empty-debug-name" } else { name.as_ref() };
        let name = CString::new(name).unwrap();
        unsafe {
            self.vk_debug_utils_device
                .set_debug_utils_object_name(
                    &vk::DebugUtilsObjectNameInfoEXT::default().object_name(name.as_c_str()).object_handle(handle),
                )
                .unwrap();
        }
    }

    pub fn cmd_begin_label<S>(&self, command_buffer: vk::CommandBuffer, name: S, color: glam::Vec4)
    where
        S: AsRef<str>,
    {
        let name = CString::new(name.as_ref()).unwrap();
        unsafe {
            self.vk_debug_utils_device.cmd_begin_debug_utils_label(
                command_buffer,
                &vk::DebugUtilsLabelEXT::default().label_name(name.as_c_str()).color(color.into()),
            );
        }
    }

    pub fn cmd_end_label(&self, command_buffer: vk::CommandBuffer)
    {
        unsafe {
            self.vk_debug_utils_device.cmd_end_debug_utils_label(command_buffer);
        }
    }

    pub fn cmd_insert_label<S>(&self, command_buffer: vk::CommandBuffer, name: S, color: glam::Vec4)
    where
        S: AsRef<str>,
    {
        let name = CString::new(name.as_ref()).unwrap();
        unsafe {
            self.vk_debug_utils_device.cmd_insert_debug_utils_label(
                command_buffer,
                &vk::DebugUtilsLabelEXT::default().label_name(name.as_c_str()).color(color.into()),
            );
        }
    }
}
