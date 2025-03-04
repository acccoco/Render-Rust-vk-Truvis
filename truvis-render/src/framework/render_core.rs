use std::{
    ffi::CStr,
    sync::{Arc, OnceLock},
};

use ash::vk;
use itertools::Itertools;
use vk_mem::Alloc;

use crate::framework::core::{
    command_buffer::CommandBuffer,
    command_pool::CommandPool,
    debug_utils::DebugUtils,
    device::Device,
    instance::Instance,
    physical_device::PhysicalDevice,
    queue::{Queue, SubmitInfo},
    synchronize::Fence,
};


pub static CORE: OnceLock<Core> = OnceLock::new();

/// Rhi 只需要做到能够创建各种资源的程度就行了
///
/// 与 VulkanSamples 的 VulkanSamle 及 ApiVulkanSample 作用类似
pub struct Core
{
    /// vk 基础函数的接口
    pub vk_pf: ash::Entry,
    instance: Instance,
    // vk_instance: Option<Instance>,

    // vk_debug_util_pf: Option<ash::extensions::ext::DebugUtils>,
    pub vk_dynamic_render_pf: ash::khr::dynamic_rendering::Device,
    pub vk_acceleration_struct_pf: ash::khr::acceleration_structure::Device,

    // vk_debug_util_messenger: Option<vk::DebugUtilsMessengerEXT>,
    physical_device: Arc<PhysicalDevice>,
    pub device: Device,

    pub vma: Option<vk_mem::Allocator>,

    pub descriptor_pool: vk::DescriptorPool,

    pub graphics_command_pool: CommandPool,
    pub transfer_command_pool: CommandPool,
    pub compute_command_pool: CommandPool,

    pub debug_utils: DebugUtils,
}

// init 相关
impl Core
{
    const MAX_VERTEX_BLENDING_MESH_CNT: u32 = 256;
    const MAX_MATERIAL_CNT: u32 = 256;

    const ENGINE_NAME: &'static str = "DruvisIII";

    pub fn new(app_name: String, instance_extra_exts: Vec<&'static CStr>) -> Self
    {
        let vk_pf = unsafe { ash::Entry::load() }.expect("Failed to load vulkan entry");

        let instance = Instance::new(&vk_pf, app_name, Self::ENGINE_NAME.to_string(), instance_extra_exts);

        let pdevice = Arc::new(Self::init_pdevice(&instance.handle));
        let device = Device::new(&instance, pdevice.clone());

        let debug_utils = DebugUtils::new(&vk_pf, &instance.handle, &device.device);

        // 在 device 以及 debug_utils 之前创建的 vk::Handle
        {
            debug_utils.set_object_debug_name(instance.handle.handle(), "instance");
            debug_utils.set_object_debug_name(pdevice.handle, "physical device");

            debug_utils.set_object_debug_name(device.device.handle(), "device");
            debug_utils.set_object_debug_name(device.graphics_queue.vk_queue, "main-graphics-queue");
            debug_utils.set_object_debug_name(device.compute_queue.vk_queue, "main-compute-queue");
            debug_utils.set_object_debug_name(device.transfer_queue.vk_queue, "main-transfer-queue");
        }

        let vk_dynamic_render_pf = ash::khr::dynamic_rendering::Device::new(&instance.handle, &device.device);
        let vk_acceleration_pf = ash::khr::acceleration_structure::Device::new(&instance.handle, &device.device);

        let descriptor_pool = Self::init_descriptor_pool(&device);
        debug_utils.set_object_debug_name(descriptor_pool, "main-descriptor-pool");

        let graphics_command_pool = Self::init_command_pool(
            &device,
            &debug_utils,
            vk::QueueFlags::GRAPHICS,
            vk::CommandPoolCreateFlags::empty(),
            "rhi-graphics-command-pool",
        );
        let compute_command_pool = Self::init_command_pool(
            &device,
            &debug_utils,
            vk::QueueFlags::COMPUTE,
            vk::CommandPoolCreateFlags::empty(),
            "rhi-compute-command-pool",
        );
        let transfer_command_pool = Self::init_command_pool(
            &device,
            &debug_utils,
            vk::QueueFlags::TRANSFER,
            vk::CommandPoolCreateFlags::empty(),
            "rhi-transfer-command-pool",
        );


        let mut rhi = Self {
            vk_pf,
            instance,
            physical_device: pdevice,
            device,
            vk_dynamic_render_pf,
            vk_acceleration_struct_pf: vk_acceleration_pf,
            vma: None,
            descriptor_pool,
            graphics_command_pool,
            transfer_command_pool,
            compute_command_pool,
            debug_utils,
        };

        rhi.init_vma();

        rhi
    }

    fn init_descriptor_pool(device: &Device) -> vk::DescriptorPool
    {
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
            let descriptor_pool = device.device.create_descriptor_pool(&pool_create_info, None).unwrap();
            descriptor_pool
        }
    }

