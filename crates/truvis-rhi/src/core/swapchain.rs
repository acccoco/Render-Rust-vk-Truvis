use std::rc::Rc;

use crate::core::device::RhiDevice;
use crate::{
    basic::FRAME_ID_MAP,
    core::{
        command_queue::RhiQueue,
        synchronize::{RhiFence, RhiSemaphore},
        window_system::MainWindow,
    },
    rhi::Rhi,
};
use ash::vk;
use itertools::Itertools;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub struct RhiSwapchainInitInfo {
    format: vk::SurfaceFormatKHR,
    swapchain_present_mode: vk::PresentModeKHR,

    window: Rc<MainWindow>,
}

impl RhiSwapchainInitInfo {
    #[inline]
    pub fn new(window: Rc<MainWindow>) -> Self {
        Self {
            format: vk::SurfaceFormatKHR {
                format: vk::Format::B8G8R8A8_UNORM,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            },
            swapchain_present_mode: vk::PresentModeKHR::MAILBOX,
            window,
        }
    }

    /// builder
    #[inline]
    pub fn format(mut self, format: vk::SurfaceFormatKHR) -> Self {
        self.format = format;
        self
    }
}

struct RhiSurface {
    handle: vk::SurfaceKHR,
    pf: ash::khr::surface::Instance,

    _window: Rc<MainWindow>,
}

impl RhiSurface {
    fn new(rhi: &Rhi, window: Rc<MainWindow>) -> Self {
        let surface_pf = ash::khr::surface::Instance::new(&rhi.vk_pf, rhi.instance());

        let surface = unsafe {
            ash_window::create_surface(
                &rhi.vk_pf,
                rhi.instance(),
                window.window().display_handle().unwrap().as_raw(),
                window.window().window_handle().unwrap().as_raw(),
                None,
            )
            .unwrap()
        };
        rhi.device.debug_utils.set_object_debug_name(surface, "main-surface");

        RhiSurface {
            handle: surface,
            pf: surface_pf,
            _window: window,
        }
    }
}

impl Drop for RhiSurface {
    fn drop(&mut self) {
        log::info!("destroying surface");
        unsafe { self.pf.destroy_surface(self.handle, None) }
    }
}

pub struct RhiSwapchain {
    swapchain_pf: ash::khr::swapchain::Device,
    swapchain_handle: vk::SwapchainKHR,

    device: Rc<RhiDevice>,

    _surface: RhiSurface,

    /// 这里的 image 并非手动创建的，因此无法使用 RhiImage 类型
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,

    pub extent: vk::Extent2D,
    pub color_format: vk::Format,
    pub color_space: vk::ColorSpaceKHR,
    pub present_mode: vk::PresentModeKHR,
}

impl RhiSwapchain {
    pub fn new(rhi: &Rhi, init_info: &RhiSwapchainInitInfo) -> Self {
        let pdevice = rhi.physical_device().handle;
        let surface = RhiSurface::new(rhi, init_info.window.clone());

        let present_mode = Self::init_present_mode(rhi, &surface, init_info.swapchain_present_mode);
        let (format, color_space) = Self::init_format_and_colorspace(rhi, &surface, init_info.format);

        let surface_capabilities =
            unsafe { surface.pf.get_physical_device_surface_capabilities(pdevice, surface.handle).unwrap() };

        let extent = surface_capabilities.current_extent;
        log::info!("surface capability extent: {:?}", extent);

        let (swapchain_handle, swapchain_pf) =
            Self::create_handle(rhi, &surface, &surface_capabilities, format, color_space, extent, present_mode);

        let (images, image_views) = Self::create_images_and_views(rhi, swapchain_handle, &swapchain_pf, format);

        Self {
            swapchain_pf,
            swapchain_handle,
            images,
            image_views,
            extent,
            color_format: format,
            color_space,
            present_mode,
            _surface: surface,
            device: rhi.device.clone(),
        }
    }

    fn create_handle(
        rhi: &Rhi,
        surface: &RhiSurface,
        surface_capabilities: &vk::SurfaceCapabilitiesKHR,
        format: vk::Format,
        color_space: vk::ColorSpaceKHR,
        extent: vk::Extent2D,
        present_mode: vk::PresentModeKHR,
    ) -> (vk::SwapchainKHR, ash::khr::swapchain::Device) {
        // 确定 image count
        // max_image_count == 0，表示不限制 image 数量
        let image_count = if surface_capabilities.max_image_count == 0 {
            surface_capabilities.min_image_count + 1
        } else {
            u32::min(surface_capabilities.max_image_count, surface_capabilities.min_image_count + 1)
        };

        log::info!("swapchain image count: {}", image_count);
        log::info!("swapchain format: {:?}", format);
        log::info!("swapchain color space: {:?}", color_space);
        log::info!("swapchain present mode: {:?}", present_mode);

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface.handle)
            .min_image_count(image_count)
            .image_format(format)
            .image_color_space(color_space)
            .image_extent(extent)
            .image_array_layers(1)
            // TRANSFER_DST 用于 Nsight 分析
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .clipped(true);

