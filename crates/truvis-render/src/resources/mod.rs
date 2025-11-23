use std::rc::Rc;

use truvis_gfx::resources::{image::GfxImage2D, texture::GfxTexture2D};

pub mod fif_buffer;

pub struct ImageLoader {}

impl ImageLoader {
    pub fn load_image(tex_path: &std::path::Path) -> GfxTexture2D {
        let img = image::ImageReader::open(tex_path).unwrap().decode().unwrap().to_rgba8();

        let image =
            Rc::new(GfxImage2D::from_rgba8(img.width(), img.height(), img.as_raw(), tex_path.to_str().unwrap()));

        GfxTexture2D::new(image, tex_path.to_str().unwrap())
    }
}
