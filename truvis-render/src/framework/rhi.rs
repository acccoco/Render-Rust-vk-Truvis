use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    rc::Rc,
};

use anyhow::Context;
use ash::{extensions::khr::Swapchain, vk, Device, Entry, Instance};
use itertools::Itertools;
use raw_window_handle::HasRawDisplayHandle;
use vk_mem::Alloc;

use crate::framework::{
    core::{command_pool::RhiCommandPool, physical_device::RhiPhysicalDevice, queue::RhiQueue},
    platform::window_system::WindowSystem,
};

static mut RHI: Option<Rhi> = None;

/// # Safety
/// very safe
pub unsafe extern "system" fn vk_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32
{
    let callback_data = *p_callback_data;

    let msg = if callback_data.p_message.is_null() {
        std::borrow::Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };


    // 按照 | 切分 msg 字符串，并在中间插入换行符
    let msg = msg.split('|').collect::<Vec<&str>>().join("\n");
    let msg = msg.split(" ] ").collect::<Vec<&str>>().join(" ]\n ");
    let format_msg = format!("[{:?}]\n {}\n", message_type, msg);

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            log::error!("{}", format_msg);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            log::warn!("{}", format_msg);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            log::info!("{}", format_msg);
        }
        _ => log::info!("{}", format_msg),
    };

    // 只有 layer developer 才需要返回 True
    vk::FALSE
}

pub struct RhiInitInfo
{
    pub app_name: Option<String>,
    pub engine_name: Option<String>,

    pub vk_version: u32,

    pub instance_layers: Vec<&'static CStr>,
    pub instance_extensions: Vec<&'static CStr>,
    pub instance_create_flags: vk::InstanceCreateFlags,
    pub device_extensions: Vec<&'static CStr>,

    pub core_features: vk::PhysicalDeviceFeatures,
    pub ext_features: Vec<Box<dyn vk::ExtendsPhysicalDeviceFeatures2>>,

    pub debug_msg_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    pub debug_msg_type: vk::DebugUtilsMessageTypeFlagsEXT,
    pub debug_callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT,

    pub frames_in_flight: u32,
}


impl RhiInitInfo
{
    const VALIDATION_LAYER_NAME: &'static CStr = cstr::cstr!("VK_LAYER_KHRONOS_validation");

    pub fn init_basic(debug_callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT) -> Self
    {
        let instance_create_flags = if cfg!(target_os = "macos") {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::empty()
        };

        let mut info = Self {
            app_name: None,
            engine_name: None,

            // 版本过低时，有些函数无法正确加载
            vk_version: vk::API_VERSION_1_3,

            instance_layers: Self::basic_instance_layers(),
            instance_extensions: Self::basic_instance_extensions(),
            instance_create_flags,
            device_extensions: Self::basic_device_extensions(),

            core_features: Default::default(),
            ext_features: vec![],
            debug_msg_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            debug_msg_type: vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION |
                vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            debug_callback,

            frames_in_flight: 3,
        };
        info.set_device_features();

        info
    }

    pub fn is_complete(&self) -> anyhow::Result<()>
    {
        self.app_name.as_ref().context("")?;
        self.engine_name.as_ref().context("")?;
        Ok(())
    }


    fn basic_device_extensions() -> Vec<&'static CStr>
    {
        let mut exts = vec![Swapchain::name()];

        if cfg!(target_os = "macos") {
            // 在 metal 上模拟出 vulkan
            exts.push(vk::KhrPortabilitySubsetFn::name());
        }

        // dynamic rendering
        exts.append(&mut vec![
            cstr::cstr!("VK_KHR_depth_stencil_resolve"),
            // cstr::cstr!("VK_KHR_multiview"),     // 于 vk-1.1 加入到 core
            // cstr::cstr!("VK_KHR_maintenance2"),  // 于 vk-1.1 加入到 core
            ash::extensions::khr::CreateRenderPass2::name(),
            ash::extensions::khr::DynamicRendering::name(),
        ]);

        // RayTracing 相关的
        exts.append(&mut vec![
            ash::extensions::khr::AccelerationStructure::name(), // 主要的 ext
            cstr::cstr!("VK_EXT_descriptor_indexing"),
            cstr::cstr!("VK_KHR_buffer_device_address"),
            ash::extensions::khr::RayTracingPipeline::name(), // 主要的 ext
            ash::extensions::khr::DeferredHostOperations::name(),
            cstr::cstr!("VK_KHR_spirv_1_4"),
            cstr::cstr!("VK_KHR_shader_float_controls"),
        ]);

        exts
    }

