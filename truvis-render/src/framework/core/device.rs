use std::{collections::HashMap, ffi::CStr, ops::Deref, rc::Rc};

use ash::vk;
use itertools::Itertools;

use crate::framework::core::{command_queue::RhiQueue, instance::RhiInstance, physical_device::RhiGpu};

pub struct RhiDevice
{
    pub handle: ash::Device,

    pub pdevice: Rc<RhiGpu>,

    pub graphics_queue_family_index: u32,
    pub compute_queue_family_index: u32,
    pub transfer_queue_family_index: u32,

    pub vk_dynamic_render_pf: Rc<ash::khr::dynamic_rendering::Device>,
    pub vk_acceleration_struct_pf: Rc<ash::khr::acceleration_structure::Device>,
}

impl Deref for RhiDevice
{
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target
    {
        &self.handle
    }
}

impl RhiDevice
{
    /// # return
    /// * (device, graphics queue, compute queue, transfer queue)
    pub fn new(
        instance: &RhiInstance,
        pdevice: Rc<RhiGpu>,
    ) -> (Rc<RhiDevice>, Rc<RhiQueue>, Rc<RhiQueue>, Rc<RhiQueue>)
    {
        let graphics_queue_family_index = pdevice.find_queue_family_index(vk::QueueFlags::GRAPHICS).unwrap();
        let compute_queue_family_index = pdevice.find_queue_family_index(vk::QueueFlags::COMPUTE).unwrap();
        let transfer_queue_family_index = pdevice.find_queue_family_index(vk::QueueFlags::TRANSFER).unwrap();

        // 记录每个 queue family index 应该创建多少个 queue
        // queue family index <-> queue num
        // hash map 会自动去重
        let mut queues = HashMap::from([
            (graphics_queue_family_index, 0),
            (compute_queue_family_index, 0),
            (transfer_queue_family_index, 0),
        ]);

        // 计算得到每个 queue 在同类 queue family 中的 index，用于从 device 中取出 queue
        let mut graphics_queue_index = 0;
        let mut compute_queue_index = 0;
        let mut transfer_queue_index = 0;
        queues.entry(graphics_queue_family_index).and_modify(|num| {
            graphics_queue_index = *num;
            *num += 1;
        });
        queues.entry(compute_queue_family_index).and_modify(|num| {
            compute_queue_index = *num;
            *num += 1;
        });
        queues.entry(transfer_queue_family_index).and_modify(|num| {
            transfer_queue_index = *num;
            *num += 1;
        });

        // 每个 queue family 的 queue 数量和 priority 数组长度保持一直
        let queue_priorities =
            queues.values().map(|count| vec![1.0 /* priority = 1.0 */; *count as usize]).collect_vec();
        let queue_create_infos = queues
            .keys()
            .map(|index| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(*index)
                    .queue_priorities(&queue_priorities[*index as usize])
            })
            .collect_vec();

        let device_exts = Self::basic_device_exts().iter().map(|e| e.as_ptr()).collect_vec();
        log::info!("device exts:");
        for ext in &device_exts {
            log::info!("\t{:?}", unsafe { CStr::from_ptr(*ext) });
        }

        let mut features = vk::PhysicalDeviceFeatures2::default().features(Self::basic_gpu_core_features());
        let mut gpu_ext_features = Self::basic_gpu_ext_features();
        unsafe {
            gpu_ext_features.iter_mut().for_each(|f| {
                let ptr = <*mut dyn vk::ExtendsPhysicalDeviceFeatures2>::cast::<vk::BaseOutStructure>(f.as_mut());
                (*ptr).p_next = features.p_next as _;
                features.p_next = ptr as _;
            });
        }

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_exts)
            .push_next(&mut features);

        unsafe {
            let device = instance.handle.create_device(pdevice.handle, &device_create_info, None).unwrap();

            let vk_dynamic_render_pf = Rc::new(ash::khr::dynamic_rendering::Device::new(&instance.handle, &device));
            let vk_acceleration_struct_pf =
                Rc::new(ash::khr::acceleration_structure::Device::new(&instance.handle, &device));

            let device = Rc::new(Self {
                handle: device,
                pdevice: pdevice.clone(),

                graphics_queue_family_index,
                compute_queue_family_index,
                transfer_queue_family_index,

                vk_dynamic_render_pf,
                vk_acceleration_struct_pf,
            });

            let graphics_queue = device.get_device_queue(graphics_queue_family_index, graphics_queue_index);
            let compute_queue = device.get_device_queue(compute_queue_family_index, compute_queue_index);
            let transfer_queue = device.get_device_queue(transfer_queue_family_index, transfer_queue_index);

            (
                device.clone(),
                Rc::new(RhiQueue {
                    handle: graphics_queue,
                    queue_family_index: graphics_queue_family_index,
                    device: device.clone(),
                }),
                Rc::new(RhiQueue {
                    handle: compute_queue,
                    queue_family_index: compute_queue_family_index,
                    device: device.clone(),
                }),
                Rc::new(RhiQueue {
                    handle: transfer_queue,
                    queue_family_index: transfer_queue_family_index,
                    device: device.clone(),
                }),
            )
        }
    }

    /// 必要的 physical device core features
    fn basic_gpu_core_features() -> vk::PhysicalDeviceFeatures
    {
        vk::PhysicalDeviceFeatures::default()
            .sampler_anisotropy(true)
            .fragment_stores_and_atomics(true)
            .independent_blend(true)
    }

    /// 必要的 physical device extension features
    fn basic_gpu_ext_features() -> Vec<Box<dyn vk::ExtendsPhysicalDeviceFeatures2>>
    {
        vec![
            Box::new(vk::PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true)),
            Box::new(vk::PhysicalDeviceBufferDeviceAddressFeatures::default().buffer_device_address(true)),
            Box::new(vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default().ray_tracing_pipeline(true)),
            Box::new(vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default().acceleration_structure(true)),
            Box::new(vk::PhysicalDeviceHostQueryResetFeatures::default().host_query_reset(true)),
            Box::new(vk::PhysicalDeviceSynchronization2Features::default().synchronization2(true)),
        ]
    }

    /// 必要的 device extensions
    fn basic_device_exts() -> Vec<&'static CStr>
    {
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
