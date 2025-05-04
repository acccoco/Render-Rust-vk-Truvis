use truvis_rhi::core::buffer::RhiBuffer;

pub struct SimpleMesh {
    pub vertex_buffer: RhiBuffer,
    pub index_buffer: RhiBuffer,
    pub index_cnt: u32,
}
