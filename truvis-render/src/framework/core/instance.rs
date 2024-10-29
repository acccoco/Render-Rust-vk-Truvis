use std::rc::Rc;

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
