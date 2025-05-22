use ash::vk;
use truvis_rhi::core::buffer::RhiBuffer;

#[derive(Default)]
pub struct Material {
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

pub struct Geometry {
    pub vertex_buffer: RhiBuffer,
    pub index_buffer: RhiBuffer,
    pub index_cnt: u32,
}

impl Geometry {
    #[inline]
    pub fn index_type() -> vk::IndexType {
        vk::IndexType::UINT32
    }
}

pub struct Mesh {
    pub geometries: Vec<Geometry>,
}

#[derive(Clone)]
pub struct Instance {
    pub mesh: uuid::Uuid,
    pub materials: Vec<uuid::Uuid>,
    pub transform: glam::Mat4,
}
