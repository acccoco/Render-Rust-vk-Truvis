use std::sync::Arc;

use ash::vk;
use itertools::Itertools;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::framework::{
    basic::FRAME_ID_MAP,
    core::synchronize::{Fence, Semaphore},
    platform::window_system::WindowSystem,
    render_core::Core,
};


struct Surface
{
    handle: vk::SurfaceKHR,
    pf: ash::khr::surface::Instance,
}

pub struct Swapchain
{
    swapchain_pf: ash::khr::swapchain::Device,
    swapchain_handle: vk::SwapchainKHR,

    surface: Surface,

    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,

    pub extent: vk::Extent2D,
    pub color_format: vk::Format,
    pub color_space: vk::ColorSpaceKHR,
    pub present_mode: vk::PresentModeKHR,
}


impl Swapchain
{
    pub fn new(rhi: &Core, init_info: &SwapchainInitInfo) -> Self
    {
        let pdevice = rhi.physical_device().handle;
        let surface = Self::create_surface(rhi, init_info.window.as_ref().unwrap());

        let present_mode = Self::init_present_mode2(rhi, &surface, init_info.swapchain_present_mode);
        let (format, color_space) = Self::init_format_and_colorspace(rhi, &surface, init_info.format);

        let surface_capabilities =
            unsafe { surface.pf.get_physical_device_surface_capabilities(pdevice, surface.handle).unwrap() };

        let extent = surface_capabilities.current_extent;

        let (swapchain_handle, swapchain_pf) =
            Self::init_handle(rhi, &surface, &surface_capabilities, format, color_space, extent, present_mode);

        let (images, image_views) = Self::init_images_and_views(rhi, swapchain_handle, &swapchain_pf, format);

        let swapchain = Self {
            swapchain_pf,
            swapchain_handle,
            images,
            image_views,
            extent,
            color_format: format,
            color_space,
            present_mode,
            surface,
        };

        swapchain
    }


    fn init_handle(
        rhi: &Core,
        surface: &Surface,
        surface_capabilities: &vk::SurfaceCapabilitiesKHR,
        format: vk::Format,
        color_space: vk::ColorSpaceKHR,
        extent: vk::Extent2D,
        present_mode: vk::PresentModeKHR,
    ) -> (vk::SwapchainKHR, ash::khr::swapchain::Device)
    {
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
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .clipped(true);

        unsafe {
            let swapchain_pf = ash::khr::swapchain::Device::new(rhi.vk_instance(), rhi.vk_device());
            let swapchain_handle = swapchain_pf.create_swapchain(&create_info, None).unwrap();
            rhi.set_debug_name(swapchain_handle, "main-swapchain");

            (swapchain_handle, swapchain_pf)
        }
    }

    fn init_images_and_views(
        rhi: &Core,
        swapchain_handle: vk::SwapchainKHR,
        swapchain_pf: &ash::khr::swapchain::Device,
        format: vk::Format,
    ) -> (Vec<vk::Image>, Vec<vk::ImageView>)
    {
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

                unsafe { rhi.vk_device().create_image_view(&create_info, None).unwrap() }
            })
            .collect_vec();

        let images = swapchain_images;

        // 为 images 和 image_views 设置 debug name
        for i in 0..images.len() {
            rhi.set_debug_name(images[i], format!("swapchain-image-{}", FRAME_ID_MAP[i]));
            rhi.set_debug_name(image_views[i], format!("swapchain-image-view-{}", FRAME_ID_MAP[i]));
        }

        (images, image_views)
    }

    /// 找到一个合适的 present mode
    ///
    /// @param present_mode: 优先使用的 present mode
    ///
    /// 可以是：immediate, mailbox, fifo, fifo_relaxed
    fn init_present_mode2(rhi: &Core, surface: &Surface, present_mode: vk::PresentModeKHR) -> vk::PresentModeKHR
    {
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
        rhi: &Core,
        surface: &Surface,
        format: vk::SurfaceFormatKHR,
    ) -> (vk::Format, vk::ColorSpaceKHR)
    {
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
    pub fn acquire_next_frame(&self, semaphore: &Semaphore, fence: Option<&Fence>) -> u32
    {
        // TODO 处理 optimal 的情况
        let (image_index, _is_optimal) = unsafe {
            self.swapchain_pf
                .acquire_next_image(
                    self.swapchain_handle,
                    u64::MAX,
                    semaphore.semaphore,
                    fence.map_or(vk::Fence::null(), |f| f.fence),
                )
                .unwrap()
        };
        // TODO 处理 optimal
        image_index
    }

    #[inline]
    pub fn submit_frame(&self, rhi: &Core, image_index: u32, wait_semaphores: &[vk::Semaphore])
    {
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(wait_semaphores)
            .image_indices(std::slice::from_ref(&image_index))
            .swapchains(std::slice::from_ref(&self.swapchain_handle));

        unsafe { self.swapchain_pf.queue_present(rhi.graphics_queue().vk_queue, &present_info).unwrap() };
    }


    fn create_surface(rhi: &Core, window: &WindowSystem) -> Surface
    {
        let surface_pf = ash::khr::surface::Instance::new(&rhi.vk_pf, rhi.vk_instance());

        let surface = unsafe {
            ash_window::create_surface(
                &rhi.vk_pf,
                rhi.vk_instance(),
                window.window().raw_display_handle().unwrap(),
                window.window().raw_window_handle().unwrap(),
                None,
            )
            .unwrap()
        };
        rhi.set_debug_name(surface, "main-surface");

        Surface {
            handle: surface,
            pf: surface_pf,
        }
    }
}


pub struct SwapchainInitInfo
{
    pub format: vk::SurfaceFormatKHR,
    pub swapchain_present_mode: vk::PresentModeKHR,

    pub window: Option<Arc<WindowSystem>>, // TODO 移除这个 Option，增加理解负担
}

impl Default for SwapchainInitInfo
{
    fn default() -> Self
    {
        Self {
            // 以下字段表示 present engine 应该如何处理线性颜色值。shader 还有 image 都不用关心这两个字段
            format: vk::SurfaceFormatKHR {
                format: vk::Format::B8G8R8A8_UNORM,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            },
            swapchain_present_mode: vk::PresentModeKHR::MAILBOX,
            window: None,
        }
    }
}