    fn basic_instance_layers() -> Vec<&'static CStr>
    {
        vec![Self::VALIDATION_LAYER_NAME]
    }

    fn basic_instance_extensions() -> Vec<&'static CStr>
    {
        let mut exts = Vec::new();

        // 这个 extension 可以单独使用，提供以下功能：
        // 1. debug messenger
        // 2. 为 vulkan object 设置 debug name
        // 2. 使用 label 标记 queue 或者 command buffer 中的一个一个 section
        // 这个 extension 可以和 validation layer 配合使用，提供更详细的信息
        exts.push(ash::extensions::ext::DebugUtils::name());

        // 追加 window system 需要的 extension
        for ext in ash_window::enumerate_required_extensions(
            WindowSystem::instance().window().raw_display_handle(),
        )
        .unwrap()
        {
            unsafe {
                exts.push(CStr::from_ptr(*ext));
            }
        }

        if cfg!(target_os = "macos") {
            // 这个扩展能够在枚举 pdevice 时，将不受支持的 pdevice 也列举出来
            // 不受支持的 pdevice 可以通过模拟层运行 vulkan
            exts.push(vk::KhrPortabilityEnumerationFn::name());

            // device extension VK_KHR_portability_subset 需要这个扩展
            exts.push(vk::KhrGetPhysicalDeviceProperties2Fn::name());
        }
        exts
    }

    fn set_device_features(&mut self)
    {
        self.core_features = vk::PhysicalDeviceFeatures::builder()
            .sampler_anisotropy(true)
            .fragment_stores_and_atomics(true)
            .independent_blend(true)
            .build();

        self.ext_features = vec![
            Box::new(
                vk::PhysicalDeviceDynamicRenderingFeatures::builder()
                    .dynamic_rendering(true)
                    .build(),
            ),
            Box::new(
                vk::PhysicalDeviceBufferDeviceAddressFeatures::builder()
                    .buffer_device_address(true)
                    .build(),
            ),
            Box::new(
                vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder()
                    .ray_tracing_pipeline(true)
                    .build(),
            ),
            Box::new(
                vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder()
                    .acceleration_structure(true)
                    .build(),
            ),
            Box::new(
                vk::PhysicalDeviceHostQueryResetFeatures::builder().host_query_reset(true).build(),
            ),
            Box::new(
                vk::PhysicalDeviceSynchronization2Features::builder()
                    .synchronization2(true)
                    .build(),
            ),
        ];
    }
}


/// Rhi 只需要做到能够创建各种资源的程度就行了
///
/// 与 VulkanSamples 的 VulkanSamle 及 ApiVulkanSample 作用类似
pub struct Rhi
{
    /// vk 基础函数的接口
    vk_pf: Option<Entry>,
    vk_instance: Option<Instance>,

    vk_debug_util_pf: Option<ash::extensions::ext::DebugUtils>,
    vk_dynamic_render_pf: Option<ash::extensions::khr::DynamicRendering>,
    vk_acceleration_pf: Option<ash::extensions::khr::AccelerationStructure>,

    vk_debug_util_messenger: Option<vk::DebugUtilsMessengerEXT>,

    physical_device: Option<RhiPhysicalDevice>,
    device: Option<Device>,

    /// 可以提交 graphics 命令，也可以进行 present 操作
    graphics_queue: Option<RhiQueue>,
    transfer_queue: Option<RhiQueue>,
    compute_queue: Option<RhiQueue>,

    vma: Option<vk_mem::Allocator>,

    descriptor_pool: Option<vk::DescriptorPool>,

    graphics_command_pool: Option<RhiCommandPool>,
    transfer_command_pool: Option<RhiCommandPool>,
    compute_command_pool: Option<RhiCommandPool>,
}

