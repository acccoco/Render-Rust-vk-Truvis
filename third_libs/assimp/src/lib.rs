use itertools::Itertools;
use model_manager::component::instance::SimpleInstance;
use model_manager::component::mat::SimpleMaterial;
use model_manager::component::mesh::SimpleMesh;
use model_manager::vertex::vertex_3d::{Vertex3D, VertexLayoutAos3D};
use std::ffi::c_void;
use std::mem::offset_of;
use truvis_rhi::core::buffer::RhiBuffer;
use truvis_rhi::rhi::Rhi;

pub mod _ffi_bindings;
use crate::_ffi_bindings::*;

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

    meshes: Vec<uuid::Uuid>,
    mats: Vec<uuid::Uuid>,
    instances: Vec<uuid::Uuid>,
}

impl AssimpSceneLoader {
    /// return: instance
    pub fn load_model(
        rhi: &Rhi,
        model_file: &std::path::Path,
        instance_register: &mut dyn FnMut(SimpleInstance) -> uuid::Uuid,
        mesh_register: &mut dyn FnMut(SimpleMesh) -> uuid::Uuid,
        mat_register: &mut dyn FnMut(SimpleMaterial) -> uuid::Uuid,
    ) -> Vec<uuid::Uuid> {
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

            scene_loader.load_mesh(rhi, mesh_register);
            scene_loader.load_mats(rhi, mat_register);
            scene_loader.load_instance(instance_register);

            free_scene(loader);

            scene_loader.instances
        }
    }

    fn load_mesh(&mut self, rhi: &Rhi, mesh_register: &mut dyn FnMut(SimpleMesh) -> uuid::Uuid) {
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

                let vertex_buffer = VertexLayoutAos3D::create_vertex_buffer(
                    rhi,
                    vertex_data,
                    format!("{}-mesh-{}", self.model_name, mesh_idx),
                );

                if mesh.face_array_.is_null() {
                    panic!("Mesh {} has no index data", mesh_idx);
                }
                let index_data =
                    std::slice::from_raw_parts(mesh.face_array_ as *const u32, mesh.face_cnt_ as usize * 3);
                let index_buffer = RhiBuffer::new_index_buffer_sync(
                    rhi,
                    index_data,
                    format!("{}-mesh-{}-indices", self.model_name, mesh_idx),
                );

                let mesh = SimpleMesh {
                    vertex_buffer,
                    index_buffer,
                    index_cnt: mesh.face_cnt_ * 3,
                };

                mesh_register(mesh)
            })
            .collect_vec();

        self.meshes = mesh_uuids;
    }

    fn load_mats(&mut self, _rhi: &Rhi, mat_register: &mut dyn FnMut(SimpleMaterial) -> uuid::Uuid) {
        let mat_cnt = unsafe { get_mat_cnt(self.loader) };

        let mat_uuids = (0..mat_cnt)
            .map(|mat_idx| unsafe {
                let mat = get_mat(self.loader, mat_idx);
                let mat = &*mat;

                let mat_uuid = mat_register(SimpleMaterial {
                    ambient: std::mem::transmute::<CxxVec4f, glam::Vec4>(mat.ambient),
                    diffuse: std::mem::transmute::<CxxVec4f, glam::Vec4>(mat.diffuse),
                    specular: std::mem::transmute::<CxxVec4f, glam::Vec4>(mat.specular),
                    emissive: std::mem::transmute::<CxxVec4f, glam::Vec4>(mat.emission),

                    diffuse_map: std::ffi::CStr::from_ptr(mat.diffuse_map.as_ptr()).to_str().unwrap().to_string(),
                    specular_map: std::ffi::CStr::from_ptr(mat.specular_map.as_ptr()).to_str().unwrap().to_string(),
                    emissive_map: std::ffi::CStr::from_ptr(mat.emissive_map.as_ptr()).to_str().unwrap().to_string(),
                    ambient_map: std::ffi::CStr::from_ptr(mat.ambient_map.as_ptr()).to_str().unwrap().to_string(),
                    ..Default::default()
                });

                mat_uuid
            })
            .collect_vec();

        self.mats = mat_uuids;
    }

    fn load_instance(&mut self, instance_register: &mut dyn FnMut(SimpleInstance) -> uuid::Uuid) {
        let instance_cnt = unsafe { get_instance_cnt(self.loader) };
        let instances = (0..instance_cnt)
            .filter_map(|instance_idx| unsafe {
                let instance = get_instance(self.loader, instance_idx);
                let instance = &*instance;

                let mesh_cnt = instance.mesh_cnt_;
                if mesh_cnt == 0 {
                    return None;
                }

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

                let mesh_ids = mesh_indices.iter().map(|mesh_idx| self.meshes[*mesh_idx as usize]).collect_vec();
                let mat_ids = mat_indices.iter().map(|mat_idx| self.mats[*mat_idx as usize]).collect_vec();

                let instance = SimpleInstance {
                    transform: std::mem::transmute::<CxxMat4f, glam::Mat4>(instance.world_transform),
                    meshes: mesh_ids,
                    mats: mat_ids,
                };

                Some(instance_register(instance))
            })
            .collect_vec();

        self.instances = instances
    }
}
