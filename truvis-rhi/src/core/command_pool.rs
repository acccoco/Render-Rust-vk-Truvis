use std::rc::Rc;

use ash::vk;

use crate::{core::device::RhiDevice, rhi::Rhi};

pub struct RhiCommandPool {
    handle: vk::CommandPool,
    _queue_family_index: u32,

    device: Rc<RhiDevice>,
}

impl RhiCommandPool {
    #[inline]
    pub fn new(rhi: &Rhi, queue_family_index: u32, flags: vk::CommandPoolCreateFlags, debug_name: &str) -> Self {
        Self::new_before_rhi(rhi.device.clone(), queue_family_index, flags, debug_name)
    }

    /// 用于在 rhi 初始化完成之前创建 command pool
    #[inline]
    pub fn new_before_rhi(
        device: Rc<RhiDevice>,
        queue_family_index: u32,
        flags: vk::CommandPoolCreateFlags,
        debug_name: &str,
    ) -> Self {
        let pool = unsafe {
            device
                .create_command_pool(
                    &vk::CommandPoolCreateInfo::default().queue_family_index(queue_family_index).flags(flags),
                    None,
                )
                .unwrap()
        };

        device.debug_utils.set_object_debug_name(pool, debug_name);
        Self {
            handle: pool,
            _queue_family_index: queue_family_index,
            device,
        }
    }

    /// getter
    #[inline]
    pub fn handle(&self) -> vk::CommandPool {
        self.handle
    }

    /// 这个调用并不会释放资源，而是将 pool 内的 command buffer 设置到初始状态
    ///
    /// reset 之后，pool 内的 command buffer 又可以重新录制命令
    pub fn reset(&self) {
        unsafe {
            self.device.reset_command_pool(self.handle, vk::CommandPoolResetFlags::RELEASE_RESOURCES).unwrap();
        }
    }
}
