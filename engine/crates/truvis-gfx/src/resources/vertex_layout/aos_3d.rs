use crate::resources::layout::GfxVertexLayout;
use crate::resources::special_buffers::vertex_buffer::GfxVertexBuffer;
use ash::vk;
use std::mem::offset_of;

#[repr(C)]
#[derive(Clone, Debug, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex3D {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
    pub uv: [f32; 2],
}

/// AoS 的顶点 buffer 布局，包含：Positions, Normals, Tangents, UVs
pub struct VertexLayoutAoS3D;

impl GfxVertexLayout for VertexLayoutAoS3D {
    fn vertex_input_bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Vertex3D>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn vertex_input_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            // positions
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex3D, position) as u32,
            },
            // normals
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex3D, normal) as u32,
            },
            // tangents
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex3D, tangent) as u32,
            },
            // bitangents
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 3,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex3D, bitangent) as u32,
            },
            // uvs
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 4,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex3D, uv) as u32,
            },
        ]
    }

    fn buffer_size(vertex_cnt: usize) -> usize {
        vertex_cnt * size_of::<Vertex3D>()
    }

    fn pos_stride() -> u32 {
        size_of::<Vertex3D>() as u32
    }

    fn pos_offset(_vertex_cnt: usize) -> vk::DeviceSize {
        offset_of!(Vertex3D, position) as vk::DeviceSize
    }
}

impl VertexLayoutAoS3D {
    #[deprecated]
    pub fn create_vertex_buffer(data: &[Vertex3D], name: impl AsRef<str>) -> GfxVertexBuffer<Self> {
        let vertex_buffer = GfxVertexBuffer::new_device_local(data.len(), name.as_ref());
        vertex_buffer.transfer_data_sync(data);

        vertex_buffer
    }
}
