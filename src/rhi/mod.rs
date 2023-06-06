use std::ffi::{c_char, CStr};

use ash::{vk, Device, Entry, Instance};
use derive_setters::Setters;
use itertools::Itertools;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::event::StartCause::Init;
use crate::window_system::WindowSystem;

const VALIDATION_LAYER_NAME: &CStr = cstr::cstr!("VK_LAYER_KHRONOS_validation");

struct RhiPhysicalDevice
{
    vk_physical_device: vk::PhysicalDevice,
    pd_props: vk::PhysicalDeviceProperties,
    pd_mem_props: vk::PhysicalDeviceMemoryProperties,
    pd_rt_pipeline_props: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
}


#[derive(Default)]
pub struct Rhi<'a>
{
    entry: Option<Entry>,

    /// 这个字段是可空的
    window_system: Option<&'a WindowSystem>,

    instance: Option<Instance>,

    debug_util_loader: Option<ash::extensions::ext::DebugUtils>,
    debug_util_messenger: Option<vk::DebugUtilsMessengerEXT>,

    surface_loader: Option<ash::extensions::khr::Surface>,
    /// 这个字段是可空的
    surface: Option<vk::SurfaceKHR>,

    physical_device: Option<RhiPhysicalDevice>,
    device: Option<Device>,

    swapchain_loader: Option<ash::extensions::khr::Swapchain>,
}

pub struct RhiInitInfo<'a>
{
    app_name: &'static CStr,
    engine_name: &'static CStr,

    vk_version: u32,

    instance_layers: Vec<&'static CStr>,
    instance_extensions: Vec<&'static CStr>,

    debug_msg_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    debug_msg_type: vk::DebugUtilsMessageTypeFlagsEXT,
    debug_callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT,

    window_system: Option<&'a WindowSystem>,
}

impl<'a> Default for RhiInitInfo<'a>
{
    fn default() -> Self
    {
        Self {
            vk_version: vk::API_VERSION_1_1,
            instance_layers: vec![cstr::cstr!("VK_LAYER_KHRONOS_validation")],
            ..todo!()
        }
    }
}

impl<'a> Rhi<'a>
{
    pub fn init(init_info: &RhiInitInfo) -> Self
    {
        let mut rhi = Self {
            entry: unsafe { Some(Entry::load().unwrap()) },
            ..Default::default()
        };

        rhi.init_instance(init_info);
        rhi.init_debug_messenger(init_info);
        rhi.init_surface(init_info);
        rhi.init_pdevice();

        rhi
    }


    fn init_instance(&mut self, init_info: &RhiInitInfo)
    {
        fn get_instance_extensions(init_info: &RhiInitInfo) -> Vec<*const c_char> {
            let mut exts = init_info.instance_extensions.iter().map(|ext| ext.as_ptr()).collect_vec();

            // 这个 extension 可以单独使用，提供以下功能：
            // 1. debug messenger
            // 2. 为 vulkan object 设置 debug name
            // 2. 使用 label 标记 queue 或者 command buffer 中的一个一个 section
            // 这个 extension 可以和 validation layer 配合使用，提供更详细的信息
            exts.push(ash::extensions::ext::DebugUtils::name().as_ptr());

            // 追加 window system 需要的 extension
            if let Some(window) = init_info.window_system {
                for ext in ash_window::enumerate_required_extensions(window.window().raw_display_handle()).unwrap() {
                    exts.push(*ext);
                }
            }

            #[cfg(target_os = "macos")]
            {
                // 这个扩展能够在枚举 pdevice 时，将不受支持的 pdevice 也列举出来
                // 不受支持的 pdevice 可以通过模拟层运行 vulkan
                exts.push(vk::KhrPortabilityEnumerationFn::name().as_ptr());

                // device extension VK_KHR_portability_subset 需要这个扩展
                exts.push(vk::KhrGetPhysicalDeviceProperties2Fn::name().as_ptr());
            }
            exts
        }
        fn get_instance_layers(init_info: &RhiInitInfo) -> Vec<*const c_char> {
            let mut layers = init_info.instance_layers.iter().map(|layer| layer.as_ptr()).collect_vec();
            layers.push(VALIDATION_LAYER_NAME.as_ptr());
            layers
        }

        let app_info = vk::ApplicationInfo::builder()
            .application_name(init_info.app_name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(init_info.engine_name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(init_info.vk_version);

        let instance_extensions = get_instance_extensions(init_info);
        let instance_layers = get_instance_layers(init_info);

        let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(init_info.debug_msg_severity)
            .message_type(init_info.debug_msg_type)
            .pfn_user_callback(init_info.debug_callback)
            .build();

        let create_flags = if cfg!(target_os = "macos") {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            Default::default()
        };

        let instance_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&instance_extensions)
            .enabled_layer_names(&instance_layers)
            .flags(create_flags)
            .push_next(&mut debug_info);

        let instance = unsafe { self.entry.as_ref().unwrap().create_instance(&instance_info, None).unwrap() };
        self.instance = Some(instance);
    }

    fn init_debug_messenger(&mut self, init_info: &RhiInitInfo) {
        let loader = ash::extensions::ext::DebugUtils::new(self.entry.as_ref().unwrap(), self.instance.as_ref().unwrap());

        let create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(init_info.debug_msg_severity)
            .message_type(init_info.debug_msg_type)
            .pfn_user_callback(init_info.debug_callback)
            .build();
        let debug_messenger = unsafe { loader.create_debug_utils_messenger(&create_info, None).unwrap() };

        self.debug_util_loader = Some(loader);
        self.debug_util_messenger = Some(debug_messenger);
    }

    fn init_surface(&mut self, init_info: &RhiInitInfo) {
        let surface_loader = ash::extensions::khr::Surface::new(self.entry.as_ref().unwrap(), self.instance.as_ref().unwrap());

        let surface = init_info.window_system.map(|window_system| unsafe {
            ash_window::create_surface(
                self.entry.as_ref().unwrap(),
                self.instance.as_ref().unwrap(),
                window_system.window().raw_display_handle(),
                window_system.window().raw_window_handle(),
                None,
            )
                .unwrap()
        });

        self.surface_loader = Some(surface_loader);
        self.surface = surface;
    }

    fn init_pdevice(&mut self) {
        //
        todo!()
    }
}
