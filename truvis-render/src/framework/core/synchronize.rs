//! 各种同步原语

use ash::vk;

use crate::framework::render_core::Core;

#[derive(Clone)]
pub struct Fence
{
    pub(crate) fence: vk::Fence,
    rhi: &'static Core,
}

impl Fence
{
    pub fn new<S>(rhi: &'static Core, signaled: bool, debug_name: S) -> Self
    where
        S: AsRef<str>,
    {
        let fence_flags = if signaled { vk::FenceCreateFlags::SIGNALED } else { vk::FenceCreateFlags::empty() };
        let fence =
            unsafe { rhi.vk_device().create_fence(&vk::FenceCreateInfo::default().flags(fence_flags), None).unwrap() };

        rhi.set_debug_name(fence, debug_name);
        Self { fence, rhi }
    }

    /// 阻塞等待 fence
    pub fn wait(&self)
    {
        unsafe {
            self.rhi.vk_device().wait_for_fences(std::slice::from_ref(&self.fence), true, u64::MAX).unwrap();
        }
    }

    pub fn reset(&mut self)
    {
        unsafe {
            self.rhi.vk_device().reset_fences(std::slice::from_ref(&self.fence)).unwrap();
        }
    }

    pub fn destroy(self)
    {
        unsafe {
            self.rhi.vk_device().destroy_fence(self.fence, None);
        }
    }
}

#[derive(Copy, Clone)]
pub struct Semaphore
{
    pub(crate) semaphore: vk::Semaphore,
    rhi: &'static Core,
}

impl Semaphore
{
    pub fn new<S>(rhi: &'static Core, debug_name: S) -> Self
    where
        S: AsRef<str>,
    {
        let semaphore = unsafe { rhi.vk_device().create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() };

        rhi.set_debug_name(semaphore, debug_name);
        Self { semaphore, rhi }
    }

    pub fn destroy(self)
    {
        unsafe {
            self.rhi.vk_device().destroy_semaphore(self.semaphore, None);
        }
    }
}


pub struct PipelineBarrier {}

impl PipelineBarrier {}
