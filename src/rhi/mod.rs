use std::ffi::CStr;

use ash::{vk, Device, Entry, Instance};
use derive_setters::Setters;

struct RhiPhysicalDevice
{
    vk_physical_device: vk::PhysicalDevice,
    pd_props: vk::PhysicalDeviceProperties,
    pd_mem_props: vk::PhysicalDeviceMemoryProperties,
    pd_rt_pipeline_props: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
}


#[derive(Default)]
pub struct Rhi
{
    entry: Option<Entry>,

    instance: Option<Instance>,
    physical_device: Option<RhiPhysicalDevice>,
    device: Option<Device>,

    swapchain_loader: ash::extensions::khr::Swapchain,
    debug_loader: ash::extensions::ext::DebugUtils,
}

pub struct RhiInitInfo
{
    api_version: u32,
    instance_layers: Vec<&'static CStr>,
    app_name: String,
    engine_name: String,
    debug_msg_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    debug_msg_type: vk::DebugUtilsMessageTypeFlagsEXT,
}

impl Default for RhiInitInfo
{
    fn default() -> Self
    {
        Self {
            api_version: vk::API_VERSION_1_1,
            instance_layers: vec![cstr::cstr!("VK_LAYER_KHRONOS_validation")],
        }
    }
}

impl Rhi
{
    pub fn init(init_info: &RhiInitInfo) -> Self
    {
        let mut rhi = Self {
            entry: unsafe { Some(Entry::load().unwrap()) },
            ..Default::default()
        };

        rhi.init_instance();

        todo!()
    }


    // TODO
    fn init_instance(&mut self, layers: &[&'static CStr])
    {
        if ENABLE_VALIDATION_LAYERS && !Self::check_validation_layer_support(entry) {
            eprintln!("validation layers requested, but not available.");
        }

        let app_name = CString::new("Hiss_render").unwrap();
        let engine_name = CString::new("Hiss").unwrap();

        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vulkan_version);

        let extensions = Self::get_instance_extensions(window);
        let layers = Self::get_instance_layers();

        let mut debug_info = Self::populate_debug_msger_create_info();

        let create_flags = if cfg!(target_os = "macos") {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            Default::default()
        };

        let instance_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layers)
            .flags(create_flags)
            .push_next(&mut debug_info);

        unsafe { entry.create_instance(&instance_info, None).unwrap() }
    }
}
