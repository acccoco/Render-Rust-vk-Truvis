use crate::renderer::bindless::BindlessManager;
use model_manager::component::instance::SimpleInstance;
use model_manager::component::mat::SimpleMaterial;
use model_manager::component::mesh::SimpleMesh;
use shader_binding::shader;
use std::collections::HashMap;
use std::rc::Rc;
use truvis_assimp::AssimpSceneLoader;
use truvis_rhi::rhi::Rhi;

pub struct SceneManager {
    pub instance_map: HashMap<uuid::Uuid, SimpleInstance>,
    pub mat_map: HashMap<uuid::Uuid, SimpleMaterial>,
    pub mesh_map: HashMap<uuid::Uuid, SimpleMesh>,

    pub point_light_map: HashMap<uuid::Uuid, shader::PointLight>,

    pub bindless_mgr: Rc<BindlessManager>,
}

impl SceneManager {
    pub fn new(bindless_mgr: Rc<BindlessManager>) -> Self {
        Self {
            instance_map: HashMap::new(),
            mat_map: HashMap::new(),
            mesh_map: HashMap::new(),
            point_light_map: HashMap::new(),
            bindless_mgr,
        }
    }

    pub fn register_model(&mut self, rhi: &Rhi, model_path: &std::path::Path) {
        AssimpSceneLoader::load_model(
            rhi,
            model_path,
            &mut |ins| {
                let guid = uuid::Uuid::new_v4();
                self.instance_map.insert(guid, ins);
                guid
            },
            &mut |mesh| {
                let guid = uuid::Uuid::new_v4();
                self.mesh_map.insert(guid, mesh);
                guid
            },
            &mut |mat| {
                let guid = uuid::Uuid::new_v4();
                self.mat_map.insert(guid, mat);
                guid
            },
        );
    }

    pub fn register_mat(&mut self, mat: SimpleMaterial) -> uuid::Uuid {
        let guid = uuid::Uuid::new_v4();
        self.mat_map.insert(guid, mat);
        guid
    }

    pub fn register_mesh(&mut self, mesh: SimpleMesh) -> uuid::Uuid {
        let guid = uuid::Uuid::new_v4();
        self.mesh_map.insert(guid, mesh);
        guid
    }

    pub fn register_instance(&mut self, instance: SimpleInstance) -> uuid::Uuid {
        let guid = uuid::Uuid::new_v4();
        self.instance_map.insert(guid, instance);
        guid
    }

    pub fn register_point_light(&mut self, light: shader::PointLight) -> uuid::Uuid {
        let guid = uuid::Uuid::new_v4();
        self.point_light_map.insert(guid, light);
        guid
    }
}
