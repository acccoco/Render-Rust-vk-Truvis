use std::ffi::CStr;

use anyhow::Context;
use ash::{extensions::khr::Swapchain, vk};
use raw_window_handle::HasRawDisplayHandle;

use crate::window_system::WindowSystem;


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

pub struct RhiInitInfo
{
    pub app_name: Option<String>,
    pub engine_name: Option<String>,

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
    const VALIDATION_LAYER_NAME: &'static CStr = cstr::cstr!("VK_LAYER_KHRONOS_validation");

    pub fn init_basic(debug_callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT) -> Self
    {
        let instance_create_flags = if cfg!(target_os = "macos") {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::empty()
        };

        let mut info = Self {
            app_name: None,
            engine_name: None,

            // 版本过低时，有些函数无法正确加载
            vk_version: vk::API_VERSION_1_3,

            instance_layers: Self::basic_instance_layers(),
            instance_extensions: Self::basic_instance_extensions(),
            instance_create_flags,
            device_extensions: Self::basic_device_extensions(),

            core_features: Default::default(),
            ext_features: vec![],
            debug_msg_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            debug_msg_type: vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION |
                vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            debug_callback,

            frames_in_flight: 3,
        };
        info.set_device_features();

        info
    }

    pub fn is_complete(&self) -> anyhow::Result<()>
    {
        self.app_name.as_ref().context("")?;
        self.engine_name.as_ref().context("")?;
        Ok(())
    }


    fn basic_device_extensions() -> Vec<&'static CStr>
    {
        let mut exts = vec![Swapchain::name()];

        if cfg!(target_os = "macos") {
            // 在 metal 上模拟出 vulkan
            exts.push(vk::KhrPortabilitySubsetFn::name());
        }

        // dynamic rendering
        exts.append(&mut vec![
            cstr::cstr!("VK_KHR_depth_stencil_resolve"),
            // cstr::cstr!("VK_KHR_multiview"),     // 于 vk-1.1 加入到 core
            // cstr::cstr!("VK_KHR_maintenance2"),  // 于 vk-1.1 加入到 core
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

    fn basic_instance_layers() -> Vec<&'static CStr>
    {
        vec![Self::VALIDATION_LAYER_NAME]
    }

    fn basic_instance_extensions() -> Vec<&'static CStr>
    {
        let mut exts = Vec::new();

        // 这个 extension 可以单独使用，提供以下功能：
        // 1. debug messenger
        // 2. 为 vulkan object 设置 debug name
        // 2. 使用 label 标记 queue 或者 command buffer 中的一个一个 section
        // 这个 extension 可以和 validation layer 配合使用，提供更详细的信息
        exts.push(ash::extensions::ext::DebugUtils::name());

        // 追加 window system 需要的 extension
        for ext in
            ash_window::enumerate_required_extensions(WindowSystem::instance().window().raw_display_handle()).unwrap()
        {
            unsafe {
                exts.push(CStr::from_ptr(*ext));
            }
        }

        if cfg!(target_os = "macos") {
            // 这个扩展能够在枚举 pdevice 时，将不受支持的 pdevice 也列举出来
            // 不受支持的 pdevice 可以通过模拟层运行 vulkan
            exts.push(vk::KhrPortabilityEnumerationFn::name());

            // device extension VK_KHR_portability_subset 需要这个扩展
            exts.push(vk::KhrGetPhysicalDeviceProperties2Fn::name());
        }
        exts
    }

    fn set_device_features(&mut self)
    {
        self.core_features = vk::PhysicalDeviceFeatures::builder()
            .sampler_anisotropy(true)
            .fragment_stores_and_atomics(true)
            .independent_blend(true)
            .build();

        self.ext_features = vec![
            Box::new(vk::PhysicalDeviceDynamicRenderingFeatures::builder().dynamic_rendering(true).build()),
            Box::new(vk::PhysicalDeviceBufferDeviceAddressFeatures::builder().buffer_device_address(true).build()),
            Box::new(vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder().ray_tracing_pipeline(true).build()),
            Box::new(
                vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder().acceleration_structure(true).build(),
            ),
            Box::new(vk::PhysicalDeviceHostQueryResetFeatures::builder().host_query_reset(true).build()),
            Box::new(vk::PhysicalDeviceSynchronization2Features::builder().synchronization2(true).build()),
        ];
    }
}
