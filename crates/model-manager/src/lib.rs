use crate::manager::instance_manager::InstanceManager;
use crate::manager::mat_manager::MatManager;
use manager::mesh_manager::MeshManager;

pub mod component;
pub mod manager;
pub mod vertex;

pub struct SceneManager {
    pub mesh_manager: MeshManager,
    pub mat_manager: MatManager,
    pub instance_manager: InstanceManager,
}
