use crate::core::frame_context::FrameContext;
use crate::subsystems::subsystem::Subsystem;
use std::collections::HashMap;
use std::path::PathBuf;
use truvis_cxx::AssimpSceneLoader;
use truvis_model_manager::components::instance::Instance;
use truvis_model_manager::components::material::Material;
use truvis_model_manager::components::mesh::Mesh;
use truvis_model_manager::guid_new_type::{InsGuid, LightGuid, MatGuid, MeshGuid};
use truvis_resource::handles::GfxTextureHandle;
use truvis_resource::texture::{GfxTexture2, ImageLoader};
use truvis_shader_binding::shader;

/// 在 CPU 侧管理场景数据
#[derive(Default)]
pub struct SceneManager {
    all_mats: HashMap<MatGuid, Material>,
    all_instances: HashMap<InsGuid, Instance>,
    all_meshes: HashMap<MeshGuid, Mesh>,

    // all_textures: HashMap<>
    all_point_lights: HashMap<LightGuid, shader::PointLight>,

    texture_map: HashMap<String, GfxTextureHandle>,
}
// getter
impl SceneManager {
    #[inline]
    pub fn mat_map(&self) -> &HashMap<MatGuid, Material> {
        &self.all_mats
    }
    #[inline]
    pub fn instance_map(&self) -> &HashMap<InsGuid, Instance> {
        &self.all_instances
    }
    #[inline]
    pub fn mesh_map(&self) -> &HashMap<MeshGuid, Mesh> {
        &self.all_meshes
    }
    #[inline]
    pub fn point_light_map(&self) -> &HashMap<LightGuid, shader::PointLight> {
        &self.all_point_lights
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.all_instances.is_empty()
            && self.all_meshes.is_empty()
            && self.all_mats.is_empty()
            && self.all_point_lights.is_empty()
    }
}
// init & destroy
impl SceneManager {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Subsystem for SceneManager {
    fn before_render(&mut self) {}
}
// tools
impl SceneManager {
    #[inline]
    pub fn get_instance(&self, guid: &InsGuid) -> Option<&Instance> {
        self.all_instances.get(guid)
    }

    #[inline]
    pub fn get_mesh(&self, guid: &MeshGuid) -> Option<&Mesh> {
        self.all_meshes.get(guid)
    }

    #[inline]
    pub fn get_material(&self, guid: &MatGuid) -> Option<&Material> {
        self.all_mats.get(guid)
    }

    #[inline]
    pub fn get_texture(&self, path: &str) -> Option<GfxTextureHandle> {
        self.texture_map.get(path).copied()
    }

    /// 向世界中添加一个外部场景
    pub fn load_scene(&mut self, model_path: &std::path::Path, _transform: &glam::Mat4) -> Vec<InsGuid> {
        AssimpSceneLoader::load_scene(
            model_path,
            |ins| {
                let guid = InsGuid::new();
                // Instance 应该有 transform 字段
                // ins.transform = *transform * ins.transform;
                self.all_instances.insert(guid, ins);
                guid
            },
            |mut mesh| {
                let guid = MeshGuid::new();
                mesh.build_blas();
                self.all_meshes.insert(guid, mesh);
                guid
            },
            |mat| {
                let guid = MatGuid::new();

                let mut bindless_manager = FrameContext::get().bindless_manager.borrow_mut();
                let mut gfx_resource_manager = FrameContext::get().gfx_resource_manager.borrow_mut();

                // 注册纹理
                if !mat.diffuse_map.is_empty() {
                    let entry = self.texture_map.entry(mat.diffuse_map.clone()).or_insert_with(|| {
                        let diffuse_map_path = PathBuf::from(&mat.diffuse_map);
                        let image = ImageLoader::load_image(&diffuse_map_path);
                        let texture = GfxTexture2::new(image, &mat.diffuse_map);
                        gfx_resource_manager.register_texture(texture)
                    });
                    bindless_manager.register_texture2(*entry);
                }

                self.all_mats.insert(guid, mat);
                guid
            },
        )
    }

    /// 向场景中添加材质
    pub fn register_mat(&mut self, mat: Material) -> MatGuid {
        let guid = MatGuid::new();
        self.all_mats.insert(guid, mat);
        guid
    }

    /// 向场景中添加 mesh
    pub fn register_mesh(&mut self, mesh: Mesh) -> MeshGuid {
        let guid = MeshGuid::new();
        self.all_meshes.insert(guid, mesh);
        guid
    }

    /// 向场景中添加 instance
    pub fn register_instance(&mut self, instance: Instance) -> InsGuid {
        let guid = InsGuid::new();
        self.all_instances.insert(guid, instance);
        guid
    }

    /// 向场景中添加点光源
    pub fn register_point_light(&mut self, light: shader::PointLight) -> LightGuid {
        let guid = LightGuid::new();
        self.all_point_lights.insert(guid, light);
        guid
    }
}

impl Drop for SceneManager {
    fn drop(&mut self) {
        log::info!("SceneManager dropped.");
    }
}
