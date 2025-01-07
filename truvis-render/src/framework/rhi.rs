use std::{
    ffi::CStr,
    sync::{Arc, OnceLock},
};

use anyhow::Context;
use ash::{extensions::khr::Swapchain, vk};
use raw_window_handle::HasRawDisplayHandle;

use crate::framework::{
    core::{
        command_pool::RhiCommandPool, debug::RhiDebugUtils, device::RhiDevice, instance::RhiInstance,
        physical_device::RhiPhysicalDevice,
    },
    platform::window_system::WindowSystem,
};

/// # Safety
/// very safe
pub unsafe extern "system" fn vk_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32
{
    let callback_data = *p_callback_data;

    let msg = if callback_data.p_message.is_null() {
        std::borrow::Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };


    // 按照 | 切分 msg 字符串，并在中间插入换行符
    let msg = msg.split('|').collect::<Vec<&str>>().join("\n");
    let msg = msg.split(" ] ").collect::<Vec<&str>>().join(" ]\n ");
    let format_msg = format!("[{:?}]\n {}\n", message_type, msg);

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            log::error!("{}", format_msg);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            log::warn!("{}", format_msg);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            log::info!("{}", format_msg);
        }
        _ => log::info!("{}", format_msg),
    };

    // 只有 layer developer 才需要返回 True
    vk::FALSE
}


pub static RHI: OnceLock<Rhi> = OnceLock::new();


/// Rhi 只需要做到能够创建各种资源的程度就行了
///
/// 与 VulkanSamples 的 VulkanSamle 及 ApiVulkanSample 作用类似
pub struct Rhi
{
    /// vk 基础函数的接口
    pub vk_pf: ash::Entry,
    instance: RhiInstance,
    // vk_instance: Option<Instance>,

    // vk_debug_util_pf: Option<ash::extensions::ext::DebugUtils>,
    pub vk_dynamic_render_pf: ash::extensions::khr::DynamicRendering,
    pub vk_acceleration_pf: ash::extensions::khr::AccelerationStructure,

    // vk_debug_util_messenger: Option<vk::DebugUtilsMessengerEXT>,
    physical_device: Arc<RhiPhysicalDevice>,
    pub device: RhiDevice,

    pub vma: vk_mem::Allocator,

    pub descriptor_pool: vk::DescriptorPool,

    pub graphics_command_pool: RhiCommandPool,
    pub transfer_command_pool: RhiCommandPool,
    pub compute_command_pool: RhiCommandPool,

    pub debug_utils: RhiDebugUtils,
}


mod _impl_init
{
    use std::{ffi::CStr, rc::Rc, sync::Arc};

    use ash::{extensions::khr::Swapchain, vk};
    use itertools::Itertools;
    use raw_window_handle::HasRawDisplayHandle;

    use crate::framework::{
        core::{
            command_pool::RhiCommandPool, debug::RhiDebugUtils, device::RhiDevice, instance::RhiInstance,
            physical_device::RhiPhysicalDevice,
        },
        platform::window_system::WindowSystem,
        rhi::Rhi,
    };

    pub struct RhiInitInfo
    {
        pub app_name: String,
        pub engine_name: String,

        pub enable_validation: bool,

        pub window: Arc<WindowSystem>,

        pub vk_version: u32,

        pub instance_layers: Vec<&'static CStr>,
        pub instance_extensions: Vec<&'static CStr>,
        pub instance_create_flags: vk::InstanceCreateFlags,
        pub device_extensions: Vec<&'static CStr>,

        pub core_features: vk::PhysicalDeviceFeatures,
        pub ext_features: Vec<Box<dyn vk::ExtendsPhysicalDeviceFeatures2>>,

        pub debug_msg_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        pub debug_msg_type: vk::DebugUtilsMessageTypeFlagsEXT,
        pub debug_callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT,

        pub frames_in_flight: u32,
    }


