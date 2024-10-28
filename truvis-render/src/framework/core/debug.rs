use std::ffi::CString;

use ash::vk;

pub struct DebugUtils
{
    vk_debug_utils: ash::extensions::ext::DebugUtils,
}

impl DebugUtils
{
    pub fn set_debug_name<T, S>(&self, device: vk::Device, handle: T, name: S)
    where
        T: vk::Handle + Copy,
        S: AsRef<str>,
    {
        let name = if name.as_ref().is_empty() { "empty-debug-name" } else { name.as_ref() };
        let name = CString::new(name).unwrap();
        unsafe {
            self.vk_debug_utils
                .set_debug_utils_object_name(
                    device,
                    &vk::DebugUtilsObjectNameInfoEXT::builder()
                        .object_name(name.as_c_str())
                        .object_type(T::TYPE)
                        .object_handle(handle.as_raw()),
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
            self.vk_debug_utils.cmd_begin_debug_utils_label(
                command_buffer,
                &vk::DebugUtilsLabelEXT::builder().label_name(name.as_c_str()).color(color.into()),
            );
        }
    }

    pub fn cmd_end_label(&self, command_buffer: vk::CommandBuffer)
    {
        unsafe {
            self.vk_debug_utils.cmd_end_debug_utils_label(command_buffer);
        }
    }

    pub fn cmd_insert_label<S>(&self, command_buffer: vk::CommandBuffer, name: S, color: glam::Vec4)
    where
        S: AsRef<str>,
    {
        let name = CString::new(name.as_ref()).unwrap();
        unsafe {
            self.vk_debug_utils.cmd_insert_debug_utils_label(
                command_buffer,
                &vk::DebugUtilsLabelEXT::builder().label_name(name.as_c_str()).color(color.into()),
            );
        }
    }
}
