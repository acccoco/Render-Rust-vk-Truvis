use std::{cell::RefCell, ffi::CStr};

use ash::vk;

use crate::{
    commands::{
        command_buffer::CommandBuffer,
        command_pool::CommandPool,
        command_queue::{CommandQueue, QueueFamily},
        submit_info::SubmitInfo,
    },
    foundation::{
        device::DeviceFunctions, instance::Instance, mem_allocator::MemAllocator, physical_device::PhysicalDevice,
    },
    resources_new::resource_manager::ResourceManager,
    vulkan_core::VulkanCore,
};

pub struct RenderContext {
    pub(crate) vk_core: VulkanCore,
    pub(crate) allocator: MemAllocator,

    /// 临时的 graphics command pool，主要用于临时的命令缓冲区
    pub(crate) temp_graphics_command_pool: CommandPool,
    pub(crate) resource_mgr: RefCell<ResourceManager>,
}

// 创建与销毁
impl RenderContext {
    // region init 相关
    const ENGINE_NAME: &'static str = "DruvisIII";

    fn new(app_name: String, instance_extra_exts: Vec<&'static CStr>) -> Self {
        let vk_ctx = VulkanCore::new(app_name, Self::ENGINE_NAME.to_string(), instance_extra_exts);

        // 注意：在初始化过程中，我们需要使用传统的参数传递方式
        // 因为 RenderContext 单例还没有被初始化
        let graphics_command_pool = CommandPool::new_internal(
            vk_ctx.device_functions.clone(),
            vk_ctx.physical_device.graphics_queue_family.clone(),
            vk::CommandPoolCreateFlags::empty(),
            "render_context-graphics",
        );

        let allocator = MemAllocator::new(
            &vk_ctx.instance.ash_instance,
            vk_ctx.physical_device.vk_handle,
            &vk_ctx.device_functions,
        );
        let resource_mgr = ResourceManager::new();

        Self {
            vk_core: vk_ctx,
            allocator,
            temp_graphics_command_pool: graphics_command_pool,
            resource_mgr: RefCell::new(resource_mgr),
        }
    }
}

// 注意：此静态变量仅用于单线程环境，符合项目要求
static mut RENDER_CONTEXT: Option<RenderContext> = None;

// 单例模式
// - RenderContext 自身的生命周期管理比较简单，因此适合使用单例模式
// - 让代码变得简单，不再需要考虑复杂的借用规则
// - 其他类的类型签名也会变得更简单
impl RenderContext {
    /// 获取单例实例
    ///
    /// # Panics
    /// 如果 RenderContext 还未初始化，此方法会 panic
    ///
    /// # Safety
    /// 此方法仅在单线程环境下安全
    #[inline]
    pub fn get() -> &'static RenderContext {
        unsafe {
            // 使用 addr_of! 避免直接对 static mut 创建引用，编译器不允许这种行为
            let ptr = std::ptr::addr_of!(RENDER_CONTEXT);
            (*ptr).as_ref().expect("RenderContext not initialized. Call RenderContext::init() first.")
        }
    }

    /// 初始化 RenderContext 单例
    ///
    /// # Parameters
    /// - `app_name`: 应用程序名称
    /// - `instance_extra_exts`: 额外的 Vulkan 实例扩展
    ///
    /// # Panics
    /// 如果 RenderContext 已经被初始化，此方法会 panic
    ///
    /// # Safety
    /// 此方法仅在单线程环境下安全
    pub fn init(app_name: String, instance_extra_exts: Vec<&'static CStr>) {
        unsafe {
            // 使用 addr_of_mut! 避免直接对 static mut 创建可变引用
            let ptr = std::ptr::addr_of_mut!(RENDER_CONTEXT);
            assert!((*ptr).is_none(), "RenderContext already initialized");
            *ptr = Some(Self::new(app_name, instance_extra_exts));
        }
    }

    /// 销毁 RenderContext 单例
    ///
    /// # Safety
    /// 调用此方法后，不应再使用 RenderContext::get()
    /// 此方法仅在单线程环境下安全
    pub fn destroy() {
        unsafe {
            // 使用 addr_of_mut! 避免直接对 static mut 创建可变引用
            let ptr = std::ptr::addr_of_mut!(RENDER_CONTEXT);
            let context = (*ptr).take().expect("RenderContext not initialized");

            // 注意：ResourceManager 可能不需要显式销毁
            // context.resource_mgr.into_inner().destroy();
            context.allocator.destroy();
            context.temp_graphics_command_pool.destroy_internal(&context.vk_core.device_functions);
            context.vk_core.destroy();
        }
    }
}

// getter
impl RenderContext {
    #[inline]
    pub fn vk_core(&self) -> &VulkanCore {
        &self.vk_core
    }

    #[inline]
    pub fn instance(&self) -> &Instance {
        &self.vk_core.instance
    }

    #[inline]
    pub fn device_functions(&self) -> &DeviceFunctions {
        &self.vk_core.device_functions
    }

    #[inline]
    pub fn allocator(&self) -> &MemAllocator {
        &self.allocator
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

// tools
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
        let command_buffer =
            CommandBuffer::new(&self.temp_graphics_command_pool, &format!("one-time-{}", name.as_ref()));

        command_buffer.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, name.as_ref());
        let result = func(&command_buffer);
        command_buffer.end();

        let command_buffer_clone = command_buffer.clone();
        self.graphics_queue().submit(vec![SubmitInfo::new(&[command_buffer_clone])], None);
        self.graphics_queue().wait_idle();
        unsafe {
            self.device_functions()
                .free_command_buffers(self.temp_graphics_command_pool.handle(), &[command_buffer.vk_handle()]);
        }

        result
    }
}
