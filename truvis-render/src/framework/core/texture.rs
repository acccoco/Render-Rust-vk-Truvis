use ash::vk;

use crate::framework::{core::image::RhiImage2D, rhi::Rhi};

pub struct RhiTexture
{
    pub image: RhiImage2D,
    pub sampler: vk::Sampler,
    pub image_view: vk::ImageView,
}

impl RhiTexture
{
    pub fn new(rhi: &Rhi, image: RhiImage2D, name: &str) -> Self
    {
        let sampler = {
            let sampler_info = vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::REPEAT)
                .address_mode_v(vk::SamplerAddressMode::REPEAT)
                .address_mode_w(vk::SamplerAddressMode::REPEAT)
                .anisotropy_enable(false)
                .max_anisotropy(1.0)
                .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
                .unnormalized_coordinates(false)
                .compare_enable(false)
                .compare_op(vk::CompareOp::ALWAYS)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .mip_lod_bias(0.0)
                .min_lod(0.0)
                .max_lod(1.0);
            rhi.create_sampler(&sampler_info, &format!("{}-sampler", name))
        };

        let image_view = {
            let create_info = vk::ImageViewCreateInfo::default()
                .image(image.handle)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_UNORM)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            rhi.create_image_view(&create_info, &format!("{}-view", name))
        };

        Self {
            image,
            sampler,
            image_view,
        }
    }
}
