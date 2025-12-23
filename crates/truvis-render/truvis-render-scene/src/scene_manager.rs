use crate::scene_data::SceneRenderData;
use slotmap::SlotMap;
use truvis_model::components::instance::Instance;
use truvis_model::components::material::Material;
use truvis_model::components::mesh::Mesh;
use truvis_model::guid_new_type::{InstanceHandle, LightHandle, MaterialHandle, MeshHandle};
use truvis_shader_binding::truvisl;

/// 在 CPU 侧管理场景数据
#[derive(Default)]
pub struct SceneManager {
    all_mats: SlotMap<MaterialHandle, Material>,
    all_instances: SlotMap<InstanceHandle, Instance>,
    all_meshes: SlotMap<MeshHandle, Mesh>,

    all_point_lights: SlotMap<LightHandle, truvisl::PointLight>,
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

    /// 获取用于渲染的场景数据
    pub fn prepare_render_data(&self) -> SceneRenderData {
        // TODO 检测 mesh 的 blas 是否构建完成，决定是否输出对应 mesh 和 instance

        SceneRenderData {
            all_instances: self.all_instances.keys().collect(),
            all_meshes: self.all_meshes.keys().collect(),
            all_materials: self.all_mats.keys().collect(),
        }
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
    }
}
