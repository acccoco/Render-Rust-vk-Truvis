//! 各种同步原语

use std::rc::Rc;

use crate::{core::device::RhiDevice, rhi::Rhi};
use ash::vk;

/// # Destroy
/// 不应该实现 Fence，因为可以 Clone，需要手动 destroy
#[derive(Clone)]
pub struct RhiFence {
    fence: vk::Fence,
    device: Rc<RhiDevice>,
}

impl RhiFence {
    /// # param
    /// * signaled - 是否创建时就 signaled
    pub fn new(rhi: &Rhi, signaled: bool, debug_name: &str) -> Self {
        let fence_flags = if signaled { vk::FenceCreateFlags::SIGNALED } else { vk::FenceCreateFlags::empty() };
        let fence =
            unsafe { rhi.device().create_fence(&vk::FenceCreateInfo::default().flags(fence_flags), None).unwrap() };

        rhi.device.debug_utils().set_object_debug_name(fence, debug_name);
        Self {
            fence,
            device: rhi.device.clone(),
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::Fence {
        self.fence
    }

    /// 阻塞等待 fence
    #[inline]
    pub fn wait(&self) {
        unsafe {
            self.device.wait_for_fences(std::slice::from_ref(&self.fence), true, u64::MAX).unwrap();
        }
    }

    #[inline]
    pub fn reset(&self) {
        unsafe {
            self.device.reset_fences(std::slice::from_ref(&self.fence)).unwrap();
        }
    }

    #[inline]
    pub fn destroy(self) {
        unsafe {
            self.device.destroy_fence(self.fence, None);
        }
    }
}

/// # Destroy
/// 不应该实现 Semaphore，因为可以 Clone，需要手动 destroy
#[derive(Clone)]
pub struct RhiSemaphore {
    semaphore: vk::Semaphore,
    device: Rc<RhiDevice>,
}

impl RhiSemaphore {
    pub fn new(rhi: &Rhi, debug_name: &str) -> Self {
        let semaphore = unsafe { rhi.device().create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() };

        rhi.device.debug_utils().set_object_debug_name(semaphore, debug_name);
        Self {
            semaphore,
            device: rhi.device.clone(),
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::Semaphore {
        self.semaphore
    }

    #[inline]
    pub fn destroy(self) {
        unsafe {
            self.device.destroy_semaphore(self.semaphore, None);
        }
    }
}

/// 便捷创建 image memory barrier 的结构体
pub struct RhiImageBarrier {
    inner: vk::ImageMemoryBarrier2<'static>,
}

impl Default for RhiImageBarrier {
    fn default() -> Self {
        Self {
            inner: vk::ImageMemoryBarrier2 {
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::UNDEFINED,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::empty(),
                    base_array_layer: 0,
                    layer_count: 1,
                    base_mip_level: 0,
                    level_count: 1,
                },
                ..Default::default()
            },
        }
    }
}

impl RhiImageBarrier {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn inner(&self) -> &vk::ImageMemoryBarrier2 {
        &self.inner
    }

    /// builder
    #[inline]
    pub fn queue_family_transfer(mut self, src_queue_family_index: u32, dst_queue_family_index: u32) -> Self {
        self.inner.src_queue_family_index = src_queue_family_index;
        self.inner.dst_queue_family_index = dst_queue_family_index;
        self
    }

    /// builder
    #[inline]
    pub fn layout_transfer(mut self, old_layout: vk::ImageLayout, new_layout: vk::ImageLayout) -> Self {
        self.inner.old_layout = old_layout;
        self.inner.new_layout = new_layout;
        self
    }

    /// builder
    #[allow(clippy::redundant_clone)]
    #[inline]
    pub fn src_mask(mut self, src_stage_mask: vk::PipelineStageFlags2, src_access_mask: vk::AccessFlags2) -> Self {
        self.inner.src_stage_mask = src_stage_mask;
        self.inner.src_access_mask = src_access_mask;
        self
    }

    /// builder
    #[allow(clippy::redundant_clone)]
    #[inline]
    pub fn dst_mask(mut self, dst_stage_mask: vk::PipelineStageFlags2, dst_access_mask: vk::AccessFlags2) -> Self {
        self.inner.dst_stage_mask = dst_stage_mask;
        self.inner.dst_access_mask = dst_access_mask;
        self
    }

    /// builder
    /// layer 和 miplevel 都使用默认值
    #[inline]
    pub fn image_aspect_flag(mut self, aspect_mask: vk::ImageAspectFlags) -> Self {
        self.inner.subresource_range.aspect_mask = aspect_mask;
        self
    }

    /// builder
    #[inline]
    pub fn image(mut self, image: vk::Image) -> Self {
        self.inner.image = image;
        self
    }
}

/// barrier 使用的 src 和 dst 访问 mask
#[derive(Copy, Clone)]
pub struct RhiBarrierMask {
    pub src_stage: vk::PipelineStageFlags2,
    pub dst_stage: vk::PipelineStageFlags2,
    pub src_access: vk::AccessFlags2,
    pub dst_access: vk::AccessFlags2,
}

pub struct RhiBufferBarrier {
    inner: vk::BufferMemoryBarrier2<'static>,
}

impl Default for RhiBufferBarrier {
    fn default() -> Self {
        Self {
            inner: vk::BufferMemoryBarrier2 {
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                ..Default::default()
            },
        }
    }
}

impl RhiBufferBarrier {
    #[inline]
    pub fn inner(&self) -> &vk::BufferMemoryBarrier2 {
        &self.inner
    }

    #[inline]
    pub fn src_mask(mut self, src_stage_mask: vk::PipelineStageFlags2, src_access_mask: vk::AccessFlags2) -> Self {
        self.inner.src_stage_mask = src_stage_mask;
        self.inner.src_access_mask = src_access_mask;
        self
    }

    #[inline]
    pub fn dst_mask(mut self, dst_stage_mask: vk::PipelineStageFlags2, dst_access_mask: vk::AccessFlags2) -> Self {
        self.inner.dst_stage_mask = dst_stage_mask;
        self.inner.dst_access_mask = dst_access_mask;
        self
    }

    #[inline]
    pub fn mask(mut self, mask: RhiBarrierMask) -> Self {
        self.inner.src_stage_mask = mask.src_stage;
        self.inner.dst_stage_mask = mask.dst_stage;
        self.inner.src_access_mask = mask.src_access;
        self.inner.dst_access_mask = mask.dst_access;
        self
    }

    #[inline]
    pub fn buffer(mut self, buffer: vk::Buffer, offset: vk::DeviceSize, size: vk::DeviceSize) -> Self {
        self.inner.buffer = buffer;
        self.inner.offset = offset;
        self.inner.size = size;
        self
    }
}
