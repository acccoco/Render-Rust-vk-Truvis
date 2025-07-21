use ash::vk;
use std::mem::offset_of;
use truvis_rhi::basic::color::LabelColor;
use truvis_rhi::core::buffer::{RhiBuffer, RhiIndexBuffer, RhiVertexBuffer};
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::synchronize::RhiBufferBarrier;
use truvis_rhi::rhi::Rhi;

/// AoS: Array of Structs
pub struct ImGuiVertex {
    pos: glam::Vec2,
    uv: glam::Vec2,
    color: u32, // R8G8B8A8
}

impl ImGuiVertex {
    pub fn vertex_input_bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<ImGuiVertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    pub fn vertex_input_attributes() -> Vec<vk::VertexInputAttributeDescription> {
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
}

/// imgui 绘制所需的 vertex buffer 和 index buffer
pub struct GuiMesh {
    pub vertex_buffer: RhiVertexBuffer<imgui::DrawVert>,
    _vertex_count: usize,
    _vertex_stage_buffer: RhiBuffer,

    pub _index_buffer: RhiIndexBuffer,
    _index_count: usize,
    _index_stage_buffer: RhiBuffer,
}

impl GuiMesh {
    pub fn new(rhi: &Rhi, cmd: &RhiCommandBuffer, frame_name: &str, draw_data: &imgui::DrawData) -> Self {
        let (vertex_buffer, vertex_cnt, vertex_stage_buffer) =
            Self::create_vertex_buffer(rhi, frame_name, cmd, draw_data);
        let (index_buffer, index_cnt, index_stage_buffer) = Self::create_index_buffer(rhi, frame_name, cmd, draw_data);

        cmd.begin_label("uipass-mesh-transfer-barrier", LabelColor::COLOR_CMD);
        {
            cmd.buffer_memory_barrier(
                vk::DependencyFlags::empty(),
                &[
                    RhiBufferBarrier::default()
                        .src_mask(vk::PipelineStageFlags2::TRANSFER, vk::AccessFlags2::TRANSFER_WRITE)
                        .dst_mask(vk::PipelineStageFlags2::INDEX_INPUT, vk::AccessFlags2::INDEX_READ)
                        .buffer(index_buffer.handle(), 0, vk::WHOLE_SIZE),
                    RhiBufferBarrier::default()
                        .src_mask(vk::PipelineStageFlags2::TRANSFER, vk::AccessFlags2::TRANSFER_WRITE)
                        .dst_mask(vk::PipelineStageFlags2::VERTEX_INPUT, vk::AccessFlags2::VERTEX_ATTRIBUTE_READ)
                        .buffer(vertex_buffer.handle(), 0, vk::WHOLE_SIZE),
                ],
            );
        }
        cmd.end_label();

        Self {
            vertex_buffer,
            _vertex_count: vertex_cnt,
            _vertex_stage_buffer: vertex_stage_buffer,

            _index_buffer: index_buffer,
            _index_count: index_cnt,
            _index_stage_buffer: index_stage_buffer,
        }
    }

    /// 从 draw data 中提取出 vertex 数据，创建 vertex buffer
    ///
    /// @return (vertex buffer, vertex count, stage buffer)
    fn create_vertex_buffer(
        rhi: &Rhi,
        frame_name: &str,
        cmd: &RhiCommandBuffer,
        draw_data: &imgui::DrawData,
    ) -> (RhiVertexBuffer<imgui::DrawVert>, usize, RhiBuffer) {
        let vertex_count = draw_data.total_vtx_count as usize;
        let mut vertices = Vec::with_capacity(vertex_count);
        for draw_list in draw_data.draw_lists() {
            vertices.extend_from_slice(draw_list.vtx_buffer());
        }

        let vertices_size = vertex_count * size_of::<imgui::DrawVert>();
        let mut vertex_buffer =
            RhiVertexBuffer::<imgui::DrawVert>::new(rhi, vertex_count, format!("{}-imgui-vertex", frame_name));
        let mut stage_buffer = RhiBuffer::new_stage_buffer(
            rhi,
            vertices_size as vk::DeviceSize,
            format!("{}-imgui-vertex-stage", frame_name),
        );
        stage_buffer.transfer_data_by_mem_map(&vertices);

        cmd.begin_label("uipass-vertex-buffer-transfer", LabelColor::COLOR_CMD);
        {
            cmd.cmd_copy_buffer(
                &stage_buffer,
                &mut vertex_buffer,
                &[vk::BufferCopy {
                    size: vertices_size as vk::DeviceSize,
                    ..Default::default()
                }],
            );
        }
        cmd.end_label();

        (vertex_buffer, vertex_count, stage_buffer)
    }

    /// 从 draw data 中提取出 index 数据，创建 index buffer
    ///
    /// @return (index buffer, index count, stage buffer)
    fn create_index_buffer(
        rhi: &Rhi,
        frame_name: &str,
        cmd: &RhiCommandBuffer,
        draw_data: &imgui::DrawData,
    ) -> (RhiIndexBuffer, usize, RhiBuffer) {
        let index_count = draw_data.total_idx_count as usize;
        let mut indices = Vec::with_capacity(index_count);
        for draw_list in draw_data.draw_lists() {
            indices.extend_from_slice(draw_list.idx_buffer());
        }

        let indices_size = index_count * size_of::<imgui::DrawIdx>();
        let mut index_buffer = RhiIndexBuffer::new(rhi, indices_size, format!("{}-imgui-index", frame_name));
        let mut stage_buffer = RhiBuffer::new_stage_buffer(
            rhi,
            indices_size as vk::DeviceSize,
            format!("{}-imgui-index-stage", frame_name),
        );
        stage_buffer.transfer_data_by_mem_map(&indices);

        cmd.begin_label("uipass-index-buffer-transfer", LabelColor::COLOR_CMD);
        {
            cmd.cmd_copy_buffer(
                &stage_buffer,
                &mut index_buffer,
                &[vk::BufferCopy {
                    size: indices_size as vk::DeviceSize,
                    ..Default::default()
                }],
            );
        }
        cmd.end_label();

        (index_buffer, index_count, stage_buffer)
    }
}
