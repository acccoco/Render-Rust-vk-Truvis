use ash::{vk, vk::Window};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::framework::{
    core::synchronize::{RhiFence, RhiSemaphore},
    platform::window_system::WindowSystem,
    rhi::Rhi,
};

static mut SWAPCHAIN: Option<RenderSwapchain> = None;

pub struct RenderSwapchain
{
    pub(crate) swapchain_pf: ash::extensions::khr::Swapchain,
    pub(crate) handle: Option<vk::SwapchainKHR>,

    pub(crate) surface: Option<vk::SurfaceKHR>,
    surface_pf: Option<ash::extensions::khr::Surface>,

    pub(crate) images: Vec<vk::Image>,
    pub(crate) image_views: Vec<vk::ImageView>,

    pub(crate) extent: Option<vk::Extent2D>,
    pub(crate) format: Option<vk::Format>,
    color_space: Option<vk::ColorSpaceKHR>,
    present_mode: Option<vk::PresentModeKHR>,

    surface_capabilities: vk::SurfaceCapabilitiesKHR,
    surface_formats: Vec<vk::SurfaceFormatKHR>,
    surface_present_modes: Vec<vk::PresentModeKHR>,

    pub(crate) color_attach_infos: Vec<vk::RenderingAttachmentInfo>,
}

pub struct RenderSwapchainInitInfo<'a>
{
    pub format: vk::SurfaceFormatKHR,
    pub swapchain_present_mode: vk::PresentModeKHR,
    pub window: Option<&'a WindowSystem>,
}

impl Default for RenderSwapchainInitInfo<'_>
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


impl RenderSwapchain
{
    #[inline]
    pub fn acquire_next_frame(&self, semaphore: &RhiSemaphore, fence: Option<&RhiFence>) -> u32
    {
        unsafe {
            // TODO 处理 optimal 的情况
            let (image_index, _is_optimal) = self
                .swapchain_pf
                .acquire_next_image(
                    self.handle.unwrap_unchecked(),
                    u64::MAX,
                    semaphore.semaphore,
                    fence.map_or(vk::Fence::null(), |f| f.fence),
                )
                .unwrap();
            // TODO 处理 optimal
            image_index
        }
    }

    #[inline]
    pub fn submit_frame(&self, rhi: &Rhi, image_index: u32, wait_semaphores: &[vk::Semaphore])
    {
        unsafe {
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(wait_semaphores)
                .image_indices(std::slice::from_ref(&image_index))
                .swapchains(std::slice::from_ref(self.handle.as_ref().unwrap_unchecked()));

            self.swapchain_pf.queue_present(rhi.graphics_queue().queue, &present_info).unwrap();
        }
    }


    fn create_surface(rhi: &Rhi, window: &WindowSystem) -> (vk::SurfaceKHR, ash::extensions::khr::Surface)
    {
        let surface_pf = ash::extensions::khr::Surface::new(rhi.vk_pf(), rhi.vk_instance());

        let surface = unsafe {
            ash_window::create_surface(
                rhi.vk_pf(),
                rhi.vk_instance(),
                window.window().raw_display_handle(),
                window.window().raw_window_handle(),
                None,
            )
            .unwrap()
        };
        rhi.set_debug_name(surface, "main-surface");

        (surface, surface_pf)
    }
}

mod _render_swapchain_property
{
    use ash::vk;

    use crate::framework::core::swapchain::RenderSwapchain;

    impl RenderSwapchain
    {
        #[inline]
        pub fn extent(&self) -> vk::Extent2D
        {
            unsafe { *self.extent.as_ref().unwrap_unchecked() }
        }

        #[inline]
        pub fn color_format(&self) -> vk::Format
        {
            unsafe { self.format.unwrap_unchecked() }
        }
    }
}

mod _render_swapchain_init
{
    use std::mem::swap;

    use ash::vk;
    use itertools::Itertools;

    use crate::framework::{
        core::swapchain::{RenderSwapchain, RenderSwapchainInitInfo, SWAPCHAIN},
        rhi::Rhi,
    };

