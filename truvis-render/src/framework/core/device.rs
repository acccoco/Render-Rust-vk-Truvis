use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use ash::vk;
use itertools::Itertools;

use crate::framework::{
    core::{
        command_pool::RhiCommandPool,
        debug::RhiDebugUtils,
        fence_pool::FencePool,
        instance::RhiInstance,
        physical_device::RhiPhysicalDevice,
        queue::RhiQueue,
        vulkan_resource::{IVulkanResource, VulkanResource},
    },
    rhi::RhiInitInfo,
};

pub struct Device
{
    inner_resource: VulkanResource<vk::Device>,

    gpu: Arc<RhiPhysicalDevice>,

    surface: vk::SurfaceKHR,

    debug_utils: RhiDebugUtils,

    enabled_extensions: Vec<String>,

    queues: Vec<Vec<RhiQueue>>,

    command_pool: RhiCommandPool,

    fence_pool: FencePool,
}

impl Device
{
    pub fn get_debug_utils(&self) -> &RhiDebugUtils
    {
        &self.debug_utils
    }
}

impl IVulkanResource for Device
{
    type Handle = vk::Device;

    fn get_inner_resource(&self) -> &VulkanResource<Self::Handle>
    {
        &self.inner_resource
    }
    fn get_inner_resource_mut(&mut self) -> &mut VulkanResource<vk::Device>
    {
        &mut self.inner_resource
    }
}


static DEVICE: OnceLock<ash::Device> = OnceLock::new();


pub struct RhiDevice
{
    pub graphics_queue: RhiQueue,
    pub transfer_queue: RhiQueue,
    pub compute_queue: RhiQueue,

    pub pdevice: Arc<RhiPhysicalDevice>,

    pub debug_utils: RhiDebugUtils,
}

impl RhiDevice
{
    pub fn old_new(
        vk_pf: &ash::Entry,
        init_info: &mut RhiInitInfo,
        instance: &RhiInstance,
        pdevice: Arc<RhiPhysicalDevice>,
    ) -> anyhow::Result<Self>
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

        let debug_utils = RhiDebugUtils::new(vk_pf, instance.get_handle(), init_info);

        unsafe {
            let device = instance.handle.create_device(pdevice.handle, &device_create_info, None).unwrap();

            let graphics_queue = device.get_device_queue(graphics_queue_family_index, graphics_queue_num);
            let compute_queue = device.get_device_queue(compute_queue_family_index, compute_queue_num);
            let transfer_queue = device.get_device_queue(transfer_queue_family_index, transfer_queue_num);

            DEVICE.get_or_init(|| device);

            // TODO
            // self.set_debug_name(graphics_queue, "graphics-queue");
            // self.set_debug_name(compute_queue, "compute-queue");
            // self.set_debug_name(transfer_queue, "transfer-queue");

            Ok(Self {
                pdevice,
                debug_utils,
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
            })
        }
    }

    pub fn device() -> &'static ash::Device
    {
        DEVICE.get().unwrap()
    }
}
