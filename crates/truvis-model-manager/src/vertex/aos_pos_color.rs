use ash::vk;
use ash::vk::DeviceSize;
use std::mem::offset_of;
use truvis_gfx::resources::special_buffers::vertex_buffer::{GfxVertexBuffer, GfxVertexLayout};

#[repr(C)]
#[derive(Clone, Debug, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexPosColor {
    pos: [f32; 4],
    color: [f32; 4],
}

pub struct VertexLayoutAoSPosColor;

impl GfxVertexLayout for VertexLayoutAoSPosColor {
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
    #[deprecated]
    pub fn create_vertex_buffer2(data: &[VertexPosColor], name: impl AsRef<str>) -> GfxVertexBuffer<Self> {
        let vertex_buffer = GfxVertexBuffer::new(data.len(), name.as_ref());
        vertex_buffer.transfer_data_sync(data);

        vertex_buffer
    }
}
