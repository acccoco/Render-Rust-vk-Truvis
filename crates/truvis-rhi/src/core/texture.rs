use std::rc::Rc;

use ash::vk;

use crate::{
    core::{
        image::{RhiImage2D, RhiImage2DView, RhiImageViewCreateInfo},
        sampler::{RhiSampler, RhiSamplerCreateInfo},
    },
    rhi::Rhi,
};

pub struct RhiTexture2D {
    image: Rc<RhiImage2D>,
    sampler: Rc<RhiSampler>,
    image_view: Rc<RhiImage2DView>,
}

impl RhiTexture2D {
    #[inline]
    pub fn new(rhi: &Rhi, image: Rc<RhiImage2D>, name: &str) -> Self {
        let sampler = Rc::new(RhiSampler::new(rhi, Rc::new(RhiSamplerCreateInfo::new()), name));

        let image_view = Rc::new(RhiImage2DView::new(
            rhi,
            image.clone(),
            RhiImageViewCreateInfo::new_image_view_2d_info(vk::Format::R8G8B8A8_UNORM, vk::ImageAspectFlags::COLOR),
            name.to_string(),
        ));

        Self {
            image,
            sampler,
            image_view,
        }
    }

    /// getter
    #[inline]
    pub fn image(&self) -> &RhiImage2D {
        &self.image
    }

    /// getter
    #[inline]
    pub fn sampler(&self) -> &RhiSampler {
        &self.sampler
    }

    /// getter
    #[inline]
    pub fn image_view(&self) -> &RhiImage2DView {
        &self.image_view
    }

    #[inline]
    pub fn descriptor_image_info(&self, layout: vk::ImageLayout) -> vk::DescriptorImageInfo {
        vk::DescriptorImageInfo::default()
            .sampler(self.sampler().handle())
            .image_view(self.image_view().handle())
            .image_layout(layout)
    }
}
