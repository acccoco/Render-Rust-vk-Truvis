/// CPU 侧的材质数据
#[derive(Default)]
pub struct Material {
    pub base_color: glam::Vec4,
    pub emissive: glam::Vec4,
    pub metallic: f32,
    pub roughness: f32,
    pub opaque: f32,

    pub diffuse_map: String,
    pub normal_map: String,
}
