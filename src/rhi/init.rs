use std::{
    collections::HashSet,
    ffi::{c_char, CStr},
};

use ash::{vk, Entry, Instance};
use itertools::Itertools;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::{
    rhi::{
        physical_device::RhiPhysicalDevice,
        queue::{RhiQueueFamilyPresentProps, RhiQueueType},
        RhiCore,
    },
    window_system::WindowSystem,
};

pub struct RhiInitInfo<'a>
{
    app_name: &'static CStr,
    engine_name: &'static CStr,

    vk_version: u32,

    instance_layers: Vec<&'static CStr>,
    instance_extensions: Vec<&'static CStr>,
    device_extensions: Vec<&'static CStr>,

    debug_msg_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    debug_msg_type: vk::DebugUtilsMessageTypeFlagsEXT,
    debug_callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT,

    window_system: Option<&'a WindowSystem>,
}

impl<'a> RhiInitInfo<'a>
{
    fn get_basic_device_extensions() -> Vec<&'static CStr>
    {
        vec![
            ash::extensions::khr::Swapchain::name(),
            #[cfg(target_os = "macos")]
            vk::KhrPortabilitySubsetFn::name(), // 这个扩展可以在 metal 上模拟出 vulkan
            // dynamic rendering 所需的 extensions
            cstr::cstr!("VK_KHR_depth_stencil_resolve"),
            cstr::cstr!("VK_KHR_multiview"),
            cstr::cstr!("VK_KHR_maintenance2"),
            ash::extensions::khr::CreateRenderPass2::name(),
            ash::extensions::khr::DynamicRendering::name(),
        ]
    }
}

impl<'a> Default for RhiInitInfo<'a>
{
    fn default() -> Self
    {
        Self {
            vk_version: vk::API_VERSION_1_1,
            instance_layers: vec![cstr::cstr!("VK_LAYER_KHRONOS_validation")],
            ..todo!()
        }
    }
}

// 初始化方法
impl RhiCore
{
    pub fn init(init_info: &RhiInitInfo) -> Self
    {
        let mut rhi = Self {
            entry: unsafe { Some(Entry::load().unwrap()) },
            ..Default::default()
        };

        rhi.init_instance(init_info);
        rhi.init_debug_messenger(init_info);
        rhi.init_surface(init_info);
        rhi.init_pdevice(init_info);
        rhi.init_queue_faimly();
        rhi.init_device_and_queue(init_info);
        rhi.init_dynamic_render_loader();

        // TODO 确定一下 各种资源的位置：pool，image format


        rhi
    }


