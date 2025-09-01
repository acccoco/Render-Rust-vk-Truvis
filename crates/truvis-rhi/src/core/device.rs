use crate::core::debug_utils::RhiDebugType;
use crate::shader_cursor::RhiWriteDescriptorSet;
use ash::vk;
use itertools::Itertools;
use std::ops::Deref;
use std::{ffi::CStr, rc::Rc};

pub struct RhiDevicePfn {
    pub(crate) vk_device_pf: ash::Device,
    pub(crate) vk_dynamic_render_pf: ash::khr::dynamic_rendering::Device,
    pub(crate) vk_acceleration_struct_pf: ash::khr::acceleration_structure::Device,
    pub(crate) vk_rt_pipeline_pf: ash::khr::ray_tracing_pipeline::Device,
}
impl Deref for RhiDevicePfn {
    type Target = ash::Device;
    fn deref(&self) -> &Self::Target {
        &self.vk_device_pf
    }
}

pub struct RhiDevice {
    /// 仅仅是函数指针，以及一个裸的 handle，可以随意 clone
    ///
    /// 不需要考虑生命周期的问题，生命周期现在是由手动控制的
    pub(crate) device_pfn: Rc<RhiDevicePfn>,
}

/// 构造与销毁
impl RhiDevice {
    pub fn new(
        instance: &ash::Instance,
        pdevice: vk::PhysicalDevice,
        queue_create_info: &[vk::DeviceQueueCreateInfo],
    ) -> Self {
        // device 所需的所有 extension
        let device_exts = Self::basic_device_exts().iter().map(|e| e.as_ptr()).collect_vec();
        let mut exts_str = String::new();
        for ext in &device_exts {
            exts_str.push_str(&format!("\n\t{:?}", unsafe { CStr::from_ptr(*ext) }));
        }
        log::info!("device exts: {}", exts_str);

        // device 所需的所有 features
        let mut all_features = vk::PhysicalDeviceFeatures2::default().features(Self::physical_device_basic_features());
        let mut physical_device_ext_features = Self::physical_device_extra_features();
        unsafe {
            physical_device_ext_features.iter_mut().for_each(|f| {
                let ptr = <*mut dyn vk::ExtendsPhysicalDeviceFeatures2>::cast::<vk::BaseOutStructure>(f.as_mut());
                (*ptr).p_next = all_features.p_next as _;
                all_features.p_next = ptr as _;
            });
        }

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(queue_create_info)
            .enabled_extension_names(&device_exts)
            .push_next(&mut all_features);

        let device = unsafe { instance.create_device(pdevice, &device_create_info, None).unwrap() };

        let vk_dynamic_render_pf = ash::khr::dynamic_rendering::Device::new(instance, &device);
        let vk_acceleration_struct_pf = ash::khr::acceleration_structure::Device::new(instance, &device);
        let vk_rt_pipeline_pf = ash::khr::ray_tracing_pipeline::Device::new(instance, &device);

        Self {
            device_pfn: Rc::new(RhiDevicePfn {
                vk_device_pf: device.clone(),
                vk_dynamic_render_pf,
                vk_acceleration_struct_pf,
                vk_rt_pipeline_pf,
            }),
        }
    }

    pub fn destroy(self) {
        log::info!("destroying device");
        unsafe {
            self.device_pfn.vk_device_pf.destroy_device(None);
        }
    }
}

/// 创建过程的辅助函数
impl RhiDevice {
    /// 必要的 physical device core features
    fn physical_device_basic_features() -> vk::PhysicalDeviceFeatures {
        vk::PhysicalDeviceFeatures::default()
            .sampler_anisotropy(true)
            .fragment_stores_and_atomics(true)
            .independent_blend(true)
            .shader_int64(true) // 用于 buffer device address
    }

    /// 必要的 physical device extension features
    fn physical_device_extra_features() -> Vec<Box<dyn vk::ExtendsPhysicalDeviceFeatures2>> {
        vec![
            Box::new(vk::PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true)),
            Box::new(vk::PhysicalDeviceBufferDeviceAddressFeatures::default().buffer_device_address(true)),
            Box::new(vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default().ray_tracing_pipeline(true)),
            Box::new(vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default().acceleration_structure(true)),
            Box::new(vk::PhysicalDeviceHostQueryResetFeatures::default().host_query_reset(true)),
            Box::new(vk::PhysicalDeviceSynchronization2Features::default().synchronization2(true)),
            Box::new(vk::PhysicalDeviceTimelineSemaphoreFeatures::default().timeline_semaphore(true)),
            Box::new(
                vk::PhysicalDeviceDescriptorIndexingFeatures::default()
                    .descriptor_binding_partially_bound(true) // 即使一些 descriptor 是 invalid
                    .runtime_descriptor_array(true)
                    .descriptor_binding_sampled_image_update_after_bind(true)
                    .descriptor_binding_storage_image_update_after_bind(true)
                    .descriptor_binding_variable_descriptor_count(true),
            ),
        ]
    }

    /// 必要的 device extensions
    fn basic_device_exts() -> Vec<&'static CStr> {
        let mut exts = vec![];

        // swapchain
        exts.push(ash::khr::swapchain::NAME);

        // dynamic rendering
        exts.append(&mut vec![
            ash::khr::depth_stencil_resolve::NAME,
            ash::khr::create_renderpass2::NAME,
            ash::khr::dynamic_rendering::NAME,
        ]);

        // RayTracing 相关的
        exts.append(&mut vec![
            ash::khr::acceleration_structure::NAME, // 主要的 ext
            ash::ext::descriptor_indexing::NAME,
            ash::khr::buffer_device_address::NAME,
            ash::khr::ray_tracing_pipeline::NAME, // 主要的 ext
            ash::khr::deferred_host_operations::NAME,
            ash::khr::spirv_1_4::NAME,
            ash::khr::shader_float_controls::NAME,
        ]);

        exts
    }
}

/// getter
impl RhiDevice {
    #[inline]
    pub fn ash_handle(&self) -> &ash::Device {
        &self.device_pfn.vk_device_pf
    }

    #[inline]
    pub fn vk_handle(&self) -> vk::Device {
        self.device_pfn.vk_device_pf.handle()
    }

    #[inline]
    pub fn dynamic_rendering_pf(&self) -> &ash::khr::dynamic_rendering::Device {
        &self.device_pfn.vk_dynamic_render_pf
    }

    #[inline]
    pub fn acceleration_structure_pf(&self) -> &ash::khr::acceleration_structure::Device {
        &self.device_pfn.vk_acceleration_struct_pf
    }

    #[inline]
    pub fn rt_pipeline_pf(&self) -> &ash::khr::ray_tracing_pipeline::Device {
        &self.device_pfn.vk_rt_pipeline_pf
    }
}

/// tools
impl RhiDevice {
    #[inline]
    pub fn write_descriptor_sets(&self, writes: &[RhiWriteDescriptorSet]) {
        let writes = writes.iter().map(|w| w.to_vk_type()).collect_vec();
        unsafe {
            self.device_pfn.vk_device_pf.update_descriptor_sets(&writes, &[]);
        }
    }
}

impl RhiDebugType for RhiDevice {
    fn debug_type_name() -> &'static str {
        "RhiDevice"
    }
    fn vk_handle(&self) -> impl vk::Handle {
        self.device_pfn.vk_device_pf.handle()
    }
}