    fn init_pdevice(instance: &ash::Instance) -> PhysicalDevice
    {
        let pdevice = unsafe {
            instance
                .enumerate_physical_devices()
                .unwrap()
                .iter()
                .map(|pdevice| PhysicalDevice::new(*pdevice, instance))
                // 优先使用独立显卡
                .find_or_first(PhysicalDevice::is_descrete_gpu)
                .unwrap()
        };

        pdevice
    }

    /// 由于 vma 恶心的生命周期设定：需要引用 Instance 以及 Device，并确保在其声明周期之内这两个的引用是有效的
    /// 因此需要在 Rhi 的其他部分都初始化完成后再初始化 vma，确保 Instance 和 Device 是 pin 的
    fn init_vma(&mut self)
    {
        let mut vma_ci =
            vk_mem::AllocatorCreateInfo::new(&self.instance.handle, &self.device.device, self.device.pdevice.handle);
        vma_ci.vulkan_api_version = vk::API_VERSION_1_3;
        vma_ci.flags = vk_mem::AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS;

        let vma = unsafe { vk_mem::Allocator::new(vma_ci).unwrap() };
        self.vma = Some(vma);
    }


    /// 仅在初始化阶段使用的一个函数
    pub(super) fn init_command_pool<S: AsRef<str> + Clone>(
        device: &Device,
        debug_utils: &DebugUtils,
        queue_flags: vk::QueueFlags,
        flags: vk::CommandPoolCreateFlags,
        debug_name: S,
    ) -> CommandPool
    {
        let queue_family_index = device.pdevice.find_queue_family_index(queue_flags).unwrap();

        let pool = unsafe {
            device
                .device
                .create_command_pool(
                    &vk::CommandPoolCreateInfo::default().queue_family_index(queue_family_index).flags(flags),
                    None,
                )
                .unwrap()
        };

        debug_utils.set_object_debug_name(pool, debug_name.clone());
        CommandPool {
            command_pool: pool,
            queue_family_index,
        }
    }
}

// 属性访问
impl Core
{
    #[inline]
    pub(crate) fn vk_instance(&self) -> &ash::Instance
    {
        &self.instance.handle
    }

    #[inline]
    pub(crate) fn vk_device(&self) -> &ash::Device
    {
        &self.device.device
    }

    #[inline]
    pub(crate) fn physical_device(&self) -> &PhysicalDevice
    {
        &self.physical_device
    }

    #[inline]
    pub fn compute_queue(&self) -> &Queue
    {
        &self.device.compute_queue
    }

    #[inline]
    pub fn graphics_queue(&self) -> &Queue
    {
        &self.device.graphics_queue
    }

    #[inline]
    pub fn transfer_queue(&self) -> &Queue
    {
        &self.device.transfer_queue
    }

    #[inline]
    pub fn vma(&self) -> &vk_mem::Allocator
    {
        &self.vma.as_ref().unwrap()
    }

    /// 将 UBO 的尺寸和 min_UBO_Offset_Align 对齐，使得得到的尺寸是 min_UBO_Offset_Align 的整数倍
    #[inline]
    pub fn ubo_offset_align(&self, ubo_size: vk::DeviceSize) -> vk::DeviceSize
    {
        let min_ubo_align = self.physical_device().properties.limits.min_uniform_buffer_offset_alignment;
        (ubo_size + min_ubo_align - 1) & !(min_ubo_align - 1)
    }
}

// 工具方法
impl Core
{
    #[inline]
    pub fn create_image<S>(&self, create_info: &vk::ImageCreateInfo, debug_name: S) -> (vk::Image, vk_mem::Allocation)
    where
        S: AsRef<str>,
    {
        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };
        let (image, allocation) = unsafe { self.vma().create_image(create_info, &alloc_info).unwrap() };