    fn init_instance(&mut self, init_info: &RhiInitInfo)
    {
        fn get_instance_extensions(init_info: &RhiInitInfo) -> Vec<*const c_char>
        {
            let mut exts = init_info.instance_extensions.iter().map(|ext| ext.as_ptr()).collect_vec();

            // 这个 extension 可以单独使用，提供以下功能：
            // 1. debug messenger
            // 2. 为 vulkan object 设置 debug name
            // 2. 使用 label 标记 queue 或者 command buffer 中的一个一个 section
            // 这个 extension 可以和 validation layer 配合使用，提供更详细的信息
            exts.push(ash::extensions::ext::DebugUtils::name().as_ptr());

            // 追加 window system 需要的 extension
            if let Some(window) = init_info.window_system {
                for ext in ash_window::enumerate_required_extensions(window.window().raw_display_handle()).unwrap() {
                    exts.push(*ext);
                }
            }

            #[cfg(target_os = "macos")]
            {
                // 这个扩展能够在枚举 pdevice 时，将不受支持的 pdevice 也列举出来
                // 不受支持的 pdevice 可以通过模拟层运行 vulkan
                exts.push(vk::KhrPortabilityEnumerationFn::name().as_ptr());

                // device extension VK_KHR_portability_subset 需要这个扩展
                exts.push(vk::KhrGetPhysicalDeviceProperties2Fn::name().as_ptr());
            }
            exts
        }


        fn get_instance_layers(init_info: &RhiInitInfo) -> Vec<*const c_char>
        {
            let mut layers = init_info.instance_layers.iter().map(|layer| layer.as_ptr()).collect_vec();
            layers.push(VALIDATION_LAYER_NAME.as_ptr());
            layers
        }


        let app_info = vk::ApplicationInfo::builder()
            .application_name(init_info.app_name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(init_info.engine_name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(init_info.vk_version);

        let instance_extensions = get_instance_extensions(init_info);
        let instance_layers = get_instance_layers(init_info);

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

        let instance = unsafe { self.entry.as_ref().unwrap().create_instance(&instance_info, None).unwrap() };
        self.instance = Some(instance);
    }

    fn init_debug_messenger(&mut self, init_info: &RhiInitInfo)
    {
        let loader =
            ash::extensions::ext::DebugUtils::new(self.entry.as_ref().unwrap(), self.instance.as_ref().unwrap());

        let create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(init_info.debug_msg_severity)
            .message_type(init_info.debug_msg_type)
            .pfn_user_callback(init_info.debug_callback)
            .build();
        let debug_messenger = unsafe { loader.create_debug_utils_messenger(&create_info, None).unwrap() };

        self.debug_util_loader = Some(loader);
        self.debug_util_messenger = Some(debug_messenger);
    }

    fn init_surface(&mut self, init_info: &RhiInitInfo)
    {
        let surface_loader =
            ash::extensions::khr::Surface::new(self.entry.as_ref().unwrap(), self.instance.as_ref().unwrap());

        let surface = init_info.window_system.map(|window_system| unsafe {
            ash_window::create_surface(
                self.entry.as_ref().unwrap(),
                self.instance.as_ref().unwrap(),
                window_system.window().raw_display_handle(),
                window_system.window().raw_window_handle(),
                None,
            )
            .unwrap()
        });

        self.surface_loader = Some(surface_loader);
        self.surface = surface;
    }

    fn init_pdevice(&mut self, init_info: &RhiInitInfo)
    {
        /// 检查 physical device 是否满足要求
        pub fn check_suitable(pdevice: &RhiPhysicalDevice, instance: &Instance, exts: &[&'static CStr]) -> bool
        {
            // check queue family
            {
                let mut support_compute = false;
                let mut support_graphics = false;
                let mut support_present = false;

                for queue_family_prop in &pdevice.queue_family_props {
                    support_compute = support_compute || queue_family_prop.compute;
                    support_graphics = support_graphics || queue_family_prop.graphics;
                    support_present =
                        support_present || (queue_family_prop.present != RhiQueueFamilyPresentProps::NoSupported);
                }

                if !(support_compute && support_graphics && support_present) {
                    return false;
                }
            }

            if !pdevice.check_device_extension_support(instance, exts) {
                return false;
            }

            if pdevice.pd_features.sample_rate_shading == vk::FALSE {
                return false;
            }

            return true;
        }

        let instance = self.instance.as_ref().unwrap();
        unsafe {
            let pd = instance
                .enumerate_physical_devices()
                .unwrap()
                .iter()
                .map(|pdevice| {
                    let mut pd = RhiPhysicalDevice::new(*pdevice, self.instance.as_ref().unwrap());
                    pd.init_queue_family_props(instance, self.surface, self.surface_loader.as_ref().unwrap());
                    pd
                })
                .filter(|pd| check_suitable(pd, instance, &init_info.device_extensions))
                // 优先使用独立显卡
                .find_or_first(RhiPhysicalDevice::is_descrete_gpu)
                .unwrap();

            self.physical_device = Some(pd);
        }
    }

    fn init_queue_faimly(&mut self)
    {
        let pdevice = self.physical_device.as_ref().unwrap();
        self.queue_family_index_compute = pdevice.find_queue_family_index(RhiQueueType::Compute);
        self.queue_family_index_present = pdevice.find_queue_family_index(RhiQueueType::Present);
        self.queue_family_index_graphics = pdevice.find_queue_family_index(RhiQueueType::Graphics);
    }

    fn init_device_and_queue(&mut self, init_info: &RhiInitInfo)
    {
        let queue_families = HashSet::from([
            self.queue_family_index_present.unwrap(),
            self.queue_family_index_compute.unwrap(),
            self.queue_family_index_graphics.unwrap(),
        ]);
        let queue_priority = [1.0];
        let queue_create_infos = queue_families
            .iter()
            .map(|q| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(*q)
                    .queue_priorities(&queue_priority)
                    .build()
            })
            .collect_vec();

        let mut physical_device_features = vk::PhysicalDeviceFeatures::builder()
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
                .create_device(self.physical_device.as_ref().unwrap().vk_physical_device, &device_create_info, None)
                .unwrap();

            let graphics_queue = device.get_device_queue(self.queue_family_index_graphics.unwrap(), 0);
            let compute_queue = device.get_device_queue(self.queue_family_index_compute.unwrap(), 0);
            let present_queue = device.get_device_queue(self.queue_family_index_present.unwrap(), 0);

            // 为 queue 设置 debug name。考虑 queue 相等的情形
            {
                let all_queue: HashSet<_> = [graphics_queue, compute_queue, present_queue].into();
                for queue in all_queue {
                    let mut name = "queue".to_string();
                    if queue == graphics_queue {
                        name.push_str(".graphics");
                    }
                    if queue == present_queue {
                        name.push_str(".present");
                    }
                    if queue == compute_queue {
                        name.push_str(".compute");
                    }
                    self.set_debug_name(graphics_queue, &name);
                }
            }

            self.device = Some(device);
            self.queue_graphics = Some(graphics_queue);
            self.queue_present = Some(present_queue);
            self.queue_compute = Some(compute_queue);
        }
    }

    fn init_dynamic_render_loader(&mut self)
    {
        let instance = self.instance.as_ref().unwrap();
        let device = self.device.as_ref().unwrap();
        self.dynamic_render_loader = Some(ash::extensions::khr::DynamicRendering::new(instance, device));
    }
}

const VALIDATION_LAYER_NAME: &CStr = cstr::cstr!("VK_LAYER_KHRONOS_validation");
