use std::{
    collections::HashSet,
    ffi::{CStr, CString},
    rc::Rc,
};

use ash::{vk, Device, Entry, Instance};
use itertools::Itertools;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use vk_mem::Alloc;

use crate::{
    rhi::{
        physical_device::RhiPhysicalDevice,
        queue::{RhiQueueFamilyPresentProps, RhiQueueType},
    },
    rhi_init_info::RhiInitInfo,
};


/// Rhi 只需要做到能够创建各种资源的程度就行了
pub struct RhiCore
{
    entry: Option<Entry>,
    instance: Option<Instance>,

    debug_util_loader: Option<ash::extensions::ext::DebugUtils>,
    surface_loader: Option<ash::extensions::khr::Surface>,
    dynamic_render_loader: Option<ash::extensions::khr::DynamicRendering>,

    debug_util_messenger: Option<vk::DebugUtilsMessengerEXT>,

    /// 这个字段是可空的
    surface: Option<vk::SurfaceKHR>,

    physical_device: Option<RhiPhysicalDevice>,
    device: Option<Device>,

    pub(crate) compute_queue_family_index: Option<u32>,
    pub(crate) graphics_queue_family_index: Option<u32>,
    pub(crate) present_queue_family_index: Option<u32>,

    compute_queue: Option<vk::Queue>,
    graphics_queue: Option<vk::Queue>,
    present_queue: Option<vk::Queue>,

    vma: Option<vk_mem::Allocator>,
}


// 属性访问
impl RhiCore
{
    #[inline]
    pub(crate) fn instance(&self) -> &Instance { unsafe { self.instance.as_ref().unwrap_unchecked() } }
    #[inline]
    pub(crate) fn device(&self) -> &Device { unsafe { self.device.as_ref().unwrap_unchecked() } }
    #[inline]
    pub(crate) fn surface(&self) -> vk::SurfaceKHR { self.surface.unwrap() }
    #[inline]
    pub(crate) fn surface_loader(&self) -> &ash::extensions::khr::Surface
    {
        unsafe { self.surface_loader.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub(crate) fn physical_device(&self) -> &RhiPhysicalDevice
    {
        unsafe { self.physical_device.as_ref().unwrap_unchecked() }
    }
    #[inline]
    pub(crate) fn compute_queue(&self) -> vk::Queue { unsafe { self.compute_queue.unwrap_unchecked() } }
    #[inline]
    pub(crate) fn graphics_queue(&self) -> vk::Queue { unsafe { self.graphics_queue.unwrap_unchecked() } }
    #[inline]
    pub(crate) fn present_queue(&self) -> vk::Queue { self.present_queue.unwrap() }
    #[inline]
    pub(crate) fn vma(&self) -> &vk_mem::Allocator { unsafe { self.vma.as_ref().unwrap_unchecked() } }
}


// 工具方法
impl RhiCore
{
    pub fn set_debug_name<T>(&self, handle: T, name: &str)
    where
        T: vk::Handle + Copy,
    {
        let name = CString::new(name).unwrap();
        unsafe {
            self.debug_util_loader
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
    pub fn try_set_debug_name<T>(&self, handle: T, name: Option<&str>)
    where
        T: vk::Handle + Copy,
    {
        if let Some(name) = name {
            self.set_debug_name(handle, name);
        }
    }


    pub fn create_command_pool(
        &self,
        queue_family_type: RhiQueueType,
        flags: vk::CommandPoolCreateFlags,
        debug_name: Option<&str>,
    ) -> vk::CommandPool
    {
        let queue_family_index = match queue_family_type {
            RhiQueueType::Compute => self.compute_queue_family_index.unwrap(),
            RhiQueueType::Graphics => self.graphics_queue_family_index.unwrap(),
            RhiQueueType::Present => self.present_queue_family_index.unwrap(),
        };

        let pool = unsafe {
            self.device()
                .create_command_pool(
                    &vk::CommandPoolCreateInfo::builder().queue_family_index(queue_family_index).flags(flags),
                    None,
                )
                .unwrap()
        };

        self.try_set_debug_name(pool, debug_name);
        pool
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
}


// 初始化方法
impl RhiCore
{
    pub fn init(init_info: &RhiInitInfo) -> Self
    {
        let mut rhi = Self {
            entry: unsafe { Some(Entry::load().unwrap()) },
            instance: None,
            debug_util_loader: None,
            debug_util_messenger: None,
            surface_loader: None,
            surface: None,
            physical_device: None,
            compute_queue_family_index: None,
            graphics_queue_family_index: None,
            present_queue_family_index: None,
            device: None,
            compute_queue: None,
            graphics_queue: None,
            present_queue: None,
            dynamic_render_loader: None,
            vma: None,
        };

        rhi.init_instance(init_info);
        rhi.init_debug_messenger(init_info);
        rhi.init_surface(init_info);
        rhi.init_pdevice(init_info);
        rhi.init_queue_faimly();
        rhi.init_device_and_queue(init_info);
        rhi.init_dynamic_render_loader();
        rhi.init_vma(init_info);

        rhi
    }


    fn init_instance(&mut self, init_info: &RhiInitInfo)
    {
        let app_info = vk::ApplicationInfo::builder()
            .application_name(init_info.app_name.as_ref().unwrap())
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(init_info.engine_name.as_ref().unwrap())
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

            if !pdevice.check_device_extension_support(exts) {
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
                    let mut pd = RhiPhysicalDevice::new(*pdevice, self.instance.as_ref().unwrap().clone());
                    pd.init_queue_family_props(self.surface, self.surface_loader.as_ref().unwrap());
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
        self.compute_queue_family_index = pdevice.find_queue_family_index(RhiQueueType::Compute);
        self.present_queue_family_index = pdevice.find_queue_family_index(RhiQueueType::Present);
        self.graphics_queue_family_index = pdevice.find_queue_family_index(RhiQueueType::Graphics);
    }

    fn init_device_and_queue(&mut self, init_info: &RhiInitInfo)
    {
        let queue_families = HashSet::from([
            self.present_queue_family_index.unwrap(),
            self.compute_queue_family_index.unwrap(),
            self.graphics_queue_family_index.unwrap(),
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

            let graphics_queue = device.get_device_queue(self.graphics_queue_family_index.unwrap(), 0);
            let compute_queue = device.get_device_queue(self.compute_queue_family_index.unwrap(), 0);
            let present_queue = device.get_device_queue(self.present_queue_family_index.unwrap(), 0);

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
            self.graphics_queue = Some(graphics_queue);
            self.present_queue = Some(present_queue);
            self.compute_queue = Some(compute_queue);
        }
    }

    fn init_dynamic_render_loader(&mut self)
    {
        let instance = self.instance.as_ref().unwrap();
        let device = self.device.as_ref().unwrap();
        self.dynamic_render_loader = Some(ash::extensions::khr::DynamicRendering::new(instance, device));
    }

    fn init_vma(&mut self, init_info: &RhiInitInfo)
    {
        let vma_create_info = vk_mem::AllocatorCreateInfo::new(
            Rc::new(self.instance.as_ref().unwrap()),
            Rc::new(self.device.as_ref().unwrap()),
            self.physical_device.as_ref().unwrap().vk_physical_device,
        )
        .vulkan_api_version(init_info.vk_version);

        self.vma = Some(vk_mem::Allocator::new(vma_create_info).unwrap());
    }
}
