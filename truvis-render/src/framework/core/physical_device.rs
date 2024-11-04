use std::ffi::CStr;

use ash::vk;

use crate::framework::core::instance::RhiInstance;

/// 表示一张物理显卡
pub struct RhiPhysicalDevice
{
    pub handle: vk::PhysicalDevice,

    features: vk::PhysicalDeviceFeatures,

    /// 当前 gpu 支持的 device extensions
    device_extensions: Vec<vk::ExtensionProperties>,

    properties: vk::PhysicalDeviceProperties,

    memory_properties: vk::PhysicalDeviceMemoryProperties,

    queue_family_properties: Vec<vk::QueueFamilyProperties>,

    /// 想要在 logical device 中启用的 features
    requested_features: vk::PhysicalDeviceFeatures,
    // pd_rt_pipeline_props: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
}

impl RhiPhysicalDevice
{
    pub fn new(pdevice: vk::PhysicalDevice, instance: &ash::Instance) -> Self
    {
        unsafe {
            let mut pd_rt_props = vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::default();
            let mut pd_props2 = vk::PhysicalDeviceProperties2::builder().push_next(&mut pd_rt_props);
            instance.get_physical_device_properties2(pdevice, &mut pd_props2);

            let gpu_name = CStr::from_ptr(pd_props2.properties.device_name.as_ptr());
            log::info!("found gpus: {:?}", gpu_name);

            let device_extensions = instance.enumerate_device_extension_properties(pdevice).unwrap();
            log::info!("device supports extensions: ");
            for ext in &device_extensions {
                let ext_name = CStr::from_ptr(ext.extension_name.as_ptr());
                log::info!("\t{:?}", ext_name.to_str().unwrap());
            }

            Self {
                memory_properties: instance.get_physical_device_memory_properties(pdevice),
                features: instance.get_physical_device_features(pdevice),
                handle: pdevice,
                properties: pd_props2.properties,
                queue_family_properties: instance.get_physical_device_queue_family_properties(pdevice),
                device_extensions,
                requested_features: vk::PhysicalDeviceFeatures::default(),
            }
        }
    }


    #[inline]
    /// 当前 gpu 是否是独立显卡
    pub fn is_descrete_gpu(&self) -> bool
    {
        self.properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
    }

    /// 获取驱动的版本信息
    pub fn get_driver_version() -> (u32, u32, u32)
    {
        todo!()
    }

    /// 找到满足条件的 queue family 的 index
    pub fn find_queue_family_index(&self, queue_flags: vk::QueueFlags) -> Option<u32>
    {
        self.queue_family_properties
            .iter()
            .enumerate()
            .find(|(_, prop)| prop.queue_flags.contains(queue_flags))
            .map(|(index, _)| index as u32)
    }

    /// 检查当前 gpu 是否支持某个 extension
    pub fn is_extension_supported(&self, requested_extension: String) -> bool
    {
        todo!()
    }

    pub fn get_features(&self) -> &vk::PhysicalDeviceFeatures
    {
        &self.features
    }

    pub fn get_handle(&self) -> vk::PhysicalDevice
    {
        self.handle
    }

    /// TODO 感觉这个方法不是一个良好的设计
    pub fn get_instance(&self) -> &RhiInstance
    {
        todo!()
    }

    pub fn get_memory_properties(&self) -> &vk::PhysicalDeviceMemoryProperties
    {
        &self.memory_properties
    }

    /// 检查当前 gpu 是否支持某种 memory type
    ///
    /// - param `type_bits`: 一个 32 位的整数，每一位代表一个 memory type
    pub fn get_memory_type(&self, type_bits: u32, properties: vk::MemoryPropertyFlags) -> u32
    {
        todo!()
    }

    pub fn get_properties(&self) -> &vk::PhysicalDeviceProperties
    {
        &self.properties
    }

    pub fn get_queue_family_properties(&self) -> &Vec<vk::QueueFamilyProperties>
    {
        &self.queue_family_properties
    }

    pub fn get_requested_features(&self) -> &vk::PhysicalDeviceFeatures
    {
        todo!()
    }

    /// 第一个图形队列是否应该有更高的优先级
    ///
    /// 在异步计算任务时可能会需要
    pub fn set_high_priority_graphics_queue_enable(&mut self, enable: bool)
    {
        todo!()
    }
}