// 初始化
impl Rhi
{
    const MAX_VERTEX_BLENDING_MESH_CNT: u32 = 256;
    const MAX_MATERIAL_CNT: u32 = 256;

    pub fn init(mut init_info: RhiInitInfo) -> anyhow::Result<()>
    {
        let mut rhi = Self {
            vk_pf: unsafe { Some(Entry::load().unwrap()) },
            vk_instance: None,
            vk_debug_util_pf: None,
            vk_debug_util_messenger: None,
            physical_device: None,
            device: None,
            compute_queue: None,
            graphics_queue: None,
            transfer_queue: None,
            vk_dynamic_render_pf: None,
            vma: None,
            descriptor_pool: None,
            graphics_command_pool: None,
            transfer_command_pool: None,
            compute_command_pool: None,
            vk_acceleration_pf: None,
        };

        rhi.init_instance(&init_info)?;
        rhi.init_debug_messenger(&init_info)?;
        rhi.init_pdevice();
        rhi.init_device_and_queue(&mut init_info);
        rhi.init_pf();
        rhi.init_vma(&init_info);
        rhi.init_descriptor_pool();
        rhi.init_default_command_pool();

        rhi.set_debug_name(rhi.physical_device().handle, "main-physical-device");
        rhi.set_debug_name(rhi.device().handle(), "main-device");
        rhi.set_debug_name(rhi.descriptor_pool.unwrap(), "main-descriptor-pool");

        unsafe {
            RHI = Some(rhi);
        }

        Ok(())
    }

    fn init_descriptor_pool(&mut self)
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

