use ash::vk;
use ash::vk::Handle;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::resources::image::GfxImage;
use truvis_gfx::resources::image_view::GfxImageView;
use truvis_gfx::resources::image_view::GfxImageViewDesc;
use truvis_gfx::sampler_manager::GfxSamplerDesc;

pub struct GfxTexture {
    image: GfxImage,
    image_view: GfxImageView,
    sampler: vk::Sampler,

    #[cfg(debug_assertions)]
    _name: String,
}
// new & init
impl GfxTexture {
    pub fn new(image: GfxImage, name: &str) -> Self {
        let sampler = Gfx::get().sampler_manager().get_sampler(&GfxSamplerDesc::default());
        let image_view = GfxImageView::new(
            image.handle(),
            GfxImageViewDesc::new_2d(image.format(), vk::ImageAspectFlags::COLOR),
            name,
        );

        Self {
            image,
            image_view,
            sampler,

            #[cfg(debug_assertions)]
            _name: name.to_string(),
        }
    }
}
// destroy
impl GfxTexture {
    pub fn destroy(mut self) {
        self.destroy_mut();
    }
    pub fn destroy_mut(&mut self) {
        self.image.destroy_mut();
        self.image_view.destroy_mut();
        self.sampler = vk::Sampler::null();
    }
}
impl Drop for GfxTexture {
    fn drop(&mut self) {
        debug_assert!(self.sampler.is_null(), "GfxTexture2 was not destroyed before drop");
    }
}
// getters
impl GfxTexture {
    #[inline]
    pub fn sampler(&self) -> vk::Sampler {
        self.sampler
    }

    #[inline]
    pub fn image_view(&self) -> &GfxImageView {
        &self.image_view
    }

    #[inline]
    pub fn image(&self) -> &GfxImage {
        &self.image
    }
}

// TODO 临时的图片加载器，后续需要整合到 TextureManager 中
pub struct ImageLoader {}
impl ImageLoader {
    pub fn load_image(tex_path: &std::path::Path) -> GfxImage {
        let img = image::ImageReader::open(tex_path).unwrap().decode().unwrap().to_rgba8();
        let width = img.width();
        let height = img.height();
        let data = img.as_raw();
        let name = tex_path.to_str().unwrap();

        GfxImage::from_rgba8(width, height, data, name)
    }
}
