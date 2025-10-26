use std::{ffi::c_void, mem::offset_of};

use itertools::Itertools;
use truvis_model_manager::components::geometry::Geometry;
use truvis_model_manager::components::instance::Instance;
use truvis_model_manager::components::material::Material;
use truvis_model_manager::components::mesh::Mesh;
use truvis_model_manager::guid_new_type::{InsGuid, MatGuid, MeshGuid};
use truvis_model_manager::vertex::aos_3d::Vertex3D;
use truvis_model_manager::vertex::soa_3d::VertexLayoutSoA3D;
use truvis_rhi::resources::special_buffers::index_buffer::Index32Buffer;

pub mod _ffi_bindings;
use crate::_ffi_bindings::*;

// TODO 使用 SoA 来简化，就可以移除这些代码了
/// 确保 Vertex3D 的布局和 C++ 中的 Vertex3D 一致
fn validate_vertex_memory_layout() {
    debug_assert!(size_of::<Vertex3D>() == size_of::<CxxVertex3D>());
    debug_assert!(offset_of!(Vertex3D, position) == offset_of!(CxxVertex3D, position));
    debug_assert!(offset_of!(Vertex3D, normal) == offset_of!(CxxVertex3D, normal));
    debug_assert!(offset_of!(Vertex3D, tangent) == offset_of!(CxxVertex3D, tangent));
    debug_assert!(offset_of!(Vertex3D, bitangent) == offset_of!(CxxVertex3D, bitangent));
    debug_assert!(offset_of!(Vertex3D, uv) == offset_of!(CxxVertex3D, uv));

    debug_assert!(size_of::<glam::Vec4>() == size_of::<CxxVec4f>());
    debug_assert!(size_of::<glam::Mat4>() == size_of::<CxxMat4f>());
}

pub struct AssimpSceneLoader {
    loader: *mut c_void,
    model_name: String,

    meshes: Vec<MeshGuid>,
    mats: Vec<MatGuid>,
    instances: Vec<InsGuid>,
}

impl AssimpSceneLoader {
    /// # return
    /// 返回整个场景的所有 instance id
    pub fn load_scene(
        model_file: &std::path::Path,
        instance_register: impl FnMut(Instance) -> InsGuid,
        mesh_register: impl FnMut(Mesh) -> MeshGuid,
        mat_register: impl FnMut(Material) -> MatGuid,
    ) -> Vec<InsGuid> {
        validate_vertex_memory_layout();

        let model_file = model_file.to_str().unwrap();
        let c_model_file = std::ffi::CString::new(model_file).unwrap();

        unsafe {
            let loader = load_scene(c_model_file.as_ptr());
            let model_name = model_file.split('/').next_back().unwrap();

            let mut scene_loader = AssimpSceneLoader {
                loader,
                model_name: model_name.to_string(),
                meshes: vec![],
                mats: vec![],
                instances: vec![],
            };

            scene_loader.load_mesh(mesh_register);
            scene_loader.load_mats(mat_register);
            scene_loader.load_instance(instance_register);

            free_scene(loader);

            scene_loader.instances
        }
    }

    /// 加载场景中基础的几何体
    fn load_mesh(&mut self, mut mesh_register: impl FnMut(Mesh) -> MeshGuid) {
        let mesh_cnt = unsafe { get_mesh_cnt(self.loader) };

        let mesh_uuids = (0..mesh_cnt)
            .map(|mesh_idx| unsafe {
                let mesh = get_mesh(self.loader, mesh_idx);
                let mesh = &*mesh;

                if mesh.vertex_array_.is_null() {
                    panic!("Mesh {} has no vertex data", mesh_idx);
                }
                let vertex_data =
                    std::slice::from_raw_parts(mesh.vertex_array_ as *const Vertex3D, mesh.vertex_cnt_ as usize);
                let positions = vertex_data.iter().map(|v| glam::Vec3::from_array(v.position)).collect_vec();
                let normals = vertex_data.iter().map(|v| glam::Vec3::from_array(v.normal)).collect_vec();
                let tangents = vertex_data.iter().map(|v| glam::Vec3::from_array(v.tangent)).collect_vec();
                let uvs = vertex_data.iter().map(|v| glam::Vec2::from_array(v.uv)).collect_vec();

                let vertex_buffer = VertexLayoutSoA3D::create_vertex_buffer(
                    &positions,
                    &normals,
                    &tangents,
                    &uvs,
                    format!("{}-mesh-{}", self.model_name, mesh_idx),
                );

                if mesh.face_array_.is_null() {
                    panic!("Mesh {} has no index data", mesh_idx);
                }
                let index_data =
                    std::slice::from_raw_parts(mesh.face_array_ as *const u32, mesh.face_cnt_ as usize * 3);

                let index_buffer =
                    Index32Buffer::new(index_data.len(), format!("{}-mesh-{}-indices", self.model_name, mesh_idx));
                index_buffer.transfer_data_sync(index_data);

                // 只有 single geometry 的 mesh
                let mesh = Mesh {
                    geometries: vec![Geometry {
                        vertex_buffer,
                        index_buffer,
                    }],
                    blas: None,
                    blas_device_address: None,
                    name: format!("{}-{}", self.model_name, mesh_idx),
                };

                mesh_register(mesh)
            })
            .collect_vec();

        self.meshes = mesh_uuids;
    }

