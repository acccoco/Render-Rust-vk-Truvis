use crate::core::image::ImageContainer;
use crate::{
    core::{
        image::{RhiImage2D, RhiImage2DView, RhiImageViewCreateInfo},
        sampler::{RhiSampler, RhiSamplerCreateInfo},
    },
    rhi::Rhi,
};
use ash::vk;
use std::rc::Rc;

#[derive(PartialOrd, PartialEq, Hash, Copy, Clone, Ord, Eq)]
pub struct Texture2DUUID(pub uuid::Uuid);

pub struct RhiTexture2D {
    image: ImageContainer,
    sampler: RhiSampler,
    image_view: RhiImage2DView,

    // FIXME 将 uuid 使用起来
    _uuid: Texture2DUUID,
}

impl RhiTexture2D {
    #[inline]
    pub fn new(rhi: &Rhi, image: Rc<RhiImage2D>, name: &str) -> Self {
        let sampler = RhiSampler::new(rhi, Rc::new(RhiSamplerCreateInfo::new()), name);

        let image_view = RhiImage2DView::new(
            rhi,
            image.handle(),
            RhiImageViewCreateInfo::new_image_view_2d_info(image.format(), vk::ImageAspectFlags::COLOR),
            name,
        );

        Self {
            image: ImageContainer::Shared(image),
            sampler,
            image_view,

            _uuid: Texture2DUUID(uuid::Uuid::new_v4()),
        }
    }

    #[inline]
    pub fn sampler(&self) -> &RhiSampler {
        &self.sampler
    }

    #[inline]
    pub fn image_view(&self) -> &RhiImage2DView {
        &self.image_view
    }

    #[inline]
    pub fn image(&self) -> vk::Image {
        self.image.vk_image()
    }

    #[inline]
    pub fn descriptor_image_info(&self, layout: vk::ImageLayout) -> vk::DescriptorImageInfo {
        vk::DescriptorImageInfo::default()
            .sampler(self.sampler().handle())
            .image_view(self.image_view().handle())
            .image_layout(layout)
    }
}

pub enum Texture2DContainer {
    Owned(Box<RhiTexture2D>),
    Shared(Rc<RhiTexture2D>),
}
impl Texture2DContainer {
    #[inline]
    pub fn texture(&self) -> &RhiTexture2D {
        match self {
            Texture2DContainer::Owned(tex) => tex,
            Texture2DContainer::Shared(tex) => tex.as_ref(),
        }
    }
}
