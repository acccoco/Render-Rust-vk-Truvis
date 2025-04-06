use std::rc::Rc;

use ash::vk;

use crate::framework::{
    core::{
        image::{RhiImage2D, RhiImage2DView, RhiImageViewCreateInfo},
        sampler::{RhiSampler, RhiSamplerCreateInfo},
    },
    render_core::Rhi,
};


// TODO 使用 private
pub struct RhiTexture2D
{
    pub image: Rc<RhiImage2D>,
    pub sampler: Rc<RhiSampler>,
    pub image_view: Rc<RhiImage2DView>,
}

impl RhiTexture2D
{
    #[inline]
    pub fn new(rhi: &Rhi, image: Rc<RhiImage2D>, name: &str) -> Self
    {
        let sampler = Rc::new(RhiSampler::new(rhi, Rc::new(RhiSamplerCreateInfo::new()), &format!("{}-sampler", name)));

        let image_view = Rc::new(RhiImage2DView::new(
            rhi,
            image.clone(),
            RhiImageViewCreateInfo::new_image_view_2d_info(vk::Format::R8G8B8A8_UNORM, vk::ImageAspectFlags::COLOR),
            format!("{}-view", name),
        ));

        Self {
            image,
            sampler,
            image_view,
        }
    }
}
