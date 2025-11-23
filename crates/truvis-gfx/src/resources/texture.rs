use std::rc::Rc;

use ash::vk;

use crate::gfx::Gfx;
use crate::resources::handles::{ImageHandle, ImageViewHandle};
use crate::sampler_manager::GfxSamplerDesc;

/// 纹理 UUID
#[derive(PartialOrd, PartialEq, Hash, Copy, Clone, Ord, Eq)]
pub struct GfxTexture2DUUID(pub uuid::Uuid);

/// 2D 纹理对象
///
/// 组合了 ImageHandle, ImageViewHandle 和 Sampler。
/// 通常用于材质系统中的纹理资源。
pub struct GfxTexture2D {
    image: ImageHandle,
    image_view: ImageViewHandle,
    sampler: vk::Sampler,

    // FIXME 将 uuid 使用起来
    _uuid: GfxTexture2DUUID,
}

impl GfxTexture2D {
    /// 创建新的 2D 纹理
    #[inline]
    pub fn new(image: ImageHandle, image_view: ImageViewHandle, _name: &str) -> Self {
        let sampler = Gfx::get().sampler_manager().get_sampler(&GfxSamplerDesc::default());

        Self {
            image,
            sampler,
            image_view,

            _uuid: GfxTexture2DUUID(uuid::Uuid::new_v4()),
        }
    }

    /// 获取 Sampler
    #[inline]
    pub fn sampler(&self) -> vk::Sampler {
        self.sampler
    }

    /// 获取 ImageView Handle
    #[inline]
    pub fn image_view(&self) -> ImageViewHandle {
        self.image_view
    }

    /// 获取 Image Handle
    #[inline]
    pub fn image(&self) -> ImageHandle {
        self.image
    }

    /// 获取 DescriptorImageInfo (用于 Descriptor Set 更新)
    #[inline]
    pub fn descriptor_image_info(&self, layout: vk::ImageLayout) -> vk::DescriptorImageInfo {
        let view = Gfx::get().resource_manager().get_image_view(self.image_view).unwrap().handle;
        vk::DescriptorImageInfo::default().sampler(self.sampler()).image_view(view).image_layout(layout)
    }
}

/// 纹理容器
///
/// 支持独占 (Owned) 和共享 (Shared) 两种所有权模式。
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
