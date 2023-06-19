use ash::vk;

pub struct RhiCreateInfoUtil;

impl RhiCreateInfoUtil
{
    #[inline]
    pub fn make_image2d_create_info(
        extent: vk::Extent2D,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
    ) -> vk::ImageCreateInfo
    {
        vk::ImageCreateInfo {
            extent: extent.into(),
            format,
            tiling: vk::ImageTiling::OPTIMAL,
            usage,
            samples: vk::SampleCountFlags::TYPE_1,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            image_type: vk::ImageType::TYPE_2D,
            array_layers: 1,
            mip_levels: 1,
            ..Default::default()
        }
    }

    #[inline]
    pub fn make_image_view_2d_create_info(
        image: vk::Image,
        format: vk::Format,
        aspect: vk::ImageAspectFlags,
    ) -> vk::ImageViewCreateInfo
    {
        vk::ImageViewCreateInfo {
            image,
            format,
            view_type: vk::ImageViewType::TYPE_2D,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: aspect,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}