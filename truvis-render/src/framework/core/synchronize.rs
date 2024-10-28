//! 各种同步原语

use ash::vk;

use crate::framework::rhi::Rhi;

#[derive(Clone)]
pub struct RhiFence
{
    pub(crate) fence: vk::Fence,
}

impl RhiFence
{
    pub fn new<S>(signaled: bool, debug_name: S) -> Self
    where
        S: AsRef<str>,
    {
        let rhi = Rhi::instance();

        let fence_flags =
            if signaled { vk::FenceCreateFlags::SIGNALED } else { vk::FenceCreateFlags::empty() };
        let fence = unsafe {
            rhi.device()
                .create_fence(&vk::FenceCreateInfo::builder().flags(fence_flags), None)
                .unwrap()
        };

        rhi.set_debug_name(fence, debug_name);
        Self { fence }
    }

    /// 阻塞等待 fence
    #[inline]
    pub fn wait(&self)
    {
        unsafe {
            Rhi::instance()
                .device()
                .wait_for_fences(std::slice::from_ref(&self.fence), true, u64::MAX)
                .unwrap();
        }
    }

    #[inline]
    pub fn reset(&mut self)
    {
        unsafe {
            Rhi::instance().device().reset_fences(std::slice::from_ref(&self.fence)).unwrap();
        }
    }

    pub fn destroy(self)
    {
        unsafe {
            Rhi::instance().device().destroy_fence(self.fence, None);
        }
    }
}

#[derive(Copy, Clone)]
pub struct RhiSemaphore
{
    pub(crate) semaphore: vk::Semaphore,
}

impl RhiSemaphore
{
    pub fn new<S>(debug_name: S) -> Self
    where
        S: AsRef<str>,
    {
        let rhi = Rhi::instance();
        let semaphore = unsafe {
            rhi.device().create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap()
        };

        rhi.set_debug_name(semaphore, debug_name);
        Self { semaphore }
    }

    pub fn destroy(self)
    {
        unsafe {
            Rhi::instance().device().destroy_semaphore(self.semaphore, None);
        }
    }
}


pub struct RhiPipelineBarrier {}

impl RhiPipelineBarrier {}
