use ash::vk;

pub struct ImageNode {
    pub stage: vk::PipelineStageFlags2,
    pub access: vk::AccessFlags2,
    pub layout: vk::ImageLayout,
}

impl ImageNode {
    #[inline]
    /// 作为 src access 时，需要去掉 READ
    pub fn src_access(&self) -> vk::AccessFlags2 {
        self.access & !(vk::AccessFlags2::SHADER_READ)
    }

    #[inline]
    pub fn dst_access(&self) -> vk::AccessFlags2 {
        self.access
    }
}
