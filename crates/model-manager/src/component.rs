use crate::vertex::vertex_3d::Vertex3D;
use ash::vk;
use itertools::Itertools;
use truvis_rhi::core::acceleration::{BlasInputInfo, RhiAcceleration};
use truvis_rhi::core::buffer::{RhiIndexBuffer, RhiVertexBuffer};
use truvis_rhi::rhi::Rhi;

#[derive(Default)]
pub struct TruMaterial {
    pub ambient: glam::Vec4,
    pub diffuse: glam::Vec4,
    pub specular: glam::Vec4,
    pub emissive: glam::Vec4,

    pub shininess: f32,
    pub alpha: f32,

    pub diffuse_map: String,
    pub ambient_map: String,
    pub emissive_map: String,
    pub specular_map: String,

    pub normal_map: String,
}

pub struct DrsGeometry<V: bytemuck::Pod> {
    pub vertex_buffer: RhiVertexBuffer<V>,
    pub index_buffer: RhiIndexBuffer,
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
    pub fn get_blas_geometry_info(&self) -> BlasInputInfo {
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
                .flags(vk::GeometryFlagsKHR::OPAQUE)
                .geometry(vk::AccelerationStructureGeometryDataKHR {
                    triangles: geometry_triangle,
                }),
            range: vk::AccelerationStructureBuildRangeInfoKHR {
                primitive_count: self.index_cnt() / 3,
                primitive_offset: 0,
                first_vertex: 0,
                // 如果上方的 geometry data 中 的 transform_data 有数据，则该 offset 用于指定 transform 的 bytes offset
                transform_offset: 0,
            },
        }
    }
}

pub struct DrsMesh {
    pub geometries: Vec<DrsGeometry<Vertex3D>>,

    pub blas: Option<RhiAcceleration>,
    pub name: String,
    pub blas_device_address: Option<vk::DeviceAddress>,
}

impl DrsMesh {
    pub fn build_blas(&mut self, rhi: &Rhi) {
        if self.blas.is_some() {
            return; // 已经构建过了
        }

        let blas_infos = self.geometries.iter().map(|g| g.get_blas_geometry_info()).collect_vec();
        let blas = RhiAcceleration::build_blas_sync(
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
    pub mesh: uuid::Uuid,
    pub materials: Vec<uuid::Uuid>,
    pub transform: glam::Mat4,
}
