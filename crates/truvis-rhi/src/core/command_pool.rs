use std::rc::Rc;

use ash::vk;

use crate::core::command_queue::RhiQueueFamily;
use crate::core::debug_utils::RhiDebugType;
use crate::core::device::RhiDevice;

/// command pool 是和 queue family 绑定的，而不是和 queue 绑定的
pub struct RhiCommandPool {
    handle: vk::CommandPool,
    _queue_family: RhiQueueFamily,

    device: Rc<RhiDevice>,
    _debug_name: String,
}
impl RhiDebugType for RhiCommandPool {
    fn debug_type_name() -> &'static str {
        "RhiCommandPool"
    }

    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
}
impl Drop for RhiCommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.handle, None);
        }
    }
}
impl RhiCommandPool {
    #[inline]
    pub fn new(
        device: Rc<RhiDevice>,
        queue_family: RhiQueueFamily,
        flags: vk::CommandPoolCreateFlags,
        debug_name: &str,
    ) -> Self {
        let pool = unsafe {
            device
                .create_command_pool(
                    &vk::CommandPoolCreateInfo::default()
                        .queue_family_index(queue_family.queue_family_index)
                        .flags(flags),
                    None,
                )
                .unwrap()
        };

        let command_pool = Self {
            handle: pool,
            _queue_family: queue_family,
            device: device.clone(),
            _debug_name: debug_name.to_string(),
        };
        device.debug_utils().set_debug_name(&command_pool, debug_name);
        command_pool
    }

    /// getter
    #[inline]
    pub fn handle(&self) -> vk::CommandPool {
        self.handle
    }

    /// 这个调用并不会释放资源，而是将 pool 内的 command buffer 设置到初始状态
    ///
    /// reset 之后，pool 内的 command buffer 又可以重新录制命令
    pub fn reset_all_buffers(&self) {
        unsafe {
            self.device.reset_command_pool(self.handle, vk::CommandPoolResetFlags::RELEASE_RESOURCES).unwrap();
        }
    }
}
