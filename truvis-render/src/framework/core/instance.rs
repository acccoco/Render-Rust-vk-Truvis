use std::{ffi::CStr, rc::Rc};

use ash::vk;

use crate::framework::core::physical_device::RhiPhysicalDevice;

pub struct RhiInstance
{
    handle: ash::Instance,

    /// 当前机器上找到的所有 physical device
    gpus: Vec<RhiPhysicalDevice>,

    enabled_extensinos: Vec<String>,

    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
    debug_report_callback: vk::DebugReportCallbackEXT,
}

impl RhiInstance
{
    pub fn new(
        application_name: String,
        required_extensions: Vec<String>,
        required_validation_layer: Vec<String>,
        api_version: u32,
    ) -> Self
    {
        let vk_entry = unsafe { ash::Entry::load() }.expect("Failed to load Vulkan entry");
        let available_instance_extensions = vk_entry
            .enumerate_instance_extension_properties(None)
            .expect("Failed to enumerate instance extensions");

        let mut enabled_extensions = Vec::new();

        // 尝试开启 DEBUG_UTILS extension
        {
            let has_debug_utils = Self::enable_extension(
                ash::extensions::ext::DebugUtils::name().to_str().unwrap().to_string(),
                &available_instance_extensions,
                &mut enabled_extensions,
            );

            if !has_debug_utils {
                log::warn!("{:?} are not available", ash::extensions::ext::DebugUtils::name());
            }
        }

        todo!()
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
        &self.enabled_extensinos
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
        required_ext_name: String,
        available_exts: &[vk::ExtensionProperties],
        enabled_extensions: &mut Vec<String>,
    ) -> bool
    {
        let available = available_exts.iter().any(|ext| {
            required_ext_name ==
                unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) }.to_str().unwrap()
        });

        if available && !enabled_extensions.iter().any(|ext| *ext == required_ext_name) {
            log::info!("Extension {} found, enabling it", required_ext_name);
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
        self.enabled_extensinos.contains(&extension.to_string())
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