    impl RhiInitInfo
    {
        pub fn init_basic(app_name: String, window: Arc<WindowSystem>, enable_validation: bool) -> Self
        {
            let core_features = vk::PhysicalDeviceFeatures::builder()
                .sampler_anisotropy(true)
                .fragment_stores_and_atomics(true)
                .independent_blend(true)
                .build();

            let ext_features: Vec<Box<dyn vk::ExtendsPhysicalDeviceFeatures2>> = vec![
                Box::new(vk::PhysicalDeviceDynamicRenderingFeatures::builder().dynamic_rendering(true).build()),
                Box::new(vk::PhysicalDeviceBufferDeviceAddressFeatures::builder().buffer_device_address(true).build()),
                Box::new(vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder().ray_tracing_pipeline(true).build()),
                Box::new(
                    vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder().acceleration_structure(true).build(),
                ),
                Box::new(vk::PhysicalDeviceHostQueryResetFeatures::builder().host_query_reset(true).build()),
                Box::new(vk::PhysicalDeviceSynchronization2Features::builder().synchronization2(true).build()),
            ];

            let info = Self {
                app_name,
                engine_name: "DruvisIII".to_string(), // 槲寄生

                enable_validation,

                window: window.clone(),

                // 版本过低时，有些函数无法正确加载
                vk_version: vk::API_VERSION_1_3,

                instance_layers: Self::basic_instance_layers(enable_validation),
                instance_extensions: Self::basic_instance_extensions(window.as_ref(), enable_validation),
                instance_create_flags: vk::InstanceCreateFlags::empty(),
                device_extensions: Self::basic_device_extensions(enable_validation),

                core_features,
                ext_features,
                debug_msg_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                debug_msg_type: vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION |
                    vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,

                debug_callback: None,

                frames_in_flight: 3,
            };

            info
        }

        pub fn set_debug_callback(&mut self, callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT)
        {
            self.debug_callback = callback;
        }


        fn basic_device_extensions(enable_validation: bool) -> Vec<&'static CStr>
        {
            let mut exts = vec![Swapchain::name()];

            // dynamic rendering
            exts.append(&mut vec![
                cstr::cstr!("VK_KHR_depth_stencil_resolve"),
                ash::extensions::khr::CreateRenderPass2::name(),
                ash::extensions::khr::DynamicRendering::name(),
            ]);

            // RayTracing 相关的
            exts.append(&mut vec![
                ash::extensions::khr::AccelerationStructure::name(), // 主要的 ext
                cstr::cstr!("VK_EXT_descriptor_indexing"),
                cstr::cstr!("VK_KHR_buffer_device_address"),
                ash::extensions::khr::RayTracingPipeline::name(), // 主要的 ext
                ash::extensions::khr::DeferredHostOperations::name(),
                cstr::cstr!("VK_KHR_spirv_1_4"),
                cstr::cstr!("VK_KHR_shader_float_controls"),
            ]);

            exts
        }

        fn basic_instance_layers(enable_validation: bool) -> Vec<&'static CStr>
        {
            let mut layers = Vec::new();
            if enable_validation {
                layers.push(cstr::cstr!("VK_LAYER_KHRONOS_validation"));
            }
            layers
        }

        fn basic_instance_extensions(window: &WindowSystem, _enable_validation: bool) -> Vec<&'static CStr>
        {
            let mut exts = Vec::new();

            // 这个 extension 可以单独使用，提供以下功能：
            // 1. debug messenger
            // 2. 为 vulkan object 设置 debug name
            // 2. 使用 label 标记 queue 或者 command buffer 中的一个一个 section
            // 这个 extension 可以和 validation layer 配合使用，提供更详细的信息
            exts.push(ash::extensions::ext::DebugUtils::name());

            // 追加 window system 需要的 extension，在 windows 下也就是 khr::Surface
            for ext in ash_window::enumerate_required_extensions(window.window().raw_display_handle()).unwrap() {
                unsafe {
                    exts.push(CStr::from_ptr(*ext));
                }
            }

            // 这个 extension 是 VK_KHR_performance_query 的前置条件，而后者是用于 stats gathering 的
            exts.push(ash::extensions::khr::GetPhysicalDeviceProperties2::name());

            exts
        }

