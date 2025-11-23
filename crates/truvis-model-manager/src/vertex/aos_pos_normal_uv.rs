use ash::vk;
use ash::vk::DeviceSize;
use std::mem::offset_of;
use truvis_gfx::resources::special_buffers::vertex_buffer::{GfxVertexBuffer, GfxVertexLayout};

/// AoS: Array of structures
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexPosNormalUv {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

pub struct VertexLayoutAoSPosNormalUv;

impl GfxVertexLayout for VertexLayoutAoSPosNormalUv {
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

    fn buffer_size(vertex_cnt: usize) -> usize {
        vertex_cnt * size_of::<VertexPosNormalUv>()
    }

    fn pos_stride() -> u32 {
        size_of::<VertexPosNormalUv>() as _
    }

    fn pos_offset(_vertex_cnt: usize) -> DeviceSize {
        offset_of!(VertexPosNormalUv, pos) as _
    }
}

impl VertexLayoutAoSPosNormalUv {
    #[deprecated]
    pub fn create_vertex_buffer2(data: &[VertexPosNormalUv], name: impl AsRef<str>) -> GfxVertexBuffer<Self> {
        let vertex_buffer = GfxVertexBuffer::new(data.len(), name.as_ref());
        vertex_buffer.transfer_data_sync(data);

        vertex_buffer
    }
}
