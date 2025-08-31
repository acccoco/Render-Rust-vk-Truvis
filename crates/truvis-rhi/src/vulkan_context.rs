use crate::core::command_queue::RhiQueue;
use crate::core::debug_utils::RhiDebugUtils;
use crate::core::device::RhiDevice;
use crate::core::instance::RhiInstance;
use crate::core::physical_device::RhiPhysicalDevice;
use ash::vk;
use std::ffi::CStr;

pub struct VulkanContext {
    /// vk 基础函数的接口
    ///
    /// 在 drop 之后，会卸载 dll，因此需要确保该字段最后 drop
    pub(crate) vk_pf: ash::Entry,

    pub(crate) instance: RhiInstance,
    pub(crate) physical_device: RhiPhysicalDevice,
    pub(crate) device: RhiDevice,

    pub(crate) debug_utils: RhiDebugUtils,

    pub(crate) graphics_queue: RhiQueue,
    pub(crate) compute_queue: RhiQueue,
    pub(crate) transfer_queue: RhiQueue,
}

/// 创建与销毁
impl VulkanContext {
    pub fn new(app_name: String, engine_name: String, instance_extra_exts: Vec<&'static CStr>) -> Self {
        let vk_pf = unsafe { ash::Entry::load() }.expect("Failed to load vulkan entry");
        let instance = RhiInstance::new(&vk_pf, app_name, engine_name, instance_extra_exts);
        let physical_device = RhiPhysicalDevice::new_descrete_physical_device(instance.ash_instance());

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

        let device = RhiDevice::new(&instance.ash_instance, physical_device.vk_handle, &queue_create_infos);
        let graphics_queue = RhiQueue {
            vk_queue: unsafe { device.get_device_queue(physical_device.graphics_queue_family.queue_family_index, 0) },
            queue_family: physical_device.graphics_queue_family.clone(),
        };
        let compute_queue = RhiQueue {
            vk_queue: unsafe { device.get_device_queue(physical_device.compute_queue_family.queue_family_index, 0) },
            queue_family: physical_device.compute_queue_family.clone(),
        };
        let transfer_queue = RhiQueue {
            vk_queue: unsafe { device.get_device_queue(physical_device.transfer_queue_family.queue_family_index, 0) },
            queue_family: physical_device.transfer_queue_family.clone(),
        };

        let debug_utils = RhiDebugUtils::new(&vk_pf, &instance.ash_instance, &device.ash_device);

        log::info!("graphics queue's queue family:\n{:#?}", graphics_queue.queue_family);
        log::info!("compute queue's queue family:\n{:#?}", compute_queue.queue_family);
        log::info!("transfer queue's queue family:\n{:#?}", transfer_queue.queue_family);

        // 在 device 以及 debug_utils 之前创建的 vk::Handle
        {
            debug_utils.set_object_debug_name(instance.vk_instance(), "RhiInstance");
            debug_utils.set_object_debug_name(physical_device.vk_handle, "RhiPhysicalDevice");

            debug_utils.set_object_debug_name(device.vk_handle(), "RhiDevice");
            debug_utils.set_object_debug_name(graphics_queue.vk_queue, "graphics_queue");
            debug_utils.set_object_debug_name(compute_queue.vk_queue, "compute_queue");
            debug_utils.set_object_debug_name(transfer_queue.vk_queue, "transfer_queue");
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

    pub fn destroy(self) {
        self.debug_utils.destroy();
        self.device.destroy();
        self.physical_device.destroy();
        self.instance.destroy();
    }
}
