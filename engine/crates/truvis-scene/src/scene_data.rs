use crate::guid_new_type::{InstanceHandle, MaterialHandle, MeshHandle};
use indexmap::IndexSet;

/// 由 SceneManager 产生，GPUScene 消费的场景结构
#[derive(Default)]
pub struct SceneRenderData {
    pub all_instances: Vec<InstanceHandle>,
    pub all_meshes: IndexSet<MeshHandle>,
    pub all_materials: IndexSet<MaterialHandle>,
}
