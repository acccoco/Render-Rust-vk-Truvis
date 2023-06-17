use ash::vk;
use itertools::Itertools;

use crate::{rhi::rhi_core::RhiCore, rhi_init_info::RhiInitInfo};


pub struct RHISwapchain
{
    pub(crate) loader: ash::extensions::khr::Swapchain,
    pub(crate) handle: Option<vk::SwapchainKHR>,
    
    pub(crate) images: Vec<vk::Image>,
    pub(crate) image_views: Vec<vk::ImageView>,

    pub(crate) extent: Option<vk::Extent2D>,
    pub(crate) format: Option<vk::Format>,
    color_space: Option<vk::ColorSpaceKHR>,
    present_mode: Option<vk::PresentModeKHR>,

    surface_capabilities: vk::SurfaceCapabilitiesKHR,
    surface_formats: Vec<vk::SurfaceFormatKHR>,
    surface_present_modes: Vec<vk::PresentModeKHR>,
}


impl RHISwapchain
{
    pub(crate) fn new(rhi_core: &RhiCore, init_info: &RhiInitInfo) -> Self
    {
        let mut swapchain = unsafe {
            let pdevice = rhi_core.physical_device().vk_physical_device;
            let surface = rhi_core.surface();

            Self {
                loader: ash::extensions::khr::Swapchain::new(rhi_core.instance(), rhi_core.device()),
                handle: None,
                images: Vec::new(),
                image_views: Vec::new(),
                extent: None,
                format: None,
                color_space: None,
                present_mode: None,
                surface_capabilities: rhi_core
                    .surface_loader()
                    .get_physical_device_surface_capabilities(pdevice, surface)
                    .unwrap(),
                surface_formats: rhi_core
                    .surface_loader()
                    .get_physical_device_surface_formats(pdevice, surface)
                    .unwrap(),
                surface_present_modes: rhi_core
                    .surface_loader()
                    .get_physical_device_surface_present_modes(pdevice, surface)
                    .unwrap(),
            }
        };
        swapchain.init_format(init_info);
        swapchain.init_present_mode(init_info);
        swapchain.init_extent();
        swapchain.init_handle(rhi_core);
        swapchain.init_images_and_views(rhi_core);

        swapchain
    }

    fn init_format(&mut self, init_info: &RhiInitInfo)
    {
        let surface_format = self
            .surface_formats
            .iter()
            .find(|f| f.format == init_info.swapchain_format && f.color_space == init_info.swapchain_color_space)
            .unwrap();

        self.format = Some(surface_format.format);
        self.color_space = Some(surface_format.color_space);
    }

    fn init_present_mode(&mut self, init_info: &RhiInitInfo)
    {
        self.present_mode = self
            .surface_present_modes
            .iter()
            .find(|p| **p == init_info.swapchain_present_mode)
            .map_or(None, |p| Some(*p));
    }

    // TODO
    fn init_extent(&mut self) { self.extent = Some(self.surface_capabilities.current_extent) }

    fn init_handle(&mut self, rhi_core: &RhiCore)
    {
        // 确定 image count
        // max_image_count == 0，表示不限制 image 数量
        let image_count = if self.surface_capabilities.max_image_count == 0 {
            self.surface_capabilities.min_image_count + 1
        } else {
            u32::min(self.surface_capabilities.max_image_count, self.surface_capabilities.min_image_count + 1)
        };

        let mut create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(rhi_core.surface())
            .min_image_count(image_count)
            .image_format(self.format.unwrap())
            .image_color_space(self.color_space.unwrap())
            .image_extent(self.extent.unwrap())
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(self.surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(self.present_mode.unwrap())
            .clipped(true);

        // 如果 present queue family 和 graphics queue family 并不是同一个 family，
        // 那么需要 swapchian image 在这两个 family 之间共享

        let swapchain_queue_indices = [
            rhi_core.graphics_queue_family_index.unwrap(),
            rhi_core.present_queue_family_index.unwrap(),
        ];
        if rhi_core.graphics_queue_family_index.unwrap() != rhi_core.present_queue_family_index.unwrap() {
            create_info = create_info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&swapchain_queue_indices);
        } else {
            create_info = create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE);
        }


        unsafe {
            self.handle = Some(self.loader.create_swapchain(&create_info, None).unwrap());
        }
    }

    fn init_images_and_views(&mut self, rhi_core: &RhiCore)
    {
        let swapchain_images = unsafe { self.loader.get_swapchain_images(self.handle.unwrap()).unwrap() };

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

                unsafe { rhi_core.device().create_image_view(&create_info, None).unwrap() }
            })
            .collect_vec();

        self.images = swapchain_images;
        self.image_views = image_views;
    }
}
