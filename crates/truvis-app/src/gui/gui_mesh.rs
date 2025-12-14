use ash::vk;

use crate::gui::gui_vertex_layout::ImGuiVertexLayoutAoS;
use truvis_gfx::resources::buffer::GfxBuffer;
use truvis_gfx::resources::special_buffers::index_buffer::GfxIndexBuffer;
use truvis_gfx::resources::special_buffers::vertex_buffer::GfxVertexBuffer;
use truvis_gfx::{
    basic::color::LabelColor,
    commands::{barrier::GfxBufferBarrier, command_buffer::GfxCommandBuffer},
};
use truvis_render::core::renderer::RenderContextMut;

/// imgui 绘制所需的 vertex buffer 和 index buffer
pub struct GuiMesh {
    pub vertex_buffer: GfxVertexBuffer<ImGuiVertexLayoutAoS>,
    _vertex_count: usize,

    pub index_buffer: GfxIndexBuffer<imgui::DrawIdx>,
}

impl GuiMesh {
    pub fn new(
        render_context_mut: &mut RenderContextMut,
        cmd: &GfxCommandBuffer,
        frame_name: &str,
        draw_data: &imgui::DrawData,
    ) -> Self {
        let (vertex_buffer, vertex_cnt) = Self::create_vertex_buffer(render_context_mut, frame_name, cmd, draw_data);
        let index_buffer = Self::create_index_buffer(render_context_mut, frame_name, cmd, draw_data);

        cmd.begin_label("uipass-mesh-transfer-barrier", LabelColor::COLOR_CMD);
        {
            cmd.buffer_memory_barrier(
                vk::DependencyFlags::empty(),
                &[
                    GfxBufferBarrier::default()
                        .src_mask(vk::PipelineStageFlags2::TRANSFER, vk::AccessFlags2::TRANSFER_WRITE)
                        .dst_mask(vk::PipelineStageFlags2::INDEX_INPUT, vk::AccessFlags2::INDEX_READ)
                        .buffer(index_buffer.vk_buffer(), 0, vk::WHOLE_SIZE),
                    GfxBufferBarrier::default()
                        .src_mask(vk::PipelineStageFlags2::TRANSFER, vk::AccessFlags2::TRANSFER_WRITE)
                        .dst_mask(vk::PipelineStageFlags2::VERTEX_INPUT, vk::AccessFlags2::VERTEX_ATTRIBUTE_READ)
                        .buffer(vertex_buffer.vk_buffer(), 0, vk::WHOLE_SIZE),
                ],
            );
        }
        cmd.end_label();

        Self {
            vertex_buffer,
            _vertex_count: vertex_cnt,

            index_buffer,
        }
    }

    /// 从 draw data 中提取出 vertex 数据，创建 vertex buffer
    ///
    /// ## Return
    /// `(vertex buffer, vertex count)`
    fn create_vertex_buffer(
        render_context_mut: &mut RenderContextMut,
        frame_name: &str,
        cmd: &GfxCommandBuffer,
        draw_data: &imgui::DrawData,
    ) -> (GfxVertexBuffer<ImGuiVertexLayoutAoS>, usize) {
        let vertex_count = draw_data.total_vtx_count as usize;
        let mut vertices = Vec::with_capacity(vertex_count);
        for draw_list in draw_data.draw_lists() {
            vertices.extend_from_slice(draw_list.vtx_buffer());
        }

        let vertices_size = vertex_count * size_of::<imgui::DrawVert>();
        let vertex_buffer =
            GfxVertexBuffer::<ImGuiVertexLayoutAoS>::new(vertex_count, format!("{}-imgui-vertex", frame_name));
        let stage_buffer = render_context_mut
            .stage_buffer_manager
            .alloc_buffer(vertices_size as vk::DeviceSize, &format!("{}-imgui-vertex-stage", frame_name));
        stage_buffer.transfer_data_by_mmap(&vertices);

        cmd.begin_label("uipass-vertex-buffer-transfer", LabelColor::COLOR_CMD);
        {
            cmd.cmd_copy_buffer(
                stage_buffer,
                &vertex_buffer,
                &[vk::BufferCopy {
                    size: vertices_size as vk::DeviceSize,
                    ..Default::default()
                }],
            );
        }
        cmd.end_label();

        (vertex_buffer, vertex_count)
    }

    /// 从 draw data 中提取出 index 数据，创建 index buffer
    ///
    /// # return
    /// (index buffer, index count, stage buffer)
    fn create_index_buffer(
        render_context_mut: &mut RenderContextMut,
        frame_name: &str,
        cmd: &GfxCommandBuffer,
        draw_data: &imgui::DrawData,
    ) -> GfxIndexBuffer<imgui::DrawIdx> {
        let index_count = draw_data.total_idx_count as usize;
        let mut indices = Vec::with_capacity(index_count);
        for draw_list in draw_data.draw_lists() {
            indices.extend_from_slice(draw_list.idx_buffer());
        }

        let index_buffer_size = index_count * size_of::<imgui::DrawIdx>();

        let index_buffer = GfxIndexBuffer::<imgui::DrawIdx>::new(index_count, format!("{}-imgui-index", frame_name));
        let stage_buffer = GfxBuffer::new_stage_buffer(
            index_buffer_size as vk::DeviceSize,
            format!("{}-imgui-index-stage", frame_name),
        );
        stage_buffer.transfer_data_by_mmap(&indices);

        cmd.begin_label("uipass-index-buffer-transfer", LabelColor::COLOR_CMD);
        {
            cmd.cmd_copy_buffer(
                &stage_buffer,
                &index_buffer,
                &[vk::BufferCopy {
                    size: index_buffer_size as vk::DeviceSize,
                    ..Default::default()
                }],
            );
        }
        cmd.end_label();

        render_context_mut.stage_buffer_manager.register_stage_buffer(stage_buffer);

        index_buffer
    }
}
