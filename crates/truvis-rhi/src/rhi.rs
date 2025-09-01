use crate::core::debug_utils::RhiDebugUtils;
use crate::{
    core::{
        mem_allocator::RhiMemAllocator, command_pool::RhiCommandPool, command_queue::RhiQueueFamily, device::RhiDevice,
        instance::RhiInstance, physical_device::RhiPhysicalDevice,
    },
    resources::resource_manager::RhiResourceManager,
    vulkan_context::VulkanContext,
};
use ash::vk;
use std::{cell::RefCell, ffi::CStr};

pub struct Rhi {
    vk_ctx: VulkanContext,
    allocator: RhiMemAllocator,
    /// 临时的 graphics command pool，主要用于临时的命令缓冲区
    temp_graphics_command_pool: RhiCommandPool,
    resource_mgr: RefCell<RhiResourceManager>,
}

/// 创建与销毁
impl Rhi {
    // region init 相关
    const ENGINE_NAME: &'static str = "DruvisIII";

    pub fn new(app_name: String, instance_extra_exts: Vec<&'static CStr>) -> Self {
        let vk_ctx = VulkanContext::new(app_name, Self::ENGINE_NAME.to_string(), instance_extra_exts);
        let graphics_command_pool = RhiCommandPool::new(
            &vk_ctx.device,
            &vk_ctx.debug_utils,
            vk_ctx.physical_device.graphics_queue_family.clone(),
            vk::CommandPoolCreateFlags::empty(),
            "rhi-graphics",
        );

        let allocator = RhiMemAllocator::new(
            &vk_ctx.instance.ash_instance,
            vk_ctx.physical_device.vk_handle,
            &vk_ctx.device.ash_device,
        );
        let resource_mgr = RhiResourceManager::new();

        Self {
            vk_ctx,
            allocator,
            temp_graphics_command_pool: graphics_command_pool,
            resource_mgr: RefCell::new(resource_mgr),
        }
    }

    pub fn desotry(mut self) {
        self.resource_mgr.get_mut().desotry();
        self.allocator.destroy();
        self.temp_graphics_command_pool.destroy(&self.vk_ctx.device);
        self.vk_ctx.destroy();
    }
}

/// getter
impl Rhi {
    #[inline]
    pub fn instance(&self) -> &RhiInstance {
        &self.vk_ctx.instance
    }

    #[inline]
    pub fn device(&self) -> &RhiDevice {
        &self.vk_ctx.device
    }

    #[inline]
    pub fn debug_utils(&self) -> &RhiDebugUtils {
        &self.vk_ctx.debug_utils
    }

    #[inline]
    pub fn physical_device(&self) -> &RhiPhysicalDevice {
        &self.vk_ctx.physical_device
    }

    #[inline]
    pub fn graphics_queue_family(&self) -> RhiQueueFamily {
        self.vk_ctx.physical_device.graphics_queue_family.clone()
    }

    #[inline]
    pub fn compute_queue_family(&self) -> RhiQueueFamily {
        self.vk_ctx.physical_device.compute_queue_family.clone()
    }

    #[inline]
    pub fn transfer_queue_family(&self) -> RhiQueueFamily {
        self.vk_ctx.physical_device.transfer_queue_family.clone()
    }

    /// 当 uniform buffer 的 descriptor 在更新时，其 offset 必须是这个值的整数倍
    ///
    /// 注：这个值一定是 power of 2
    #[inline]
    pub fn min_ubo_offset_align(&self) -> vk::DeviceSize {
        self.vk_ctx.physical_device.basic_props.limits.min_uniform_buffer_offset_alignment
    }

    #[inline]
    pub fn rt_pipeline_props(&self) -> &vk::PhysicalDeviceRayTracingPipelinePropertiesKHR<'_> {
        &self.vk_ctx.physical_device.rt_pipeline_props
    }
}

/// tools
impl Rhi {
    /// 根据给定的格式，返回支持的格式
    pub fn find_supported_format(
        &self,
        candidates: &[vk::Format],
        tiling: vk::ImageTiling,
        features: vk::FormatFeatureFlags,
    ) -> Vec<vk::Format> {
        candidates
            .iter()
            .filter(|f| {
                let props = unsafe {
                    self.instance().get_physical_device_format_properties(self.physical_device().vk_handle, **f)
                };
                match tiling {
                    vk::ImageTiling::LINEAR => props.linear_tiling_features.contains(features),
                    vk::ImageTiling::OPTIMAL => props.optimal_tiling_features.contains(features),
                    _ => panic!("not supported tiling."),
                }
            })
            .copied()
            .collect()
    }
}
