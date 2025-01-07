use anyhow::Context;
use itertools::Itertools;

use crate::framework::core::physical_device::RhiPhysicalDevice;


pub struct RhiInstance
{
    pub handle: ash::Instance,

    /// 当前机器上找到的所有 physical device
    pub gpus: Vec<RhiPhysicalDevice>,
}


mod _impl_init
{
    use std::{
        collections::HashSet,
        ffi::{c_char, CStr, CString},
    };

    use ash::{vk, Instance};
    use itertools::Itertools;

    use crate::framework::{
        core::{instance::RhiInstance, physical_device::RhiPhysicalDevice},
        rhi::RhiInitInfo,
    };

    impl RhiInstance
    {
        /// 设置所需的 layers 和 extensions，创建 vk instance
        pub fn new(vk_entry: &ash::Entry, init_info: &RhiInitInfo) -> Self
        {
            let app_name = CString::new(init_info.app_name.as_str()).unwrap();
            let engine_name = CString::new(init_info.engine_name.as_str()).unwrap();
            let app_info = vk::ApplicationInfo::builder()
                .api_version(init_info.vk_version)
                .application_name(app_name.as_ref())
                .application_version(vk::make_api_version(0, 1, 0, 0))
                .engine_name(engine_name.as_ref())
                .engine_version(vk::make_api_version(0, 1, 0, 0));

            let enabled_extensions = Self::get_extensions(vk_entry, init_info);
            let enabled_layers = Self::get_layers(vk_entry, init_info);

            let mut instance_ci = vk::InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_extension_names(&enabled_extensions)
                .enabled_layer_names(&enabled_layers)
                .flags(init_info.instance_create_flags);

            // 为 instance info 添加 debug messenger
            let mut debug_utils_messenger_ci = init_info.get_debug_utils_messenger_ci();

            if init_info.enable_validation {
                instance_ci = instance_ci.push_next(&mut debug_utils_messenger_ci);
            }

            let handle = unsafe { vk_entry.create_instance(&instance_ci, None).unwrap() };

            let gpus = Self::query_gpus(&handle);

            let instance = Self { handle, gpus };
            instance
        }


        /// instance 所需的所有 extension
        ///
        /// # params
        /// enable_validation 是否开启 validation layers
        ///
        /// # return
        /// instance 所需的，且受支持的 extension
        fn get_extensions(vk_entry: &ash::Entry, init_info: &RhiInitInfo) -> Vec<*const c_char>
        {
            let all_ext_props = vk_entry.enumerate_instance_extension_properties(None).unwrap();
            let mut enabled_extensions: HashSet<&'static CStr> = HashSet::new();

            let mut enable_ext = |ext: &'static CStr| {
                let supported = all_ext_props
                    .iter()
                    .any(|supported_ext| ext == unsafe { CStr::from_ptr(supported_ext.extension_name.as_ptr()) });
                if supported {
                    enabled_extensions.insert(ext);
                } else {
                    panic!("Required instance extensions ({:?}) are missin", ext)
                }
            };

            // 检查外部传入的 extension 是否支持
            for ext in &init_info.instance_extensions {
                enable_ext(*ext);
            }

            enabled_extensions.iter().map(|ext| ext.as_ptr()).collect_vec()
        }

        /// instance 所需的所有 layers
        fn get_layers(vk_entry: &ash::Entry, init_info: &RhiInitInfo) -> Vec<*const c_char>
        {
            let all_layer_props = vk_entry.enumerate_instance_layer_properties().unwrap();

            let mut validation_layers = Vec::new();

            let mut enable_layer = |layer: &'static CStr| {
                let is_layer_supported = all_layer_props
                    .iter()
                    .any(|available_layer| layer == unsafe { CStr::from_ptr(available_layer.layer_name.as_ptr()) });
                if is_layer_supported {
                    validation_layers.push(layer);
                } else {
                    panic!("Required instance layers ({:?}) are missing", layer);
                }
            };

            for layer in &init_info.instance_layers {
                enable_layer(*layer);
            }

            validation_layers.iter().map(|ext| ext.as_ptr()).collect_vec()
        }


        /// 找到机器上所有的 PhysicalDevice, 并缓存到 self.gpus 中
        fn query_gpus(instance: &Instance) -> Vec<RhiPhysicalDevice>
        {
            unsafe {
                instance
                    .enumerate_physical_devices()
                    .unwrap()
                    .iter()
                    .map(|pdevice| RhiPhysicalDevice::new(*pdevice, instance))
                    .collect()
            }
        }
    }
}
