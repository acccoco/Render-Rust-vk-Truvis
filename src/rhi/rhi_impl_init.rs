use std::{collections::HashMap, ffi::CString, rc::Rc};

use ash::{vk, Entry};
use itertools::Itertools;

use crate::{
    rhi::{physical_device::RhiPhysicalDevice, rhi_init_info::RhiInitInfo, Rhi, RHI},
    rhi_type::queue::RhiQueue,
};

// 初始化
impl Rhi
{
    const MAX_VERTEX_BLENDING_MESH_CNT: u32 = 256;
    const MAX_MATERIAL_CNT: u32 = 256;

    pub fn init(mut init_info: RhiInitInfo)
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

        rhi.init_instance(&init_info);
        rhi.init_debug_messenger(&init_info);
        rhi.init_pdevice();
        rhi.init_device_and_queue(&mut init_info);
        rhi.init_pf();
        rhi.init_vma(&init_info);
        rhi.init_descriptor_pool();
        rhi.init_default_command_pool();

        rhi.set_debug_name(rhi.physical_device().vk_pdevice, "main-physical-device");
        rhi.set_debug_name(rhi.device().handle(), "main-device");
        rhi.set_debug_name(rhi.descriptor_pool.unwrap(), "main-descriptor-pool");

        unsafe {
            RHI = Some(rhi);
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
        self.vk_instance = Some(instance);
    }

    fn init_debug_messenger(&mut self, init_info: &RhiInitInfo)
    {
        let loader =
            ash::extensions::ext::DebugUtils::new(self.vk_pf.as_ref().unwrap(), self.vk_instance.as_ref().unwrap());

        let create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(init_info.debug_msg_severity)
            .message_type(init_info.debug_msg_type)
            .pfn_user_callback(init_info.debug_callback)
            .build();
        let debug_messenger = unsafe { loader.create_debug_utils_messenger(&create_info, None).unwrap() };

        self.vk_debug_util_pf = Some(loader);
        self.vk_debug_util_messenger = Some(debug_messenger);
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
            let device = self
                .vk_instance
                .as_ref()
                .unwrap()
                .create_device(self.physical_device.as_ref().unwrap().vk_pdevice, &device_create_info, None)
                .unwrap();

            let graphics_queue = device.get_device_queue(graphics_queue_family_index, graphics_queue_num);
            let compute_queue = device.get_device_queue(compute_queue_family_index, compute_queue_num);
            let transfer_queue = device.get_device_queue(transfer_queue_family_index, transfer_queue_num);

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

        self.vk_dynamic_render_pf = Some(ash::extensions::khr::DynamicRendering::new(instance, device));
        self.vk_acceleration_pf = Some(ash::extensions::khr::AccelerationStructure::new(instance, device));
    }

    fn init_vma(&mut self, init_info: &RhiInitInfo)
    {
        let vma_create_info = vk_mem::AllocatorCreateInfo::new(
            Rc::new(self.vk_instance.as_ref().unwrap()),
            Rc::new(self.device.as_ref().unwrap()),
            self.physical_device.as_ref().unwrap().vk_pdevice,
        )
        .vulkan_api_version(init_info.vk_version)
        .flags(vk_mem::AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS);

        self.vma = Some(vk_mem::Allocator::new(vma_create_info).unwrap());
    }
}
