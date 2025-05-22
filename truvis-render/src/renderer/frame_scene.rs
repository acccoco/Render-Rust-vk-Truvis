use crate::renderer::bindless::BindlessManager;
use crate::renderer::scene_manager::TheWorld;
use glam::Vec4Swizzles;
use model_manager::component::Geometry;
use shader_binding::shader;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;

struct DrawGeometry {
    ins_id: uuid::Uuid,
    mesh_id: uuid::Uuid,
    submesh_idx: usize,
    mat_id: uuid::Uuid,
}

/// 用于构建传输到 GPU 的场景数据
pub struct GpuScene {
    gpu_meshes: Vec<DrawGeometry>,

    gpu_mats: Vec<uuid::Uuid>,

    /// 用于从 mat id 找到 gpu 中对应的材质 idx
    ///
    /// 🔑：mai id
    ///
    /// 📦：index in mats
    mat_map: HashMap<uuid::Uuid, usize>,

    scene_mgr: Rc<RefCell<TheWorld>>,
    bindless_mgr: Rc<RefCell<BindlessManager>>,
}

impl GpuScene {
    pub fn new(scene_mgr: Rc<RefCell<TheWorld>>, bindless_mgr: Rc<RefCell<BindlessManager>>) -> Self {
        Self {
            gpu_meshes: Vec::new(),
            gpu_mats: Vec::new(),
            mat_map: HashMap::new(),
            scene_mgr,
            bindless_mgr,
        }
    }

    /// 在每一帧开始时调用，将场景数据转换为 GPU 可读的形式
    pub fn prepare_render_data(&mut self, frame_idx: usize) {
        self.bindless_mgr.borrow_mut().prepare_render_data(frame_idx);

        self.gen_mats();
        self.gen_draw_mesh();
    }

    /// 将已经准备好的 GPU 格式的场景数据写入 Device Buffer 中
    pub fn write_to_buffer(&self, buffer: &mut shader::FrameData) {
        self.write_instance_buffer(&mut buffer.ins_data);
        self.write_mesh_buffer(&mut buffer.mat_data);
        self.write_light_buffer(&mut buffer.light_data);
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
    fn gen_draw_mesh(&mut self) {
        let scene_mgr = self.scene_mgr.borrow();
        self.gpu_meshes = scene_mgr
            .instance_map
            .iter()
            .flat_map(|(ins_id, ins)| {
                ins.materials.iter().enumerate().map(|(submesh_idx, mat_id)| DrawGeometry {
                    ins_id: *ins_id,
                    mesh_id: ins.mesh,
                    mat_id: *mat_id,
                    submesh_idx,
                })
            })
            .collect();
    }

    /// 将所有的材质转换为 Vector，准备上传到 GPU
    fn gen_mats(&mut self) {
        self.gpu_mats.clear();
        self.gpu_mats.reserve(self.scene_mgr.borrow().mat_map.len());

        self.mat_map.clear();
        for (mat_idx, (mat_id, _)) in self.scene_mgr.borrow().mat_map.iter().enumerate() {
            self.gpu_mats.push(*mat_id);
            self.mat_map.insert(*mat_id, mat_idx);
        }
    }

    /// 将数据转换为 shader 中的实例数据
    ///
    /// 其中 buffer 可以是 stage buffer 的内存映射
    fn write_instance_buffer(&self, buffer: &mut shader::InstanceData) {
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

    fn write_mesh_buffer(&self, buffer: &mut shader::MatData) {
        if buffer.materials.len() < self.gpu_mats.len() {
            panic!("material cnt can not be larger than buffer");
        }

        buffer.mat_count.x = self.gpu_mats.len() as u32;
        for (mat_idx, mat_id) in self.gpu_mats.iter().enumerate() {
            let scene_mgr = self.scene_mgr.borrow();
            let mat = scene_mgr.mat_map.get(mat_id).unwrap();
            buffer.materials[mat_idx] = shader::PBRMaterial {
                base_color: mat.diffuse.xyz().into(),
                emissive: mat.emissive.xyz().into(),
                metallic: 0.5,
                roughness: 0.5,
                diffuse_map: self.bindless_mgr.borrow().get_texture_idx(&mat.diffuse_map).unwrap_or(0),
                normal_map: 0,
                ..Default::default()
            };
        }
    }

    fn write_light_buffer(&self, buffer: &mut shader::LightData) {
        if buffer.lights.len() < self.scene_mgr.borrow().point_light_map.len() {
            panic!("point light cnt can not be larger than buffer");
        }

        buffer.light_count.x = self.scene_mgr.borrow().point_light_map.len() as u32;
        for (light_idx, (_, point_light)) in self.scene_mgr.borrow().point_light_map.iter().enumerate() {
            buffer.lights[light_idx] = *point_light;
        }
    }
}
