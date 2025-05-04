use crate::vertex::VertexLayout;
use ash::vk;
use std::mem::offset_of;
use truvis_rhi::core::buffer::RhiBuffer;
use truvis_rhi::rhi::Rhi;
use crate::component::mesh::SimpleMesh;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct VertexPosColor {
    pos: [f32; 4],
    color: [f32; 4],
}

pub struct VertexAosLayoutPosColor;
impl VertexLayout for VertexAosLayoutPosColor {
    fn vertex_input_bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<VertexPosColor>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn vertex_input_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(VertexPosColor, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(VertexPosColor, color) as u32,
            },
        ]
    }
}

impl VertexAosLayoutPosColor {
    pub fn create_vertex_buffer(rhi: &Rhi, data: &[VertexPosColor], name: impl AsRef<str>) -> RhiBuffer {
        let mut vertex_buffer = RhiBuffer::new_vertex_buffer(rhi, size_of_val(data), name.as_ref());
        vertex_buffer.transfer_data_sync(rhi, data);

        vertex_buffer
    }

    /// return: (vertex_buffer, index_buffer)
    pub fn triangle(rhi: &Rhi) -> SimpleMesh {
        let vertex_buffer = Self::create_vertex_buffer(rhi, &shape::TRIANGLE_VERTEX_DATA, "triangle-vertex-buffer");

        let mut index_buffer =
            RhiBuffer::new_index_buffer(rhi, size_of_val(&shape::TRIANGLE_INDEX_DATA), "triangle-index-buffer");
        index_buffer.transfer_data_sync(rhi, &shape::TRIANGLE_INDEX_DATA);

        SimpleMesh {
            vertex_buffer,
            index_buffer,
            index_cnt: shape::TRIANGLE_INDEX_DATA.len() as u32,
        }
    }

    pub fn rectangle(rhi: &Rhi) -> SimpleMesh {
        let vertex_buffer = Self::create_vertex_buffer(rhi, &shape::RECTANGLE_VERTEX_DATA, "rectangle-vertex-buffer");

        let mut index_buffer =
            RhiBuffer::new_index_buffer(rhi, size_of_val(&shape::RECTANGLE_INDEX_DATA), "rectangle-index-buffer");
        index_buffer.transfer_data_sync(rhi, &shape::RECTANGLE_INDEX_DATA);

        SimpleMesh {
            vertex_buffer,
            index_buffer,
            index_cnt: shape::RECTANGLE_INDEX_DATA.len() as u32,
        }
    }
}

mod shape {
    use crate::vertex::vertex_pc::VertexPosColor;

    /// 位于 RightHand-Y-Up 的坐标系，XY 平面上的一个正立的三角形
    pub const TRIANGLE_INDEX_DATA: [u32; 3] = [0u32, 1, 2];
    pub const TRIANGLE_VERTEX_DATA: [VertexPosColor; 3] = [
        VertexPosColor {
            pos: [-1.0, -1.0, 0.0, 1.0],
            color: [0.0, 1.0, 0.0, 1.0],
        },
        VertexPosColor {
            pos: [1.0, -1.0, 0.0, 1.0],
            color: [0.0, 0.0, 1.0, 1.0],
        },
        VertexPosColor {
            pos: [0.0, 1.0, 0.0, 1.0],
            color: [1.0, 0.0, 0.0, 1.0],
        },
    ];

    pub const RECTANGLE_INDEX_DATA: [u32; 6] = [
        0u32, 1, 2, //
        0, 2, 3,
    ];
    pub const RECTANGLE_VERTEX_DATA: [VertexPosColor; 4] = [
        // left bottom
        VertexPosColor {
            pos: [-1.0, 1.0, 0.0, 1.0],
            color: [0.2, 0.2, 0.0, 1.0],
        },
        // right bottom
        VertexPosColor {
            pos: [1.0, 1.0, 0.0, 1.0],
            color: [0.8, 0.2, 0.0, 1.0],
        },
        // right top
        VertexPosColor {
            pos: [1.0, -1.0, 0.0, 1.0],
            color: [0.8, 0.8, 0.0, 1.0],
        },
        // left top
        VertexPosColor {
            pos: [-1.0, -1.0, 0.0, 1.0],
            color: [0.2, 0.8, 0.0, 1.0],
        },
    ];
}