    impl RenderSwapchain
    {
        pub fn new(rhi: &Rhi, init_info: &RenderSwapchainInitInfo) -> Self
        {
            let mut swapchain = unsafe {
                let pdevice = rhi.physical_device().handle;
                let (surface, surface_pf) = Self::create_surface(rhi, init_info.window.unwrap());

                Self {
                    swapchain_pf: ash::extensions::khr::Swapchain::new(rhi.vk_instance(), rhi.device()),
                    handle: None,
                    images: Vec::new(),
                    image_views: Vec::new(),
                    extent: None,
                    format: None,
                    color_space: None,
                    present_mode: None,
                    surface_capabilities: surface_pf
                        .get_physical_device_surface_capabilities(pdevice, surface)
                        .unwrap(),
                    surface_formats: surface_pf.get_physical_device_surface_formats(pdevice, surface).unwrap(),
                    surface_present_modes: surface_pf
                        .get_physical_device_surface_present_modes(pdevice, surface)
                        .unwrap(),
                    surface: Some(surface),
                    surface_pf: Some(surface_pf),
                    color_attach_infos: vec![],
                }
            };
            swapchain.init_format(init_info.format);
            swapchain.init_present_mode(init_info.swapchain_present_mode);
            swapchain.init_extent();
            swapchain.init_handle(rhi);
            swapchain.init_images_and_views(rhi);
            swapchain.init_color_attachs();

            swapchain
        }

        fn init_color_attachs(&mut self)
        {
            self.color_attach_infos = self
                .image_views
                .iter()
                .map(|v| {
                    vk::RenderingAttachmentInfo::builder()
                        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .image_view(*v)
                        .load_op(vk::AttachmentLoadOp::CLEAR)
                        .store_op(vk::AttachmentStoreOp::STORE)
                        .clear_value(vk::ClearValue {
                            color: vk::ClearColorValue {
                                float32: [0_f32, 0_f32, 0_f32, 1_f32],
                            },
                        })
                        .build()
                })
                .collect();
            //
        }


        fn init_extent(&mut self)
        {
            self.extent = Some(self.surface_capabilities.current_extent)
        }

        fn init_handle(&mut self, rhi: &Rhi)
        {
            // 确定 image count
            // max_image_count == 0，表示不限制 image 数量
            let image_count = if self.surface_capabilities.max_image_count == 0 {
                self.surface_capabilities.min_image_count + 1
            } else {
                u32::min(self.surface_capabilities.max_image_count, self.surface_capabilities.min_image_count + 1)
            };

            let create_info = vk::SwapchainCreateInfoKHR::builder()
                .surface(self.surface.unwrap())
                .min_image_count(image_count)
                .image_format(self.format.unwrap())
                .image_color_space(self.color_space.unwrap())
                .image_extent(self.extent.unwrap())
                .image_array_layers(1)
                // TRANSFER_DST 用于 Nsight 分析
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
                .pre_transform(self.surface_capabilities.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(self.present_mode.unwrap())
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .clipped(true);

            unsafe {
                let swapchain = self.swapchain_pf.create_swapchain(&create_info, None).unwrap();
                rhi.set_debug_name(swapchain, "main-swapchain");
                self.handle = Some(swapchain);
            }
        }

        fn init_images_and_views(&mut self, rhi: &Rhi)
        {
            let swapchain_images = unsafe { self.swapchain_pf.get_swapchain_images(self.handle.unwrap()).unwrap() };

            let image_views = swapchain_images
                .iter()
                .map(|img| {
                    let create_info = vk::ImageViewCreateInfo::builder()
                        .image(*img)
                        .format(self.format.unwrap())
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .subresource_range(
                            vk::ImageSubresourceRange::builder()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .layer_count(1)
                                .level_count(1)
                                .build(),
                        );

                    unsafe { rhi.device().create_image_view(&create_info, None).unwrap() }
                })
                .collect_vec();

            self.images = swapchain_images;
            self.image_views = image_views;

            for i in 0..self.images.len() {
                rhi.set_debug_name(self.images[i], &format!("swapchain-image-{}", i));
                rhi.set_debug_name(self.image_views[i], &format!("swapchain-image-view-{}", i));
            }
        }

        fn init_present_mode(&mut self, present_mode: vk::PresentModeKHR)
        {
            self.present_mode = self.surface_present_modes.iter().find(|p| **p == present_mode).copied();
        }


        fn init_format(&mut self, format: vk::SurfaceFormatKHR)
        {
            let surface_format = self.surface_formats.iter().find(|f| **f == format).unwrap();

            self.format = Some(surface_format.format);
            self.color_space = Some(surface_format.color_space);
        }
    }
}
