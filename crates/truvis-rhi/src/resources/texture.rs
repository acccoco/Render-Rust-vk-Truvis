use std::rc::Rc;

use ash::vk;

use crate::{
    descriptors::sampler::{Sampler, SamplerCreateInfo},
    render_context::RenderContext,
    resources::{
        image::{Image2D, ImageContainer},
        image_view::{Image2DView, ImageViewCreateInfo},
    },
};

#[derive(PartialOrd, PartialEq, Hash, Copy, Clone, Ord, Eq)]
pub struct Texture2DUUID(pub uuid::Uuid);

pub struct Texture2D {
    image: ImageContainer,
    sampler: Sampler,
    image_view: Image2DView,

    // FIXME 将 uuid 使用起来
    _uuid: Texture2DUUID,
}

impl Texture2D {
    #[inline]
    pub fn new(image: Rc<Image2D>, name: &str) -> Self {
        let sampler = Sampler::new(Rc::new(SamplerCreateInfo::new()), name);

        let image_view = Image2DView::new(
            image.handle(),
            ImageViewCreateInfo::new_image_view_2d_info(image.format(), vk::ImageAspectFlags::COLOR),
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
    pub fn sampler(&self) -> &Sampler {
        &self.sampler
    }

    #[inline]
    pub fn image_view(&self) -> &Image2DView {
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
    Owned(Box<Texture2D>),
    Shared(Rc<Texture2D>),
}
impl Texture2DContainer {
    #[inline]
    pub fn texture(&self) -> &Texture2D {
        match self {
            Texture2DContainer::Owned(tex) => tex,
            Texture2DContainer::Shared(tex) => tex.as_ref(),
        }
    }
}
