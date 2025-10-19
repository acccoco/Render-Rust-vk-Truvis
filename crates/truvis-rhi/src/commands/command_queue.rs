use std::rc::Rc;

use ash::vk;
use itertools::Itertools;

use crate::{
    commands::{fence::Fence, submit_info::SubmitInfo},
    foundation::{debug_messenger::DebugType, device::DeviceFunctions},
};

#[derive(Clone, Debug)]
pub struct QueueFamily {
    pub name: String,
    pub queue_family_index: u32,
    pub queue_flags: vk::QueueFlags,
    pub queue_count: u32,
}

/// # destroy
///
/// RhiQueueFamily 在 RhiDevice 销毁时会被销毁
pub struct CommandQueue {
    pub(crate) vk_queue: vk::Queue,
    pub(crate) queue_family: QueueFamily,
    pub(crate) device_functions: Rc<DeviceFunctions>,
}
impl DebugType for CommandQueue {
    fn debug_type_name() -> &'static str {
        "RhiQueue"
    }
    fn vk_handle(&self) -> impl vk::Handle {
        self.vk_queue
    }
}

// getter
impl CommandQueue {
    #[inline]
    pub fn queue_family(&self) -> &QueueFamily {
        &self.queue_family
    }

    #[inline]
    pub fn handle(&self) -> vk::Queue {
        self.vk_queue
    }
}

// tools
impl CommandQueue {
    pub fn submit(&self, batches: Vec<SubmitInfo>, fence: Option<Fence>) {
        unsafe {
            // batches 的存在是有必要的，submit_infos 引用的 batches 的内存
            let batches = batches.iter().map(|b| b.submit_info()).collect_vec();
            self.device_functions
                .device
                .queue_submit2(self.vk_queue, &batches, fence.map_or(vk::Fence::null(), |f| f.handle()))
                .unwrap()
        }
    }

    /// 根据 specification，vkQueueWaitIdle 应该和 Fence 效率相同
    #[inline]
    pub fn wait_idle(&self) {
        unsafe { self.device_functions.device.queue_wait_idle(self.vk_queue).unwrap() }
    }
}

// debug 相关命令
impl CommandQueue {
    #[inline]
    pub fn begin_label<S>(&self, label_name: S, label_color: glam::Vec4)
    where
        S: AsRef<str>,
    {
        let name = std::ffi::CString::new(label_name.as_ref()).unwrap();
        unsafe {
            self.device_functions.debug_utils.queue_begin_debug_utils_label(
                self.vk_queue,
                &vk::DebugUtilsLabelEXT::default().label_name(name.as_c_str()).color(label_color.into()),
            );
        }
    }

    #[inline]
    pub fn end_label(&self) {
        unsafe {
            self.device_functions.debug_utils.queue_end_debug_utils_label(self.vk_queue);
        }
    }

    #[inline]
    pub fn insert_label<S>(&self, label_name: S, label_color: glam::Vec4)
    where
        S: AsRef<str>,
    {
        let name = std::ffi::CString::new(label_name.as_ref()).unwrap();
        unsafe {
            self.device_functions.debug_utils.queue_insert_debug_utils_label(
                self.vk_queue,
                &vk::DebugUtilsLabelEXT::default().label_name(name.as_c_str()).color(label_color.into()),
            );
        }
    }
}
