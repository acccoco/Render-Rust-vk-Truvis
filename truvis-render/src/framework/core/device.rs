use std::{collections::HashMap, sync::Arc};

use ash::vk;
use itertools::Itertools;

use crate::framework::{
    core::{debug::RhiDebugUtils, instance::RhiInstance, physical_device::RhiPhysicalDevice, queue::RhiQueue},
    rhi::RhiInitInfo,
};

pub struct RhiDevice
{
    pub device: ash::Device,

    pub graphics_queue: RhiQueue,
    pub transfer_queue: RhiQueue,
    pub compute_queue: RhiQueue,

    pub pdevice: Arc<RhiPhysicalDevice>,
}

impl RhiDevice
{
    pub fn new(
        init_info: &mut RhiInitInfo,
        instance: &RhiInstance,
        pdevice: Arc<RhiPhysicalDevice>,
        debug_utils: &RhiDebugUtils,
    ) -> Self
    {
        let graphics_queue_family_index = pdevice.find_queue_family_index(vk::QueueFlags::GRAPHICS).unwrap();
        let compute_queue_family_index = pdevice.find_queue_family_index(vk::QueueFlags::COMPUTE).unwrap();
        let transfer_queue_family_index = pdevice.find_queue_family_index(vk::QueueFlags::TRANSFER).unwrap();

        let mut queues = HashMap::from([
            (graphics_queue_family_index, 0),
            (compute_queue_family_index, 0),
            (transfer_queue_family_index, 0),
        ]);

        // num 表示 “号码”
        let mut graphics_queue_num = 0;
        let mut compute_queue_num = 0;
        let mut transfer_queue_num = 0;
        queues.entry(graphics_queue_family_index).and_modify(|num| {
            graphics_queue_num = *num;
            *num += 1;
        });
        queues.entry(compute_queue_family_index).and_modify(|num| {
            compute_queue_num = *num;
            *num += 1;
        });
        queues.entry(transfer_queue_family_index).and_modify(|num| {
            transfer_queue_num = *num;
            *num += 1;
        });

        // 每个 queue family 的 queue 数量通过 priority 数组的长度指定
        let queue_priorities = queues.values().map(|count| vec![1.0; *count as usize]).collect_vec();
        let queue_create_infos = queues
            .keys()
            .map(|index| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(*index)
                    .queue_priorities(&queue_priorities[*index as usize])
                    .build()
            })
            .collect_vec();

        let device_exts = init_info.device_extensions.iter().map(|e| e.as_ptr()).collect_vec();

        let mut features = vk::PhysicalDeviceFeatures2::builder().features(init_info.core_features).build();
        unsafe {
            init_info.ext_features.iter_mut().for_each(|f| {
                let ptr = <*mut dyn vk::ExtendsPhysicalDeviceFeatures2>::cast::<vk::BaseOutStructure>(f.as_mut());
                (*ptr).p_next = features.p_next as _;
                features.p_next = ptr as _;
            });
        }

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_exts)
            .push_next(&mut features);

        unsafe {
            let device = instance.handle.create_device(pdevice.handle, &device_create_info, None).unwrap();

            let graphics_queue = device.get_device_queue(graphics_queue_family_index, graphics_queue_num);
            let compute_queue = device.get_device_queue(compute_queue_family_index, compute_queue_num);
            let transfer_queue = device.get_device_queue(transfer_queue_family_index, transfer_queue_num);

            debug_utils.set_debug_name(device.handle(), graphics_queue, "graphics-queue");
            debug_utils.set_debug_name(device.handle(), compute_queue, "compute-queue");
            debug_utils.set_debug_name(device.handle(), transfer_queue, "transfer-queue");
            debug_utils.set_debug_name(device.handle(), device.handle(), "device");

            Self {
                device,
                pdevice,
                graphics_queue: RhiQueue {
                    queue: graphics_queue,
                    queue_family_index: graphics_queue_family_index,
                },
                transfer_queue: RhiQueue {
                    queue: transfer_queue,
                    queue_family_index: transfer_queue_family_index,
                },
                compute_queue: RhiQueue {
                    queue: compute_queue,
                    queue_family_index: compute_queue_family_index,
                },
            }
        }
    }
}
