use ash::{vk, Device};

use crate::rhi::physical_device::RhiPhysicalDevice;


pub struct RHISwapchain
{
    pub loader: ash::extensions::khr::Swapchain,
    pub handle: vk::SwapchainKHR,

    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,

    pub extent: vk::Extent2D,
    pub format: vk::Format,
}


impl RHISwapchain
{
    pub fn new(
        instance: &ash::Instance,
        surface: &RHISurface,
        pdevice: &RhiPhysicalDevice,
        device: &Device,
        queue_indices: &QueueFamilyIndices,
    ) -> Self
    {
        let support_details = Self::query_surface_support(surface, pdevice.0);
        let surface_format = Self::choose_swapchain_format(&support_details.formats);
        let present_mode = Self::choose_swapchain_present_mode(&support_details.present_modes);
        let capabilities = &support_details.capabilities;
        let extent = Self::choose_swapchain_extent(capabilities);

        // max_image_count == 0，表示不限制 image 数量
        let image_count = if capabilities.max_image_count == 0 {
            capabilities.min_image_count + 1
        } else {
            u32::min(capabilities.max_image_count, capabilities.min_image_count + 1)
        };

        let mut create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface.handle)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        let swapchain_queue_indices = [queue_indices.grahics.unwrap(), queue_indices.present.unwrap()];
        if queue_indices.grahics != queue_indices.present {
            create_info = create_info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&swapchain_queue_indices);
        } else {
            create_info = create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE);
        }

        let swapchain_loader = ash::extensions::khr::Swapchain::new(instance, device);
        let swapchain = unsafe { swapchain_loader.create_swapchain(&create_info, None).unwrap() };

        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain).unwrap() }
            .iter()
            .map(|img| RHIImage(*img))
            .collect_vec();

        let swapchain_image_views = Self::create_swapchain_views(device, &swapchain_images, surface_format.format);

        Self {
            loader: swapchain_loader,
            handle: swapchain,
            _images: swapchain_images,
            image_views: swapchain_image_views,
            format: surface_format.format,
            extent,
            _current_image_index: Default::default(),
        }
    }

    pub fn destroy(&mut self, device: &Device)
    {
        unsafe {
            for view in self.image_views.iter() {
                device.destroy_image_view(view.0, None);
            }
            self.loader.destroy_swapchain(self.handle, None);
        }
    }

    fn query_surface_support(surface: &RHISurface, pdevice: vk::PhysicalDevice) -> SurfaceSupportDetails
    {
        unsafe {
            let capabilities = surface
                .loader
                .get_physical_device_surface_capabilities(pdevice, surface.handle)
                .unwrap();
            let formats = surface
                .loader
                .get_physical_device_surface_formats(pdevice, surface.handle)
                .unwrap();
            let present_modes = surface
                .loader
                .get_physical_device_surface_present_modes(pdevice, surface.handle)
                .unwrap();
            SurfaceSupportDetails {
                capabilities,
                formats,
                present_modes,
            }
        }
    }

    fn choose_swapchain_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR
    {
        *formats
            .iter()
            .find_or_first(|f| {
                f.format == vk::Format::B8G8R8A8_UNORM && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap()
    }

    fn choose_swapchain_present_mode(modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR
    {
        *modes
            .iter()
            .find(|m| **m == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(&vk::PresentModeKHR::FIFO)
    }

    fn choose_swapchain_extent(capabilities: &vk::SurfaceCapabilitiesKHR) -> vk::Extent2D
    {
        // NOTE 暂时不考虑 window 因素
        capabilities.current_extent
    }

    fn create_swapchain_views(device: &Device, swapchain_images: &[RHIImage], format: vk::Format) -> Vec<RHIImageView>
    {
        swapchain_images
            .iter()
            .map(|img| {
                let create_info = vk::ImageViewCreateInfo::builder()
                    .image(img.0)
                    .format(format)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .level_count(1)
                            .build(),
                    );

                unsafe { RHIImageView(device.create_image_view(&create_info, None).unwrap()) }
            })
            .collect_vec()
    }
}