        pub fn get_debug_utils_messenger_ci(&self) -> vk::DebugUtilsMessengerCreateInfoEXT
        {
            vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(self.debug_msg_severity)
                .message_type(self.debug_msg_type)
                .pfn_user_callback(self.debug_callback)
                .build()
        }

        pub fn add_instance_extension(&mut self, exts: &[&'static CStr])
        {
            self.instance_extensions.extend_from_slice(exts);
        }

        pub fn add_instance_layers(&mut self, layers: &[&'static CStr])
        {
            self.instance_layers.extend_from_slice(layers);
        }

        pub fn add_device_extensions(&mut self, exts: &[&'static CStr])
        {
            self.device_extensions.extend_from_slice(exts);
        }
    }


    impl Rhi
    {
        const MAX_VERTEX_BLENDING_MESH_CNT: u32 = 256;
        const MAX_MATERIAL_CNT: u32 = 256;

        pub fn new(mut init_info: RhiInitInfo) -> Self
        {
            let vk_pf = unsafe { ash::Entry::load() }.expect("Failed to load vulkan entry");

            let instance = RhiInstance::new(&vk_pf, &init_info);

            let debug_utils = RhiDebugUtils::new(&vk_pf, &instance.handle, &init_info);

            let pdevice = Arc::new(Self::init_pdevice(&instance.handle));
            let device = RhiDevice::new(&mut init_info, &instance, pdevice.clone(), &debug_utils);

            // 在 device 以及 debug_utils 之前创建的 vk::Handle
            {
                debug_utils.set_debug_name(device.device.handle(), instance.handle.handle(), "instance");
                debug_utils.set_debug_name(device.device.handle(), pdevice.handle, "physical device");
            }

            let vk_dynamic_render_pf = ash::extensions::khr::DynamicRendering::new(&instance.handle, &device.device);
            let vk_acceleration_pf = ash::extensions::khr::AccelerationStructure::new(&instance.handle, &device.device);

            let vma = Self::init_vma(&instance, &device, &init_info);

            let descriptor_pool = Self::init_descriptor_pool(&device);
            debug_utils.set_debug_name(device.device.handle(), descriptor_pool, "main-descriptor-pool");

            let graphics_command_pool = Self::init_command_pool(
                &device,
                &debug_utils,
                vk::QueueFlags::GRAPHICS,
                vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                "rhi-graphics-command-pool",
            );
            let compute_command_pool = Self::init_command_pool(
                &device,
                &debug_utils,
                vk::QueueFlags::COMPUTE,
                vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                "rhi-compute-command-pool",
            );
            let transfer_command_pool = Self::init_command_pool(
                &device,
                &debug_utils,
                vk::QueueFlags::TRANSFER,
                vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                "rhi-transfer-command-pool",
            );


            let rhi = Self {
                vk_pf,
                instance,
                physical_device: pdevice,
                device,
                vk_dynamic_render_pf,
                vk_acceleration_pf,
                vma,
                descriptor_pool,
                graphics_command_pool,
                transfer_command_pool,
                compute_command_pool,
                debug_utils,
            };

            rhi
        }

        fn init_descriptor_pool(device: &RhiDevice) -> vk::DescriptorPool
        {
            let pool_size = [
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
                    descriptor_count: 128,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: Self::MAX_VERTEX_BLENDING_MESH_CNT + 32,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: Self::MAX_MATERIAL_CNT + 32,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: Self::MAX_MATERIAL_CNT + 32,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::INPUT_ATTACHMENT,
                    descriptor_count: 32,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
                    descriptor_count: 32,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_IMAGE,
                    descriptor_count: 32,
                },
            ];

            let pool_create_info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&pool_size)
                .max_sets(Self::MAX_MATERIAL_CNT + Self::MAX_VERTEX_BLENDING_MESH_CNT + 32);

            unsafe {
                let descriptor_pool = device.device.create_descriptor_pool(&pool_create_info, None).unwrap();
                descriptor_pool
            }
        }

        fn init_pdevice(instance: &ash::Instance) -> RhiPhysicalDevice
        {
            let pdevice = unsafe {
                instance
                    .enumerate_physical_devices()
                    .unwrap()
                    .iter()
                    .map(|pdevice| RhiPhysicalDevice::new(*pdevice, instance))
                    // 优先使用独立显卡
                    .find_or_first(RhiPhysicalDevice::is_descrete_gpu)
                    .unwrap()
            };

            pdevice
        }

        fn init_vma(instance: &RhiInstance, device: &RhiDevice, init_info: &RhiInitInfo) -> vk_mem::Allocator
        {
            let vma_create_info = vk_mem::AllocatorCreateInfo::new(
                Rc::new(&instance.handle),
                Rc::new(&device.device),
                device.pdevice.handle,
            )
            .vulkan_api_version(init_info.vk_version)
            .flags(vk_mem::AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS);

            let vma = vk_mem::Allocator::new(vma_create_info).unwrap();
            vma
        }

        /// 仅在初始化阶段使用的一个函数
        pub(super) fn init_command_pool<S: AsRef<str> + Clone>(
            device: &RhiDevice,
            debug_utils: &RhiDebugUtils,
            queue_flags: vk::QueueFlags,
            flags: vk::CommandPoolCreateFlags,
            debug_name: S,
        ) -> RhiCommandPool
        {
            let queue_family_index = device.pdevice.find_queue_family_index(queue_flags).unwrap();

            let pool = unsafe {
                device
                    .device
                    .create_command_pool(
                        &vk::CommandPoolCreateInfo::builder().queue_family_index(queue_family_index).flags(flags),
                        None,
                    )
                    .unwrap()
            };

            debug_utils.set_debug_name(device.device.handle(), pool, debug_name.clone());
            RhiCommandPool {
                command_pool: pool,
                queue_family_index,
            }
        }
    }
}

pub use _impl_init::RhiInitInfo;


mod _impl_property
{
    use crate::framework::{
        core::{physical_device::RhiPhysicalDevice, queue::RhiQueue},
        rhi::Rhi,
    };

    impl Rhi
    {
        #[inline]
        pub(crate) fn vk_instance(&self) -> &ash::Instance
        {
            &self.instance.handle
        }

        #[inline]
        pub(crate) fn vk_device(&self) -> &ash::Device
        {
            &self.device.device
        }

        #[inline]
        pub(crate) fn physical_device(&self) -> &RhiPhysicalDevice
        {
            &self.physical_device
        }

        #[inline]
        pub fn compute_queue(&self) -> &RhiQueue
        {
            &self.device.compute_queue
        }

        #[inline]
        pub fn graphics_queue(&self) -> &RhiQueue
        {
            &self.device.graphics_queue
        }

        #[inline]
        pub fn transfer_queue(&self) -> &RhiQueue
        {
            &self.device.transfer_queue
        }
    }
}

// 工具方法
mod _impl_tools
{
    use ash::vk;
    use itertools::Itertools;
    use vk_mem::Alloc;

    use crate::framework::{
        core::{
            command_pool::RhiCommandPool,
            queue::{RhiQueue, RhiSubmitBatch},
            synchronize::RhiFence,
        },
        rhi::Rhi,
    };

    impl Rhi
    {
        #[inline]
        pub(crate) fn set_debug_name<T, S>(&self, handle: T, name: S)
        where
            T: vk::Handle + Copy,
            S: AsRef<str>,
        {
            self.debug_utils.set_debug_name(self.vk_device().handle(), handle, name);
        }

        #[inline]
        pub fn set_debug_label(&self)
        {
            todo!()
            // self.debug_util_pf.unwrap().cmd_begin_debug_utils_label()
        }

        pub fn create_image<S>(
            &self,
            create_info: &vk::ImageCreateInfo,
            debug_name: S,
        ) -> (vk::Image, vk_mem::Allocation)
        where
            S: AsRef<str>,
        {
            let alloc_info = vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            };
            let (image, allocation) = unsafe { self.vma.create_image(create_info, &alloc_info).unwrap() };

            self.set_debug_name(image, debug_name);
            (image, allocation)
        }

        #[inline]
        pub fn create_image_view<S>(&self, create_info: &vk::ImageViewCreateInfo, debug_name: S) -> vk::ImageView
        where
            S: AsRef<str>,
        {
            let view = unsafe { self.vk_device().create_image_view(create_info, None).unwrap() };

            self.set_debug_name(view, debug_name);
            view
        }


        pub(crate) fn find_supported_format(
            &self,
            candidates: &[vk::Format],
            tiling: vk::ImageTiling,
            features: vk::FormatFeatureFlags,
        ) -> Vec<vk::Format>
        {
            candidates
                .iter()
                .filter(|f| {
                    let props = unsafe {
                        self.vk_instance().get_physical_device_format_properties(self.physical_device().handle, **f)
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

        pub fn reset_command_pool(&self, command_pool: &mut RhiCommandPool)
        {
            unsafe {
                self.vk_device()
                    .reset_command_pool(command_pool.command_pool, vk::CommandPoolResetFlags::RELEASE_RESOURCES)
                    .unwrap();
            }
        }

        pub fn wait_for_fence(&self, fence: &RhiFence)
        {
            unsafe {
                self.vk_device().wait_for_fences(std::slice::from_ref(&fence.fence), true, u64::MAX).unwrap();
            }
        }

        pub fn reset_fence(&self, fence: &RhiFence)
        {
            unsafe {
                self.vk_device().reset_fences(std::slice::from_ref(&fence.fence)).unwrap();
            }
        }

        pub fn queue_submit(&self, queue: &RhiQueue, batches: Vec<RhiSubmitBatch>, fence: Option<RhiFence>)
        {
            unsafe {
                // batches 的存在是有必要的，submit_infos 引用的 batches 的内存
                let batches = batches.iter().map(|b| b.to_vk_batch()).collect_vec();
                let submit_infos = batches.iter().map(|b| b.submit_info()).collect_vec();

                self.vk_device()
                    .queue_submit(queue.queue, &submit_infos, fence.map_or(vk::Fence::null(), |f| f.fence))
                    .unwrap();
            }
        }

        pub fn create_render_pass(&self, render_pass_ci: &vk::RenderPassCreateInfo, debug_name: &str)
            -> vk::RenderPass
        {
            let render_pass = unsafe { self.vk_device().create_render_pass(render_pass_ci, None).unwrap() };
            self.set_debug_name(render_pass, debug_name);
            render_pass
        }

        pub fn create_pipeline_cache(
            &self,
            pipeline_cache_ci: &vk::PipelineCacheCreateInfo,
            debug_name: &str,
        ) -> vk::PipelineCache
        {
            let pipeline_cache = unsafe { self.vk_device().create_pipeline_cache(pipeline_cache_ci, None).unwrap() };
            self.set_debug_name(pipeline_cache, debug_name);
            pipeline_cache
        }

        pub fn get_depth_format(&self) -> vk::Format
        {
            let depth_formats = vec![
                vk::Format::D32_SFLOAT_S8_UINT,
                vk::Format::D32_SFLOAT,
                vk::Format::D32_SFLOAT_S8_UINT,
                vk::Format::D16_UNORM_S8_UINT,
                vk::Format::D16_UNORM,
            ];

            let depth_format = self.find_supported_format(
                &depth_formats,
                vk::ImageTiling::OPTIMAL,
                vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
            );

            depth_format.first().copied().unwrap()
        }

        pub fn create_frame_buffer(
            &self,
            frame_buffer_ci: &vk::FramebufferCreateInfo,
            debug_name: &str,
        ) -> vk::Framebuffer
        {
            let frame_buffer = unsafe { self.vk_device().create_framebuffer(frame_buffer_ci, None).unwrap() };
            self.set_debug_name(frame_buffer, debug_name);
            frame_buffer
        }

        pub fn create_command_pool<S: AsRef<str> + Clone>(
            &self,
            queue_flags: vk::QueueFlags,
            flags: vk::CommandPoolCreateFlags,
            debug_name: S,
        ) -> RhiCommandPool
        {
            Self::init_command_pool(&self.device, &self.debug_utils, queue_flags, flags, debug_name)
        }
    }
}
