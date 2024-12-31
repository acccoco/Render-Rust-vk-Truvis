//! 各种同步原语

use ash::vk;

use crate::framework::rhi::Rhi;

#[derive(Clone)]
pub struct RhiFence
{
    pub(crate) fence: vk::Fence,
    rhi: &'static Rhi,
}

impl RhiFence
{
    pub fn new<S>(rhi: &'static Rhi, signaled: bool, debug_name: S) -> Self
    where
        S: AsRef<str>,
    {
        let fence_flags = if signaled { vk::FenceCreateFlags::SIGNALED } else { vk::FenceCreateFlags::empty() };
        let fence =
            unsafe { rhi.device().create_fence(&vk::FenceCreateInfo::builder().flags(fence_flags), None).unwrap() };

        rhi.set_debug_name(fence, debug_name);
        Self { fence, rhi }
    }

    /// 阻塞等待 fence
    pub fn wait(&self)
    {
        unsafe {
            self.rhi.device().wait_for_fences(std::slice::from_ref(&self.fence), true, u64::MAX).unwrap();
        }
    }

    pub fn reset(&mut self)
    {
        unsafe {
            self.rhi.device().reset_fences(std::slice::from_ref(&self.fence)).unwrap();
        }
    }

    pub fn destroy(self)
    {
        unsafe {
            self.rhi.device().destroy_fence(self.fence, None);
        }
    }
}

#[derive(Copy, Clone)]
pub struct RhiSemaphore
{
    pub(crate) semaphore: vk::Semaphore,
    rhi: &'static Rhi,
}

impl RhiSemaphore
{
    pub fn new<S>(rhi: &'static Rhi, debug_name: S) -> Self
    where
        S: AsRef<str>,
    {
        let semaphore = unsafe { rhi.device().create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() };

        rhi.set_debug_name(semaphore, debug_name);
        Self { semaphore, rhi }
    }

    pub fn destroy(self)
    {
        unsafe {
            self.rhi.device().destroy_semaphore(self.semaphore, None);
        }
    }
}


pub struct RhiPipelineBarrier {}

impl RhiPipelineBarrier {}
