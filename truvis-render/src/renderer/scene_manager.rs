use crate::renderer::bindless::BindlessManager;
use model_manager::component::{DrsInstance, DrsMesh, DrsMaterial};
use shader_binding::shader;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use truvis_cxx::AssimpSceneLoader;
use truvis_rhi::rhi::Rhi;

pub struct SceneManager {
    mat_map: HashMap<uuid::Uuid, DrsMaterial>,
    instance_map: HashMap<uuid::Uuid, DrsInstance>,
    mesh_map: HashMap<uuid::Uuid, DrsMesh>,

    point_light_map: HashMap<uuid::Uuid, shader::PointLight>,

    bindless_mgr: Rc<RefCell<BindlessManager>>,
}
// getter
impl SceneManager {
    #[inline]
    pub fn mat_map(&self) -> &HashMap<uuid::Uuid, DrsMaterial> {
        &self.mat_map
    }
    #[inline]
    pub fn instance_map(&self) -> &HashMap<uuid::Uuid, DrsInstance> {
        &self.instance_map
    }
    #[inline]
    pub fn mesh_map(&self) -> &HashMap<uuid::Uuid, DrsMesh> {
        &self.mesh_map
    }
    #[inline]
    pub fn point_light_map(&self) -> &HashMap<uuid::Uuid, shader::PointLight> {
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
    pub fn get_instance(&self, guid: &uuid::Uuid) -> Option<&DrsInstance> {
        self.instance_map.get(guid)
    }

    #[inline]
    pub fn get_mesh(&self, guid: &uuid::Uuid) -> Option<&DrsMesh> {
        self.mesh_map.get(guid)
    }

    #[inline]
    pub fn get_material(&self, guid: &uuid::Uuid) -> Option<&DrsMaterial> {
        self.mat_map.get(guid)
    }

    /// 向世界中添加一个外部场景
    pub fn load_scene(&mut self, rhi: &Rhi, model_path: &std::path::Path, transform: &glam::Mat4) -> Vec<uuid::Uuid> {
        let mut ins_guids = vec![];

        AssimpSceneLoader::load_scene(
            rhi,
            model_path,
            |mut ins| {
                let guid = uuid::Uuid::new_v4();
                ins.transform = *transform * ins.transform;
                self.instance_map.insert(guid, ins);
                ins_guids.push(guid);
                guid
            },
            |mut mesh| {
                let guid = uuid::Uuid::new_v4();
                mesh.build_blas(rhi);
                self.mesh_map.insert(guid, mesh);
                guid
            },
            |mat| {
                let guid = uuid::Uuid::new_v4();

                // 注册纹理
                let mut bindless_mgr = self.bindless_mgr.borrow_mut();
                if !mat.diffuse_map.is_empty() {
                    bindless_mgr.register_texture_by_path(rhi, mat.diffuse_map.clone());
                }

                self.mat_map.insert(guid, mat);
                guid
            },
        );

        ins_guids
    }

    /// 向场景中添加材质
    pub fn register_mat(&mut self, mat: DrsMaterial) -> uuid::Uuid {
        let guid = uuid::Uuid::new_v4();
        self.mat_map.insert(guid, mat);
        guid
    }

    /// 向场景中添加 mesh
    pub fn register_mesh(&mut self, mesh: DrsMesh) -> uuid::Uuid {
        let guid = uuid::Uuid::new_v4();
        self.mesh_map.insert(guid, mesh);
        guid
    }

    /// 向场景中添加 instance
    pub fn register_instance(&mut self, instance: DrsInstance) -> uuid::Uuid {
        let guid = uuid::Uuid::new_v4();
        self.instance_map.insert(guid, instance);
        guid
    }

    /// 向场景中添加点光源
    pub fn register_point_light(&mut self, light: shader::PointLight) -> uuid::Uuid {
        let guid = uuid::Uuid::new_v4();
        self.point_light_map.insert(guid, light);
        guid
    }
}
