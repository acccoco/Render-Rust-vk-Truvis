use std::{cell::RefCell, ffi::CStr, rc::Rc};

use ash::vk;

use crate::{
    commands::{
        command_buffer::CommandBuffer,
        command_pool::CommandPool,
        command_queue::{CommandQueue, QueueFamily},
        submit_info::SubmitInfo,
    },
    foundation::{
        device::{Device, DeviceFunctions},
        instance::Instance,
        mem_allocator::MemAllocator,
        physical_device::PhysicalDevice,
    },
    resources_new::resource_manager::ResourceManager,
    vulkan_core::VulkanCore,
};

pub struct RenderContext {
    pub(crate) vk_core: VulkanCore,
    pub(crate) allocator: Rc<MemAllocator>,

    /// 临时的 graphics command pool，主要用于临时的命令缓冲区
    pub(crate) temp_graphics_command_pool: CommandPool,
    pub(crate) resource_mgr: RefCell<ResourceManager>,
}

/// 创建与销毁
impl RenderContext {
    // region init 相关
    const ENGINE_NAME: &'static str = "DruvisIII";

    pub fn new(app_name: String, instance_extra_exts: Vec<&'static CStr>) -> Self {
        let vk_ctx = VulkanCore::new(app_name, Self::ENGINE_NAME.to_string(), instance_extra_exts);
        let graphics_command_pool = CommandPool::new(
            vk_ctx.device.functions.clone(),
            vk_ctx.physical_device.graphics_queue_family.clone(),
            vk::CommandPoolCreateFlags::empty(),
            "rhi-graphics",
        );

        let allocator = MemAllocator::new(
            &vk_ctx.instance.ash_instance,
            vk_ctx.physical_device.vk_handle,
            &vk_ctx.device.functions,
        );
        let resource_mgr = ResourceManager::new();

        Self {
            vk_core: vk_ctx,
            allocator: Rc::new(allocator),
            temp_graphics_command_pool: graphics_command_pool,
            resource_mgr: RefCell::new(resource_mgr),
        }
    }

    pub fn desotry(mut self) {
        self.resource_mgr.get_mut().desotry();
        self.allocator.destroy();
        self.temp_graphics_command_pool.destroy();
        self.vk_core.destroy();
    }
}

/// getter
impl RenderContext {
    #[inline]
    pub fn instance(&self) -> &Instance {
        &self.vk_core.instance
    }

    #[inline]
    pub fn device(&self) -> &Device {
        &self.vk_core.device
    }

    #[inline]
    pub fn device_functions(&self) -> Rc<DeviceFunctions> {
        self.vk_core.device.functions.clone()
    }

    #[inline]
    pub fn allocator(&self) -> Rc<MemAllocator> {
        self.allocator.clone()
    }

    #[inline]
    pub fn physical_device(&self) -> &PhysicalDevice {
        &self.vk_core.physical_device
    }

    #[inline]
    pub fn graphics_queue_family(&self) -> QueueFamily {
        self.vk_core.physical_device.graphics_queue_family.clone()
    }

    #[inline]
    pub fn compute_queue_family(&self) -> QueueFamily {
        self.vk_core.physical_device.compute_queue_family.clone()
    }

    #[inline]
    pub fn transfer_queue_family(&self) -> QueueFamily {
        self.vk_core.physical_device.transfer_queue_family.clone()
    }

    #[inline]
    pub fn graphics_queue(&self) -> &CommandQueue {
        &self.vk_core.graphics_queue
    }

    #[inline]
    pub fn compute_queue(&self) -> &CommandQueue {
        &self.vk_core.compute_queue
    }

    #[inline]
    pub fn transfer_queue(&self) -> &CommandQueue {
        &self.vk_core.transfer_queue
    }

    /// 当 uniform buffer 的 descriptor 在更新时，其 offset 必须是这个值的整数倍
    ///
    /// 注：这个值一定是 power of 2
    #[inline]
    pub fn min_ubo_offset_align(&self) -> vk::DeviceSize {
        self.vk_core.physical_device.basic_props.limits.min_uniform_buffer_offset_alignment
    }

    #[inline]
    pub fn rt_pipeline_props(&self) -> &vk::PhysicalDeviceRayTracingPipelinePropertiesKHR<'_> {
        &self.vk_core.physical_device.rt_pipeline_props
    }
}

/// tools
impl RenderContext {
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
                    self.instance()
                        .ash_instance
                        .get_physical_device_format_properties(self.physical_device().vk_handle, **f)
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

    /// 立即执行某个 command，并同步等待执行结果
    pub fn one_time_exec<F, R>(&self, func: F, name: impl AsRef<str>) -> R
    where
        F: FnOnce(&CommandBuffer) -> R,
    {
        let command_buffer = CommandBuffer::new(
            self.device_functions(),
            &self.temp_graphics_command_pool,
            &format!("one-time-{}", name.as_ref()),
        );

        command_buffer.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, name.as_ref());
        let result = func(&command_buffer);
        command_buffer.end();

        let command_buffer_clone = command_buffer.clone();
        self.graphics_queue().submit(vec![SubmitInfo::new(&[command_buffer_clone])], None);
        self.graphics_queue().wait_idle();
        command_buffer.free();

        result
    }
}
