use crate::gui::gui_vertex_layout::ImGuiVertexLayoutAoS;
use ash::vk;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::resources::handles::{IndexBufferHandle, VertexBufferHandle};
use truvis_gfx::resources::resource_data::BufferType;
use truvis_gfx::{
    basic::color::LabelColor,
    commands::{barrier::GfxBufferBarrier, command_buffer::GfxCommandBuffer},
};

/// imgui 绘制所需的 vertex buffer 和 index buffer
pub struct GuiMesh {
    pub vertex_buffer: VertexBufferHandle<ImGuiVertexLayoutAoS>,
    _vertex_count: usize,
    pub index_buffer: IndexBufferHandle,
}

impl GuiMesh {
    pub fn new(cmd: &GfxCommandBuffer, frame_name: &str, draw_data: &imgui::DrawData) -> Self {
        let (vertex_buffer, vertex_cnt) = Self::create_vertex_buffer(frame_name, cmd, draw_data);
        let index_buffer = Self::create_index_buffer(frame_name, cmd, draw_data);

        let rm = Gfx::get().resource_manager();
        let v_buffer = rm.get_vertex_buffer(vertex_buffer).unwrap().buffer;
        let i_buffer = rm.get_index_buffer(index_buffer).unwrap().buffer;

        cmd.begin_label("uipass-mesh-transfer-barrier", LabelColor::COLOR_CMD);
        {
            cmd.buffer_memory_barrier(
                vk::DependencyFlags::empty(),
                &[
                    GfxBufferBarrier::default()
                        .src_mask(vk::PipelineStageFlags2::TRANSFER, vk::AccessFlags2::TRANSFER_WRITE)
                        .dst_mask(vk::PipelineStageFlags2::INDEX_INPUT, vk::AccessFlags2::INDEX_READ)
                        .buffer(i_buffer, 0, vk::WHOLE_SIZE),
                    GfxBufferBarrier::default()
                        .src_mask(vk::PipelineStageFlags2::TRANSFER, vk::AccessFlags2::TRANSFER_WRITE)
                        .dst_mask(vk::PipelineStageFlags2::VERTEX_INPUT, vk::AccessFlags2::VERTEX_ATTRIBUTE_READ)
                        .buffer(v_buffer, 0, vk::WHOLE_SIZE),
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
        frame_name: &str,
        cmd: &GfxCommandBuffer,
        draw_data: &imgui::DrawData,
    ) -> (VertexBufferHandle<ImGuiVertexLayoutAoS>, usize) {
        let vertex_count = draw_data.total_vtx_count as usize;
        let mut vertices = Vec::with_capacity(vertex_count);
        for draw_list in draw_data.draw_lists() {
            vertices.extend_from_slice(draw_list.vtx_buffer());
        }

        let vertices_size = vertex_count * size_of::<imgui::DrawVert>();

        let mut rm = Gfx::get().resource_manager();
        let vertex_buffer =
            rm.create_vertex_buffer::<ImGuiVertexLayoutAoS>(vertex_count, format!("{}-imgui-vertex", frame_name));

        let stage_buffer_handle = rm.create_buffer(
            vertices_size as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            true,
            BufferType::Stage,
            format!("{}-imgui-vertex-stage", frame_name),
        );

        {
            let stage_buffer = rm.get_buffer_mut(stage_buffer_handle).unwrap();
            if let Some(ptr) = stage_buffer.mapped_ptr {
                unsafe {
                    std::ptr::copy_nonoverlapping(vertices.as_ptr(), ptr as *mut imgui::DrawVert, vertices.len());
                }
            }
        }

        let src_buffer = rm.get_buffer(stage_buffer_handle).unwrap().buffer;
        let dst_buffer = rm.get_vertex_buffer(vertex_buffer).unwrap().buffer;

        cmd.begin_label("uipass-vertex-buffer-transfer", LabelColor::COLOR_CMD);
        unsafe {
            Gfx::get().gfx_device().cmd_copy_buffer(
                cmd.vk_handle(),
                src_buffer,
                dst_buffer,
                &[vk::BufferCopy {
                    size: vertices_size as vk::DeviceSize,
                    ..Default::default()
                }],
            );
        }
        cmd.end_label();

        rm.destroy_buffer_immediate(stage_buffer_handle);

        (vertex_buffer, vertex_count)
    }

    /// 从 draw data 中提取出 index 数据，创建 index buffer
    ///
    /// @return (index buffer, index count, stage buffer)
    fn create_index_buffer(frame_name: &str, cmd: &GfxCommandBuffer, draw_data: &imgui::DrawData) -> IndexBufferHandle {
        let index_count = draw_data.total_idx_count as usize;
        let mut indices = Vec::with_capacity(index_count);
        for draw_list in draw_data.draw_lists() {
            indices.extend_from_slice(draw_list.idx_buffer());
        }

        let index_buffer_size = index_count * size_of::<imgui::DrawIdx>();

        let mut rm = Gfx::get().resource_manager();
        let index_buffer = rm.create_index_buffer::<imgui::DrawIdx>(index_count, format!("{}-imgui-index", frame_name));

        let stage_buffer_handle = rm.create_buffer(
            index_buffer_size as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            true,
            BufferType::Stage,
            format!("{}-imgui-index-stage", frame_name),
        );

        {
            let stage_buffer = rm.get_buffer_mut(stage_buffer_handle).unwrap();
            if let Some(ptr) = stage_buffer.mapped_ptr {
                unsafe {
                    std::ptr::copy_nonoverlapping(indices.as_ptr(), ptr as *mut imgui::DrawIdx, indices.len());
                }
            }
        }

        let src_buffer = rm.get_buffer(stage_buffer_handle).unwrap().buffer;
        let dst_buffer = rm.get_index_buffer(index_buffer).unwrap().buffer;

        cmd.begin_label("uipass-index-buffer-transfer", LabelColor::COLOR_CMD);
        unsafe {
            Gfx::get().gfx_device().cmd_copy_buffer(
                cmd.vk_handle(),
                src_buffer,
                dst_buffer,
                &[vk::BufferCopy {
                    size: index_buffer_size as vk::DeviceSize,
                    ..Default::default()
                }],
            );
        }
        cmd.end_label();

        rm.destroy_buffer_immediate(stage_buffer_handle);

        index_buffer
    }
}
