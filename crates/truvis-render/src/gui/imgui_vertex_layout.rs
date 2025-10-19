use ash::vk;
use std::mem::offset_of;
use truvis_rhi::resources::special_buffers::vertex_buffer::VertexLayout;

/// AoS: Array of Structs
pub struct ImGuiVertex {
    pos: glam::Vec2,
    uv: glam::Vec2,
    color: u32, // R8G8B8A8
}

pub struct ImGuiVertexLayoutAoS;
impl VertexLayout for ImGuiVertexLayoutAoS {
    fn vertex_input_bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<ImGuiVertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn vertex_input_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(ImGuiVertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(ImGuiVertex, uv) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R8G8B8A8_UNORM,
                offset: offset_of!(ImGuiVertex, color) as u32,
            },
        ]
    }

    fn pos3d_attribute() -> (u32, u32) {
        // 没有必要有这个属性
        unimplemented!()
    }

    fn buffer_size(vertex_cnt: usize) -> usize {
        vertex_cnt * size_of::<ImGuiVertex>()
    }
}

impl ImGuiVertex {}
