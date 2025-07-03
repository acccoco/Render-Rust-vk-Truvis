use std::rc::Rc;
use truvis_rhi::core::image::RhiImage2D;
use truvis_rhi::core::texture::RhiTexture2D;
use truvis_rhi::rhi::Rhi;

pub struct ImageLoader {}

impl ImageLoader {
    pub fn load_image(rhi: &Rhi, tex_path: &std::path::Path) -> RhiTexture2D {
        let img = image::ImageReader::open(tex_path).unwrap().decode().unwrap().to_rgba8();

        let image =
            Rc::new(RhiImage2D::from_rgba8(rhi, img.width(), img.height(), img.as_raw(), tex_path.to_str().unwrap()));

        let tex = RhiTexture2D::new(rhi, image, tex_path.to_str().unwrap());

        tex
    }
}
