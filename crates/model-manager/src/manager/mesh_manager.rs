use crate::component::mesh::SimpleMesh;
use std::collections::HashMap;

#[derive(Default)]
pub struct MeshManager {
    pub mesh_map: HashMap<uuid::Uuid, SimpleMesh>,
}

impl MeshManager {
    pub fn register_mesh(&mut self, mesh: SimpleMesh) -> uuid::Uuid {
        let guid = uuid::Uuid::new_v4();
        self.mesh_map.insert(guid, mesh);
        guid
    }
}