        self.set_debug_name(image, debug_name);
        (image, allocation)
    }

    #[inline]
    pub fn create_image_view<S>(&self, create_info: &vk::ImageViewCreateInfo, debug_name: S) -> vk::ImageView
    where
        S: AsRef<str>,
    {
        let view = unsafe { self.vk_device().create_image_view(create_info, None).unwrap() };

        self.set_debug_name(view, debug_name);
        view
    }


    pub(crate) fn find_supported_format(
        &self,
        candidates: &[vk::Format],
        tiling: vk::ImageTiling,
        features: vk::FormatFeatureFlags,
    ) -> Vec<vk::Format>
    {
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
    pub fn reset_command_pool(&self, command_pool: &mut CommandPool)
    {
        unsafe {
            self.vk_device()
                .reset_command_pool(command_pool.command_pool, vk::CommandPoolResetFlags::RELEASE_RESOURCES)
                .unwrap();
        }
    }

    #[inline]
    pub fn wait_for_fence(&self, fence: &Fence)
    {
        unsafe {
            self.vk_device().wait_for_fences(std::slice::from_ref(&fence.fence), true, u64::MAX).unwrap();
        }
    }

    #[inline]
    pub fn reset_fence(&self, fence: &Fence)
    {
        unsafe {
            self.vk_device().reset_fences(std::slice::from_ref(&fence.fence)).unwrap();
        }
    }


    #[inline]
    pub fn graphics_queue_submit_cmds(&self, infos: Vec<CommandBuffer>)
    {
        let cmds = infos.iter().map(|c| c.command_buffer).collect_vec();
        let submit_info = vk::SubmitInfo::default().command_buffers(&cmds);
        unsafe {
            self.vk_device()
                .queue_submit(self.graphics_queue().vk_queue, std::slice::from_ref(&submit_info), vk::Fence::null())
                .unwrap()
        }
    }

    #[inline]
    pub fn graphics_queue_submit(&self, infos: Vec<SubmitInfo>, fence: Option<Fence>)
    {
        // batches 的存在是有必要的，submit_infos 引用的 batches 的内存
        let batches = infos.iter().map(|b| b.to_vk_batch()).collect_vec();
        let submit_infos = batches.iter().map(|b| b.submit_info()).collect_vec();

        unsafe {
            self.vk_device()
                .queue_submit(
                    self.graphics_queue().vk_queue,
                    &submit_infos,
                    fence.map_or(vk::Fence::null(), |f| f.fence),
                )
                .unwrap();
        }
    }

    #[inline]
    pub fn create_render_pass(&self, render_pass_ci: &vk::RenderPassCreateInfo, debug_name: &str) -> vk::RenderPass
    {
        let render_pass = unsafe { self.vk_device().create_render_pass(render_pass_ci, None).unwrap() };
        self.set_debug_name(render_pass, debug_name);
        render_pass
    }

    #[inline]
    pub fn create_pipeline_cache(
        &self,
        pipeline_cache_ci: &vk::PipelineCacheCreateInfo,
        debug_name: &str,
    ) -> vk::PipelineCache
    {
        let pipeline_cache = unsafe { self.vk_device().create_pipeline_cache(pipeline_cache_ci, None).unwrap() };
        self.set_debug_name(pipeline_cache, debug_name);
        pipeline_cache
    }

    #[inline]
    pub fn create_frame_buffer(&self, frame_buffer_ci: &vk::FramebufferCreateInfo, debug_name: &str)
        -> vk::Framebuffer
    {
        let frame_buffer = unsafe { self.vk_device().create_framebuffer(frame_buffer_ci, None).unwrap() };
        self.set_debug_name(frame_buffer, debug_name);
        frame_buffer
    }

    #[inline]
    pub fn create_command_pool<S: AsRef<str> + Clone>(
        &self,
        queue_flags: vk::QueueFlags,
        flags: vk::CommandPoolCreateFlags,
        debug_name: S,
    ) -> CommandPool
    {
        Self::init_command_pool(&self.device, &self.debug_utils, queue_flags, flags, debug_name)
    }

    #[inline]
    pub fn create_sampler(&self, sampler_ci: &vk::SamplerCreateInfo, name: &str) -> vk::Sampler
    {
        let sampler = unsafe { self.vk_device().create_sampler(sampler_ci, None).unwrap() };
        self.set_debug_name(sampler, name);
        sampler
    }

    #[inline]
    pub fn create_descriptor_pool(
        &self,
        descriptor_pool_ci: &vk::DescriptorPoolCreateInfo,
        name: &str,
    ) -> vk::DescriptorPool
    {
        let pool = unsafe { self.vk_device().create_descriptor_pool(descriptor_pool_ci, None).unwrap() };
        self.set_debug_name(pool, name);
        pool
    }

    #[inline]
    pub fn allocate_descriptor_sets(&self, alloc_info: &vk::DescriptorSetAllocateInfo) -> Vec<vk::DescriptorSet>
    {
        unsafe { self.vk_device().allocate_descriptor_sets(alloc_info).unwrap() }
    }

    #[inline]
    pub fn write_descriptor_sets(&self, writes: &[vk::WriteDescriptorSet])
    {
        unsafe {
            self.vk_device().update_descriptor_sets(writes, &[]);
        }
    }
}

// debug label 相关
impl Core
{
    #[inline]
    pub fn set_debug_name<T, S>(&self, handle: T, name: S)
    where
        T: vk::Handle + Copy,
        S: AsRef<str>,
    {
        self.debug_utils.set_object_debug_name(handle, name);
    }

    #[inline]
    pub fn graphics_queue_begin_label(&self, label: &str, color: glam::Vec4)
    {
        self.debug_utils.begin_queue_label(self.device.graphics_queue.vk_queue, label, color);
    }

    #[inline]
    pub fn graphics_queue_end_label(&self)
    {
        self.debug_utils.end_queue_label(self.device.graphics_queue.vk_queue);
    }
}
