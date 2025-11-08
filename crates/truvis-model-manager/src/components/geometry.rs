use crate::vertex::aos_3d::VertexLayoutAoS3D;
use crate::vertex::soa_3d::VertexLayoutSoA3D;
use ash::vk;
use truvis_gfx::raytracing::acceleration::BlasInputInfo;
use truvis_gfx::resources::special_buffers::index_buffer::Index32Buffer;
use truvis_gfx::resources::special_buffers::vertex_buffer::{VertexBuffer, VertexLayout};

/// CPU 侧的几何体数据
pub struct Geometry<L: VertexLayout> {
    pub vertex_buffer: VertexBuffer<L>,
    pub index_buffer: Index32Buffer,
}
pub type GeometryAoS3D = Geometry<VertexLayoutAoS3D>;
pub type GeometrySoA3D = Geometry<VertexLayoutSoA3D>;

// getters
impl<L: VertexLayout> Geometry<L> {
    #[inline]
    pub fn index_type() -> vk::IndexType {
        vk::IndexType::UINT32
    }

    #[inline]
    pub fn index_cnt(&self) -> u32 {
        self.index_buffer.index_cnt() as u32
    }
}

// tools
impl<L: VertexLayout> Geometry<L> {
    pub fn get_blas_geometry_info(&self) -> BlasInputInfo<'_> {
        let geometry_triangle = vk::AccelerationStructureGeometryTrianglesDataKHR {
            vertex_format: vk::Format::R32G32B32_SFLOAT,
            vertex_data: vk::DeviceOrHostAddressConstKHR {
                device_address: self.vertex_buffer.pos_address(),
            },
            vertex_stride: L::pos_stride() as vk::DeviceSize,
            // spec 上说应该是 vertex cnt - 1，应该是用作 index
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
