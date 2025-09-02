use std::ffi::CStr;

use ash::vk;

use crate::{
    commands::command_queue::CommandQueue,
    foundation::{debug_messenger::DebugMsger, device::Device, instance::Instance, physical_device::PhysicalDevice},
};

pub struct VulkanCore
{
    /// vk 基础函数的接口
    ///
    /// 在 drop 之后，会卸载 dll，因此需要确保该字段最后 drop
    pub(crate) vk_pf: ash::Entry,

    pub(crate) instance: Instance,
    pub(crate) physical_device: PhysicalDevice,
    pub(crate) device: Device,

    pub(crate) debug_utils: DebugMsger,

    pub(crate) graphics_queue: CommandQueue,
    pub(crate) compute_queue: CommandQueue,
    pub(crate) transfer_queue: CommandQueue,
}

/// 创建与销毁
impl VulkanCore
{
    pub fn new(app_name: String, engine_name: String, instance_extra_exts: Vec<&'static CStr>) -> Self
    {
        let vk_pf = unsafe { ash::Entry::load() }.expect("Failed to load vulkan entry");
        let instance = Instance::new(&vk_pf, app_name, engine_name, instance_extra_exts);
        let physical_device = PhysicalDevice::new_descrete_physical_device(instance.ash_instance());

        // graphics, compute, transfer 各创建一个
        let queue_create_infos = [
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(physical_device.graphics_queue_family.queue_family_index)
                .queue_priorities(&[1.0]),
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(physical_device.compute_queue_family.queue_family_index)
                .queue_priorities(&[1.0]),
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(physical_device.transfer_queue_family.queue_family_index)
                .queue_priorities(&[1.0]),
        ];

        let device = Device::new(&instance.ash_instance, physical_device.vk_handle, &queue_create_infos);
        let graphics_queue = CommandQueue {
            vk_queue: unsafe {
                device.functions.get_device_queue(physical_device.graphics_queue_family.queue_family_index, 0)
            },
            queue_family: physical_device.graphics_queue_family.clone(),
            device_functions: device.functions.clone(),
        };
        let compute_queue = CommandQueue {
            vk_queue: unsafe {
                device.functions.get_device_queue(physical_device.compute_queue_family.queue_family_index, 0)
            },
            queue_family: physical_device.compute_queue_family.clone(),
            device_functions: device.functions.clone(),
        };
        let transfer_queue = CommandQueue {
            vk_queue: unsafe {
                device.functions.get_device_queue(physical_device.transfer_queue_family.queue_family_index, 0)
            },
            queue_family: physical_device.transfer_queue_family.clone(),
            device_functions: device.functions.clone(),
        };

        let debug_utils = DebugMsger::new(&vk_pf, &instance.ash_instance);

        log::info!("graphics queue's queue family:\n{:#?}", graphics_queue.queue_family);
        log::info!("compute queue's queue family:\n{:#?}", compute_queue.queue_family);
        log::info!("transfer queue's queue family:\n{:#?}", transfer_queue.queue_family);

        // 在 device 以及 debug_utils 之前创建的 vk::Handle
        {
            device.functions.set_object_debug_name(instance.vk_instance(), "RhiInstance");
            device.functions.set_object_debug_name(physical_device.vk_handle, "RhiPhysicalDevice");

            device.functions.set_object_debug_name(device.vk_handle(), "RhiDevice");
            device.functions.set_object_debug_name(graphics_queue.vk_queue, "graphics_queue");
            device.functions.set_object_debug_name(compute_queue.vk_queue, "compute_queue");
            device.functions.set_object_debug_name(transfer_queue.vk_queue, "transfer_queue");
        }

        Self {
            vk_pf,
            instance,
            physical_device,
            device,
            debug_utils,
            graphics_queue,
            compute_queue,
            transfer_queue,
        }
    }

    pub fn destroy(self)
    {
        self.debug_utils.destroy();
        self.device.destroy();
        self.physical_device.destroy();
        self.instance.destroy();
    }
}
