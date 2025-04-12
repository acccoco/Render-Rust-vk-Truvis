use ash::vk;
use std::mem::offset_of;

/// Aos: Array of Structures
#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct VertexPCAoS {
    pos: [f32; 4],
    color: [f32; 4],
}
impl VertexPCAoS {
    pub const TRIANGLE_INDEX_DATA: [u32; 3] = [0u32, 1, 2];
    pub const TRIANGLE_VERTEX_DATA: [VertexPCAoS; 3] = [
        VertexPCAoS {
            pos: [-1.0, 1.0, 0.0, 1.0],
            color: [0.0, 1.0, 0.0, 1.0],
        },
        VertexPCAoS {
            pos: [1.0, 1.0, 0.0, 1.0],
            color: [0.0, 0.0, 1.0, 1.0],
        },
        VertexPCAoS {
            pos: [0.0, -1.0, 0.0, 1.0],
            color: [1.0, 0.0, 0.0, 1.0],
        },
    ];

    pub const RECTANGLE_INDEX_DATA: [u32; 6] = [
        0u32, 1, 2, //
        0, 2, 3,
    ];
    pub const RECTANGLE_VERTEX_DATA: [VertexPCAoS; 4] = [
        // left bottom
        VertexPCAoS {
            pos: [-1.0, 1.0, 0.0, 1.0],
            color: [0.2, 0.2, 0.0, 1.0],
        },
        // right bottom
        VertexPCAoS {
            pos: [1.0, 1.0, 0.0, 1.0],
            color: [0.8, 0.2, 0.0, 1.0],
        },
        // right top
        VertexPCAoS {
            pos: [1.0, -1.0, 0.0, 1.0],
            color: [0.8, 0.8, 0.0, 1.0],
        },
        // left top
        VertexPCAoS {
            pos: [-1.0, -1.0, 0.0, 1.0],
            color: [0.2, 0.8, 0.0, 1.0],
        },
    ];

    pub fn vertex_input_bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<VertexPCAoS>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    pub fn vertex_input_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(VertexPCAoS, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(VertexPCAoS, color) as u32,
            },
        ]
    }
}
