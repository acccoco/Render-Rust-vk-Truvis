use crate::renderer::bindless::BindlessManager;
use shader_binding::shader;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use model_manager::component::{Instance, Mesh, Material};
use truvis_cxx::AssimpSceneLoader;
use truvis_rhi::rhi::Rhi;

pub struct TheWorld {
    pub mat_map: HashMap<uuid::Uuid, Material>,
    pub instance_map: HashMap<uuid::Uuid, Instance>,
    pub mesh_map: HashMap<uuid::Uuid, Mesh>,

    pub point_light_map: HashMap<uuid::Uuid, shader::PointLight>,

    bindless_mgr: Rc<RefCell<BindlessManager>>,
}

impl TheWorld {
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
    pub fn get_instance(&self, guid: &uuid::Uuid) -> Option<&Instance> {
        self.instance_map.get(guid)
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
            |mesh| {
                let guid = uuid::Uuid::new_v4();
                self.mesh_map.insert(guid, mesh);
                guid
            },
            |mat| {
                let guid = uuid::Uuid::new_v4();

                // 注册纹理
                let mut bindless_mgr = self.bindless_mgr.borrow_mut();
                if !mat.diffuse_map.is_empty() {
                    bindless_mgr.register_texture(rhi, mat.diffuse_map.clone());
                }

                self.mat_map.insert(guid, mat);
                guid
            },
        );

        ins_guids
    }

    /// 向场景中添加材质
    pub fn register_mat(&mut self, mat: Material) -> uuid::Uuid {
        let guid = uuid::Uuid::new_v4();
        self.mat_map.insert(guid, mat);
        guid
    }

    /// 向场景中添加 mesh
    pub fn register_mesh(&mut self, mesh: Mesh) -> uuid::Uuid {
        let guid = uuid::Uuid::new_v4();
        self.mesh_map.insert(guid, mesh);
        guid
    }

    /// 向场景中添加 instance
    pub fn register_instance(&mut self, instance: Instance) -> uuid::Uuid {
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
