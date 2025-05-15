use ash::vk;
use itertools::Itertools;
use std::ffi::CStr;
use std::ptr::{null, null_mut};

/// 表示一张物理显卡
pub struct RhiPhysicalDevice {
    pub handle: vk::PhysicalDevice,

    /// 当前 gpu 支持的 features
    pub features: vk::PhysicalDeviceFeatures,

    /// 当前 gpu 支持的 device extensions
    pub device_extensions: Vec<vk::ExtensionProperties>,

    /// 当前 gpu 的基础属性
    pub basic_props: vk::PhysicalDeviceProperties,

    /// 当前 gpu 的 ray tracing 属性
    pub rt_props: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR<'static>,

    /// 当前 gpu 的加速结构属性
    pub acc_props: vk::PhysicalDeviceAccelerationStructurePropertiesKHR<'static>,

    pub memory_properties: vk::PhysicalDeviceMemoryProperties,

    pub queue_family_properties: Vec<vk::QueueFamilyProperties>,
}

impl RhiPhysicalDevice {
    /// 创建一个新的物理显卡实例
    ///
    /// 优先选择独立显卡，如果没有则选择第一个可用的显卡
    pub fn new_descrete_physical_device(instance: &ash::Instance) -> Self {
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
            // 找到符合 ray tracing 条件的 gpu
            let rt_props;
            let basic_props;
            let acc_props;
            {
                let mut pdevice_raytracing_props = vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::default();
                let mut pdevice_acc_props = vk::PhysicalDeviceAccelerationStructurePropertiesKHR::default();
                let mut pdevice_props2 = vk::PhysicalDeviceProperties2::default()
                    .push_next(&mut pdevice_raytracing_props)
                    .push_next(&mut pdevice_acc_props);
                instance.get_physical_device_properties2(pdevice, &mut pdevice_props2);

                basic_props = pdevice_props2.properties;
                let physical_device_name = CStr::from_ptr(basic_props.device_name.as_ptr());
                log::info!("found gpu: {:?}", physical_device_name);

                pdevice_raytracing_props.p_next = null_mut();
                rt_props = pdevice_raytracing_props;
                log::info!("gpu ray tracing props: {:#?}", rt_props);

                pdevice_acc_props.p_next = null_mut();
                acc_props = pdevice_acc_props;
                log::info!("gpu acceleration structure props: {:#?}", acc_props);
            }

            // 找到当前 gpu 支持的 extensions，并打印出来
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
                basic_props,
                rt_props,
                acc_props,
                queue_family_properties: instance.get_physical_device_queue_family_properties(pdevice),
                device_extensions,
            }
        }
    }

    #[inline]
    /// 当前 gpu 是否是独立显卡
    pub fn is_descrete_gpu(&self) -> bool {
        self.basic_props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
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
