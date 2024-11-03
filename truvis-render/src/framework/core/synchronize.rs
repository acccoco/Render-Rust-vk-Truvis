//! 各种同步原语

use ash::vk;

use crate::framework::rhi::Rhi;

#[derive(Clone)]
pub struct RhiFence<'a>
{
    pub(crate) fence: vk::Fence,
    rhi: &'a Rhi,
}

impl<'a> RhiFence<'a>
{
    pub fn new<S>(rhi: &'a Rhi, signaled: bool, debug_name: S) -> Self
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
    #[inline]
    pub fn wait(&self)
    {
        unsafe {
            self.rhi.device().wait_for_fences(std::slice::from_ref(&self.fence), true, u64::MAX).unwrap();
        }
    }

    #[inline]
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
pub struct RhiSemaphore<'a>
{
    pub(crate) semaphore: vk::Semaphore,
    rhi: &'a Rhi,
}

impl<'a> RhiSemaphore<'a>
{
    pub fn new<S>(rhi: &'a Rhi, debug_name: S) -> Self
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
