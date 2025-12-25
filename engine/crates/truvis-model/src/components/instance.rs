use crate::guid_new_type::{MaterialHandle, MeshHandle};

/// CPU 侧的 Instance 数据
#[derive(Clone)]
pub struct Instance {
    pub mesh: MeshHandle,
    pub materials: Vec<MaterialHandle>,
    pub transform: glam::Mat4,
}