        unsafe {
            let swapchain_pf = ash::khr::swapchain::Device::new(rhi.instance(), rhi.device());
            let swapchain_handle = swapchain_pf.create_swapchain(&create_info, None).unwrap();
            rhi.device.debug_utils.set_object_debug_name(swapchain_handle, "main-swapchain");

            (swapchain_handle, swapchain_pf)
        }
    }

    fn create_images_and_views(
        rhi: &Rhi,
        swapchain_handle: vk::SwapchainKHR,
        swapchain_pf: &ash::khr::swapchain::Device,
        format: vk::Format,
    ) -> (Vec<vk::Image>, Vec<vk::ImageView>) {
        let swapchain_images = unsafe { swapchain_pf.get_swapchain_images(swapchain_handle).unwrap() };

        let image_views = swapchain_images
            .iter()
            .map(|img| {
                let create_info = vk::ImageViewCreateInfo::default()
                    .image(*img)
                    .format(format)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .level_count(1),
                    );

                unsafe { rhi.device().create_image_view(&create_info, None).unwrap() }
            })
            .collect_vec();

        let images = swapchain_images;

        // 为 images 和 image_views 设置 debug name
        for i in 0..images.len() {
            rhi.device.debug_utils.set_object_debug_name(images[i], format!("swapchain-image-{}", FRAME_ID_MAP[i]));
            rhi.device
                .debug_utils
                .set_object_debug_name(image_views[i], format!("swapchain-image-view-{}", FRAME_ID_MAP[i]));
        }

        (images, image_views)
    }

    /// 找到一个合适的 present mode
    ///
    /// @param present_mode: 优先使用的 present mode
    ///
    /// 可以是：immediate, mailbox, fifo, fifo_relaxed
    fn init_present_mode(rhi: &Rhi, surface: &RhiSurface, present_mode: vk::PresentModeKHR) -> vk::PresentModeKHR {
        unsafe {
            surface
                .pf
                .get_physical_device_surface_present_modes(rhi.physical_device().handle, surface.handle)
                .unwrap()
                .iter()
                .find_or_first(|p| **p == present_mode)
                .copied()
                .unwrap()
        }
    }

    /// 找到合适的 format 和 colorspace
    ///
    /// @param format: 优先使用的 format
    ///
    /// panic: 如果没有找到，就 panic
    fn init_format_and_colorspace(
        rhi: &Rhi,
        surface: &RhiSurface,
        format: vk::SurfaceFormatKHR,
    ) -> (vk::Format, vk::ColorSpaceKHR) {
        let surface_format = unsafe {
            surface
                .pf
                .get_physical_device_surface_formats(rhi.physical_device().handle, surface.handle)
                .unwrap()
                .into_iter()
                .find(|f| *f == format)
                .unwrap()
        };

        (surface_format.format, surface_format.color_space)
    }

    #[inline]
    pub fn acquire_next_frame(&self, semaphore: &RhiSemaphore, fence: Option<&RhiFence>) -> u32 {
        let (image_index, is_optimal) = unsafe {
            self.swapchain_pf
                .acquire_next_image(
                    self.swapchain_handle,
                    u64::MAX,
                    semaphore.semaphore,
                    fence.map_or(vk::Fence::null(), |f| f.fence),
                )
                .unwrap()
        };

        // TODO 解决 suboptimal 的问题
        if !is_optimal {
            // log::warn!("swapchain acquire image index {} is not optimal", image_index);
        }

        image_index
    }

    #[inline]
    pub fn submit_frame(&self, queue: &RhiQueue, image_index: u32, wait_semaphores: &[RhiSemaphore]) {
        let wait_semaphores = wait_semaphores.iter().map(|s| s.semaphore).collect_vec();
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .image_indices(std::slice::from_ref(&image_index))
            .swapchains(std::slice::from_ref(&self.swapchain_handle));

        unsafe { self.swapchain_pf.queue_present(queue.handle, &present_info).unwrap() };
    }
}

impl Drop for RhiSwapchain {
    fn drop(&mut self) {
        log::info!("destroying swapchain");
        unsafe {
            for view in &self.image_views {
                self.device.destroy_image_view(*view, None);
            }
            self.swapchain_pf.destroy_swapchain(self.swapchain_handle, None);
        }
    }
}