        let pool_create_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_size)
            .max_sets(Self::MAX_MATERIAL_CNT + Self::MAX_VERTEX_BLENDING_MESH_CNT + 32);

        unsafe {
            self.descriptor_pool =
                Some(self.device().create_descriptor_pool(&pool_create_info, None).unwrap());
        }
    }

    fn init_default_command_pool(&mut self)
    {
        self.graphics_command_pool = self.create_command_pool(
            vk::QueueFlags::GRAPHICS,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            "rhi-graphics-command-pool",
        );
        self.compute_command_pool = self.create_command_pool(
            vk::QueueFlags::COMPUTE,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            "rhi-compute-command-pool",
        );
        self.transfer_command_pool = self.create_command_pool(
            vk::QueueFlags::TRANSFER,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            "rhi-transfer-command-pool",
        );

        // 非空检测
        self.compute_command_pool.as_ref().unwrap();
        self.graphics_command_pool.as_ref().unwrap();
        self.transfer_command_pool.as_ref().unwrap();
    }

    fn init_instance(&mut self, init_info: &RhiInitInfo) -> anyhow::Result<()>
    {
        let app_name =
            CString::new(init_info.app_name.as_ref().context("")?.as_str()).context("")?;
        let engine_name =
            CString::new(init_info.engine_name.as_ref().context("")?.as_str()).context("")?;
        let app_info = vk::ApplicationInfo::builder()
            .application_name(app_name.as_ref())
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(engine_name.as_ref())
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(init_info.vk_version);

        let instance_extensions =
            init_info.instance_extensions.iter().map(|x| x.as_ptr()).collect_vec();
        let instance_layers = init_info.instance_layers.iter().map(|l| l.as_ptr()).collect_vec();

        let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(init_info.debug_msg_severity)
            .message_type(init_info.debug_msg_type)
            .pfn_user_callback(init_info.debug_callback)
            .build();

        let create_flags = if cfg!(target_os = "macos") {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            Default::default()
        };

        let instance_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&instance_extensions)
            .enabled_layer_names(&instance_layers)
            .flags(create_flags)
            .push_next(&mut debug_info);

        let instance =
            unsafe { self.vk_pf.as_ref().unwrap().create_instance(&instance_info, None)? };
        self.vk_instance = Some(instance);

        Ok(())
    }

    fn init_debug_messenger(&mut self, init_info: &RhiInitInfo) -> anyhow::Result<()>
    {
        let loader = ash::extensions::ext::DebugUtils::new(
            self.vk_pf.as_ref().unwrap(),
            self.vk_instance.as_ref().context("")?,
        );

        let create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(init_info.debug_msg_severity)
            .message_type(init_info.debug_msg_type)
            .pfn_user_callback(init_info.debug_callback)
            .build();
        let debug_messenger = unsafe { loader.create_debug_utils_messenger(&create_info, None)? };

        self.vk_debug_util_pf = Some(loader);
        self.vk_debug_util_messenger = Some(debug_messenger);

        Ok(())
    }


    fn init_pdevice(&mut self)
    {
        let instance = self.vk_instance.as_ref().unwrap();
        unsafe {
            let pd = instance
                .enumerate_physical_devices()
                .unwrap()
                .iter()
                .map(|pdevice| RhiPhysicalDevice::new(*pdevice, self.vk_instance.as_ref().unwrap()))
                // 优先使用独立显卡
                .find_or_first(RhiPhysicalDevice::is_descrete_gpu)
                .unwrap();

            self.physical_device = Some(pd);
        }
    }


    fn init_device_and_queue(&mut self, init_info: &mut RhiInitInfo)
    {
        let graphics_queue_family_index =
            self.physical_device().find_queue_family_index(vk::QueueFlags::GRAPHICS).unwrap();
        let compute_queue_family_index =
            self.physical_device().find_queue_family_index(vk::QueueFlags::COMPUTE).unwrap();
        let transfer_queue_family_index =
            self.physical_device().find_queue_family_index(vk::QueueFlags::TRANSFER).unwrap();

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
        let queue_priorities =
            queues.values().map(|count| vec![1.0; *count as usize]).collect_vec();
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

        let mut features =
            vk::PhysicalDeviceFeatures2::builder().features(init_info.core_features).build();
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
            let device = self
                .vk_instance
                .as_ref()
                .unwrap()
                .create_device(
                    self.physical_device.as_ref().unwrap().handle,
                    &device_create_info,
                    None,
                )
                .unwrap();

            let graphics_queue =
                device.get_device_queue(graphics_queue_family_index, graphics_queue_num);
            let compute_queue =
                device.get_device_queue(compute_queue_family_index, compute_queue_num);
            let transfer_queue =
                device.get_device_queue(transfer_queue_family_index, transfer_queue_num);

            self.device = Some(device);

            self.set_debug_name(graphics_queue, "graphics-queue");
            self.set_debug_name(compute_queue, "compute-queue");
            self.set_debug_name(transfer_queue, "transfer-queue");

            self.graphics_queue = Some(RhiQueue {
                queue: graphics_queue,
                queue_family_index: graphics_queue_family_index,
            });
            self.transfer_queue = Some(RhiQueue {
                queue: transfer_queue,
                queue_family_index: transfer_queue_family_index,
            });
            self.compute_queue = Some(RhiQueue {
                queue: compute_queue,
                queue_family_index: compute_queue_family_index,
            });
        }
    }

    fn init_pf(&mut self)
    {
        let instance = self.vk_instance.as_ref().unwrap();
        let device = self.device.as_ref().unwrap();

        self.vk_dynamic_render_pf =
            Some(ash::extensions::khr::DynamicRendering::new(instance, device));
        self.vk_acceleration_pf =
            Some(ash::extensions::khr::AccelerationStructure::new(instance, device));
    }

    fn init_vma(&mut self, init_info: &RhiInitInfo)
    {
        let vma_create_info = vk_mem::AllocatorCreateInfo::new(
            Rc::new(self.vk_instance.as_ref().unwrap()),
            Rc::new(self.device.as_ref().unwrap()),
            self.physical_device.as_ref().unwrap().handle,
        )
        .vulkan_api_version(init_info.vk_version)
        .flags(vk_mem::AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS);

        self.vma = Some(vk_mem::Allocator::new(vma_create_info).unwrap());
    }
}


