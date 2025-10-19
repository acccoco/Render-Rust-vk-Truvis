use std::{cell::RefCell, collections::HashMap, rc::Rc};

use model_manager::guid_new_type::{InsGuid, LightGuid, MatGuid, MeshGuid};
use model_manager::components::instance::Instance;
use model_manager::components::material::Material;
use model_manager::components::mesh::Mesh;
use shader_binding::shader;
use shader_binding::shader::Scene;
use truvis_cxx::AssimpSceneLoader;

use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_context::FrameContext;

/// 在 CPU 侧管理场景数据
pub struct SceneManager {
    mat_map: HashMap<MatGuid, Material>,
    instance_map: HashMap<InsGuid, Instance>,
    mesh_map: HashMap<MeshGuid, Mesh>,

    point_light_map: HashMap<LightGuid, shader::PointLight>,
}
// getter
impl SceneManager {
    #[inline]
    pub fn mat_map(&self) -> &HashMap<MatGuid, Material> {
        &self.mat_map
    }
    #[inline]
    pub fn instance_map(&self) -> &HashMap<InsGuid, Instance> {
        &self.instance_map
    }
    #[inline]
    pub fn mesh_map(&self) -> &HashMap<MeshGuid, Mesh> {
        &self.mesh_map
    }
    #[inline]
    pub fn point_light_map(&self) -> &HashMap<LightGuid, shader::PointLight> {
        &self.point_light_map
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.instance_map.is_empty()
            && self.mesh_map.is_empty()
            && self.mat_map.is_empty()
            && self.point_light_map.is_empty()
    }
}
// init & destroy
impl SceneManager {
    pub fn new() -> Self {
        Self {
            mat_map: HashMap::new(),
            point_light_map: HashMap::new(),
            instance_map: HashMap::new(),
            mesh_map: HashMap::new(),
        }
    }
}
// tools
impl SceneManager {
    #[inline]
    pub fn get_instance(&self, guid: &InsGuid) -> Option<&Instance> {
        self.instance_map.get(guid)
    }

    #[inline]
    pub fn get_mesh(&self, guid: &MeshGuid) -> Option<&Mesh> {
        self.mesh_map.get(guid)
    }

    #[inline]
    pub fn get_material(&self, guid: &MatGuid) -> Option<&Material> {
        self.mat_map.get(guid)
    }

    /// 向世界中添加一个外部场景
    pub fn load_scene(&mut self, model_path: &std::path::Path, transform: &glam::Mat4) -> Vec<InsGuid> {
        AssimpSceneLoader::load_scene(
            model_path,
            |ins| {
                let guid = InsGuid::new();
                // Instance 应该有 transform 字段
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
                let mut bindless_mgr = FrameContext::get().bindless_mgr.borrow_mut();
                if !mat.diffuse_map.is_empty() {
                    bindless_mgr.register_texture_by_path(mat.diffuse_map.clone());
                }

                self.mat_map.insert(guid, mat);
                guid
            },
        )
    }

    /// 向场景中添加材质
    pub fn register_mat(&mut self, mat: Material) -> MatGuid {
        let guid = MatGuid::new();
        self.mat_map.insert(guid, mat);
        guid
    }

    /// 向场景中添加 mesh
    pub fn register_mesh(&mut self, mesh: Mesh) -> MeshGuid {
        let guid = MeshGuid::new();
        self.mesh_map.insert(guid, mesh);
        guid
    }

    /// 向场景中添加 instance
    pub fn register_instance(&mut self, instance: Instance) -> InsGuid {
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

impl Drop for SceneManager {
    fn drop(&mut self) {
        log::info!("SceneManager dropped.");
    }
}
