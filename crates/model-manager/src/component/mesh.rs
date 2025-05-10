use ash::vk;
use truvis_rhi::core::buffer::RhiBuffer;

pub struct SimpleMesh {
    pub vertex_buffer: RhiBuffer,
    pub index_buffer: RhiBuffer,
    pub index_cnt: u32,
}

impl SimpleMesh {
    #[inline]
    pub fn index_type() -> vk::IndexType {
        vk::IndexType::UINT32
    }
}
