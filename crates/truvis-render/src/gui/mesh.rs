use crate::gui::imgui_vertex_layout::ImGuiVertexLayoutAoS;
use crate::renderer::frame_context::FrameContext;
use ash::vk;
use truvis_rhi::resources::special_buffers::vertex_buffer::VertexBuffer;
use truvis_rhi::{
    basic::color::LabelColor,
    commands::{barrier::BufferBarrier, command_buffer::CommandBuffer},
    resources::special_buffers::index_buffer::IndexBuffer,
};

/// imgui 绘制所需的 vertex buffer 和 index buffer
pub struct GuiMesh {
    pub vertex_buffer: VertexBuffer<ImGuiVertexLayoutAoS>,
    _vertex_count: usize,

    pub _index_buffer: IndexBuffer,
    _index_count: usize,
}

impl GuiMesh {
    pub fn new(cmd: &CommandBuffer, frame_name: &str, draw_data: &imgui::DrawData) -> Self {
        let (vertex_buffer, vertex_cnt) = Self::create_vertex_buffer(frame_name, cmd, draw_data);
        let (index_buffer, index_cnt) = Self::create_index_buffer(frame_name, cmd, draw_data);

        cmd.begin_label("uipass-mesh-transfer-barrier", LabelColor::COLOR_CMD);
        {
            cmd.buffer_memory_barrier(
                vk::DependencyFlags::empty(),
                &[
                    BufferBarrier::default()
                        .src_mask(vk::PipelineStageFlags2::TRANSFER, vk::AccessFlags2::TRANSFER_WRITE)
                        .dst_mask(vk::PipelineStageFlags2::INDEX_INPUT, vk::AccessFlags2::INDEX_READ)
                        .buffer(index_buffer.handle(), 0, vk::WHOLE_SIZE),
                    BufferBarrier::default()
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

            _index_buffer: index_buffer,
            _index_count: index_cnt,
        }
    }

    /// 从 draw data 中提取出 vertex 数据，创建 vertex buffer
    ///
    /// ## Return
    /// `(vertex buffer, vertex count)`
    fn create_vertex_buffer(
        frame_name: &str,
        cmd: &CommandBuffer,
        draw_data: &imgui::DrawData,
    ) -> (VertexBuffer<ImGuiVertexLayoutAoS>, usize) {
        let vertex_count = draw_data.total_vtx_count as usize;
        let mut vertices = Vec::with_capacity(vertex_count);
        for draw_list in draw_data.draw_lists() {
            vertices.extend_from_slice(draw_list.vtx_buffer());
        }

        let vertices_size = vertex_count * size_of::<imgui::DrawVert>();
        let mut vertex_buffer =
            VertexBuffer::<ImGuiVertexLayoutAoS>::new(vertex_count, format!("{}-imgui-vertex", frame_name));
        let mut upload_buffer_mgr = FrameContext::upload_buffer_mgr_mut();
        let stage_buffer = upload_buffer_mgr
            .alloc_buffer(vertices_size as vk::DeviceSize, &format!("{}-imgui-vertex-stage", frame_name));
        stage_buffer.transfer_data_by_mmap(&vertices);

        cmd.begin_label("uipass-vertex-buffer-transfer", LabelColor::COLOR_CMD);
        {
            cmd.cmd_copy_buffer_1(
                &stage_buffer,
                &mut vertex_buffer,
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
    /// @return (index buffer, index count, stage buffer)
    fn create_index_buffer(frame_name: &str, cmd: &CommandBuffer, draw_data: &imgui::DrawData) -> (IndexBuffer, usize) {
        let index_count = draw_data.total_idx_count as usize;
        let mut indices = Vec::with_capacity(index_count);
        for draw_list in draw_data.draw_lists() {
            indices.extend_from_slice(draw_list.idx_buffer());
        }

        let indices_size = index_count * size_of::<imgui::DrawIdx>();
        let mut index_buffer = IndexBuffer::new(indices_size, format!("{}-imgui-index", frame_name));
        let mut upload_buffer_mgr = FrameContext::upload_buffer_mgr_mut();
        let stage_buffer = upload_buffer_mgr
            .alloc_buffer(indices_size as vk::DeviceSize, &format!("{}-imgui-index-stage", frame_name));
        stage_buffer.transfer_data_by_mmap(&indices);

        cmd.begin_label("uipass-index-buffer-transfer", LabelColor::COLOR_CMD);
        {
            cmd.cmd_copy_buffer_1(
                &stage_buffer,
                &mut index_buffer,
                &[vk::BufferCopy {
                    size: indices_size as vk::DeviceSize,
                    ..Default::default()
                }],
            );
        }
        cmd.end_label();

        (index_buffer, index_count)
    }
}
