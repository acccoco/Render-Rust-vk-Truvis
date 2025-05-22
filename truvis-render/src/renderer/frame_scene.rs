use crate::renderer::bindless::BindlessManager;
use crate::renderer::scene_manager::TheWorld;
use glam::Vec4Swizzles;
use model_manager::component::Geometry;
use shader_binding::shader;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use truvis_rhi::core::buffer::RhiStructuredBuffer;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::rhi::Rhi;

struct DrawGeometry {
    ins_id: uuid::Uuid,
    mesh_id: uuid::Uuid,
    submesh_idx: usize,
    mat_id: uuid::Uuid,
}

/// 数据以顺序的方式存储，同时查找时间为 O(1)
#[derive(Default)]
struct FlattenMap<T: std::hash::Hash + Eq + Copy> {
    /// 顺序形式存储的数据
    linear_storage: Vec<T>,

    /// 用于查找数据的 HashMap
    query_table: HashMap<T, usize>,
}
impl<T: std::hash::Hash + Eq + Copy> FlattenMap<T> {
    /// 找到数据在 store 中的索引
    #[inline]
    pub fn at(&self, key: &T) -> Option<usize> {
        self.query_table.get(key).copied()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.linear_storage.len()
    }

    /// 清空所有数据
    #[inline]
    pub fn clear(&mut self) {
        self.linear_storage.clear();
        self.query_table.clear();
    }

    /// 为数据预留空间
    #[inline]
    pub fn reserve(&mut self, cnt: usize) {
        self.linear_storage.reserve(cnt);
    }

    /// 将数据插入到 store 中
    #[inline]
    pub fn insert(&mut self, key: T) {
        if let Some(idx) = self.query_table.get(&key) {
            panic!("key already exists in store");
        }

        let idx = self.linear_storage.len();
        self.linear_storage.push(key);
        self.query_table.insert(key, idx);
    }
}

/// 用于构建传输到 GPU 的场景数据
pub struct GpuScene {
    /// GPU 中以顺序存储的 instance
    gpu_instances: Vec<uuid::Uuid>,

    /// GPU 中以顺序存储的材质信息
    gpu_mats: FlattenMap<uuid::Uuid>,

    /// GPU 中以顺序存储的 mesh 信息
    ///
    /// 每个 mesh 会被分为多个 submesh，且每个 mesh 的 submesh 会被顺序存储
    gpu_meshes: FlattenMap<uuid::Uuid>,

    /// 每个 mesh 的 geometry 在 mesh 的所有 geometry 中的索引
    mesh_geometry_map: HashMap<uuid::Uuid, (usize, usize)>,

    scene_mgr: Rc<RefCell<TheWorld>>,
    bindless_mgr: Rc<RefCell<BindlessManager>>,

    ligth_buffer: RhiStructuredBuffer<shader::PointLight>,
    light_stage_buffer: RhiStructuredBuffer<shader::PointLight>,
    material_buffer: RhiStructuredBuffer<shader::PBRMaterial>,
    material_stage_buffer: RhiStructuredBuffer<shader::PBRMaterial>,
    geometry_buffer: RhiStructuredBuffer<shader::Geometry>,
    geometry_stage_buffer: RhiStructuredBuffer<shader::Geometry>,
    instance_buffer: RhiStructuredBuffer<shader::Instance>,
    instance_stage_buffer: RhiStructuredBuffer<shader::Instance>,
    instance_material_buffer: RhiStructuredBuffer<u32>,
    instance_material_stage_buffer: RhiStructuredBuffer<u32>,
    instance_geometry_buffer: RhiStructuredBuffer<u32>,
    instance_geometry_stage_buffer: RhiStructuredBuffer<u32>,
}

impl GpuScene {
    pub fn new(rhi: &Rhi, scene_mgr: Rc<RefCell<TheWorld>>, bindless_mgr: Rc<RefCell<BindlessManager>>) -> Self {
        Self {
            scene_mgr,
            bindless_mgr,

            gpu_instances: vec![],
            gpu_mats: FlattenMap::default(),
            gpu_meshes: FlattenMap::default(),
            mesh_geometry_map: HashMap::new(),

            ligth_buffer: RhiStructuredBuffer::new_ubo(rhi, 512, "light buffer"),
            light_stage_buffer: RhiStructuredBuffer::new_stage_buffer(rhi, 512, "light stage buffer"),
            material_buffer: RhiStructuredBuffer::new_ubo(rhi, 1024, "material buffer"),
            material_stage_buffer: RhiStructuredBuffer::new_stage_buffer(rhi, 1024, "material stage buffer"),
            geometry_buffer: RhiStructuredBuffer::new_ubo(rhi, 1024 * 8, "geometry buffer"),
            geometry_stage_buffer: RhiStructuredBuffer::new_stage_buffer(rhi, 1024 * 8, "geometry stage buffer"),
            instance_buffer: RhiStructuredBuffer::new_ubo(rhi, 1024, "instance buffer"),
            instance_stage_buffer: RhiStructuredBuffer::new_stage_buffer(rhi, 1024, "instance stage buffer"),
            instance_material_buffer: RhiStructuredBuffer::new_ubo(rhi, 1024 * 8, "instance material buffer"),
            instance_material_stage_buffer: RhiStructuredBuffer::new_stage_buffer(
                rhi,
                1024 * 8,
                "instance material stage buffer",
            ),
            instance_geometry_buffer: RhiStructuredBuffer::new_ubo(rhi, 1024 * 8, "instance geometry buffer"),
            instance_geometry_stage_buffer: RhiStructuredBuffer::new_stage_buffer(
                rhi,
                1024 * 8,
                "instance geometry stage buffer",
            ),
        }
    }

