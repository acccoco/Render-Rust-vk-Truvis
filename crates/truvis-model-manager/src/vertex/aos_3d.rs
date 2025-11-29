use ash::vk;
use std::mem::offset_of;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::resources::handles::VertexBufferHandle;
use truvis_gfx::resources::layout::GfxVertexLayout;
use truvis_gfx::resources::resource_data::BufferType;

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
    pub fn create_vertex_buffer(data: &[Vertex3D], name: impl AsRef<str>) -> VertexBufferHandle<Self> {
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
                    std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut Vertex3D, data.len());
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
