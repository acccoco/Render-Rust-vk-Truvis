use std::rc::Rc;

use ash::vk;

use crate::{
    foundation::{debug_messenger::DebugType, device::DeviceFunctions},
    render_context::RenderContext,
};

/// # Destroy
/// 不应该实现 Semaphore，因为可以 Clone，需要手动 destroy
#[derive(Clone)]
pub struct Semaphore
{
    semaphore: vk::Semaphore,
    device_functions: Rc<DeviceFunctions>,
}

/// 创建与销毁
impl Semaphore
{
    pub fn new(device_functions: Rc<DeviceFunctions>, debug_name: &str) -> Self
    {
        let semaphore =
            unsafe { device_functions.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() };

        let semaphore = Self {
            semaphore,
            device_functions: device_functions.clone(),
        };
        device_functions.set_debug_name(&semaphore, debug_name);
        semaphore
    }

    pub fn new_timeline(rhi: &RenderContext, initial_value: u64, debug_name: &str) -> Self
    {
        let mut timeline_type_ci = vk::SemaphoreTypeCreateInfo::default()
            .semaphore_type(vk::SemaphoreType::TIMELINE)
            .initial_value(initial_value);
        let timeline_semaphore_ci = vk::SemaphoreCreateInfo::default().push_next(&mut timeline_type_ci);
        let semaphore = unsafe { rhi.device().ash_handle().create_semaphore(&timeline_semaphore_ci, None).unwrap() };

        let semaphore = Self {
            semaphore,
            device_functions: rhi.device().functions.clone(),
        };
        rhi.device_functions().set_debug_name(&semaphore, debug_name);
        semaphore
    }
    #[inline]
    pub fn destroy(self)
    {
        unsafe {
            self.device_functions.destroy_semaphore(self.semaphore, None);
        }
    }
}

/// getters
impl Semaphore
{
    #[inline]
    pub fn handle(&self) -> vk::Semaphore
    {
        self.semaphore
    }
}

/// tools
impl Semaphore
{
    #[inline]
    pub fn wait_timeline(&self, timeline_value: u64, timeout_ns: u64)
    {
        unsafe {
            let wait_semaphore = [self.semaphore];
            let wait_info = vk::SemaphoreWaitInfo::default()
                .semaphores(&wait_semaphore)
                .values(std::slice::from_ref(&timeline_value));
            self.device_functions.wait_semaphores(&wait_info, timeout_ns).unwrap();
        }
    }
}

impl DebugType for Semaphore
{
    fn debug_type_name() -> &'static str
    {
        "RhiSemaphore"
    }

    fn vk_handle(&self) -> impl vk::Handle
    {
        self.semaphore
    }
}