    /// 在每一帧开始时调用，将场景数据转换为 GPU 可读的形式
    pub fn prepare_render_data(&mut self, frame_idx: usize) {
        self.bindless_mgr.borrow_mut().prepare_render_data(frame_idx);

        self.prepare_mat_data();
        self.prepare_mesh_data();
    }

    /// 将已经准备好的 GPU 格式的场景数据写入 Device Buffer 中
    pub fn upload_to_buffer(&self, buffer: &mut shader::FrameData) {
        self.upload_instance_buffer(&mut buffer.ins_data);
        self.upload_material_buffer(&mut buffer.mat_data);
        self.upload_light_buffer(&mut buffer.light_data);
    }

    /// 绘制场景中的所有示例
    pub fn draw(&self, cmd: &RhiCommandBuffer, before_draw: &mut dyn FnMut(u32)) {
        for (ins_idx, sub_mesh) in self.gpu_meshes.iter().enumerate() {
            let scene_mgr = self.scene_mgr.borrow();
            let mesh = scene_mgr.mesh_map.get(&sub_mesh.mesh_id).unwrap();
            let geometry = mesh.geometries.get(sub_mesh.submesh_idx).unwrap();

            cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&geometry.vertex_buffer), &[0]);
            cmd.cmd_bind_index_buffer(&geometry.index_buffer, 0, Geometry::index_type());

            before_draw(ins_idx as u32);
            cmd.draw_indexed(geometry.index_cnt, 0, 1, 0, 0);
        }
    }

    /// 将所有的实例转换为 Vector，准备上传到 GPU
    fn prepare_mesh_data(&mut self) {
        let scene_mgr = self.scene_mgr.borrow();

        self.mesh_geometry_map.clear();
        self.gpu_meshes.clear();
        self.gpu_meshes.reserve(scene_mgr.mesh_map.len());

        let mut geometry_idx = 0;
        for (mesh_id, mesh) in scene_mgr.mesh_map.iter() {
            self.gpu_meshes.insert(*mesh_id);
            self.mesh_geometry_map.insert(*mesh_id, (geometry_idx, mesh.geometries.len()));
            geometry_idx += mesh.geometries.len();
        }
    }

    /// 在每一帧的绘制之前，将所有的材质转换为 Vector，准备上传到 GPU
    fn prepare_mat_data(&mut self) {
        let scene_mgr = self.scene_mgr.borrow();

        self.gpu_mats.clear();
        self.gpu_mats.reserve(scene_mgr.mat_map.len());

        for (mat_id, _) in scene_mgr.mat_map.iter() {
            self.gpu_mats.insert(*mat_id);
        }
    }

    /// 将数据转换为 shader 中的实例数据
    ///
    /// 其中 buffer 可以是 stage buffer 的内存映射
    fn upload_instance_buffer(&self, buffer: &mut shader::InstanceData) {
        if buffer.instances.len() < self.gpu_meshes.len() {
            panic!("instance cnt can not be larger than buffer");
        }

        buffer.instance_count.x = self.gpu_meshes.len() as u32;
        for (ins_idx, draw_mesh) in self.gpu_meshes.iter().enumerate() {
            let scene_mgr = self.scene_mgr.borrow();
            let instance = scene_mgr.instance_map.get(&draw_mesh.ins_id).unwrap();
            buffer.instances[ins_idx] = shader::SubMesh {
                model: instance.transform.into(),
                inv_model: instance.transform.inverse().into(),
                mat_id: *self.mat_map.get(&draw_mesh.mat_id).unwrap() as u32,
                ..Default::default()
            };
        }
    }

    /// 将 material 数据上传到 material buffer 中
    fn upload_material_buffer(&mut self) {
        self.material_stage_buffer.map();
        let gpu_mat_slices = self.material_stage_buffer.mapped_slice();
        if self.gpu_mats.len() < gpu_mat_slices.len() {
            panic!("material cnt can not be larger than buffer");
        }

        let scene_mgr = self.scene_mgr.borrow();
        let bindless_mgr = self.bindless_mgr.borrow();
        for (mat_idx, mat_uuid) in self.gpu_mats.linear_storage.iter().enumerate() {
            let mat = scene_mgr.mat_map.get(mat_uuid).unwrap();
            gpu_mat_slices[mat_idx] = shader::PBRMaterial {
                base_color: mat.diffuse.xyz().into(),
                emissive: mat.emissive.xyz().into(),
                metallic: 0.5,
                roughness: 0.5,
                diffuse_map: bindless_mgr.get_texture_idx(&mat.diffuse_map).unwrap_or(0),
                normal_map: 0,
                ..Default::default()
            };
        }

        let buffer_size = self.material_stage_buffer.size();
        self.material_stage_buffer.flush(0, buffer_size);
        self.material_stage_buffer.unmap();
    }

    fn upload_light_buffer(&self, buffer: &mut [shader::PointLight]) {
        if buffer.len() < self.scene_mgr.borrow().point_light_map.len() {
            panic!("point light cnt can not be larger than buffer");
        }

        buffer.light_count.x = self.scene_mgr.borrow().point_light_map.len() as u32;
        for (light_idx, (_, point_light)) in self.scene_mgr.borrow().point_light_map.iter().enumerate() {
            buffer.lights[light_idx] = *point_light;
        }
    }
}
