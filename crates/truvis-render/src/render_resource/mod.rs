use std::rc::Rc;

use truvis_gfx::resources::{image::Image2D, texture::Texture2D};

pub struct ImageLoader {}

impl ImageLoader {
    pub fn load_image(tex_path: &std::path::Path) -> Texture2D {
        let img = image::ImageReader::open(tex_path).unwrap().decode().unwrap().to_rgba8();

        let image = Rc::new(Image2D::from_rgba8(img.width(), img.height(), img.as_raw(), tex_path.to_str().unwrap()));

        Texture2D::new(image, tex_path.to_str().unwrap())
    }
}
