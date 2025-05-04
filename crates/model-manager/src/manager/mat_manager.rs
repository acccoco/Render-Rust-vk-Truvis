use crate::component::mat::SimpleMaterial;

#[derive(Default)]
pub struct MatManager {
    pub mat_map: std::collections::HashMap<uuid::Uuid, Box<SimpleMaterial>>,
}

impl MatManager {
    pub fn register_mat(&mut self, mat: SimpleMaterial) -> uuid::Uuid {
        let guid = uuid::Uuid::new_v4();
        self.mat_map.insert(guid, Box::new(mat));
        guid
    }
}
