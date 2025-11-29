use crate::vertex::soa_3d::VertexLayoutSoA3D;
use ash::vk;
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::raytracing::acceleration::GfxBlasInputInfo;
use truvis_gfx::resources::handles::{IndexBufferHandle, VertexBufferHandle};
use truvis_gfx::resources::layout::GfxVertexLayout;

/// 几何体数据（包含顶点和索引缓冲）
///
/// 封装 GPU 顶点缓冲和索引缓冲，支持泛型顶点布局。
/// 可用于光栅化渲染和光线追踪加速结构构建。
///
/// # 类型别名
/// - `GeometryAoS3D`: AoS 3D 顶点布局（Position + Normal + TexCoord）
/// - `GeometrySoA3D`: SoA 3D 顶点布局（分离存储）
pub struct Geometry<L: GfxVertexLayout> {
    pub vertex_buffer: VertexBufferHandle<L>,
    pub index_buffer: IndexBufferHandle,
}
pub type GeometrySoA3D = Geometry<VertexLayoutSoA3D>;

// getters
impl<L: GfxVertexLayout> Geometry<L> {
    #[inline]
    pub fn index_type() -> vk::IndexType {
        vk::IndexType::UINT32
    }

    #[inline]
    pub fn index_cnt(&self) -> u32 {
        let resource_manager = Gfx::get().resource_manager();
        let buffer = resource_manager.get_index_buffer(self.index_buffer).expect("Index buffer not found");
        buffer.element_count
    }
}

// tools
impl<L: GfxVertexLayout> Geometry<L> {
    pub fn get_blas_geometry_info(&self) -> GfxBlasInputInfo<'_> {
        let resource_manager = Gfx::get().resource_manager();
        let v_buffer = resource_manager.get_vertex_buffer(self.vertex_buffer).expect("Vertex buffer not found");
        let i_buffer = resource_manager.get_index_buffer(self.index_buffer).expect("Index buffer not found");

        let geometry_triangle = vk::AccelerationStructureGeometryTrianglesDataKHR {
            vertex_format: vk::Format::R32G32B32_SFLOAT,
            vertex_data: vk::DeviceOrHostAddressConstKHR {
                device_address: v_buffer.device_addr.unwrap() + L::pos_offset(v_buffer.element_count as usize),
            },
            vertex_stride: L::pos_stride() as vk::DeviceSize,
            // spec 上说应该是 vertex cnt - 1，应该是用作 index
            max_vertex: v_buffer.element_count - 1,
            index_type: Self::index_type(),
            index_data: vk::DeviceOrHostAddressConstKHR {
                device_address: i_buffer.device_addr.unwrap(),
            },

            // 并不需要为每个 geometry 设置变换数据
            transform_data: vk::DeviceOrHostAddressConstKHR::default(),

            ..Default::default()
        };

        GfxBlasInputInfo {
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

impl GeometrySoA3D {
    #[inline]
    pub fn cmd_bind_index_buffer(&self, cmd: &GfxCommandBuffer) {
        let resource_manager = Gfx::get().resource_manager();
        let buffer = resource_manager.get_index_buffer(self.index_buffer).expect("Index buffer not found");
        cmd.cmd_bind_index_buffer_raw(buffer.buffer, 0, Self::index_type());
    }

    #[inline]
    pub fn cmd_bind_vertex_buffers(&self, cmd: &GfxCommandBuffer) {
        let resource_manager = Gfx::get().resource_manager();
        let buffer = resource_manager.get_vertex_buffer(self.vertex_buffer).expect("Vertex buffer not found");
        let vertex_cnt = buffer.element_count as usize;

        cmd.cmd_bind_vertex_buffers(
            0,
            &[buffer.buffer; 4],
            &[
                VertexLayoutSoA3D::pos_offset(vertex_cnt),
                VertexLayoutSoA3D::normal_offset(vertex_cnt),
                VertexLayoutSoA3D::tangent_offset(vertex_cnt),
                VertexLayoutSoA3D::uv_offset(vertex_cnt),
            ],
        );
    }
}
