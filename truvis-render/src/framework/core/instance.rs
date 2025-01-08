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
            let app_info = vk::ApplicationInfo::default()
                .api_version(init_info.vk_version)
                .application_name(app_name.as_ref())
                .application_version(vk::make_api_version(0, 1, 0, 0))
                .engine_name(engine_name.as_ref())
                .engine_version(vk::make_api_version(0, 1, 0, 0));

            let enabled_extensions = Self::get_extensions(vk_entry, init_info);
            let enabled_layers = Self::get_layers(vk_entry, init_info);

            let mut instance_ci = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_extension_names(&enabled_extensions)
                .enabled_layer_names(&enabled_layers)
                .flags(init_info.instance_create_flags);

            // 为 instance info 添加 debug messenger
            let mut debug_utils_messenger_ci = init_info.get_debug_utils_messenger_ci();

            // validation layer settings
            let layer_name = cstr::cstr!("VK_LAYER_KHRONOS_validation");
            let setting_validate_core = vk::TRUE;
            let setting_validate_sync = vk::TRUE;
            let setting_thread_safety = vk::TRUE;
            let setting_debug_action = [cstr::cstr!("VK_DBG_LAYER_ACTION_LOG_MSG")];
            let setting_report_flags = [
                cstr::cstr!("info"),
                cstr::cstr!("warn"),
                cstr::cstr!("perf"),
                cstr::cstr!("error"),
                cstr::cstr!("debug"),
            ];
            let setting_enable_message_limit = vk::TRUE;
            let setting_duplicate_message_limit = 3;
            let mut validation_settings: Vec<vk::LayerSettingEXT> = Vec::new();
            if init_info.enable_validation {
                unsafe {
                    validation_settings = vec![
                        Self::get_layer_setting_for_single(
                            layer_name,
                            cstr::cstr!("validate_core"),
                            vk::LayerSettingTypeEXT::BOOL32,
                            &setting_validate_core,
                        ),
                        Self::get_layer_setting_for_single(
                            layer_name,
                            cstr::cstr!("validate_sync"),
                            vk::LayerSettingTypeEXT::BOOL32,
                            &setting_validate_sync,
                        ),
                        Self::get_layer_setting_for_single(
                            layer_name,
                            cstr::cstr!("thread_safety"),
                            vk::LayerSettingTypeEXT::BOOL32,
                            &setting_thread_safety,
                        ),
                        Self::get_layer_setting_for_array(
                            layer_name,
                            cstr::cstr!("debug_action"),
                            vk::LayerSettingTypeEXT::STRING,
                            &setting_debug_action,
                        ),
                        Self::get_layer_setting_for_array(
                            layer_name,
                            cstr::cstr!("report_flags"),
                            vk::LayerSettingTypeEXT::STRING,
                            &setting_report_flags,
                        ),
                        Self::get_layer_setting_for_single(
                            layer_name,
                            cstr::cstr!("enable_message_limit"),
                            vk::LayerSettingTypeEXT::BOOL32,
                            &setting_enable_message_limit,
                        ),
                        Self::get_layer_setting_for_single(
                            layer_name,
                            cstr::cstr!("duplicate_message_limit"),
                            vk::LayerSettingTypeEXT::INT32,
                            &setting_duplicate_message_limit,
                        ),
                    ];
                }
            }
            let mut layer_settings_ci = vk::LayerSettingsCreateInfoEXT::default().settings(&validation_settings);

            if init_info.enable_validation {
                instance_ci = instance_ci.push_next(&mut debug_utils_messenger_ci);
                instance_ci = instance_ci.push_next(&mut layer_settings_ci);
            }

            let handle = unsafe { vk_entry.create_instance(&instance_ci, None).unwrap() };

            let gpus = Self::query_gpus(&handle);

            let instance = Self { handle, gpus };
            instance
        }

        fn get_layer_setting_for_single<'a, T>(
            layer_name: &'static CStr,
            setting_name: &'static CStr,
            ty: vk::LayerSettingTypeEXT,
            value: &'a T,
        ) -> vk::LayerSettingEXT<'a>
        {
            vk::LayerSettingEXT {
                p_layer_name: layer_name.as_ptr(),
                p_setting_name: setting_name.as_ptr(),
                ty,
                value_count: 1,
                p_values: value as *const _ as *const std::ffi::c_void,
                ..Default::default()
            }
        }

        fn get_layer_setting_for_array<'a, T>(
            layer_name: &'static CStr,
            setting_name: &'static CStr,
            ty: vk::LayerSettingTypeEXT,
            value: &'a [T],
        ) -> vk::LayerSettingEXT<'a>
        {
            vk::LayerSettingEXT {
                p_layer_name: layer_name.as_ptr(),
                p_setting_name: setting_name.as_ptr(),
                ty,
                value_count: value.len() as u32,
                p_values: value.as_ptr() as *const std::ffi::c_void,
                ..Default::default()
            }
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
            let all_ext_props = unsafe { vk_entry.enumerate_instance_extension_properties(None).unwrap() };
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
            let all_layer_props = unsafe { vk_entry.enumerate_instance_layer_properties().unwrap() };

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
