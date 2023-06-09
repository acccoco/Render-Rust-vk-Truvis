use std::{collections::HashSet, ffi::CStr};

use ash::{vk, Instance};
use itertools::Itertools;

use crate::rhi::queue::{RhiQueueFamilyPresentProps, RhiQueueFamilyProps, RhiQueueType};


pub struct RhiPhysicalDevice
{
    pub vk_physical_device: vk::PhysicalDevice,
    pub pd_props: vk::PhysicalDeviceProperties,
    pub pd_mem_props: vk::PhysicalDeviceMemoryProperties,
    pub pd_features: vk::PhysicalDeviceFeatures,
    pub pd_rt_pipeline_props: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
    pub queue_family_props: Vec<RhiQueueFamilyProps>,
}

impl RhiPhysicalDevice
{
    pub fn new(pdevice: vk::PhysicalDevice, instance: &Instance) -> Self
    {
        unsafe {
            let mut pd_rt_props = vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::default();
            let mut pd_props2 = vk::PhysicalDeviceProperties2::builder().push_next(&mut pd_rt_props);
            instance.get_physical_device_properties2(pdevice, &mut pd_props2);

            Self {
                vk_physical_device: pdevice,
                pd_props: pd_props2.properties,
                pd_mem_props: instance.get_physical_device_memory_properties(pdevice),
                pd_features: instance.get_physical_device_features(pdevice),
                pd_rt_pipeline_props: pd_rt_props,
                queue_family_props: Vec::new(),
            }
        }
    }

    pub fn init_queue_family_props(
        &mut self,
        instance: &Instance,
        surface: Option<vk::SurfaceKHR>,
        surface_loader: &ash::extensions::khr::Surface,
    )
    {
        unsafe {
            let queue_family_present_prop = |i: u32| {
                if let Some(surface) = surface {
                    if surface_loader
                        .get_physical_device_surface_support(self.vk_physical_device, i, surface)
                        .unwrap()
                    {
                        RhiQueueFamilyPresentProps::Supported
                    } else {
                        RhiQueueFamilyPresentProps::NoSupported
                    }
                } else {
                    RhiQueueFamilyPresentProps::NoSurface
                }
            };

            self.queue_family_props = instance
                .get_physical_device_queue_family_properties(self.vk_physical_device)
                .iter()
                .enumerate()
                .map(|(i, prop)| RhiQueueFamilyProps {
                    compute: prop.queue_flags.contains(vk::QueueFlags::COMPUTE),
                    graphics: prop.queue_flags.contains(vk::QueueFlags::GRAPHICS),
                    present: queue_family_present_prop(i as u32),
                })
                .collect_vec();
        }
    }

    #[inline]
    pub fn is_descrete_gpu(&self) -> bool { self.pd_props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU }


    pub fn find_queue_family_index(&self, queue_type: RhiQueueType) -> Option<u32>
    {
        self.queue_family_props
            .iter()
            .enumerate()
            .find(|(_, prop)| match queue_type {
                RhiQueueType::Compute => prop.compute,
                RhiQueueType::Graphics => prop.graphics,
                RhiQueueType::Present => prop.present == RhiQueueFamilyPresentProps::Supported,
            })
            .map(|(index, _)| index as u32)
    }


    /// physical device 是否支持指定的所有扩展
    pub fn check_device_extension_support(&self, instance: &Instance, exts: &[&'static CStr]) -> bool
    {
        unsafe {
            let supported_exts = instance
                .enumerate_device_extension_properties(self.vk_physical_device)
                .unwrap()
                .iter()
                .map(|ext| CStr::from_ptr(ext.extension_name.as_ptr()))
                .collect_vec();

            let mut required_exts: HashSet<_> = exts.iter().collect();
            for ext in supported_exts {
                required_exts.remove(&ext);
            }
            required_exts.is_empty()
        }
    }
}
