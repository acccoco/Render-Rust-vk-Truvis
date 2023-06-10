use std::ffi::CStr;

use ash::{extensions::khr::Swapchain, vk};
use raw_window_handle::HasRawDisplayHandle;

use crate::window_system::WindowSystem;

pub struct RhiInitInfo<'a>
{
    pub app_name: Option<&'static CStr>,
    pub engine_name: Option<&'static CStr>,

    pub vk_version: u32,

    pub instance_layers: Vec<&'static CStr>,
    pub instance_extensions: Vec<&'static CStr>,
    pub device_extensions: Vec<&'static CStr>,

    pub debug_msg_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    pub debug_msg_type: vk::DebugUtilsMessageTypeFlagsEXT,
    pub debug_callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT,

    // 该值由 Rhi 写入
    pub(crate) window_system: Option<&'a WindowSystem>,

    pub swapchain_format: vk::Format,
    pub swapchain_color_space: vk::ColorSpaceKHR,
    pub swapchain_present_mode: vk::PresentModeKHR,

    pub frames_in_flight: u32,

    pub depth_format_dedicate: Vec<vk::Format>,
}


impl<'a> RhiInitInfo<'a>
{
    const VALIDATION_LAYER_NAME: &'static CStr = cstr::cstr!("VK_LAYER_KHRONOS_validation");

    pub fn init_basic(
        window_system: Option<&'a WindowSystem>,
        debug_callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT,
    ) -> Self
    {
        Self {
            app_name: None,
            engine_name: None,

            vk_version: vk::API_VERSION_1_1,

            instance_layers: Self::basic_instance_layers(),
            instance_extensions: Self::basic_instance_extensions(window_system),
            device_extensions: Self::basic_device_extensions(),

            debug_msg_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            debug_msg_type: vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION |
                vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            debug_callback,

            window_system,

            // 以下字段表示 present engine 应该如何处理线性颜色值。shader 还有 image 都不用关心这两个字段
            swapchain_format: vk::Format::B8G8R8A8_UNORM,
            swapchain_color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            swapchain_present_mode: vk::PresentModeKHR::MAILBOX,

            frames_in_flight: 3,

            depth_format_dedicate: vec![
                vk::Format::D32_SFLOAT,
                vk::Format::D32_SFLOAT_S8_UINT,
                vk::Format::D24_UNORM_S8_UINT,
            ],
        }
    }

    pub fn is_complete(&self) -> bool
    {
        (|| {
            self.app_name?;
            self.engine_name?;
            Some(())
        })()
        .is_some()
    }


    fn basic_device_extensions() -> Vec<&'static CStr>
    {
        vec![
            Swapchain::name(),
            #[cfg(target_os = "macos")]
            vk::KhrPortabilitySubsetFn::name(), // 这个扩展可以在 metal 上模拟出 vulkan
            // dynamic rendering 所需的 extensions
            cstr::cstr!("VK_KHR_depth_stencil_resolve"),
            cstr::cstr!("VK_KHR_multiview"),
            cstr::cstr!("VK_KHR_maintenance2"),
            ash::extensions::khr::CreateRenderPass2::name(),
            ash::extensions::khr::DynamicRendering::name(),
        ]
    }

    fn basic_instance_layers() -> Vec<&'static CStr> { vec![Self::VALIDATION_LAYER_NAME] }

    fn basic_instance_extensions(window_system: Option<&WindowSystem>) -> Vec<&'static CStr>
    {
        let mut exts = Vec::new();

        // 这个 extension 可以单独使用，提供以下功能：
        // 1. debug messenger
        // 2. 为 vulkan object 设置 debug name
        // 2. 使用 label 标记 queue 或者 command buffer 中的一个一个 section
        // 这个 extension 可以和 validation layer 配合使用，提供更详细的信息
        exts.push(ash::extensions::ext::DebugUtils::name());

        // 追加 window system 需要的 extension
        if let Some(window) = window_system {
            for ext in ash_window::enumerate_required_extensions(window.window().raw_display_handle()).unwrap() {
                unsafe {
                    exts.push(CStr::from_ptr(*ext));
                }
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
