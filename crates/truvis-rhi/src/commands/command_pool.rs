use std::rc::Rc;

use ash::vk;

use crate::{
    commands::command_queue::QueueFamily,
    foundation::{debug_messenger::DebugType, device::DeviceFunctions},
};

/// command pool 是和 queue family 绑定的，而不是和 queue 绑定的
pub struct CommandPool
{
    handle: vk::CommandPool,
    _queue_family: QueueFamily,

    _debug_name: String,

    pub(crate) device_functions: Rc<DeviceFunctions>,
}
/// 构造函数
impl CommandPool
{
    #[inline]
    pub fn new(
        device_functions: Rc<DeviceFunctions>,
        queue_family: QueueFamily,
        flags: vk::CommandPoolCreateFlags,
        debug_name: &str,
    ) -> Self
    {
        let pool = unsafe {
            device_functions
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
            _debug_name: debug_name.to_string(),
            device_functions: device_functions.clone(),
        };
        device_functions.set_debug_name(&command_pool, debug_name);
        command_pool
    }

    pub fn destroy(self)
    {
        unsafe {
            self.device_functions.destroy_command_pool(self.handle, None);
        }
    }
}

/// getters
impl CommandPool
{
    /// getter
    #[inline]
    pub fn handle(&self) -> vk::CommandPool
    {
        self.handle
    }
}
/// tools
impl CommandPool
{
    /// 这个调用并不会释放资源，而是将 pool 内的 command buffer 设置到初始状态
    ///
    /// reset 之后，pool 内的 command buffer 又可以重新录制命令
    pub fn reset_all_buffers(&self)
    {
        unsafe {
            self.device_functions
                .reset_command_pool(self.handle, vk::CommandPoolResetFlags::RELEASE_RESOURCES)
                .unwrap();
        }
    }
}

impl DebugType for CommandPool
{
    fn debug_type_name() -> &'static str
    {
        "RhiCommandPool"
    }

    fn vk_handle(&self) -> impl vk::Handle
    {
        self.handle
    }
}
