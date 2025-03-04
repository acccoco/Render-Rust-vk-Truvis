use std::ffi::{CStr, CString};

use ash::vk;


pub struct DebugUtils
{
    pub vk_debug_utils_instance: ash::ext::debug_utils::Instance,
    pub vk_debug_utils_device: ash::ext::debug_utils::Device,
    pub vk_debug_utils_messenger: vk::DebugUtilsMessengerEXT,
}

static mut DEBUG_MSG_TYPE: vk::DebugUtilsMessageTypeFlagsEXT = vk::DebugUtilsMessageTypeFlagsEXT::empty();
static mut DEBUG_MSG_SEVERITY: vk::DebugUtilsMessageSeverityFlagsEXT = vk::DebugUtilsMessageSeverityFlagsEXT::empty();

impl DebugUtils
{
    pub fn new(vk_pf: &ash::Entry, instance: &ash::Instance, device: &ash::Device) -> Self
    {
        let loader = ash::ext::debug_utils::Instance::new(vk_pf, instance);

        let create_info = Self::get_debug_utils_messenger_ci();
        let debug_messenger = unsafe { loader.create_debug_utils_messenger(&create_info, None).unwrap() };

        let debug_utils = ash::ext::debug_utils::Device::new(instance, device);

        Self {
            vk_debug_utils_instance: loader,
            vk_debug_utils_messenger: debug_messenger,
            vk_debug_utils_device: debug_utils,
        }
    }

    pub fn get_debug_msg_type() -> vk::DebugUtilsMessageTypeFlagsEXT
    {
        unsafe {
            if vk::DebugUtilsMessageTypeFlagsEXT::empty() == DEBUG_MSG_TYPE {
                DEBUG_MSG_TYPE =
                    vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE;
            }
            DEBUG_MSG_TYPE
        }
    }

    pub fn get_debug_msg_severity() -> vk::DebugUtilsMessageSeverityFlagsEXT
    {
        unsafe {
            if vk::DebugUtilsMessageSeverityFlagsEXT::empty() == DEBUG_MSG_SEVERITY {
                DEBUG_MSG_SEVERITY =
                    vk::DebugUtilsMessageSeverityFlagsEXT::WARNING | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR;
            }
            DEBUG_MSG_SEVERITY
        }
    }

    /// debug messenger 的回调函数
    /// # Safety
    unsafe extern "system" fn vk_debug_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
        _user_data: *mut std::os::raw::c_void,
    ) -> vk::Bool32
    {
        let callback_data = *p_callback_data;

        let msg = if callback_data.p_message.is_null() {
            std::borrow::Cow::from("")
        } else {
            CStr::from_ptr(callback_data.p_message).to_string_lossy()
        };


        // 按照 | 切分 msg 字符串，并在中间插入换行符
        // let msg = msg.split('|').collect::<Vec<&str>>().join("\n\t");
        // let msg = msg.split(" ] ").collect::<Vec<&str>>().join(" ]\n\t ");
        let format_msg = format!("[{:?}]\n{}\n", message_type, msg);

        match message_severity {
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
                log::error!("{}", format_msg);
            }
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
                log::warn!("{}", format_msg);
            }
            vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
                log::info!("{}", format_msg);
            }
            _ => log::info!("{}", format_msg),
        };

        // 只有 layer developer 才需要返回 True
        vk::FALSE
    }

    pub fn get_debug_utils_messenger_ci() -> vk::DebugUtilsMessengerCreateInfoEXT<'static>
    {
        vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(Self::get_debug_msg_severity())
            .message_type(Self::get_debug_msg_type())
            .pfn_user_callback(Some(Self::vk_debug_callback))
    }

    #[inline]
    pub fn set_object_debug_name<T, S>(&self, handle: T, name: S)
    where
        T: vk::Handle + Copy,
        S: AsRef<str>,
    {
        let name = CString::new(name.as_ref()).unwrap();
        unsafe {
            self.vk_debug_utils_device
                .set_debug_utils_object_name(
                    &vk::DebugUtilsObjectNameInfoEXT::default().object_name(name.as_c_str()).object_handle(handle),
                )
                .unwrap();
        }
    }

    #[inline]
    pub fn set_object_debug_tag<T>(&self, handle: T, tag: u64)
    where
        T: vk::Handle + Copy,
    {
        todo!("暂时还不知道这个有什么作用")
    }

    #[inline]
    pub fn cmd_begin_debug_label<S>(&self, command_buffer: vk::CommandBuffer, label_name: S, label_color: glam::Vec4)
    where
        S: AsRef<str>,
    {
        let name = CString::new(label_name.as_ref()).unwrap();
        unsafe {
            self.vk_debug_utils_device.cmd_begin_debug_utils_label(
                command_buffer,
                &vk::DebugUtilsLabelEXT::default().label_name(name.as_c_str()).color(label_color.into()),
            );
        }
    }

    #[inline]
    pub fn cmd_end_debug_label(&self, command_buffer: vk::CommandBuffer)
    {
        unsafe {
            self.vk_debug_utils_device.cmd_end_debug_utils_label(command_buffer);
        }
    }

    #[inline]
    pub fn cmd_insert_debug_label<S>(&self, command_buffer: vk::CommandBuffer, label_name: S, label_color: glam::Vec4)
    where
        S: AsRef<str>,
    {
        let name = CString::new(label_name.as_ref()).unwrap();
        unsafe {
            self.vk_debug_utils_device.cmd_insert_debug_utils_label(
                command_buffer,
                &vk::DebugUtilsLabelEXT::default().label_name(name.as_c_str()).color(label_color.into()),
            );
        }
    }

    #[inline]
    pub fn begin_queue_label<S>(&self, queue: vk::Queue, label_name: S, label_color: glam::Vec4)
    where
        S: AsRef<str>,
    {
        let name = CString::new(label_name.as_ref()).unwrap();
        unsafe {
            self.vk_debug_utils_device.queue_begin_debug_utils_label(
                queue,
                &vk::DebugUtilsLabelEXT::default().label_name(name.as_c_str()).color(label_color.into()),
            );
        }
    }

    #[inline]
    pub fn end_queue_label(&self, queue: vk::Queue)
    {
        unsafe {
            self.vk_debug_utils_device.queue_end_debug_utils_label(queue);
        }
    }

    #[inline]
    pub fn insert_queue_label<S>(&self, queue: vk::Queue, label_name: S, label_color: glam::Vec4)
    where
        S: AsRef<str>,
    {
        let name = CString::new(label_name.as_ref()).unwrap();
        unsafe {
            self.vk_debug_utils_device.queue_insert_debug_utils_label(
                queue,
                &vk::DebugUtilsLabelEXT::default().label_name(name.as_c_str()).color(label_color.into()),
            );
        }
    }
}