// 属性访问
impl Rhi
{
    #[inline]
    pub fn graphics_command_pool(&self) -> &RhiCommandPool
    {
        self.graphics_command_pool.as_ref().unwrap()
    }
    #[inline]
    pub fn compute_command_pool(&self) -> &RhiCommandPool
    {
        self.compute_command_pool.as_ref().unwrap()
    }
    #[inline]
    pub fn transfer_command_pool(&self) -> &RhiCommandPool
    {
        self.transfer_command_pool.as_ref().unwrap()
    }
    #[inline]
    pub fn instance() -> &'static Self
    {
        unsafe { RHI.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub(crate) fn vk_instance(&self) -> &Instance
    {
        unsafe { self.vk_instance.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub(crate) fn device(&self) -> &Device
    {
        unsafe { self.device.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub(crate) fn physical_device(&self) -> &RhiPhysicalDevice
    {
        unsafe { self.physical_device.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub fn compute_queue(&self) -> &RhiQueue
    {
        unsafe { self.compute_queue.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub fn graphics_queue(&self) -> &RhiQueue
    {
        unsafe { self.graphics_queue.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub fn transfer_queue(&self) -> &RhiQueue
    {
        unsafe { self.transfer_queue.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub fn descriptor_pool(&self) -> vk::DescriptorPool
    {
        unsafe { self.descriptor_pool.unwrap_unchecked() }
    }
    #[inline]
    pub(crate) fn vma(&self) -> &vk_mem::Allocator
    {
        unsafe { self.vma.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub(crate) fn vk_pf(&self) -> &Entry
    {
        unsafe { self.vk_pf.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub(crate) fn dynamic_render_pf(&self) -> &ash::extensions::khr::DynamicRendering
    {
        unsafe { self.vk_dynamic_render_pf.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub(crate) fn acceleration_structure_pf(&self) -> &ash::extensions::khr::AccelerationStructure
    {
        unsafe { self.vk_acceleration_pf.as_ref().unwrap_unchecked() }
    }
}


// 工具方法
impl Rhi
{
    /// 需要在 debug_util_pf 以及 device 初始化完成后调用
    pub(crate) fn set_debug_name<T, S>(&self, handle: T, name: S)
    where
        T: vk::Handle + Copy,
        S: AsRef<str>,
    {
        let name = if name.as_ref().is_empty() { "empty-debug-name" } else { name.as_ref() };
        let name = CString::new(name).unwrap();
        unsafe {
            self.vk_debug_util_pf
                .as_ref()
                .unwrap()
                .set_debug_utils_object_name(
                    self.device.as_ref().unwrap().handle(),
                    &vk::DebugUtilsObjectNameInfoEXT::builder()
                        .object_name(name.as_c_str())
                        .object_type(T::TYPE)
                        .object_handle(handle.as_raw()),
                )
                .unwrap();
        }
    }

    pub fn set_debug_label(&self)
    {
        todo!()
        // self.debug_util_pf.unwrap().cmd_begin_debug_utils_label()
    }

    pub fn create_command_pool<S: AsRef<str> + Clone>(
        &self,
        queue_flags: vk::QueueFlags,
        flags: vk::CommandPoolCreateFlags,
        debug_name: S,
    ) -> Option<RhiCommandPool>
    {
        let queue_family_index = self.physical_device().find_queue_family_index(queue_flags)?;

        let pool = unsafe {
            self.device()
                .create_command_pool(
                    &vk::CommandPoolCreateInfo::builder()
                        .queue_family_index(queue_family_index)
                        .flags(flags),
                    None,
                )
                .unwrap()
        };

        self.set_debug_name(pool, debug_name);
        Some(RhiCommandPool {
            command_pool: pool,
            queue_family_index,
        })
    }

    pub fn create_image<S>(
        &self,
        create_info: &vk::ImageCreateInfo,
        debug_name: S,
    ) -> (vk::Image, vk_mem::Allocation)
    where
        S: AsRef<str>,
    {
        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };
        let (image, allocation) =
            unsafe { self.vma().create_image(create_info, &alloc_info).unwrap() };

        self.set_debug_name(image, debug_name);
        (image, allocation)
    }

    #[inline]
    pub fn create_image_view<S>(
        &self,
        create_info: &vk::ImageViewCreateInfo,
        debug_name: S,
    ) -> vk::ImageView
    where
        S: AsRef<str>,
    {
        let view = unsafe { self.device().create_image_view(create_info, None).unwrap() };

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
                    self.vk_instance().get_physical_device_format_properties(
                        self.physical_device().handle,
                        **f,
                    )
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
