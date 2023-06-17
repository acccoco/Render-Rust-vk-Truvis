use ash::{vk, Instance};
use command_pool::RhiCommandPool;


pub(crate) mod command_pool;
pub(crate) mod create_utils;
pub(crate) mod init_info;
pub(crate) mod physical_device;
pub(crate) mod queue;

use std::{collections::HashMap, ffi::CString, rc::Rc};

use ash::{Device, Entry};
use itertools::Itertools;
use vk_mem::Alloc;

use crate::rhi::{init_info::RhiInitInfo, physical_device::RhiPhysicalDevice, queue::RhiQueue};


static mut G_RHI: Option<Rhi> = None;


/// Rhi 只需要做到能够创建各种资源的程度就行了
pub struct Rhi
{
    vk_pf: Option<Entry>,
    instance: Option<Instance>,

    debug_util_pf: Option<ash::extensions::ext::DebugUtils>,
    dynamic_render_pf: Option<ash::extensions::khr::DynamicRendering>,

    debug_util_messenger: Option<vk::DebugUtilsMessengerEXT>,

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

// 属性访问
impl Rhi
{
    #[inline]
    pub(crate) fn graphics_command_pool(&self) -> &RhiCommandPool { self.graphics_command_pool.as_ref().unwrap() }
    #[inline]
    pub(crate) fn instance() -> &'static Self { unsafe { G_RHI.as_ref().unwrap_unchecked() } }
    #[inline]
    pub(crate) fn vk_instance(&self) -> &Instance { unsafe { self.instance.as_ref().unwrap_unchecked() } }
    #[inline]
    pub(crate) fn device(&self) -> &Device { unsafe { self.device.as_ref().unwrap_unchecked() } }
    #[inline]
    pub(crate) fn physical_device(&self) -> &RhiPhysicalDevice
    {
        unsafe { self.physical_device.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub(crate) fn compute_queue(&self) -> &RhiQueue { unsafe { self.compute_queue.as_ref().unwrap_unchecked() } }
    #[inline]
    pub(crate) fn graphics_queue(&self) -> &RhiQueue { unsafe { self.graphics_queue.as_ref().unwrap_unchecked() } }
    #[inline]
    pub(crate) fn transfer_queue(&self) -> &RhiQueue { unsafe { self.transfer_queue.as_ref().unwrap_unchecked() } }
    #[inline]
    pub(crate) fn vma(&self) -> &vk_mem::Allocator { unsafe { self.vma.as_ref().unwrap_unchecked() } }
    #[inline]
    pub(crate) fn vk_pf(&self) -> &Entry { unsafe { self.vk_pf.as_ref().unwrap_unchecked() } }
}

// 工具方法
impl Rhi
{
    pub(crate) fn set_debug_name<T>(&self, handle: T, name: &str)
    where
        T: vk::Handle + Copy,
    {
        let name = CString::new(name).unwrap();
        unsafe {
            self.debug_util_pf
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

    #[inline]
    pub(crate) fn try_set_debug_name<T>(&self, handle: T, name: Option<&str>)
    where
        T: vk::Handle + Copy,
    {
        if let Some(name) = name {
            self.set_debug_name(handle, name);
        }
    }


    pub fn create_command_pool(
        &self,
        queue_flags: vk::QueueFlags,
        flags: vk::CommandPoolCreateFlags,
        debug_name: Option<&str>,
    ) -> Option<RhiCommandPool>
    {
        let queue_family_index = self.physical_device().find_queue_family_index(queue_flags)?;

        let pool = unsafe {
            self.device()
                .create_command_pool(
                    &vk::CommandPoolCreateInfo::builder().queue_family_index(queue_family_index).flags(flags),
                    None,
                )
                .unwrap()
        };

        self.try_set_debug_name(pool, debug_name);
        Some(RhiCommandPool {
            command_pool: pool,
            queue_family_index,
        })
    }

    pub fn create_image(
        &self,
        create_info: &vk::ImageCreateInfo,
        debug_name: Option<&str>,
    ) -> (vk::Image, vk_mem::Allocation)
    {
        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };
        let (image, allocation) = unsafe { self.vma().create_image(create_info, &alloc_info).unwrap() };

        self.try_set_debug_name(image, debug_name);
        (image, allocation)
    }

    #[inline]
    pub fn create_image_view(&self, create_info: &vk::ImageViewCreateInfo, debug_name: Option<&str>) -> vk::ImageView
    {
        let view = unsafe { self.device().create_image_view(create_info, None).unwrap() };

        self.try_set_debug_name(view, debug_name);
        view
    }

    #[inline]
    pub fn create_semaphore(&self, debug_name: Option<&str>) -> vk::Semaphore
    {
        let semaphore = unsafe { self.device().create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() };

        self.try_set_debug_name(semaphore, debug_name);
        semaphore
    }

    #[inline]
    pub fn create_fence(&self, signaled: bool, debug_name: Option<&str>) -> vk::Fence
    {
        let fence_flags = if signaled {
            vk::FenceCreateFlags::SIGNALED
        } else {
            vk::FenceCreateFlags::empty()
        };
        let fence =
            unsafe { self.device().create_fence(&vk::FenceCreateInfo::builder().flags(fence_flags), None).unwrap() };

        self.try_set_debug_name(fence, debug_name);
        fence
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
                    self.vk_instance().get_physical_device_format_properties(self.physical_device().vk_pdevice, **f)
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


// 初始化
impl Rhi
{
    const MAX_VERTEX_BLENDING_MESH_CNT: u32 = 256;
    const MAX_MATERIAL_CNT: u32 = 256;

    pub fn init(init_info: &RhiInitInfo)
    {
        let mut rhi = Self {
            vk_pf: unsafe { Some(Entry::load().unwrap()) },
            instance: None,
            debug_util_pf: None,
            debug_util_messenger: None,
            physical_device: None,
            device: None,
            compute_queue: None,
            graphics_queue: None,
            transfer_queue: None,
            dynamic_render_pf: None,
            vma: None,
            descriptor_pool: None,
            graphics_command_pool: None,
            transfer_command_pool: None,
            compute_command_pool: None,
        };

        rhi.init_instance(init_info);
        rhi.init_debug_messenger(init_info);
        rhi.init_pdevice();
        rhi.init_device_and_queue(init_info);
        rhi.init_dynamic_render_loader();
        rhi.init_vma(init_info);
        rhi.init_descriptor_pool();
        rhi.init_default_command_pool();

        unsafe {
            G_RHI = Some(rhi);
        }
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
            self.descriptor_pool = Some(self.device().create_descriptor_pool(&pool_create_info, None).unwrap());
        }
    }

    fn init_default_command_pool(&mut self)
    {
        self.graphics_command_pool = self.create_command_pool(
            vk::QueueFlags::GRAPHICS,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            Some("rhi-graphics"),
        );
        self.compute_command_pool = self.create_command_pool(
            vk::QueueFlags::COMPUTE,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            Some("rhi-compute"),
        );
        self.transfer_command_pool = self.create_command_pool(
            vk::QueueFlags::TRANSFER,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            Some("rhi-transfer"),
        );

        // 非空检测
        self.compute_command_pool.as_ref().unwrap();
        self.graphics_command_pool.as_ref().unwrap();
        self.transfer_command_pool.as_ref().unwrap();
    }

    fn init_instance(&mut self, init_info: &RhiInitInfo)
    {
        let app_name = CString::new(init_info.app_name.as_ref().unwrap().as_str()).unwrap();
        let engine_name = CString::new(init_info.engine_name.as_ref().unwrap().as_str()).unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(app_name.as_ref())
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(engine_name.as_ref())
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(init_info.vk_version);

        let instance_extensions = init_info.instance_extensions.iter().map(|x| x.as_ptr()).collect_vec();
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

        let instance = unsafe { self.vk_pf.as_ref().unwrap().create_instance(&instance_info, None).unwrap() };
        self.instance = Some(instance);
    }

    fn init_debug_messenger(&mut self, init_info: &RhiInitInfo)
    {
        let loader =
            ash::extensions::ext::DebugUtils::new(self.vk_pf.as_ref().unwrap(), self.instance.as_ref().unwrap());

        let create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(init_info.debug_msg_severity)
            .message_type(init_info.debug_msg_type)
            .pfn_user_callback(init_info.debug_callback)
            .build();
        let debug_messenger = unsafe { loader.create_debug_utils_messenger(&create_info, None).unwrap() };

        self.debug_util_pf = Some(loader);
        self.debug_util_messenger = Some(debug_messenger);
    }


    fn init_pdevice(&mut self)
    {
        let instance = self.instance.as_ref().unwrap();
        unsafe {
            let pd = instance
                .enumerate_physical_devices()
                .unwrap()
                .iter()
                .map(|pdevice| RhiPhysicalDevice::new(*pdevice, self.instance.as_ref().unwrap()))
                // 优先使用独立显卡
                .find_or_first(RhiPhysicalDevice::is_descrete_gpu)
                .unwrap();

            self.physical_device = Some(pd);
        }
    }


    fn init_device_and_queue(&mut self, init_info: &RhiInitInfo)
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

        let physical_device_features = vk::PhysicalDeviceFeatures::builder()
            .sampler_anisotropy(true)
            .fragment_stores_and_atomics(true)
            .independent_blend(true);

        let device_exts = init_info.device_extensions.iter().map(|e| e.as_ptr()).collect_vec();

        // dynamic rendering 所需的 feature
        let mut dynamic_render_feature = vk::PhysicalDeviceDynamicRenderingFeatures::builder().dynamic_rendering(true);

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_features(&physical_device_features)
            .enabled_extension_names(&device_exts)
            .push_next(&mut dynamic_render_feature);

        unsafe {
            let device = self
                .instance
                .as_ref()
                .unwrap()
                .create_device(self.physical_device.as_ref().unwrap().vk_pdevice, &device_create_info, None)
                .unwrap();

            let graphics_queue = device.get_device_queue(graphics_queue_family_index, graphics_queue_num);
            let compute_queue = device.get_device_queue(compute_queue_family_index, compute_queue_num);
            let transfer_queue = device.get_device_queue(transfer_queue_family_index, transfer_queue_num);

            self.device = Some(device);

            self.set_debug_name(graphics_queue, "graphics");
            self.set_debug_name(compute_queue, "compute");
            self.set_debug_name(transfer_queue, "transfer");

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

    fn init_dynamic_render_loader(&mut self)
    {
        let instance = self.instance.as_ref().unwrap();
        let device = self.device.as_ref().unwrap();
        self.dynamic_render_pf = Some(ash::extensions::khr::DynamicRendering::new(instance, device));
    }

    fn init_vma(&mut self, init_info: &RhiInitInfo)
    {
        let vma_create_info = vk_mem::AllocatorCreateInfo::new(
            Rc::new(self.instance.as_ref().unwrap()),
            Rc::new(self.device.as_ref().unwrap()),
            self.physical_device.as_ref().unwrap().vk_pdevice,
        )
        .vulkan_api_version(init_info.vk_version);

        self.vma = Some(vk_mem::Allocator::new(vma_create_info).unwrap());
    }
}
