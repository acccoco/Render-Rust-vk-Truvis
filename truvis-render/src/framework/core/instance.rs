use std::{
    ffi::{CStr, CString},
    rc::Rc,
};

use ash::vk;
use itertools::Itertools;

use crate::framework::{core::physical_device::RhiPhysicalDevice, rhi::vk_debug_callback};

pub struct RhiInstance
{
    handle: ash::Instance,

    /// 当前机器上找到的所有 physical device
    gpus: Vec<RhiPhysicalDevice>,

    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
    debug_report_callback: vk::DebugReportCallbackEXT,
}


/// 所需的 layers 是否全部支持
fn validate_layers(required: &[&'static CStr], available: &[vk::LayerProperties]) -> bool
{
    required.iter().all(|layer| {
        let found = available
            .iter()
            .any(|available_layer| *layer == unsafe { CStr::from_ptr(available_layer.layer_name.as_ptr()) });
        if !found {
            log::error!("Validation Layer {:?} not found", layer);
        }

        found
    })
}

/// 返回一系列的尝试启用的 Validation Layers
fn get_optimal_validation_layers(supported_instance_layers: &[vk::LayerProperties]) -> Vec<&'static CStr>
{
    let validation_layer_priority_list = [
        // 首选这个 validation layer
        vec![cstr::cstr!("VK_LAYER_KHRONOS_validation")],
        // fallback: 选择 LunarG meta layer
        vec![cstr::cstr!("VK_LAYER_LUNARG_standard_validation")],
        // fallback: 选择
        vec![
            cstr::cstr!("VK_LAYER_GOOGLE_threading"),
            cstr::cstr!("VK_LAYER_LUNARG_parameter_validation"),
            cstr::cstr!("VK_LAYER_LUNARG_object_tracker"),
            cstr::cstr!("VK_LAYER_LUNARG_core_validation"),
            cstr::cstr!("VK_LAYER_GOOGLE_unique_objects"),
        ],
        // fallback: 选择 LunarG core layer
        vec![cstr::cstr!("VK_LAYER_LUNARG_core_validation")],
    ];

    for validation_layers in validation_layer_priority_list.iter() {
        if validate_layers(validation_layers, supported_instance_layers) {
            return validation_layers.clone();
        }

        log::error!("Couldn't enable validation layers - falling back");
    }

    Vec::new()
}


impl RhiInstance
{
    /// 设置所需的 layers 和 extensions，创建 vk instance
    ///
    /// # Arguments
    ///
    /// * `required_extensions`: 额外需要的 extension。（extension-name; optional）
    /// * `api_version`: vk 的版本，会影响某些函数的调用
    pub fn new(
        vk_entry: &ash::Entry,
        application_name: String,
        required_extensions: Vec<(&'static CStr, bool)>,
        required_validation_layer: Vec<&'static CStr>,
        api_version: u32,
    ) -> Self
    {
        /// instance 所需的所有 extension
        fn get_extensions(
            required_extensions: Vec<(&'static CStr, bool)>,
            available_instance_extensions: &[vk::ExtensionProperties],
        ) -> Vec<&'static CStr>
        {
            let mut enabled_extensions = Vec::new();

            // 尝试开启 DEBUG_UTILS extension
            #[cfg(feature = "validation")]
            {
                let has_debug_utils = Self::enable_extension(
                    ash::extensions::ext::DebugUtils::name(),
                    &available_instance_extensions,
                    &mut enabled_extensions,
                );

                if !has_debug_utils {
                    log::warn!(
                        "{:?} are not available; disableing debug reporting",
                        ash::extensions::ext::DebugUtils::name()
                    );
                }
            }

            // 显示在 surface 上所需的 extension
            enabled_extensions.push(ash::extensions::khr::Surface::name());

            // 这个 extension 时 VK_KHR_performance_query 的前置条件；而后者是用于 stats gathering 的
            RhiInstance::enable_extension(
                ash::extensions::khr::GetPhysicalDeviceProperties2::name(),
                &available_instance_extensions,
                &mut enabled_extensions,
            );

            // 检查外部传入的 extension 是否支持
            let mut extension_error = false;
            for extension in required_extensions {
                let (extension_name, extension_is_optional) = extension;
                if !RhiInstance::enable_extension(
                    extension_name.clone(),
                    &available_instance_extensions,
                    &mut enabled_extensions,
                ) {
                    if extension_is_optional {
                        log::warn!(
                            "Optional instance extension {:?} not available, some features may be disabled",
                            extension_name
                        );
                    } else {
                        log::error!("Required instance extension {:?} not available, cannot run", extension_name);
                        extension_error = true;
                    }
                }
            }
            if extension_error {
                panic!("Required instance extensions are missin");
            }

            enabled_extensions
        }

        /// instance 所需的所有 layers
        fn get_layers(
            mut required_validation_layers: Vec<&'static CStr>,
            supported_validation_layers: &[vk::LayerProperties],
        ) -> Vec<&'static CStr>
        {
            #[cfg(feature = "validation")]
            {
                let optimal_validation_layers = get_optimal_validation_layers(&supported_validation_layers);
                required_validation_layers.extend(optimal_validation_layers);
            }


            if validate_layers(&required_validation_layers, &supported_validation_layers) {
                log::info!("Enabled Validation Layers:");
                for layer in &required_validation_layers {
                    log::info!("\t{:?}", layer);
                }
            } else {
                panic!("Required validation layers are missing.");
            }

            required_validation_layers
        }

        let application_name = CString::new(application_name.as_str()).unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .api_version(vk::API_VERSION_1_3)
            .application_name(application_name.as_ref())
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(cstr::cstr!("Truvis"))
            .engine_version(vk::make_api_version(0, 1, 0, 0));

        let enabled_extensions =
            get_extensions(required_extensions, &vk_entry.enumerate_instance_extension_properties(None).unwrap())
                .iter()
                .map(|ext| ext.as_ptr())
                .collect_vec();
        let enabled_layers =
            get_layers(required_validation_layer, &vk_entry.enumerate_instance_layer_properties().unwrap())
                .iter()
                .map(|layer| layer.as_ptr())
                .collect_vec();
        let mut instance_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&enabled_extensions)
            .enabled_layer_names(&enabled_layers);

        fn get_debug_utils_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT
        {
            vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::WARNING | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(vk_debug_callback))
                .build()
        }

        // 为 instance info 添加 debug messenger
        #[cfg(feature = "validation")]
        {
            let mut debug_utils_create_info = get_debug_utils_create_info();
            instance_info = instance_info.push_next(&mut debug_utils_create_info);
        }

        let handle = unsafe { vk_entry.create_instance(&instance_info, None) }.unwrap();

        let debug_utils_messenger = None;
        #[cfg(feature = "validation")]
        {
            let debug_utils_pf = ash::extensions::ext::DebugUtils::new(&vk_entry, &handle);
            let debug_utils_create_info = get_debug_utils_create_info();
            let debug_utils_messenger =
                Some(unsafe { debug_utils_pf.create_debug_utils_messenger(&debug_utils_create_info, None).unwrap() });
        }


        let mut s = Self {
            handle,
            gpus: Vec::new(),
            debug_utils_messenger: debug_utils_messenger.unwrap(),
            debug_report_callback: vk::DebugReportCallbackEXT::null(),
        };

        s.query_gpus();

        s
    }

    /// 尝试得到机器上第一个可用的 discrete gpu
    fn get_first_gpu(&self) -> RhiPhysicalDevice
    {
        todo!()
    }

    pub fn get_handle(&self) -> &ash::Instance
    {
        &self.handle
    }

    pub fn get_extensions(&self) -> &Vec<String>
    {
        todo!()
    }

    /// 尝试开启某项 extension
    ///
    /// # Arguments
    ///
    /// * `required_ext_name`: 尝试开启的 extension
    /// * `available_exts`: 机器上所有可用的 extension
    /// * `enabled_extensions`: 已经开启的 extension；如果成功开启，则会添加到这个 vec 中
    ///
    /// # Returns
    ///
    /// * 是否成功开启了这个 extension
    fn enable_extension(
        required_ext_name: &'static CStr,
        available_exts: &[vk::ExtensionProperties],
        enabled_extensions: &mut Vec<&'static CStr>,
    ) -> bool
    {
        let available = available_exts
            .iter()
            .any(|ext| required_ext_name == unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) });

        if available && !enabled_extensions.iter().any(|ext| *ext == required_ext_name) {
            log::info!("Extension {:?} found, enabling it", required_ext_name);
            enabled_extensions.push(required_ext_name);
        }

        available
    }

    /// 尝试找到第一个可以渲染到给定 surface 上的 discrete gpu
    pub fn get_suitable_gpu(&self, surface: vk::SurfaceKHR) -> &RhiPhysicalDevice
    {
        todo!()
    }

    /// 检查是否启用了某个 extension
    pub fn is_enabled(&self, extension: &str) -> bool
    {
        todo!()
    }

    /// 找到机器上所有的 GPU，并缓存到 self.gpus 中
    fn query_gpus(&mut self)
    {
        unsafe {
            self.gpus = self
                .handle
                .enumerate_physical_devices()
                .unwrap()
                .iter()
                .map(|pdevice| RhiPhysicalDevice::new(*pdevice, &self.handle))
                .collect();
        }
    }
}
