use ash::vk;

use crate::{foundation::debug_messenger::DebugType, render_context::RenderContext};

/// # Destroy
/// 不应该实现 Fence，因为可以 Clone，需要手动 destroy
#[derive(Clone)]
pub struct Fence {
    fence: vk::Fence,
}

impl DebugType for Fence {
    fn debug_type_name() -> &'static str {
        "RhiFence"
    }

    fn vk_handle(&self) -> impl vk::Handle {
        self.fence
    }
}

/// 创建与销毁
impl Fence {
    /// # param
    /// * signaled - 是否创建时就 signaled
    pub fn new(signaled: bool, debug_name: &str) -> Self {
        let device_functions = RenderContext::get().device_functions();
        let fence_flags = if signaled { vk::FenceCreateFlags::SIGNALED } else { vk::FenceCreateFlags::empty() };
        let fence =
            unsafe { device_functions.create_fence(&vk::FenceCreateInfo::default().flags(fence_flags), None).unwrap() };

        let fence = Self { fence };
        device_functions.set_debug_name(&fence, debug_name);
        fence
    }
    #[inline]
    pub fn destroy(self) {
        let device_functions = RenderContext::get().device_functions();
        unsafe {
            device_functions.destroy_fence(self.fence, None);
        }
    }
}

/// getters
impl Fence {
    #[inline]
    pub fn handle(&self) -> vk::Fence {
        self.fence
    }
}

/// tools
impl Fence {
    /// 阻塞等待 fence
    #[inline]
    pub fn wait(&self) {
        let device_functions = RenderContext::get().device_functions();
        unsafe {
            device_functions.wait_for_fences(std::slice::from_ref(&self.fence), true, u64::MAX).unwrap();
        }
    }

    #[inline]
    pub fn reset(&self) {
        let device_functions = RenderContext::get().device_functions();
        unsafe {
            device_functions.reset_fences(std::slice::from_ref(&self.fence)).unwrap();
        }
    }
}
