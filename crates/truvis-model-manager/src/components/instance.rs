use crate::guid_new_type::{MatGuid, MeshGuid};

/// CPU 侧的 Instance 数据
#[derive(Clone)]
pub struct Instance {
    pub mesh: MeshGuid,
    pub materials: Vec<MatGuid>,
    pub transform: glam::Mat4,
}
