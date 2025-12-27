use crate::commands::command_queue::GfxCommandQueue;
use crate::commands::fence::GfxFence;
use crate::commands::semaphore::GfxSemaphore;
use crate::gfx::Gfx;
use crate::gfx_core::GfxCore;
use crate::resources::image_view::{GfxImageView, GfxImageViewDesc};
use crate::swapchain::surface::GfxSurface;
use ash::vk;
use ash::vk::Handle;
use itertools::Itertools;

pub struct GfxRenderSwapchain {
    _surface: GfxSurface,
    swapchain_handle: vk::SwapchainKHR,

    /// 这里的 image 并非手动创建的，因此无法使用 GfxImage 类型
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<GfxImageView>,
    swapchain_image_index: usize,

    color_format: vk::Format,
    swapchain_extent: vk::Extent2D,
}

// new & init
impl GfxRenderSwapchain {
    pub fn new(
        vk_core: &GfxCore,
        raw_display_handle: raw_window_handle::RawDisplayHandle,
        raw_window_handle: raw_window_handle::RawWindowHandle,
        present_mode: vk::PresentModeKHR,
        surface_format: vk::SurfaceFormatKHR,
    ) -> Self {
        let surface = GfxSurface::new(vk_core, raw_display_handle, raw_window_handle);
        let extent = surface.capabilities.current_extent;

        let swapchain_handle =
            Self::create_swapchain(&surface, surface_format.format, surface_format.color_space, extent, present_mode);

        let images = unsafe { vk_core.gfx_device.swapchain.get_swapchain_images(swapchain_handle).unwrap() };
        for (img_idx, img) in images.iter().enumerate() {
            vk_core.gfx_device.set_object_debug_name(*img, format!("swapchain-image-{img_idx}"));
        }
        let image_views = images
            .iter()
            .enumerate()
            .map(|(idx, img)| {
                GfxImageView::new(
                    *img,
                    GfxImageViewDesc::new_2d(surface_format.format, vk::ImageAspectFlags::COLOR),
                    format!("swapchain-{}", idx),
                )
            })
            .collect_vec();

        Self {
            _surface: surface,
            swapchain_handle,
            swapchain_images: images,
            swapchain_image_views: image_views,
            swapchain_image_index: 0,
            swapchain_extent: extent,
            color_format: surface_format.format,
        }
    }

    fn create_swapchain(
        surface: &GfxSurface,
        format: vk::Format,
        color_space: vk::ColorSpaceKHR,
        extent: vk::Extent2D,
        present_mode: vk::PresentModeKHR,
    ) -> vk::SwapchainKHR {
        // 确定 image count
        // max_image_count == 0，表示不限制 image 数量
        let image_count = if surface.capabilities.max_image_count == 0 {
            surface.capabilities.min_image_count + 1
        } else {
            u32::min(surface.capabilities.max_image_count, surface.capabilities.min_image_count + 1)
        };

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface.handle)
            .min_image_count(image_count)
            .image_format(format)
            .image_color_space(color_space)
            .image_extent(extent)
            .image_array_layers(1)
            // TRANSFER_DST 用于 Nsight 分析
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
            .pre_transform(surface.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .clipped(true);

        let gfx_device = Gfx::get().gfx_device();
        unsafe {
            let swapchain_handle = gfx_device.swapchain.create_swapchain(&create_info, None).unwrap();
            gfx_device.set_object_debug_name(swapchain_handle, "main");

            swapchain_handle
        }
    }
}
pub struct GfxSwapchainImageInfo {
    pub image_extent: vk::Extent2D,
    pub image_cnt: usize,
    pub image_format: vk::Format,
}
// getters
impl GfxRenderSwapchain {
    #[inline]
    pub fn present_images(&self) -> Vec<vk::Image> {
        self.swapchain_images.clone()
    }

    #[inline]
    pub fn extent(&self) -> vk::Extent2D {
        self.swapchain_extent
    }

    #[inline]
    pub fn current_image(&self) -> vk::Image {
        self.swapchain_images[self.swapchain_image_index]
    }

    #[inline]
    pub fn current_image_index(&self) -> usize {
        self.swapchain_image_index
    }

    #[inline]
    pub fn current_image_view(&self) -> &GfxImageView {
        &self.swapchain_image_views[self.swapchain_image_index]
    }

    #[inline]
    pub fn image_infos(&self) -> GfxSwapchainImageInfo {
        GfxSwapchainImageInfo {
            image_extent: self.swapchain_extent,
            image_cnt: self.swapchain_images.len(),
            image_format: self.color_format,
        }
    }
}
// tools
impl GfxRenderSwapchain {
    /// timeout: nano seconds
    #[inline]
    pub fn acquire_next_image(&mut self, semaphore: Option<&GfxSemaphore>, fence: Option<&GfxFence>, timeout: u64) {
        let (image_index, is_optimal) = unsafe {
            Gfx::get()
                .gfx_device()
                .swapchain
                .acquire_next_image(
                    self.swapchain_handle,
                    timeout,
                    semaphore.map_or(vk::Semaphore::null(), |s| s.handle()),
                    fence.map_or(vk::Fence::null(), |f| f.handle()),
                )
                .unwrap()
        };

        // TODO 解决 suboptimal 的问题
        if !is_optimal {
            // log::warn!("swapchain acquire image index {} is not optimal",
            // image_index);
        }

        self.swapchain_image_index = image_index as usize;
    }

    #[inline]
    pub fn present_image(&self, queue: &GfxCommandQueue, wait_semaphores: &[GfxSemaphore]) {
        let wait_semaphores = wait_semaphores.iter().map(|s| s.handle()).collect_vec();
        let image_indices = [self.swapchain_image_index as u32];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .image_indices(&image_indices)
            .swapchains(std::slice::from_ref(&self.swapchain_handle));

        unsafe { Gfx::get().gfx_device().swapchain.queue_present(queue.handle(), &present_info).unwrap() };
    }
}
// destroy
impl GfxRenderSwapchain {
    pub fn destroy(mut self) {
        unsafe {
            let gfx_device = Gfx::get().gfx_device();
            for image_view in self.swapchain_image_views.drain(..) {
                image_view.destroy();
            }
            gfx_device.swapchain.destroy_swapchain(self.swapchain_handle, None);
        }
        self.swapchain_images.clear();
        self.swapchain_handle = vk::SwapchainKHR::null();
    }
}
impl Drop for GfxRenderSwapchain {
    fn drop(&mut self) {
        debug_assert!(self.swapchain_images.is_empty());
        debug_assert!(self.swapchain_handle.is_null());
    }
}
