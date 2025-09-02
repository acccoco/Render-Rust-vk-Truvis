use ash::vk;
use itertools::Itertools;
use truvis_rhi::{
    raytracing::acceleration::{Acceleration, BlasInputInfo},
    render_context::RenderContext,
    resources::special_buffers::{index_buffer::IndexBuffer, vertex_buffer::VertexBuffer},
};

use crate::{
    guid_new_type::{MatGuid, MeshGuid},
    vertex::vertex_3d::Vertex3D,
};

#[derive(Default)]
pub struct DrsMaterial {
    pub base_color: glam::Vec4,
    pub emissive: glam::Vec4,
    pub metallic: f32,
    pub roughness: f32,
    pub opaque: f32,

    pub diffuse_map: String,
    pub normal_map: String,
}

pub struct DrsGeometry<V: bytemuck::Pod> {
    pub vertex_buffer: VertexBuffer<V>,
    pub index_buffer: IndexBuffer,
}
pub type DrsGeometry3D = DrsGeometry<Vertex3D>;
impl<V: bytemuck::Pod> DrsGeometry<V> {
    #[inline]
    pub fn index_type() -> vk::IndexType {
        vk::IndexType::UINT32
    }

    #[inline]
    pub fn index_cnt(&self) -> u32 {
        self.index_buffer.index_cnt() as u32
    }
}
impl DrsGeometry3D {
    pub fn get_blas_geometry_info(&self) -> BlasInputInfo<'_> {
        let geometry_triangle = vk::AccelerationStructureGeometryTrianglesDataKHR {
            vertex_format: vk::Format::R32G32B32_SFLOAT,
            vertex_data: vk::DeviceOrHostAddressConstKHR {
                device_address: self.vertex_buffer.device_address(),
            },
            vertex_stride: size_of::<Vertex3D>() as vk::DeviceSize,
            // spec 上说应该是 vertex cnt - 1
            max_vertex: self.vertex_buffer.vertex_cnt() as u32 - 1,
            index_type: Self::index_type(),
            index_data: vk::DeviceOrHostAddressConstKHR {
                device_address: self.index_buffer.device_address(),
            },

            // 并不需要为每个 geometry 设置变换数据
            transform_data: vk::DeviceOrHostAddressConstKHR::default(),

            ..Default::default()
        };

        BlasInputInfo {
            geometry: vk::AccelerationStructureGeometryKHR::default()
                .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                // OPAQUE 表示永远不会调用 anyhit shader
                // NO_DUPLICATE 表示 primitive 只会被 any hit shader 命中一次
                .flags(vk::GeometryFlagsKHR::NO_DUPLICATE_ANY_HIT_INVOCATION)
                .geometry(vk::AccelerationStructureGeometryDataKHR {
                    triangles: geometry_triangle,
                }),
            range: vk::AccelerationStructureBuildRangeInfoKHR {
                primitive_count: self.index_cnt() / 3,
                primitive_offset: 0,
                first_vertex: 0,
                // 如果上方的 geometry data 中 的 transform_data 有数据，则该 offset 用于指定
                // transform 的 bytes offset
                transform_offset: 0,
            },
        }
    }
}

pub struct DrsMesh {
    pub geometries: Vec<DrsGeometry<Vertex3D>>,

    pub blas: Option<Acceleration>,
    pub name: String,
    pub blas_device_address: Option<vk::DeviceAddress>,
}

impl DrsMesh {
    pub fn build_blas(&mut self, rhi: &RenderContext) {
        if self.blas.is_some() {
            return; // 已经构建过了
        }

        let blas_infos = self.geometries.iter().map(|g| g.get_blas_geometry_info()).collect_vec();
        let blas = Acceleration::build_blas_sync(
            rhi,
            &blas_infos,
            vk::BuildAccelerationStructureFlagsKHR::empty(),
            format!("{}-Blas", self.name),
        );

        self.blas_device_address = Some(blas.get_device_address());
        self.blas = Some(blas);
    }
}

#[derive(Clone)]
pub struct DrsInstance {
    pub mesh: MeshGuid,
    pub materials: Vec<MatGuid>,
    pub transform: glam::Mat4,
}
