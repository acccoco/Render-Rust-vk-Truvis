use std::{ffi::CStr, rc::Rc};

use crate::core::descriptor_pool::{RhiDescriptorPool, RhiDescriptorPoolCreateInfo};
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
    pub vk_pf: ash::Entry,
    instance: Rc<RhiInstance>,
    physical_device: Rc<RhiPhysicalDevice>,
    pub device: Rc<RhiDevice>,

    pub allocator: Rc<RhiAllocator>,

    pub graphics_command_pool: Rc<RhiCommandPool>,
    pub transfer_command_pool: Rc<RhiCommandPool>,
    pub compute_command_pool: Rc<RhiCommandPool>,

    pub graphics_queue: Rc<RhiQueue>,
    pub compute_queue: Rc<RhiQueue>,
    pub transfer_queue: Rc<RhiQueue>,

    descriptor_pool: RhiDescriptorPool,
}

const DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT: u32 = 256;
const DESCRIPTOR_POOL_MAX_MATERIAL_CNT: u32 = 256;
const DESCRIPTOR_POOL_MAX_BINDLESS_TEXTURE_CNT: u32 = 128;

// init 相关
impl Rhi {
    const ENGINE_NAME: &'static str = "DruvisIII";

    pub fn new(app_name: String, instance_extra_exts: Vec<&'static CStr>) -> Self {
        let vk_pf = unsafe { ash::Entry::load() }.expect("Failed to load vulkan entry");

        let instance = Rc::new(RhiInstance::new(&vk_pf, app_name, Self::ENGINE_NAME.to_string(), instance_extra_exts));

        let physical_device = Rc::new(RhiPhysicalDevice::new_descrete_physical_device(&instance.handle));
        let (device, graphics_queue, compute_queue, transfer_queue) =
            RhiDevice::new(&vk_pf, &instance, physical_device.clone());

        // 在 device 以及 debug_utils 之前创建的 vk::Handle
        {
            device.debug_utils.set_object_debug_name(instance.handle.handle(), "instance");
            device.debug_utils.set_object_debug_name(physical_device.handle, "physical device");

            device.debug_utils.set_object_debug_name(device.handle.handle(), "device");
            device.debug_utils.set_object_debug_name(graphics_queue.handle, "main-graphics-queue");
            device.debug_utils.set_object_debug_name(compute_queue.handle, "main-compute-queue");
            device.debug_utils.set_object_debug_name(transfer_queue.handle, "main-transfer-queue");
        }

        let graphics_command_pool = Rc::new(RhiCommandPool::new_before_rhi(
            device.clone(),
            device.graphics_queue_family_index,
            vk::CommandPoolCreateFlags::empty(),
            "rhi-graphics-command-pool",
        ));
        let compute_command_pool = Rc::new(RhiCommandPool::new_before_rhi(
            device.clone(),
            device.compute_queue_family_index,
            vk::CommandPoolCreateFlags::empty(),
            "rhi-compute-command-pool",
        ));
        let transfer_command_pool = Rc::new(RhiCommandPool::new_before_rhi(
            device.clone(),
            device.transfer_queue_family_index,
            vk::CommandPoolCreateFlags::empty(),
            "rhi-transfer-command-pool",
        ));

        let allocator = Rc::new(RhiAllocator::new(instance.clone(), physical_device.clone(), device.clone()));

        let descriptor_pool = Rhi::init_descriptor_pool(device.clone());

        Self {
            vk_pf,
            instance,
            physical_device,
            device,
            allocator,
            graphics_command_pool,
            transfer_command_pool,
            compute_command_pool,
            graphics_queue,
            compute_queue,
            transfer_queue,
            descriptor_pool,
        }
    }

    fn init_descriptor_pool(device: Rc<RhiDevice>) -> RhiDescriptorPool {
        let pool_size = vec![
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
                descriptor_count: 128,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: DESCRIPTOR_POOL_MAX_MATERIAL_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: DESCRIPTOR_POOL_MAX_MATERIAL_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::INPUT_ATTACHMENT,
                descriptor_count: 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
                descriptor_count: 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: DESCRIPTOR_POOL_MAX_BINDLESS_TEXTURE_CNT + 32,
            },
        ];

        let pool_ci = Rc::new(RhiDescriptorPoolCreateInfo::new(
            vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET | vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
            DESCRIPTOR_POOL_MAX_MATERIAL_CNT + DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT + 32,
            pool_size,
        ));

        RhiDescriptorPool::new(device, pool_ci, "ctx-descriptor-pool")
    }
}

// 属性访问
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
    pub fn descriptor_pool(&self) -> &RhiDescriptorPool {
        &self.descriptor_pool
    }
}

// 工具方法
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
