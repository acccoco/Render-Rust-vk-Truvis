use std::{ffi::CStr, rc::Rc};

use crate::core::command_queue::RhiQueueFamily;
use crate::core::{
    allocator::RhiAllocator, command_pool::RhiCommandPool, command_queue::RhiQueue, device::RhiDevice,
    instance::RhiInstance, physical_device::RhiPhysicalDevice,
};
use ash::vk;

/// Rhi 只需要做到能够创建各种资源的程度就行了
///
/// 与 VulkanSamples 的 VulkanSamle 及 ApiVulkanSample 作用类似
pub struct Rhi {
    /// vk 基础函数的接口
    ///
    /// 在 drop 之后，会卸载 dll，因此需要确保该字段最后 drop
    pub vk_pf: Rc<ash::Entry>,
    instance: Rc<RhiInstance>,
    physical_device: Rc<RhiPhysicalDevice>,
    pub device: Rc<RhiDevice>,

    pub allocator: Rc<RhiAllocator>,

    /// 临时的 graphics command pool，主要用于临时的命令缓冲区
    pub temp_graphics_command_pool: Rc<RhiCommandPool>,

    pub graphics_queue: Rc<RhiQueue>,
    pub compute_queue: Rc<RhiQueue>,
    pub transfer_queue: Rc<RhiQueue>,
}

impl Drop for Rhi {
    fn drop(&mut self) {
        log::info!("destroy rhi.");
    }
}

// init
impl Rhi {
    // region init 相关
    const ENGINE_NAME: &'static str = "DruvisIII";

    pub fn new(app_name: String, instance_extra_exts: Vec<&'static CStr>) -> Self {
        let vk_pf = Rc::new(unsafe { ash::Entry::load() }.expect("Failed to load vulkan entry"));

        let instance =
            Rc::new(RhiInstance::new(vk_pf.clone(), app_name, Self::ENGINE_NAME.to_string(), instance_extra_exts));

        let physical_device = Rc::new(RhiPhysicalDevice::new_descrete_physical_device(instance.handle()));

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

        let device =
            Rc::new(RhiDevice::new(vk_pf.clone(), instance.clone(), physical_device.clone(), &queue_create_infos));

        let graphics_queue = Rc::new(RhiQueue {
            handle: unsafe { device.get_device_queue(physical_device.graphics_queue_family.queue_family_index, 0) },
            queue_family: physical_device.graphics_queue_family.clone(),
            device: device.clone(),
        });
        let compute_queue = Rc::new(RhiQueue {
            handle: unsafe { device.get_device_queue(physical_device.compute_queue_family.queue_family_index, 0) },
            queue_family: physical_device.compute_queue_family.clone(),
            device: device.clone(),
        });
        let transfer_queue = Rc::new(RhiQueue {
            handle: unsafe { device.get_device_queue(physical_device.transfer_queue_family.queue_family_index, 0) },
            queue_family: physical_device.transfer_queue_family.clone(),
            device: device.clone(),
        });

        log::info!("graphics queue's queue family:\n{:#?}", graphics_queue.queue_family);
        log::info!("compute queue's queue family:\n{:#?}", compute_queue.queue_family);
        log::info!("transfer queue's queue family:\n{:#?}", transfer_queue.queue_family);

        // 在 device 以及 debug_utils 之前创建的 vk::Handle
        {
            device.debug_utils().set_debug_name(instance.as_ref(), "main");
            device.debug_utils().set_debug_name(physical_device.as_ref(), "main");

            device.debug_utils().set_debug_name(device.as_ref(), "main");
            device.debug_utils().set_debug_name(graphics_queue.as_ref(), "graphics");
            device.debug_utils().set_debug_name(compute_queue.as_ref(), "compute");
            device.debug_utils().set_debug_name(transfer_queue.as_ref(), "transfer");
        }

        let graphics_command_pool = Rc::new(RhiCommandPool::new(
            device.clone(),
            physical_device.graphics_queue_family.clone(),
            vk::CommandPoolCreateFlags::empty(),
            "rhi-graphics",
        ));

        let allocator = Rc::new(RhiAllocator::new(instance.clone(), physical_device.clone(), device.clone()));

        Self {
            vk_pf,
            instance,
            physical_device,
            device,
            allocator,
            temp_graphics_command_pool: graphics_command_pool,
            graphics_queue,
            compute_queue,
            transfer_queue,
        }
    }
}

// getter
impl Rhi {
    #[inline]
    pub fn instance(&self) -> &RhiInstance {
        &self.instance
    }

    #[inline]
    pub fn device(&self) -> &RhiDevice {
        &self.device
    }

    #[inline]
    pub fn physical_device(&self) -> &RhiPhysicalDevice {
        &self.physical_device
    }

    #[inline]
    pub fn graphics_queue_family(&self) -> RhiQueueFamily {
        self.physical_device.graphics_queue_family.clone()
    }

    #[inline]
    pub fn compute_queue_family(&self) -> RhiQueueFamily {
        self.physical_device.compute_queue_family.clone()
    }

    #[inline]
    pub fn transfer_queue_family(&self) -> RhiQueueFamily {
        self.physical_device.transfer_queue_family.clone()
    }
}

// tools
impl Rhi {
    /// 根据给定的格式，返回支持的格式
    pub fn find_supported_format(
        &self,
        candidates: &[vk::Format],
        tiling: vk::ImageTiling,
        features: vk::FormatFeatureFlags,
    ) -> Vec<vk::Format> {
        candidates
            .iter()
            .filter(|f| {
                let props = unsafe {
                    self.instance().get_physical_device_format_properties(self.physical_device().handle, **f)
                };
                match tiling {
                    vk::ImageTiling::LINEAR => props.linear_tiling_features.contains(features),
                    vk::ImageTiling::OPTIMAL => props.optimal_tiling_features.contains(features),
                    _ => panic!("not supported tiling."),
                }
            })
            .copied()
            .collect()
    }
}
impl Rhi {}
