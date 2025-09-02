use ash::vk;
use shader_layout_trait::ShaderBindingItem;

pub struct WriteDescriptorSet {
    pub dst_set: vk::DescriptorSet,
    pub dst_binding: u32,
    pub dst_array_element: u32,
    pub descriptor_type: vk::DescriptorType,

    pub buffers: Vec<vk::DescriptorBufferInfo>,
    pub images: Vec<vk::DescriptorImageInfo>,
}

impl WriteDescriptorSet {
    pub fn to_vk_type(&self) -> vk::WriteDescriptorSet<'_> {
        vk::WriteDescriptorSet {
            dst_set: self.dst_set,
            dst_binding: self.dst_binding,
            dst_array_element: self.dst_array_element,
            descriptor_count: usize::max(self.buffers.len(), self.images.len()) as u32,
            descriptor_type: self.descriptor_type,
            // 选择 buffer ptr 还是 image ptr，是由 descriptor type 控制的
            p_buffer_info: self.buffers.as_ptr(),
            p_image_info: self.images.as_ptr(),
            ..Default::default()
        }
    }
}

/// 用于通过 DescriptorBinding Item 来操作对应 descriptor set 的对应 binding
pub trait ShaderCursor {
    fn get_binding(&self) -> &ShaderBindingItem;

    /// 确保当前 descriptor 是 buffer
    fn write_buffer(
        &self,
        dst_set: vk::DescriptorSet,
        start_array: u32,
        buffers: Vec<vk::DescriptorBufferInfo>,
    ) -> WriteDescriptorSet {
        let item = self.get_binding();
        WriteDescriptorSet {
            dst_set,
            dst_binding: item.binding,
            dst_array_element: start_array,
            buffers,
            descriptor_type: item.descriptor_type,
            images: vec![],
        }
    }

    /// 确保当前 descriptor 是 image
    fn write_image(
        &self,
        dst_set: vk::DescriptorSet,
        start_array: u32,
        images: Vec<vk::DescriptorImageInfo>,
    ) -> WriteDescriptorSet {
        let item = self.get_binding();
        WriteDescriptorSet {
            dst_set,
            dst_binding: item.binding,
            dst_array_element: start_array,
            descriptor_type: item.descriptor_type,
            buffers: vec![],
            images,
        }
    }
}

impl ShaderCursor for ShaderBindingItem {
    fn get_binding(&self) -> &ShaderBindingItem {
        self
    }
}
