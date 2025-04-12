use std::ffi::CStr;

use ash::vk;
use itertools::Itertools;

/// 表示一张物理显卡
pub struct RhiPhysicalDevice {
    pub handle: vk::PhysicalDevice,

    pub features: vk::PhysicalDeviceFeatures,

    /// 当前 gpu 支持的 device extensions
    pub device_extensions: Vec<vk::ExtensionProperties>,

    pub properties: vk::PhysicalDeviceProperties,

    pub memory_properties: vk::PhysicalDeviceMemoryProperties,

    pub queue_family_properties: Vec<vk::QueueFamilyProperties>,
}

impl RhiPhysicalDevice {
    /// 创建一个新的物理显卡实例
    ///
    /// 优先选择独立显卡，如果没有则选择第一个可用的显卡
    pub fn new_descrete_gpu(instance: &ash::Instance) -> Self {
        unsafe {
            instance
                .enumerate_physical_devices()
                .unwrap()
                .iter()
                .map(|pdevice| RhiPhysicalDevice::new(*pdevice, instance))
                // 优先使用独立显卡
                .find_or_first(RhiPhysicalDevice::is_descrete_gpu)
                .unwrap()
        }
    }

    pub fn new(pdevice: vk::PhysicalDevice, instance: &ash::Instance) -> Self {
        unsafe {
            let mut pd_rt_props = vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::default();
            let mut pd_props2 = vk::PhysicalDeviceProperties2::default().push_next(&mut pd_rt_props);
            instance.get_physical_device_properties2(pdevice, &mut pd_props2);

            let gpu_name = CStr::from_ptr(pd_props2.properties.device_name.as_ptr());
            log::info!("found gpus: {:?}", gpu_name);

            let device_extensions = instance.enumerate_device_extension_properties(pdevice).unwrap();
            log::debug!("device supports extensions: ");
            for ext in &device_extensions {
                let ext_name = CStr::from_ptr(ext.extension_name.as_ptr());
                log::debug!("\t{:?}", ext_name.to_str().unwrap());
            }

            Self {
                memory_properties: instance.get_physical_device_memory_properties(pdevice),
                features: instance.get_physical_device_features(pdevice),
                handle: pdevice,
                properties: pd_props2.properties,
                queue_family_properties: instance.get_physical_device_queue_family_properties(pdevice),
                device_extensions,
            }
        }
    }

    #[inline]
    /// 当前 gpu 是否是独立显卡
    pub fn is_descrete_gpu(&self) -> bool {
        self.properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
    }

    /// 找到满足条件的 queue family 的 index
    pub fn find_queue_family_index(&self, queue_flags: vk::QueueFlags) -> Option<u32> {
        self.queue_family_properties
            .iter()
            .enumerate()
            .find(|(_, prop)| prop.queue_flags.contains(queue_flags))
            .map(|(index, _)| index as u32)
    }
}
