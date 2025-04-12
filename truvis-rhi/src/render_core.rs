use std::{ffi::CStr, rc::Rc};

use crate::core::{
    allocator::RhiAllocator, command_pool::RhiCommandPool, command_queue::RhiQueue, debug_utils::RhiDebugUtils,
    device::RhiDevice, instance::RhiInstance, physical_device::RhiGpu,
};
use crate::shader_cursor::RhiWriteDescriptorSet;
use ash::vk;
use itertools::Itertools;

/// Rhi 只需要做到能够创建各种资源的程度就行了
///
/// 与 VulkanSamples 的 VulkanSamle 及 ApiVulkanSample 作用类似
pub struct Rhi {
    /// vk 基础函数的接口
    pub vk_pf: ash::Entry,
    instance: Rc<RhiInstance>,
    physical_device: Rc<RhiGpu>,
    pub device: Rc<RhiDevice>,

    pub allocator: Rc<RhiAllocator>,

    pub descriptor_pool: vk::DescriptorPool,

    pub graphics_command_pool: Rc<RhiCommandPool>,
    pub transfer_command_pool: Rc<RhiCommandPool>,
    pub compute_command_pool: Rc<RhiCommandPool>,

    pub debug_utils: Rc<RhiDebugUtils>,

    pub graphics_queue: Rc<RhiQueue>,
    pub compute_queue: Rc<RhiQueue>,
    pub transfer_queue: Rc<RhiQueue>,
}

// init 相关
impl Rhi {
    const MAX_VERTEX_BLENDING_MESH_CNT: u32 = 256;
    const MAX_MATERIAL_CNT: u32 = 256;

    const ENGINE_NAME: &'static str = "DruvisIII";

    pub fn new(app_name: String, instance_extra_exts: Vec<&'static CStr>) -> Self {
        let vk_pf = unsafe { ash::Entry::load() }.expect("Failed to load vulkan entry");

        let instance = Rc::new(RhiInstance::new(&vk_pf, app_name, Self::ENGINE_NAME.to_string(), instance_extra_exts));

        let pdevice = Rc::new(Self::init_pdevice(&instance.handle));
        let (device, graphics_queue, compute_queue, transfer_queue) = RhiDevice::new(&instance, pdevice.clone());

        let debug_utils = Rc::new(RhiDebugUtils::new(&vk_pf, &instance.handle, &device.handle));

        // 在 device 以及 debug_utils 之前创建的 vk::Handle
        {
            debug_utils.set_object_debug_name(instance.handle.handle(), "instance");
            debug_utils.set_object_debug_name(pdevice.handle, "physical device");

            debug_utils.set_object_debug_name(device.handle.handle(), "device");
            debug_utils.set_object_debug_name(graphics_queue.handle, "main-graphics-queue");
            debug_utils.set_object_debug_name(compute_queue.handle, "main-compute-queue");
            debug_utils.set_object_debug_name(transfer_queue.handle, "main-transfer-queue");
        }

        let descriptor_pool = Self::init_descriptor_pool(&device);
        debug_utils.set_object_debug_name(descriptor_pool, "main-descriptor-pool");

        let graphics_command_pool = Rc::new(RhiCommandPool::new_before_rhi(
            device.clone(),
            device.graphics_queue_family_index,
            vk::CommandPoolCreateFlags::empty(),
            debug_utils.clone(),
            "rhi-graphics-command-pool",
        ));
        let compute_command_pool = Rc::new(RhiCommandPool::new_before_rhi(
            device.clone(),
            device.compute_queue_family_index,
            vk::CommandPoolCreateFlags::empty(),
            debug_utils.clone(),
            "rhi-compute-command-pool",
        ));
        let transfer_command_pool = Rc::new(RhiCommandPool::new_before_rhi(
            device.clone(),
            device.transfer_queue_family_index,
            vk::CommandPoolCreateFlags::empty(),
            debug_utils.clone(),
            "rhi-transfer-command-pool",
        ));

        let allocator = Rc::new(RhiAllocator::new(instance.clone(), pdevice.clone(), device.clone()));

        Self {
            vk_pf,
            instance,
            physical_device: pdevice,
            device,
            allocator,
            descriptor_pool,
            graphics_command_pool,
            transfer_command_pool,
            compute_command_pool,
            graphics_queue,
            compute_queue,
            transfer_queue,
            debug_utils,
        }
    }

