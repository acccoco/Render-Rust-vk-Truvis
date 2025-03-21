//! 无光照材质系统


use ash::vk;

use crate::framework::core::descriptor::DescriptorBindings;

struct UnlitMatBindings;

impl DescriptorBindings for UnlitMatBindings
{
    // FIXME
    fn bindings() -> Vec<vk::DescriptorSetLayoutBinding<'static>>
    {
        vec![
            // color texture
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
            // material params
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ]
    }
}


#[repr(C)]
struct UnlitMat {}
