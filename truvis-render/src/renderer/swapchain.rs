use crate::renderer::window_system::MainWindow;
use ash::vk;
use itertools::Itertools;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::rc::Rc;
use truvis_rhi::core::command_queue::RhiQueue;
use truvis_rhi::core::debug_utils::RhiDebugType;
use truvis_rhi::core::device::RhiDevice;
use truvis_rhi::core::synchronize::{RhiFence, RhiSemaphore};
use truvis_rhi::rhi::Rhi;

struct RhiSurface {
    handle: vk::SurfaceKHR,
    pf: ash::khr::surface::Instance,
}
impl RhiSurface {
    fn new(rhi: &Rhi, window: &MainWindow) -> Self {
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
        let surface = RhiSurface {
            handle: surface,
            pf: surface_pf,
        };
        rhi.device.debug_utils().set_debug_name(&surface, "main-surface");

        surface
    }
}
impl Drop for RhiSurface {
    fn drop(&mut self) {
        unsafe { self.pf.destroy_surface(self.handle, None) }
    }
}
impl RhiDebugType for RhiSurface {
    fn debug_type_name() -> &'static str {
        "RhiSurface"
    }
    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
}

pub struct RenderSwapchain {
    swapchain_pf: ash::khr::swapchain::Device,
    swapchain_handle: vk::SwapchainKHR,

    _device: Rc<RhiDevice>,

    _surface: RhiSurface,

    /// 这里的 image 并非手动创建的，因此无法使用 RhiImage 类型
    images: Vec<vk::Image>,

    swapchain_image_index: usize,

    extent: vk::Extent2D,
}
// getter
impl RenderSwapchain {
    #[inline]
    pub fn present_images(&self) -> Vec<vk::Image> {
        self.images.clone()
    }

    #[inline]
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    #[inline]
    pub fn current_present_image(&self) -> vk::Image {
        self.images[self.swapchain_image_index]
    }

    #[inline]
    pub fn current_present_image_index(&self) -> usize {
        self.swapchain_image_index
    }
}
impl RenderSwapchain {
    pub fn new(
        rhi: &Rhi,
        window: &MainWindow,
        present_mode: vk::PresentModeKHR,
        surface_format: vk::SurfaceFormatKHR,
    ) -> Self {
        let pdevice = rhi.physical_device().handle;
        let surface = RhiSurface::new(rhi, window);

        let present_mode = Self::init_present_mode(rhi, &surface, present_mode);
        let (format, color_space) = Self::init_format_and_colorspace(rhi, &surface, surface_format);

        let surface_capabilities =
            unsafe { surface.pf.get_physical_device_surface_capabilities(pdevice, surface.handle).unwrap() };

        let extent = surface_capabilities.current_extent;

        let (swapchain_handle, swapchain_pf) =
            Self::create_handle(rhi, &surface, &surface_capabilities, format, color_space, extent, present_mode);

        let images = unsafe { swapchain_pf.get_swapchain_images(swapchain_handle).unwrap() };

        Self {
            swapchain_pf,
            swapchain_handle,
            images,
            swapchain_image_index: 0,
            extent,
            _surface: surface,
            _device: rhi.device.clone(),
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

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface.handle)
            .min_image_count(image_count)
            .image_format(format)
            .image_color_space(color_space)
            .image_extent(extent)
            .image_array_layers(1)
            // TRANSFER_DST 用于 Nsight 分析
            .image_usage(
                vk::ImageUsageFlags::COLOR_ATTACHMENT
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::STORAGE,
            )
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .clipped(true);

        unsafe {
            let swapchain_pf = ash::khr::swapchain::Device::new(rhi.instance(), rhi.device());
            let swapchain_handle = swapchain_pf.create_swapchain(&create_info, None).unwrap();
            rhi.device.debug_utils().set_object_debug_name(swapchain_handle, "main-swapchain");

            (swapchain_handle, swapchain_pf)
        }
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
    pub fn acquire(&mut self, semaphore: &RhiSemaphore, fence: Option<&RhiFence>) {
        let (image_index, is_optimal) = unsafe {
            self.swapchain_pf
                .acquire_next_image(
                    self.swapchain_handle,
                    u64::MAX,
                    semaphore.handle(),
                    fence.map_or(vk::Fence::null(), |f| f.handle()),
                )
                .unwrap()
        };

        // TODO 解决 suboptimal 的问题
        if !is_optimal {
            // log::warn!("swapchain acquire image index {} is not optimal", image_index);
        }

        self.swapchain_image_index = image_index as usize;
    }

    #[inline]
    pub fn submit(&self, queue: &RhiQueue, wait_semaphores: &[RhiSemaphore]) {
        let wait_semaphores = wait_semaphores.iter().map(|s| s.handle()).collect_vec();
        let image_indices = [self.swapchain_image_index as u32];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .image_indices(&image_indices)
            .swapchains(std::slice::from_ref(&self.swapchain_handle));

        unsafe { self.swapchain_pf.queue_present(queue.handle(), &present_info).unwrap() };
    }
}
impl Drop for RenderSwapchain {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_pf.destroy_swapchain(self.swapchain_handle, None);
        }
    }
}