    /// 加载场景中的所有材质
    fn load_mats(&mut self, mut mat_register: impl FnMut(Material) -> MatGuid) {
        let mat_cnt = unsafe { get_mat_cnt(self.loader) };

        let mat_uuids = (0..mat_cnt)
            .map(|mat_idx| unsafe {
                let mat = get_mat(self.loader, mat_idx);
                let mat = &*mat;

                mat_register(Material {
                    base_color: std::mem::transmute::<CxxVec4f, glam::Vec4>(mat.base_color),
                    emissive: std::mem::transmute::<CxxVec4f, glam::Vec4>(mat.emissive_color),
                    metallic: mat.metallic_factor,
                    roughness: mat.roughness_factor,
                    opaque: mat.opaque_factor,

                    diffuse_map: std::ffi::CStr::from_ptr(mat.diffuse_map.as_ptr()).to_str().unwrap().to_string(),
                    normal_map: std::ffi::CStr::from_ptr(mat.normal_map.as_ptr()).to_str().unwrap().to_string(),
                })
            })
            .collect_vec();

        self.mats = mat_uuids;
    }

    /// 加载场景中的所有 instance
    ///
    /// 由于 Assimp 的复用层级是 geometry，而应用需要的复用层级是 mesh
    ///
    /// 因此将 Assimp 中的一个 Instance 拆分为多个 Instance，将其 geometry
    /// 提升为 mesh
    fn load_instance(&mut self, mut instance_register: impl FnMut(Instance) -> InsGuid) {
        let instance_cnt = unsafe { get_instance_cnt(self.loader) };
        let instances = (0..instance_cnt)
            .filter_map(|instance_idx| unsafe {
                let instance = get_instance(self.loader, instance_idx);
                let instance = &*instance;

                let mesh_cnt = instance.mesh_cnt_;
                if mesh_cnt == 0 { None } else { Some(instance) }
            })
            .flat_map(|instance| unsafe {
                let mesh_cnt = instance.mesh_cnt_;

                let mat_indices = if !instance.mat_indices_.is_null() {
                    std::slice::from_raw_parts(instance.mat_indices_, mesh_cnt as usize)
                } else {
                    &[]
                };
                let mesh_indices = if !instance.mesh_indices_.is_null() {
                    std::slice::from_raw_parts(instance.mesh_indices_, mesh_cnt as usize)
                } else {
                    &[]
                };

                let mesh_uuids = mesh_indices.iter().map(|mesh_idx| self.meshes[*mesh_idx as usize]);
                let mat_uuids = mat_indices.iter().map(|mat_idx| self.mats[*mat_idx as usize]);

                let mut ins_uuids = Vec::with_capacity(mesh_cnt as usize);
                for (mesh_uuid, mat_uuid) in std::iter::zip(mesh_uuids, mat_uuids) {
                    let instance = Instance {
                        transform: std::mem::transmute::<CxxMat4f, glam::Mat4>(instance.world_transform),
                        mesh: mesh_uuid,
                        materials: vec![mat_uuid],
                    };
                    ins_uuids.push(instance_register(instance));
                }

                ins_uuids.into_iter()
            })
            .collect_vec();

        self.instances = instances
    }
}
