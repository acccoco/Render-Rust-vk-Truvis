use std::rc::Rc;

use ash::vk;

use crate::gfx::Gfx;
use crate::resources::{
    image::{GfxImage2D, ImageContainer},
    image_view::{GfxImage2DView, GfxImageViewCreateInfo},
};
use crate::sampler_manager::GfxSamplerDesc;

#[derive(PartialOrd, PartialEq, Hash, Copy, Clone, Ord, Eq)]
pub struct GfxTexture2DUUID(pub uuid::Uuid);

pub struct GfxTexture2D {
    image: ImageContainer,
    image_view: GfxImage2DView,
    sampler: vk::Sampler,

    // FIXME 将 uuid 使用起来
    _uuid: GfxTexture2DUUID,
}

impl GfxTexture2D {
    #[inline]
    pub fn new(image: Rc<GfxImage2D>, name: &str) -> Self {
        let sampler = Gfx::get().sampler_manager().get_sampler(&GfxSamplerDesc::default());

        let image_view = GfxImage2DView::new(
            image.handle(),
            GfxImageViewCreateInfo::new_image_view_2d_info(image.format(), vk::ImageAspectFlags::COLOR),
            name,
        );

        Self {
            image: ImageContainer::Shared(image),
            sampler: sampler,
            image_view,

            _uuid: GfxTexture2DUUID(uuid::Uuid::new_v4()),
        }
    }

    #[inline]
    pub fn sampler(&self) -> vk::Sampler {
        self.sampler
    }

    #[inline]
    pub fn image_view(&self) -> &GfxImage2DView {
        &self.image_view
    }

    #[inline]
    pub fn image(&self) -> vk::Image {
        self.image.vk_image()
    }

    #[inline]
    pub fn descriptor_image_info(&self, layout: vk::ImageLayout) -> vk::DescriptorImageInfo {
        vk::DescriptorImageInfo::default()
            .sampler(self.sampler())
            .image_view(self.image_view().handle())
            .image_layout(layout)
    }
}

pub enum Texture2DContainer {
    Owned(Box<GfxTexture2D>),
    Shared(Rc<GfxTexture2D>),
}
impl Texture2DContainer {
    #[inline]
    pub fn texture(&self) -> &GfxTexture2D {
        match self {
            Texture2DContainer::Owned(tex) => tex,
            Texture2DContainer::Shared(tex) => tex.as_ref(),
        }
    }
}
