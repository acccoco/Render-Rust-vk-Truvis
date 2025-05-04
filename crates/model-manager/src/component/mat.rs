#[derive(Default)]
pub struct SimpleMaterial {
    pub ambient: glam::Vec4,
    pub diffuse: glam::Vec4,
    pub specular: glam::Vec4,
    pub emissive: glam::Vec4,

    pub shininess: f32,
    pub alpha: f32,

    pub diffuse_map: String,
    pub ambient_map: String,
    pub emissive_map: String,
    pub specular_map: String,

    pub normal_map: String,
}
