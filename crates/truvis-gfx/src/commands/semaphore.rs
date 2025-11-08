use ash::vk;

use crate::{foundation::debug_messenger::DebugType, render_context::RenderContext};

/// # Destroy
/// 不应该实现 Semaphore，因为可以 Clone，需要手动 destroy
#[derive(Clone)]
pub struct Semaphore {
    semaphore: vk::Semaphore,
}

// 创建与销毁
impl Semaphore {
    pub fn new(debug_name: &str) -> Self {
        let device_functions = RenderContext::get().device_functions();
        let semaphore =
            unsafe { device_functions.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() };

        let semaphore = Self { semaphore };
        device_functions.set_debug_name(&semaphore, debug_name);
        semaphore
    }

    pub fn new_timeline(initial_value: u64, debug_name: &str) -> Self {
        let device_functions = RenderContext::get().device_functions();
        let mut timeline_type_ci = vk::SemaphoreTypeCreateInfo::default()
            .semaphore_type(vk::SemaphoreType::TIMELINE)
            .initial_value(initial_value);
        let timeline_semaphore_ci = vk::SemaphoreCreateInfo::default().push_next(&mut timeline_type_ci);
        let semaphore = unsafe { device_functions.create_semaphore(&timeline_semaphore_ci, None).unwrap() };

        let semaphore = Self { semaphore };
        device_functions.set_debug_name(&semaphore, debug_name);
        semaphore
    }
    #[inline]
    pub fn destroy(self) {
        let device_functions = RenderContext::get().device_functions();
        unsafe {
            device_functions.destroy_semaphore(self.semaphore, None);
        }
    }
}

// getters
impl Semaphore {
    #[inline]
    pub fn handle(&self) -> vk::Semaphore {
        self.semaphore
    }
}

// tools
impl Semaphore {
    #[inline]
    pub fn wait_timeline(&self, timeline_value: u64, timeout_ns: u64) {
        let device_functions = RenderContext::get().device_functions();
        unsafe {
            let wait_semaphore = [self.semaphore];
            let wait_info = vk::SemaphoreWaitInfo::default()
                .semaphores(&wait_semaphore)
                .values(std::slice::from_ref(&timeline_value));
            device_functions.wait_semaphores(&wait_info, timeout_ns).unwrap();
        }
    }
}

impl DebugType for Semaphore {
    fn debug_type_name() -> &'static str {
        "GfxSemaphore"
    }

    fn vk_handle(&self) -> impl vk::Handle {
        self.semaphore
    }
}
