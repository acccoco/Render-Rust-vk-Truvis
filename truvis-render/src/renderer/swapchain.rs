use crate::pipeline_settings::FRAME_ID_MAP;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::window_system::MainWindow;
use ash::vk;
use itertools::Itertools;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use shader_binding::shader;
use std::iter::zip;
use std::rc::Rc;
use truvis_rhi::core::command_queue::RhiQueue;
use truvis_rhi::core::debug_utils::RhiDebugType;
use truvis_rhi::core::device::RhiDevice;
use truvis_rhi::core::image::{RhiImage2DView, RhiImageViewCreateInfo};
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

pub struct RhiSwapchain {
    swapchain_pf: ash::khr::swapchain::Device,
    swapchain_handle: vk::SwapchainKHR,

    _device: Rc<RhiDevice>,

    _surface: RhiSurface,

    /// 这里的 image 并非手动创建的，因此无法使用 RhiImage 类型
    images: Vec<vk::Image>,
    image_views: Vec<Rc<RhiImage2DView>>,
    image_keywords: Vec<String>,

    swapchain_image_index: usize,

    extent: vk::Extent2D,
    color_format: vk::Format,
    color_space: vk::ColorSpaceKHR,
    present_mode: vk::PresentModeKHR,
}
// getter
impl RhiSwapchain {
    #[inline]
    pub fn images(&self) -> &[vk::Image] {
        &self.images
    }

    #[inline]
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    #[inline]
    pub fn color_format(&self) -> vk::Format {
        self.color_format
    }

    #[inline]
    pub fn color_space(&self) -> vk::ColorSpaceKHR {
        self.color_space
    }

    #[inline]
    pub fn present_mode(&self) -> vk::PresentModeKHR {
        self.present_mode
    }

    #[inline]
    pub fn current_present_image(&self) -> vk::Image {
        self.images[self.swapchain_image_index]
    }

    #[inline]
    pub fn current_present_image_view(&self) -> vk::ImageView {
        self.image_views[self.swapchain_image_index].handle()
    }

    #[inline]
    pub fn current_present_bindless_handle(&self, bindless_mgr: &BindlessManager) -> shader::ImageHandle {
        bindless_mgr.get_image_idx(&self.image_keywords[self.swapchain_image_index]).unwrap()
    }
}
impl RhiSwapchain {
    pub fn new(
        rhi: &Rhi,
        window: &MainWindow,
        present_mode: vk::PresentModeKHR,
        surface_format: vk::SurfaceFormatKHR,
        bindless_mgr: &mut BindlessManager,
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

        let (images, image_views) = Self::create_images_and_views(rhi, swapchain_handle, &swapchain_pf, format);

        let image_keywords =
            images.iter().enumerate().map(|(i, _)| format!("swapchain-image-{}", FRAME_ID_MAP[i])).collect_vec();
        for (image_view, keyword) in zip(&image_views, &image_keywords) {
            bindless_mgr.register_image(keyword.clone(), image_view.clone());
        }

        Self {
            swapchain_pf,
            swapchain_handle,
            images,
            image_views,
            image_keywords,
            swapchain_image_index: 0,
            extent,
            color_format: format,
            color_space,
            present_mode,
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

    fn create_images_and_views(
        rhi: &Rhi,
        swapchain_handle: vk::SwapchainKHR,
        swapchain_pf: &ash::khr::swapchain::Device,
        format: vk::Format,
    ) -> (Vec<vk::Image>, Vec<Rc<RhiImage2DView>>) {
        let swapchain_images = unsafe { swapchain_pf.get_swapchain_images(swapchain_handle).unwrap() };

        // 为 images 设置 debug name
        for (i, img) in swapchain_images.iter().enumerate() {
            rhi.device.debug_utils().set_object_debug_name(*img, format!("swapchain-image-{i}"));
        }

        let image_views = swapchain_images
            .iter()
            .enumerate()
            .map(|(idx, img)| {
                let ci = RhiImageViewCreateInfo::new_image_view_2d_info(format, vk::ImageAspectFlags::COLOR);
                let image_view =
                    RhiImage2DView::new_with_raw_image(rhi, *img, ci, format!("swapchain-image-view-{idx}"));
                Rc::new(image_view)
            })
            .collect_vec();

        (swapchain_images, image_views)
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

    /// 需要手动销毁 swapchain
    pub fn destroy(mut self, bindless_mgr: &mut BindlessManager) {
        unsafe {
            for view in std::mem::take(&mut self.image_views) {
                drop(view)
            }
            for keyword in std::mem::take(&mut self.image_keywords) {
                bindless_mgr.unregister_image(&keyword);
            }
            self.swapchain_pf.destroy_swapchain(self.swapchain_handle, None);
        }
    }
}
impl Drop for RhiSwapchain {
    fn drop(&mut self) {
        assert!(self.image_views.is_empty(), "RhiSwapchain should be destroyed manually");
    }
}
