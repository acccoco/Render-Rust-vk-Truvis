use std::ffi::{CStr, CString};

use ash::vk;

pub struct RhiDebugUtils {
    pub vk_debug_utils_instance: ash::ext::debug_utils::Instance,
    pub vk_debug_utils_device: ash::ext::debug_utils::Device,
    pub vk_debug_utils_messenger: vk::DebugUtilsMessengerEXT,
}
impl Drop for RhiDebugUtils {
    fn drop(&mut self) {
        unsafe {
            log::info!("Destroying RhiDebugUtils");
            self.vk_debug_utils_instance.destroy_debug_utils_messenger(self.vk_debug_utils_messenger, None);
        }
    }
}

/// debug messenger 的回调函数
/// # Safety
unsafe extern "system" fn vk_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = unsafe { *p_callback_data };

    let msg = if callback_data.p_message.is_null() {
        std::borrow::Cow::from("")
    } else {
        unsafe { CStr::from_ptr(callback_data.p_message).to_string_lossy() }
    };

    let format_msg = format!("[{:?}]\n{}\n", message_type, msg);

    // 这个看起来像个 bug
    let skip_msg = [
        "DebugTypePointer: expected operand Base Type is not a valid debug type",
        "DebugFunctionDefinition: must be in the entry basic block of the function",
    ];

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            for skip_msg in &skip_msg {
                if format_msg.contains(skip_msg) {
                    log::warn!("Skipping message: {}", skip_msg);
                    return vk::FALSE; // 返回 False 表示不需要 layer developer 处理
                }
            }
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

impl RhiDebugUtils {
    pub fn new(vk_pf: &ash::Entry, instance: &ash::Instance, device: &ash::Device) -> Self {
        let loader = ash::ext::debug_utils::Instance::new(vk_pf, instance);

        let create_info = Self::debug_utils_messenger_ci();
        let debug_messenger = unsafe { loader.create_debug_utils_messenger(&create_info, None).unwrap() };

        let vk_debug_utils_device = ash::ext::debug_utils::Device::new(instance, device);

        Self {
            vk_debug_utils_instance: loader,
            vk_debug_utils_messenger: debug_messenger,
            vk_debug_utils_device,
        }
    }

    /// 存放 msg 参数，用于初始化 debug messenger
    pub fn debug_msg_type() -> vk::DebugUtilsMessageTypeFlagsEXT {
        static mut DEBUG_MSG_TYPE: vk::DebugUtilsMessageTypeFlagsEXT = vk::DebugUtilsMessageTypeFlagsEXT::empty();
        unsafe {
            if vk::DebugUtilsMessageTypeFlagsEXT::empty() == DEBUG_MSG_TYPE {
                DEBUG_MSG_TYPE = vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE;
            }
            DEBUG_MSG_TYPE
        }
    }

    /// 存放 msg 参数，用于初始化 debug messenger
    pub fn debug_msg_severity() -> vk::DebugUtilsMessageSeverityFlagsEXT {
        static mut DEBUG_MSG_SEVERITY: vk::DebugUtilsMessageSeverityFlagsEXT =
            vk::DebugUtilsMessageSeverityFlagsEXT::empty();
        unsafe {
            if vk::DebugUtilsMessageSeverityFlagsEXT::empty() == DEBUG_MSG_SEVERITY {
                DEBUG_MSG_SEVERITY =
                    vk::DebugUtilsMessageSeverityFlagsEXT::WARNING | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR;
            }
            DEBUG_MSG_SEVERITY
        }
    }

    /// 用于创建 debug messenger 的结构体
    pub fn debug_utils_messenger_ci() -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
        vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(Self::debug_msg_severity())
            .message_type(Self::debug_msg_type())
            .pfn_user_callback(Some(vk_debug_callback))
    }

    #[inline]
    pub fn set_object_debug_name(&self, handle: impl vk::Handle + Copy, name: impl AsRef<str>) {
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
    pub fn set_object_debug_tag<T>(&self, _handle: T, _tag: u64)
    where
        T: vk::Handle + Copy,
    {
        todo!("暂时还不知道这个有什么作用")
    }

    /// - command type: state, action
    /// - supported queue type: graphics, compute
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

    /// - command type: state, action
    /// - supported queue type: graphics, compute
    #[inline]
    pub fn cmd_end_debug_label(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            self.vk_debug_utils_device.cmd_end_debug_utils_label(command_buffer);
        }
    }

    /// - command type: action
    /// - supported queue type: graphics, compute
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
    pub fn end_queue_label(&self, queue: vk::Queue) {
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
