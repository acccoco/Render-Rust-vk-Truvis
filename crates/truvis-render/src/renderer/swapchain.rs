use std::rc::Rc;

use ash::vk;
use itertools::Itertools;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use truvis_rhi::{
    commands::{command_queue::CommandQueue, fence::Fence, semaphore::Semaphore},
    foundation::{debug_messenger::DebugType, device::Device},
    render_context::RenderContext,
    resources::image_view::{Image2DView, ImageViewCreateInfo},
};

use crate::pipeline_settings::PresentSettings;

struct RhiSurface {
    handle: vk::SurfaceKHR,
    pf: ash::khr::surface::Instance,

    capabilities: vk::SurfaceCapabilitiesKHR,
}
impl RhiSurface {
    fn new(rhi: &RenderContext, window: &winit::window::Window) -> Self {
        let surface_pf = ash::khr::surface::Instance::new(&rhi.vk_pf, rhi.instance());

        let surface = unsafe {
            ash_window::create_surface(
                &rhi.vk_pf,
                rhi.instance(),
                window.display_handle().unwrap().as_raw(),
                window.window_handle().unwrap().as_raw(),
                None,
            )
            .unwrap()
        };

        let surface_capabilities = unsafe {
            surface_pf.get_physical_device_surface_capabilities(rhi.physical_device().handle, surface).unwrap()
        };

        let surface = RhiSurface {
            handle: surface,
            pf: surface_pf,
            capabilities: surface_capabilities,
        };
        rhi.device.debug_utils().set_debug_name(&surface, "main");

        surface
    }
}
impl Drop for RhiSurface {
    fn drop(&mut self) {
        unsafe { self.pf.destroy_surface(self.handle, None) }
    }
}
impl DebugType for RhiSurface {
    fn debug_type_name() -> &'static str {
        "RhiSurface"
    }
    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
}

pub struct RenderSwapchain {
    _device: Rc<RhiDeviceFunctions>,
    _surface: RhiSurface,
    swapchain_pf: ash::khr::swapchain::Device,
    swapchain_handle: vk::SwapchainKHR,

    /// 这里的 image 并非手动创建的，因此无法使用 RhiImage 类型
    images: Vec<vk::Image>,
    image_views: Vec<Image2DView>,
    swapchain_image_index: usize,

    color_format: vk::Format,
    extent: vk::Extent2D,
}
impl RenderSwapchain {
    // region ============== constructor ============

    pub fn new(
        rhi: &RenderContext,
        window: &winit::window::Window,
        present_mode: vk::PresentModeKHR,
        surface_format: vk::SurfaceFormatKHR,
    ) -> Self {
        let surface = RhiSurface::new(rhi, window);
        let swapchain_pf = ash::khr::swapchain::Device::new(rhi.instance(), rhi.device());

        let extent = surface.capabilities.current_extent;

        let swapchain_handle = Self::create_swapchain(
            rhi,
            &swapchain_pf,
            &surface,
            surface_format.format,
            surface_format.color_space,
            extent,
            present_mode,
        );

        let images = unsafe { swapchain_pf.get_swapchain_images(swapchain_handle).unwrap() };
        for (img_idx, img) in images.iter().enumerate() {
            rhi.device.debug_utils().set_object_debug_name(*img, format!("swapchain-image-{img_idx}"));
        }
        let image_views = images
            .iter()
            .enumerate()
            .map(|(idx, img)| {
                Image2DView::new(
                    rhi,
                    *img,
                    ImageViewCreateInfo::new_image_view_2d_info(surface_format.format, vk::ImageAspectFlags::COLOR),
                    format!("swapchain-{}", idx),
                )
            })
            .collect_vec();

        Self {
            _device: rhi.device.clone(),
            _surface: surface,
            swapchain_pf,
            swapchain_handle,
            images,
            image_views,
            swapchain_image_index: 0,
            extent,
            color_format: surface_format.format,
        }
    }

    fn create_swapchain(
        rhi: &RenderContext,
        swapchain_pf: &ash::khr::swapchain::Device,
        surface: &RhiSurface,
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
            .image_usage(
                vk::ImageUsageFlags::COLOR_ATTACHMENT
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::STORAGE,
            )
            .pre_transform(surface.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .clipped(true);

        unsafe {
            let swapchain_handle = swapchain_pf.create_swapchain(&create_info, None).unwrap();
            rhi.device.debug_utils().set_object_debug_name(swapchain_handle, "main");

            swapchain_handle
        }
    }

    // endregion ===================

    // region ============== getter ============

    #[inline]
    pub fn present_images(&self) -> Vec<vk::Image> {
        self.images.clone()
    }

    #[inline]
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    #[inline]
    pub fn current_image(&self) -> vk::Image {
        self.images[self.swapchain_image_index]
    }

    #[inline]
    pub fn current_image_index(&self) -> usize {
        self.swapchain_image_index
    }

    #[inline]
    pub fn current_image_view(&self) -> &Image2DView {
        &self.image_views[self.swapchain_image_index]
    }

    #[inline]
    pub fn present_settings(&self) -> PresentSettings {
        PresentSettings {
            canvas_extent: self.extent,
            swapchain_image_cnt: self.images.len(),
            color_format: self.color_format,
        }
    }

    // endregion ===================

    /// timeout: nano seconds
    #[inline]
    pub fn acquire_next_image(&mut self, semaphore: Option<&Semaphore>, fence: Option<&Fence>, timeout: u64) {
        let (image_index, is_optimal) = unsafe {
            self.swapchain_pf
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
    pub fn present_image(&self, queue: &CommandQueue, wait_semaphores: &[Semaphore]) {
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
