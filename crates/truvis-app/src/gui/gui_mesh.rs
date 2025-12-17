use ash::vk;
use imgui::DrawData;
use crate::gui::gui_vertex_layout::ImGuiVertexLayoutAoS;
use truvis_gfx::resources::buffer::GfxBuffer;
use truvis_gfx::resources::special_buffers::index_buffer::GfxIndexBuffer;
use truvis_gfx::resources::special_buffers::vertex_buffer::GfxVertexBuffer;
use truvis_gfx::{
    basic::color::LabelColor,
    commands::{barrier::GfxBufferBarrier, command_buffer::GfxCommandBuffer},
};
use truvis_render_base::pipeline_settings::FrameLabel;
use truvis_render_graph::render_context::{RenderContext, RenderContextMut};

/// imgui 绘制所需的 vertex buffer 和 index buffer
pub struct GuiMesh {
    pub vertex_buffer: GfxVertexBuffer<ImGuiVertexLayoutAoS>,
    pub vertex_count: usize,

    pub index_buffer: GfxIndexBuffer<imgui::DrawIdx>,
    pub index_count: usize,
}

impl GuiMesh {
    pub fn new(frame_label: FrameLabel) -> Self {
        // 初始大小为 64KB
        let vertex_count = 64 * 1024 / size_of::<ImGuiVertexLayoutAoS>();
        // 初始大小为 96KB
        let index_count = 96 * 1024 / size_of::<imgui::DrawIdx>();

        Self {
            vertex_count,
            index_count,
            vertex_buffer: Self::new_vertex_buffer(frame_label, vertex_count),
            index_buffer: Self::new_index_buffer(frame_label, index_count),
        }
    }

    fn new_vertex_buffer(frame_label: FrameLabel, vertex_cnt: usize) -> GfxVertexBuffer<ImGuiVertexLayoutAoS> {
        GfxVertexBuffer::<ImGuiVertexLayoutAoS>::new(vertex_cnt, true, format!("imgui-vertex-{}", frame_label))
    }

    fn new_index_buffer(frame_label: FrameLabel, index_cnt: usize) -> GfxIndexBuffer<imgui::DrawIdx> {
        GfxIndexBuffer::<imgui::DrawIdx>::new(index_cnt, true, format!("imgui-index-{}", frame_label))
    }

    pub fn grow_if_needed(&mut self, frame_label: FrameLabel, draw_data: &imgui::DrawData) {
        if (draw_data.total_vtx_count as usize) > self.vertex_count {
            self.vertex_count = (draw_data.total_vtx_count as usize).next_power_of_two();
            self.vertex_buffer = Self::new_vertex_buffer(frame_label, self.vertex_count);
        }

        if (draw_data.total_idx_count as usize) > self.index_count {
            self.index_count = (draw_data.total_idx_count as usize).next_power_of_two();
            self.index_buffer = Self::new_index_buffer(frame_label, self.index_count);
        }
    }

    pub fn fill_vertex_buffer(&mut self, draw_data: &DrawData) {

    }

    pub fn new_2(
        render_context: &RenderContext,
        render_context_mut: &mut RenderContextMut,
        cmd: &GfxCommandBuffer,
        frame_name: &str,
        draw_data: &imgui::DrawData,
    ) -> Self {
        let (vertex_buffer, vertex_cnt) =
            Self::create_vertex_buffer(render_context, render_context_mut, frame_name, cmd, draw_data);
        let index_buffer = Self::create_index_buffer(render_context, render_context_mut, frame_name, cmd, draw_data);

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
            vertex_count: vertex_cnt,

            index_buffer,
        }
    }

    /// 从 draw data 中提取出 vertex 数据，创建 vertex buffer
    ///
    /// ## Return
    /// `(vertex buffer, vertex count)`
    fn create_vertex_buffer(
        render_context: &RenderContext,
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
        let vertex_buffer = GfxVertexBuffer::<ImGuiVertexLayoutAoS>::new_device_local(
            vertex_count,
            format!("{}-imgui-vertex", frame_name),
        );
        let stage_buffer = render_context_mut.stage_buffer_manager.alloc_buffer(
            &render_context.frame_counter,
            vertices_size as vk::DeviceSize,
            &format!("{}-imgui-vertex-stage", frame_name),
        );
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
        render_context: &RenderContext,
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

        let index_buffer =
            GfxIndexBuffer::<imgui::DrawIdx>::new_device_local(index_count, format!("{}-imgui-index", frame_name));
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

        render_context_mut.stage_buffer_manager.register_stage_buffer(&render_context.frame_counter, stage_buffer);

        index_buffer
    }
}
