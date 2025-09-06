use std::{cell::RefCell, collections::HashMap, rc::Rc};

use model_manager::{
    component::{DrsInstance, DrsMaterial, DrsMesh},
    guid_new_type::{InsGuid, LightGuid, MatGuid, MeshGuid},
};
use shader_binding::shader;
use truvis_cxx::AssimpSceneLoader;

use crate::renderer::bindless::BindlessManager;

pub struct SceneManager {
    mat_map: HashMap<MatGuid, DrsMaterial>,
    instance_map: HashMap<InsGuid, DrsInstance>,
    mesh_map: HashMap<MeshGuid, DrsMesh>,

    point_light_map: HashMap<LightGuid, shader::PointLight>,

    bindless_mgr: Rc<RefCell<BindlessManager>>,
}
/// getter
impl SceneManager {
    #[inline]
    pub fn mat_map(&self) -> &HashMap<MatGuid, DrsMaterial> {
        &self.mat_map
    }
    #[inline]
    pub fn instance_map(&self) -> &HashMap<InsGuid, DrsInstance> {
        &self.instance_map
    }
    #[inline]
    pub fn mesh_map(&self) -> &HashMap<MeshGuid, DrsMesh> {
        &self.mesh_map
    }
    #[inline]
    pub fn point_light_map(&self) -> &HashMap<LightGuid, shader::PointLight> {
        &self.point_light_map
    }
}
impl SceneManager {
    pub fn new(bindless_mgr: Rc<RefCell<BindlessManager>>) -> Self {
        Self {
            mat_map: HashMap::new(),
            point_light_map: HashMap::new(),
            instance_map: HashMap::new(),
            mesh_map: HashMap::new(),
            bindless_mgr,
        }
    }

    /// getter
    #[inline]
    pub fn get_instance(&self, guid: &InsGuid) -> Option<&DrsInstance> {
        self.instance_map.get(guid)
    }

    #[inline]
    pub fn get_mesh(&self, guid: &MeshGuid) -> Option<&DrsMesh> {
        self.mesh_map.get(guid)
    }

    #[inline]
    pub fn get_material(&self, guid: &MatGuid) -> Option<&DrsMaterial> {
        self.mat_map.get(guid)
    }

    /// 向世界中添加一个外部场景
    pub fn load_scene(&mut self, model_path: &std::path::Path, transform: &glam::Mat4) -> Vec<InsGuid> {
        AssimpSceneLoader::load_scene(
            model_path,
            |ins| {
                let guid = InsGuid::new();
                // DrsInstance 应该有 transform 字段
                // ins.transform = *transform * ins.transform;
                self.instance_map.insert(guid, ins);
                guid
            },
            |mut mesh| {
                let guid = MeshGuid::new();
                mesh.build_blas();
                self.mesh_map.insert(guid, mesh);
                guid
            },
            |mat| {
                let guid = MatGuid::new();

                // 注册纹理
                let mut bindless_mgr = self.bindless_mgr.borrow_mut();
                if !mat.diffuse_map.is_empty() {
                    bindless_mgr.register_texture_by_path(mat.diffuse_map.clone());
                }

                self.mat_map.insert(guid, mat);
                guid
            },
        )
    }

    /// 向场景中添加材质
    pub fn register_mat(&mut self, mat: DrsMaterial) -> MatGuid {
        let guid = MatGuid::new();
        self.mat_map.insert(guid, mat);
        guid
    }

    /// 向场景中添加 mesh
    pub fn register_mesh(&mut self, mesh: DrsMesh) -> MeshGuid {
        let guid = MeshGuid::new();
        self.mesh_map.insert(guid, mesh);
        guid
    }

    /// 向场景中添加 instance
    pub fn register_instance(&mut self, instance: DrsInstance) -> InsGuid {
        let guid = InsGuid::new();
        self.instance_map.insert(guid, instance);
        guid
    }

    /// 向场景中添加点光源
    pub fn register_point_light(&mut self, light: shader::PointLight) -> LightGuid {
        let guid = LightGuid::new();
        self.point_light_map.insert(guid, light);
        guid
    }
}
