use crate::components::geometry::Geometry;
use ash::vk;
use ash::vk::DeviceSize;
use std::mem::offset_of;
use truvis_rhi::resources::special_buffers::index_buffer::Index32Buffer;
use truvis_rhi::resources::special_buffers::vertex_buffer::{VertexBuffer, VertexLayout};

#[repr(C)]
#[derive(Clone, Debug, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexPosColor {
    pos: [f32; 4],
    color: [f32; 4],
}

pub struct VertexLayoutAoSPosColor;
impl VertexLayout for VertexLayoutAoSPosColor {
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

    fn buffer_size(vertex_cnt: usize) -> usize {
        vertex_cnt * size_of::<VertexPosColor>()
    }

    fn pos_stride() -> u32 {
        size_of::<VertexPosColor>() as _
    }

    fn pos_offset(_vertex_cnt: usize) -> DeviceSize {
        offset_of!(VertexPosColor, pos) as _
    }
}

impl VertexLayoutAoSPosColor {
    pub fn create_vertex_buffer2(data: &[VertexPosColor], name: impl AsRef<str>) -> VertexBuffer<Self> {
        let vertex_buffer = VertexBuffer::new(data.len(), name.as_ref());
        vertex_buffer.transfer_data_sync(data);

        vertex_buffer
    }

    /// return: (vertex_buffer, index_buffer)
    pub fn triangle() -> Geometry<Self> {
        let vertex_buffer = Self::create_vertex_buffer2(&shape::TRIANGLE_VERTEX_DATA, "triangle-vertex-buffer");

        let index_buffer = Index32Buffer::new(shape::TRIANGLE_INDEX_DATA.len(), "triangle-index-buffer");
        index_buffer.transfer_data_sync(&shape::TRIANGLE_INDEX_DATA);

        Geometry {
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn rectangle() -> Geometry<Self> {
        let vertex_buffer = Self::create_vertex_buffer2(&shape::RECTANGLE_VERTEX_DATA, "rectangle-vertex-buffer");

        let index_buffer = Index32Buffer::new(shape::RECTANGLE_INDEX_DATA.len(), "rectangle-index-buffer");
        index_buffer.transfer_data_sync(&shape::RECTANGLE_INDEX_DATA);

        Geometry {
            vertex_buffer,
            index_buffer,
        }
    }
}

/// 定义了使用 AoS: Pos + Color 格式的顶点数据的基本图形
mod shape {
    use crate::vertex::aos_pos_color::VertexPosColor;

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
