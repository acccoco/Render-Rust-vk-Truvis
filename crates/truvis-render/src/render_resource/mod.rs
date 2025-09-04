use std::rc::Rc;

use truvis_rhi::{
    render_context::RenderContext,
    resources::{image::Image2D, texture::Texture2D},
};

pub struct ImageLoader {}

impl ImageLoader {
    pub fn load_image(render_context: &RenderContext, tex_path: &std::path::Path) -> Texture2D {
        let img = image::ImageReader::open(tex_path).unwrap().decode().unwrap().to_rgba8();

        let image = Rc::new(Image2D::from_rgba8(
            render_context,
            img.width(),
            img.height(),
            img.as_raw(),
            tex_path.to_str().unwrap(),
        ));

        let tex = Texture2D::new(image.clone(), tex_path.to_str().unwrap());

        tex
    }
}
