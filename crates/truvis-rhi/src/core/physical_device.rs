use crate::core::command_queue::RhiQueueFamily;
use ash::vk;
use itertools::Itertools;
use std::ffi::CStr;
use std::ptr::null_mut;

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
    pub rt_pipeline_props: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR<'static>,

    /// 当前 gpu 的加速结构属性
    pub acc_struct_props: vk::PhysicalDeviceAccelerationStructurePropertiesKHR<'static>,

    pub mem_props: vk::PhysicalDeviceMemoryProperties,

    pub graphics_queue_family: RhiQueueFamily,
    pub compute_queue_family: RhiQueueFamily,
    pub transfer_queue_family: RhiQueueFamily,
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

    fn new(pdevice: vk::PhysicalDevice, instance: &ash::Instance) -> Self {
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

                // 基础的 props
                basic_props = pdevice_props2.properties;
                let physical_device_name = CStr::from_ptr(basic_props.device_name.as_ptr());
                log::info!("found gpu: {:?}", physical_device_name);

                // ray tracing props
                pdevice_raytracing_props.p_next = null_mut();
                rt_props = pdevice_raytracing_props;
                log::info!("physical deviceray tracing props:\n{:#?}", rt_props);

                // 加速结构 props
                pdevice_acc_props.p_next = null_mut();
                acc_props = pdevice_acc_props;
                log::info!("physical deivceacceleration structure props:\n{:#?}", acc_props);
            }

            // 找到当前 gpu 支持的 extensions，并打印出来
            let device_extensions = instance.enumerate_device_extension_properties(pdevice).unwrap();
            log::debug!("physical device supports extensions: ");
            for ext in &device_extensions {
                let ext_name = CStr::from_ptr(ext.extension_name.as_ptr());
                log::debug!("\t{:?}", ext_name.to_str().unwrap());
            }

            // 找到所有的队列信息并打印出来
            let queue_familiy_props = instance.get_physical_device_queue_family_properties(pdevice);
            log::debug!("physical device: queue family props:\n{:#?}", queue_familiy_props);

            // graphics queue family: 需要支持 graphics
            let graphics_queue_family = queue_familiy_props
                .iter()
                .enumerate()
                .find(|(_, props)| !(props.queue_flags & vk::QueueFlags::GRAPHICS).is_empty())
                .map(|(idx, props)| RhiQueueFamily {
                    name: "graphics".to_string(),
                    queue_family_index: idx as u32,
                    queue_flags: props.queue_flags,
                    queue_count: props.queue_count,
                })
                .unwrap();

            // compute queue family: 需要支持 compute，且和前面的 graphics queue family 不是同一个
            let compute_queue_family = queue_familiy_props
                .iter()
                .enumerate()
                .find(|(idx, props)| {
                    !(props.queue_flags & vk::QueueFlags::COMPUTE).is_empty()
                        && *idx as u32 != graphics_queue_family.queue_family_index
                })
                .map(|(idx, props)| RhiQueueFamily {
                    name: "compute".to_string(),
                    queue_family_index: idx as u32,
                    queue_flags: props.queue_flags,
                    queue_count: props.queue_count,
                })
                .unwrap();

            // transfer queue family: 需要支持 transfer，且和前面的 graphics queue family, compute queue family 不是同一个
            let transfer_queue_family = queue_familiy_props
                .iter()
                .enumerate()
                .find(|(idx, props)| {
                    !(props.queue_flags & vk::QueueFlags::TRANSFER).is_empty()
                        && *idx as u32 != graphics_queue_family.queue_family_index
                        && *idx as u32 != compute_queue_family.queue_family_index
                })
                .map(|(idx, props)| RhiQueueFamily {
                    name: "transfer".to_string(),
                    queue_family_index: idx as u32,
                    queue_flags: props.queue_flags,
                    queue_count: props.queue_count,
                })
                .unwrap();

            Self {
                mem_props: instance.get_physical_device_memory_properties(pdevice),
                features: instance.get_physical_device_features(pdevice),
                handle: pdevice,
                basic_props,
                rt_pipeline_props: rt_props,
                acc_struct_props: acc_props,
                graphics_queue_family,
                compute_queue_family,
                transfer_queue_family,
                device_extensions,
            }
        }
    }

    #[inline]
    /// 当前 gpu 是否是独立显卡
    pub fn is_descrete_gpu(&self) -> bool {
        self.basic_props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
    }
}
