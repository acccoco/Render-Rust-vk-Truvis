use ash::vk;
use itertools::Itertools;
use std::{ffi::CStr, ops::Deref, rc::Rc};

use crate::core::command_queue::RhiQueueFamily;
use crate::core::debug_utils::RhiDebugUtils;
use crate::core::{instance::RhiInstance, physical_device::RhiPhysicalDevice};
use crate::shader_cursor::RhiWriteDescriptorSet;

pub struct RhiDevice {
    pub handle: ash::Device,

    pub pdevice: Rc<RhiPhysicalDevice>,

    pub vk_dynamic_render_pf: Rc<ash::khr::dynamic_rendering::Device>,
    pub vk_acceleration_struct_pf: Rc<ash::khr::acceleration_structure::Device>,

    pub debug_utils: Rc<RhiDebugUtils>,
}

impl Deref for RhiDevice {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl RhiDevice {
    /// # return
    /// * (device, graphics queue, compute queue, transfer queue)
    pub fn new(
        vk_pf: &ash::Entry,
        instance: &RhiInstance,
        pdevice: Rc<RhiPhysicalDevice>,
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

        let device = unsafe { instance.handle.create_device(pdevice.handle, &device_create_info, None).unwrap() };

        let debug_utils = Rc::new(RhiDebugUtils::new(vk_pf, &instance.handle, &device));

        let vk_dynamic_render_pf = Rc::new(ash::khr::dynamic_rendering::Device::new(&instance.handle, &device));
        let vk_acceleration_struct_pf =
            Rc::new(ash::khr::acceleration_structure::Device::new(&instance.handle, &device));

        Self {
            handle: device,
            pdevice: pdevice.clone(),

            vk_dynamic_render_pf,
            vk_acceleration_struct_pf,

            debug_utils,
        }
    }

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

impl RhiDevice {
    /// 将 UBO 的尺寸和 min_UBO_Offset_Align 对齐，使得得到的尺寸是 min_UBO_Offset_Align 的整数倍
    #[inline]
    pub fn aligned_ubo_size<T: bytemuck::Pod>(&self) -> vk::DeviceSize {
        let min_ubo_align = self.pdevice.basic_props.limits.min_uniform_buffer_offset_alignment;
        let ubo_size = size_of::<T>() as vk::DeviceSize;
        (ubo_size + min_ubo_align - 1) & !(min_ubo_align - 1)
    }

    #[inline]
    pub fn min_ubo_offset_align(&self) -> vk::DeviceSize {
        self.pdevice.basic_props.limits.min_uniform_buffer_offset_alignment
    }

    #[inline]
    pub fn graphics_queue_family(&self) -> RhiQueueFamily {
        self.pdevice.graphics_queue_family.clone()
    }

    #[inline]
    pub fn compute_queue_family(&self) -> RhiQueueFamily {
        self.pdevice.compute_queue_family.clone()
    }

    #[inline]
    pub fn transfer_queue_family(&self) -> RhiQueueFamily {
        self.pdevice.transfer_queue_family.clone()
    }

    #[inline]
    pub fn create_render_pass(&self, render_pass_ci: &vk::RenderPassCreateInfo, debug_name: &str) -> vk::RenderPass {
        let render_pass = unsafe { self.handle.create_render_pass(render_pass_ci, None).unwrap() };
        self.debug_utils.set_object_debug_name(render_pass, debug_name);
        render_pass
    }

    #[inline]
    pub fn create_pipeline_cache(
        &self,
        pipeline_cache_ci: &vk::PipelineCacheCreateInfo,
        debug_name: &str,
    ) -> vk::PipelineCache {
        let pipeline_cache = unsafe { self.handle.create_pipeline_cache(pipeline_cache_ci, None).unwrap() };
        self.debug_utils.set_object_debug_name(pipeline_cache, debug_name);
        pipeline_cache
    }

    #[inline]
    pub fn create_frame_buffer(
        &self,
        frame_buffer_ci: &vk::FramebufferCreateInfo,
        debug_name: &str,
    ) -> vk::Framebuffer {
        let frame_buffer = unsafe { self.handle.create_framebuffer(frame_buffer_ci, None).unwrap() };
        self.debug_utils.set_object_debug_name(frame_buffer, debug_name);
        frame_buffer
    }

    #[inline]
    pub fn write_descriptor_sets(&self, writes: &[RhiWriteDescriptorSet]) {
        let writes = writes.iter().map(|w| w.to_vk_type()).collect_vec();
        unsafe {
            self.handle.update_descriptor_sets(&writes, &[]);
        }
    }
}
