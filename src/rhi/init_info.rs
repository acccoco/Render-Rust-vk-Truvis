use std::ffi::CStr;

use ash::{extensions::khr::Swapchain, vk};
use raw_window_handle::HasRawDisplayHandle;

use crate::window_system::WindowSystem;

pub struct RhiInitInfo
{
    pub app_name: Option<String>,
    pub engine_name: Option<String>,

    pub vk_version: u32,

    pub instance_layers: Vec<&'static CStr>,
    pub instance_extensions: Vec<&'static CStr>,
    pub device_extensions: Vec<&'static CStr>,

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
        Self {
            app_name: None,
            engine_name: None,

            vk_version: vk::API_VERSION_1_1,

            instance_layers: Self::basic_instance_layers(),
            instance_extensions: Self::basic_instance_extensions(),
            device_extensions: Self::basic_device_extensions(),

            debug_msg_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            debug_msg_type: vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION |
                vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            debug_callback,

            frames_in_flight: 3,
        }
    }

    pub fn is_complete(&self) -> Option<()>
    {
        self.app_name.as_ref()?;
        self.engine_name.as_ref()?;
        Some(())
    }


    fn basic_device_extensions() -> Vec<&'static CStr>
    {
        vec![
            Swapchain::name(),
            #[cfg(target_os = "macos")]
            vk::KhrPortabilitySubsetFn::name(), // 这个扩展可以在 metal 上模拟出 vulkan
            // dynamic rendering 所需的 extensions
            cstr::cstr!("VK_KHR_depth_stencil_resolve"),
            // cstr::cstr!("VK_KHR_multiview"),     // 于 vk-1.1 加入到 core
            // cstr::cstr!("VK_KHR_maintenance2"),  // 于 vk-1.1 加入到 core
            ash::extensions::khr::CreateRenderPass2::name(),
            ash::extensions::khr::DynamicRendering::name(),
        ]
    }

    fn basic_instance_layers() -> Vec<&'static CStr> { vec![Self::VALIDATION_LAYER_NAME] }

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

        #[cfg(target_os = "macos")]
        {
            // 这个扩展能够在枚举 pdevice 时，将不受支持的 pdevice 也列举出来
            // 不受支持的 pdevice 可以通过模拟层运行 vulkan
            exts.push(vk::KhrPortabilityEnumerationFn::name());

            // device extension VK_KHR_portability_subset 需要这个扩展
            exts.push(vk::KhrGetPhysicalDeviceProperties2Fn::name());
        }
        exts
    }
}