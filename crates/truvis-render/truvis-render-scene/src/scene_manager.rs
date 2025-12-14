use slotmap::SlotMap;
use std::collections::HashMap;
use std::path::PathBuf;
use truvis_model_manager::assimp_loader::AssimpSceneLoader;
use truvis_model_manager::components::instance::Instance;
use truvis_model_manager::components::material::Material;
use truvis_model_manager::components::mesh::Mesh;
use truvis_model_manager::guid_new_type::{InstanceHandle, LightHandle, MaterialHandle, MeshHandle};
use truvis_render_base::bindless_manager::BindlessManager;
use truvis_resource::gfx_resource_manager::GfxResourceManager;
use truvis_resource::handles::GfxTextureHandle;
use truvis_resource::texture::{GfxTexture2, ImageLoader};
use truvis_shader_binding::truvisl;

/// 在 CPU 侧管理场景数据
#[derive(Default)]
pub struct SceneManager {
    all_mats: SlotMap<MaterialHandle, Material>,
    all_instances: SlotMap<InstanceHandle, Instance>,
    all_meshes: SlotMap<MeshHandle, Mesh>,

    all_point_lights: SlotMap<LightHandle, truvisl::PointLight>,

    texture_map: HashMap<String, GfxTextureHandle>,
}
// new & init
impl SceneManager {
    pub fn new() -> Self {
        Self::default()
    }
}
// getter
impl SceneManager {
    #[inline]
    pub fn mat_map(&self) -> &SlotMap<MaterialHandle, Material> {
        &self.all_mats
    }
    #[inline]
    pub fn instance_map(&self) -> &SlotMap<InstanceHandle, Instance> {
        &self.all_instances
    }
    #[inline]
    pub fn mesh_map(&self) -> &SlotMap<MeshHandle, Mesh> {
        &self.all_meshes
    }
    #[inline]
    pub fn point_light_map(&self) -> &SlotMap<LightHandle, truvisl::PointLight> {
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
// tools
impl SceneManager {
    #[inline]
    pub fn get_instance(&self, handle: InstanceHandle) -> Option<&Instance> {
        self.all_instances.get(handle)
    }

    #[inline]
    pub fn get_mesh(&self, handle: MeshHandle) -> Option<&Mesh> {
        self.all_meshes.get(handle)
    }

    #[inline]
    pub fn get_material(&self, handle: MaterialHandle) -> Option<&Material> {
        self.all_mats.get(handle)
    }

    #[inline]
    pub fn get_texture(&self, path: &str) -> Option<GfxTextureHandle> {
        self.texture_map.get(path).copied()
    }

    /// 向世界中添加一个外部场景
    pub fn load_scene(
        &mut self,
        gfx_resource_manager: &mut GfxResourceManager,
        bindless_manager: &mut BindlessManager,
        model_path: &std::path::Path,
        _transform: &glam::Mat4,
    ) -> Vec<InstanceHandle> {
        AssimpSceneLoader::load_scene(
            model_path,
            |ins| {
                // Instance 应该有 transform 字段
                // ins.transform = *transform * ins.transform;
                self.all_instances.insert(ins)
            },
            |mut mesh| {
                mesh.build_blas();
                self.all_meshes.insert(mesh)
            },
            |mat| {
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

                self.all_mats.insert(mat)
            },
        )
    }

    /// 向场景中添加材质
    pub fn register_mat(&mut self, mat: Material) -> MaterialHandle {
        self.all_mats.insert(mat)
    }

    /// 向场景中添加 mesh
    pub fn register_mesh(&mut self, mesh: Mesh) -> MeshHandle {
        self.all_meshes.insert(mesh)
    }

    /// 向场景中添加 instance
    pub fn register_instance(&mut self, instance: Instance) -> InstanceHandle {
        self.all_instances.insert(instance)
    }

    /// 向场景中添加点光源
    pub fn register_point_light(&mut self, light: truvisl::PointLight) -> LightHandle {
        self.all_point_lights.insert(light)
    }
}
impl Drop for SceneManager {
    fn drop(&mut self) {
        log::info!("SceneManager dropped.");
    }
}
// destroy
impl SceneManager {
    pub fn destroy(self) {}
    pub fn destroy_mut(&mut self) {
        self.all_mats.clear();
        self.all_instances.clear();
        self.all_meshes.clear();
        self.all_point_lights.clear();
        self.texture_map.clear();
    }
}
