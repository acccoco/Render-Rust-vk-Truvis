use crate::vertex::VertexLayout;
use ash::vk;
use std::mem::offset_of;
use truvis_rhi::core::resources::special_buffers::vertex_buffer::RhiVertexBuffer;
use truvis_rhi::rhi::Rhi;

#[repr(C)]
#[derive(Clone, Debug, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex3D {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
    pub uv: [f32; 2],
}

pub struct VertexLayoutAos3D;
impl VertexLayout for VertexLayoutAos3D {
    fn vertex_input_bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Vertex3D>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn vertex_input_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex3D, position) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex3D, normal) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex3D, tangent) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 3,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex3D, bitangent) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 4,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex3D, uv) as u32,
            },
        ]
    }
}

impl VertexLayoutAos3D {
    pub fn create_vertex_buffer(rhi: &Rhi, data: &[Vertex3D], name: impl AsRef<str>) -> RhiVertexBuffer<Vertex3D> {
        let mut vertex_buffer = RhiVertexBuffer::new(rhi, data.len(), name.as_ref());
        vertex_buffer.transfer_data_sync(rhi, data);

        vertex_buffer
    }
}
