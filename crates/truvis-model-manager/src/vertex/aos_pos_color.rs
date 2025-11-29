use ash::vk;
use ash::vk::DeviceSize;
use std::mem::offset_of;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::resources::handles::VertexBufferHandle;
use truvis_gfx::resources::layout::GfxVertexLayout;
use truvis_gfx::resources::resource_data::BufferType;

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
    pub fn create_vertex_buffer(data: &[VertexPosColor], name: impl AsRef<str>) -> VertexBufferHandle<Self> {
        let mut rm = Gfx::get().resource_manager();
        let vertex_buffer_handle = rm.create_vertex_buffer::<Self>(data.len(), name.as_ref());

        // Upload data
        let stage_buffer_handle = rm.create_buffer(
            std::mem::size_of_val(data) as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            true,
            BufferType::Stage,
            format!("{}-stage", name.as_ref()),
        );

        {
            let stage_buffer = rm.get_buffer_mut(stage_buffer_handle).unwrap();
            if let Some(ptr) = stage_buffer.mapped_ptr {
                unsafe {
                    std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut VertexPosColor, data.len());
                }
            }
        }

        let src_buffer = rm.get_buffer(stage_buffer_handle).unwrap().buffer;
        let dst_buffer = rm.get_vertex_buffer(vertex_buffer_handle).unwrap().buffer;

        Gfx::get().one_time_exec(
            |cmd| {
                let copy_region = vk::BufferCopy {
                    src_offset: 0,
                    dst_offset: 0,
                    size: std::mem::size_of_val(data) as u64,
                };
                unsafe {
                    Gfx::get().gfx_device().cmd_copy_buffer(cmd.vk_handle(), src_buffer, dst_buffer, &[copy_region]);
                }
            },
            "upload_vertex_buffer",
        );

        rm.destroy_buffer_immediate(stage_buffer_handle);

        vertex_buffer_handle
    }
}