    fn init_descriptor_pool(device: &RhiDevice) -> vk::DescriptorPool {
        let pool_size = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
                descriptor_count: 128,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: Self::MAX_VERTEX_BLENDING_MESH_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: Self::MAX_MATERIAL_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: Self::MAX_MATERIAL_CNT + 32,
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
                descriptor_count: 32,
            },
        ];

        let pool_create_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_size)
            .max_sets(Self::MAX_MATERIAL_CNT + Self::MAX_VERTEX_BLENDING_MESH_CNT + 32);

        unsafe {
            let descriptor_pool = device.handle.create_descriptor_pool(&pool_create_info, None).unwrap();
            descriptor_pool
        }
    }

    fn init_pdevice(instance: &ash::Instance) -> RhiGpu {
        let pdevice = unsafe {
            instance
                .enumerate_physical_devices()
                .unwrap()
                .iter()
                .map(|pdevice| RhiGpu::new(*pdevice, instance))
                // 优先使用独立显卡
                .find_or_first(RhiGpu::is_descrete_gpu)
                .unwrap()
        };

        pdevice
    }
}

// 属性访问
impl Rhi {
    #[inline]
    pub(crate) fn vk_instance(&self) -> &ash::Instance {
        &self.instance.handle
    }

    #[inline]
    pub(crate) fn vk_device(&self) -> &ash::Device {
        &self.device.handle
    }

    #[inline]
    pub(crate) fn physical_device(&self) -> &RhiGpu {
        &self.physical_device
    }

    /// 将 UBO 的尺寸和 min_UBO_Offset_Align 对齐，使得得到的尺寸是 min_UBO_Offset_Align 的整数倍
    #[inline]
    pub fn ubo_offset_align(&self, ubo_size: vk::DeviceSize) -> vk::DeviceSize {
        let min_ubo_align = self.physical_device().properties.limits.min_uniform_buffer_offset_alignment;
        (ubo_size + min_ubo_align - 1) & !(min_ubo_align - 1)
    }
}

// TODO 区分一下 Rhi 的定位。以下的方法放在 Device 里面似乎也是 ok 的
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
                    self.vk_instance().get_physical_device_format_properties(self.physical_device().handle, **f)
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

    #[inline]
    pub fn create_render_pass(&self, render_pass_ci: &vk::RenderPassCreateInfo, debug_name: &str) -> vk::RenderPass {
        let render_pass = unsafe { self.device.create_render_pass(render_pass_ci, None).unwrap() };
        self.set_debug_name(render_pass, debug_name);
        render_pass
    }

    #[inline]
    pub fn create_pipeline_cache(
        &self,
        pipeline_cache_ci: &vk::PipelineCacheCreateInfo,
        debug_name: &str,
    ) -> vk::PipelineCache {
        let pipeline_cache = unsafe { self.device.create_pipeline_cache(pipeline_cache_ci, None).unwrap() };
        self.set_debug_name(pipeline_cache, debug_name);
        pipeline_cache
    }

    #[inline]
    pub fn create_frame_buffer(
        &self,
        frame_buffer_ci: &vk::FramebufferCreateInfo,
        debug_name: &str,
    ) -> vk::Framebuffer {
        let frame_buffer = unsafe { self.device.create_framebuffer(frame_buffer_ci, None).unwrap() };
        self.set_debug_name(frame_buffer, debug_name);
        frame_buffer
    }

    // TODO 放到 descriptor 里面去
    // TODO 抽象出 RhiDescriptorSet
    #[inline]
    pub fn allocate_descriptor_sets(&self, alloc_info: &vk::DescriptorSetAllocateInfo) -> Vec<vk::DescriptorSet> {
        unsafe { self.vk_device().allocate_descriptor_sets(alloc_info).unwrap() }
    }

    // FIXME remove me
    #[inline]
    pub fn write_descriptor_sets(&self, writes: &[vk::WriteDescriptorSet]) {
        unsafe {
            self.vk_device().update_descriptor_sets(writes, &[]);
        }
    }

    #[inline]
    pub fn write_descriptor_sets2(&self, writes: &[RhiWriteDescriptorSet]) {
        let writes = writes.iter().map(|w| w.to_vk_type()).collect_vec();
        unsafe {
            self.vk_device().update_descriptor_sets(&writes, &[]);
        }
    }
}

// debug label 相关
impl Rhi {
    #[inline]
    pub fn set_debug_name<T, S>(&self, handle: T, name: S)
    where
        T: vk::Handle + Copy,
        S: AsRef<str>,
    {
        self.debug_utils.set_object_debug_name(handle, name);
    }
}
