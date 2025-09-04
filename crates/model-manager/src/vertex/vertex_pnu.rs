use std::mem::offset_of;

use ash::vk;
use truvis_rhi::resources::special_buffers::{index_buffer::IndexBuffer, vertex_buffer::VertexBuffer};

use crate::{component::DrsGeometry, vertex::VertexLayout};

/// AoS: Array of structures
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexPosNormalUv {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}
impl VertexPosNormalUv {
    const fn new(pos: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self { pos, normal, uv }
    }
}

pub struct VertexLayoutAosPosNormalUv;
impl VertexLayout for VertexLayoutAosPosNormalUv {
    fn vertex_input_bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<VertexPosNormalUv>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn vertex_input_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(VertexPosNormalUv, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(VertexPosNormalUv, normal) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(VertexPosNormalUv, uv) as u32,
            },
        ]
    }
}

impl VertexLayoutAosPosNormalUv {
    pub fn create_vertex_buffer(data: &[VertexPosNormalUv], name: impl AsRef<str>) -> VertexBuffer<VertexPosNormalUv> {
        let mut vertex_buffer = VertexBuffer::new(data.len(), name.as_ref());
        vertex_buffer.transfer_data_sync(data);

        vertex_buffer
    }

    pub fn cube() -> DrsGeometry<VertexPosNormalUv> {
        let vertex_buffer = Self::create_vertex_buffer(&shape::Cube::VERTICES, "cube-vertex-buffer");

        let mut index_buffer = IndexBuffer::new(shape::Cube::INDICES.len(), "cube-index-buffer");
        index_buffer.transfer_data_sync(&shape::Cube::INDICES);

        DrsGeometry {
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn floor() -> DrsGeometry<VertexPosNormalUv> {
        let vertex_buffer = Self::create_vertex_buffer(&shape::Floor::VERTICES, "floor-vertex-buffer");

        let mut index_buffer = IndexBuffer::new(shape::Floor::INDICES.len(), "floor-index-buffer");
        index_buffer.transfer_data_sync(&shape::Floor::INDICES);

        DrsGeometry {
            vertex_buffer,
            index_buffer,
        }
    }
}

mod shape {
    use crate::vertex::vertex_pnu::VertexPosNormalUv;

    /// Y-up, Right hand 的坐标系中：
    ///
    /// 面片位于 xz 平面上，朝向 y+（顶点顺序和法线保持一致）
    ///
    /// 两个三角形的顺序为：ABC, ACD
    ///
    /// ```text
    ///            z^
    ///             |
    ///      B-------------A
    ///       |     |     |
    /// ------|-----|-----|------>x
    ///       |     |     |
    ///      C-------------D
    ///             |
    /// ```
    pub struct Floor;
    impl Floor {
        const VERTEX_A: VertexPosNormalUv = VertexPosNormalUv::new([5.0, 0.0, 5.0], [0.0, 1.0, 0.0], [1.0, 0.0]);
        const VERTEX_B: VertexPosNormalUv = VertexPosNormalUv::new([-5.0, 0.0, 5.0], [0.0, 1.0, 0.0], [0.0, 0.0]);
        const VERTEX_C: VertexPosNormalUv = VertexPosNormalUv::new([-5.0, 0.0, -5.0], [0.0, 1.0, 0.0], [0.0, 1.0]);
        const VERTEX_D: VertexPosNormalUv = VertexPosNormalUv::new([5.0, 0.0, -5.0], [0.0, 1.0, 0.0], [1.0, 1.0]);

        pub const VERTICES: [VertexPosNormalUv; 4] = [Self::VERTEX_A, Self::VERTEX_B, Self::VERTEX_C, Self::VERTEX_D];

        pub const INDICES: [u32; 6] = [
            0, 1, 2, //
            0, 2, 3,
        ];
    }

    /// Y-up, X-Right, Right hand 的坐标系中：
    pub struct Cube;
    impl Cube {
        const TOP_A: VertexPosNormalUv = VertexPosNormalUv::new([0.5, 0.5, -0.5], [0.0, 1.0, 0.0], [1.0, 0.0]);
        const TOP_B: VertexPosNormalUv = VertexPosNormalUv::new([-0.5, 0.5, -0.5], [0.0, 1.0, 0.0], [0.0, 0.0]);
        const TOP_C: VertexPosNormalUv = VertexPosNormalUv::new([-0.5, 0.5, 0.5], [0.0, 1.0, 0.0], [0.0, 1.0]);
        const TOP_D: VertexPosNormalUv = VertexPosNormalUv::new([0.5, 0.5, 0.5], [0.0, 1.0, 0.0], [1.0, 1.0]);

        const BOTTOM_A: VertexPosNormalUv = VertexPosNormalUv::new([0.5, -0.5, -0.5], [0.0, -1.0, 0.0], [1.0, 0.0]);
        const BOTTOM_B: VertexPosNormalUv = VertexPosNormalUv::new([-0.5, -0.5, -0.5], [0.0, -1.0, 0.0], [0.0, 0.0]);
        const BOTTOM_C: VertexPosNormalUv = VertexPosNormalUv::new([-0.5, -0.5, 0.5], [0.0, -1.0, 0.0], [0.0, 1.0]);
        const BOTTOM_D: VertexPosNormalUv = VertexPosNormalUv::new([0.5, -0.5, 0.5], [0.0, -1.0, 0.0], [1.0, 1.0]);

        const NEAR_A: VertexPosNormalUv = VertexPosNormalUv::new([0.5, 0.5, 0.5], [0.0, 0.0, 1.0], [1.0, 0.0]);
        const NEAR_B: VertexPosNormalUv = VertexPosNormalUv::new([-0.5, 0.5, 0.5], [0.0, 0.0, 1.0], [0.0, 0.0]);
        const NEAR_C: VertexPosNormalUv = VertexPosNormalUv::new([-0.5, -0.5, 0.5], [0.0, 0.0, 1.0], [0.0, 1.0]);
        const NEAR_D: VertexPosNormalUv = VertexPosNormalUv::new([0.5, -0.5, 0.5], [0.0, 0.0, 1.0], [1.0, 1.0]);

        const FAR_A: VertexPosNormalUv = VertexPosNormalUv::new([0.5, 0.5, -0.5], [0.0, 0.0, -1.0], [1.0, 0.0]);
        const FAR_B: VertexPosNormalUv = VertexPosNormalUv::new([-0.5, 0.5, -0.5], [0.0, 0.0, -1.0], [0.0, 0.0]);
        const FAR_C: VertexPosNormalUv = VertexPosNormalUv::new([-0.5, -0.5, -0.5], [0.0, 0.0, -1.0], [0.0, 1.0]);
        const FAR_D: VertexPosNormalUv = VertexPosNormalUv::new([0.5, -0.5, -0.5], [0.0, 0.0, -1.0], [1.0, 1.0]);

        const LEFT_A: VertexPosNormalUv = VertexPosNormalUv::new([-0.5, 0.5, 0.5], [-1.0, 0.0, 0.0], [1.0, 0.0]);
        const LEFT_B: VertexPosNormalUv = VertexPosNormalUv::new([-0.5, 0.5, -0.5], [-1.0, 0.0, 0.0], [0.0, 0.0]);
        const LEFT_C: VertexPosNormalUv = VertexPosNormalUv::new([-0.5, -0.5, -0.5], [-1.0, 0.0, 0.0], [0.0, 1.0]);
        const LEFT_D: VertexPosNormalUv = VertexPosNormalUv::new([-0.5, -0.5, 0.5], [-1.0, 0.0, 0.0], [1.0, 1.0]);

        const RIGHT_A: VertexPosNormalUv = VertexPosNormalUv::new([0.5, 0.5, 0.5], [1.0, 0.0, 0.0], [1.0, 0.0]);
        const RIGHT_B: VertexPosNormalUv = VertexPosNormalUv::new([0.5, 0.5, -0.5], [1.0, 0.0, 0.0], [0.0, 0.0]);
        const RIGHT_C: VertexPosNormalUv = VertexPosNormalUv::new([0.5, -0.5, -0.5], [1.0, 0.0, 0.0], [0.0, 1.0]);
        const RIGHT_D: VertexPosNormalUv = VertexPosNormalUv::new([0.5, -0.5, 0.5], [1.0, 0.0, 0.0], [1.0, 1.0]);

        pub const VERTICES: [VertexPosNormalUv; 24] = [
            Self::TOP_A,
            Self::TOP_B,
            Self::TOP_C,
            Self::TOP_D,
            //
            Self::BOTTOM_A,
            Self::BOTTOM_B,
            Self::BOTTOM_C,
            Self::BOTTOM_D,
            //
            Self::NEAR_A,
            Self::NEAR_B,
            Self::NEAR_C,
            Self::NEAR_D,
            //
            Self::FAR_A,
            Self::FAR_B,
            Self::FAR_C,
            Self::FAR_D,
            //
            Self::LEFT_A,
            Self::LEFT_B,
            Self::LEFT_C,
            Self::LEFT_D,
            //
            Self::RIGHT_A,
            Self::RIGHT_B,
            Self::RIGHT_C,
            Self::RIGHT_D,
        ];

        pub const INDICES: [u32; 36] = [
            0, 1, 2, 0, 2, 3, // top
            4, 6, 5, 4, 7, 6, // bottom
            8, 9, 10, 8, 10, 11, // near
            12, 14, 13, 12, 15, 14, // far
            16, 17, 18, 16, 18, 19, // left
            20, 22, 21, 20, 23, 22, // right
        ];
    }
}
