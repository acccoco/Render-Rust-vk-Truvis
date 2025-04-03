use ash::vk;


/// # NOTE
/// 由于使用了 macro，因此不允许 combined image sampler，只能够使用单独的 sampler
pub trait ShaderBindingLayout
{
    fn get_shader_bindings() -> Vec<(&'static str, u32)>;
    fn get_bindings() -> Vec<vk::DescriptorSetLayoutBinding<'static>>
    {
        let bindings = Self::get_shader_bindings();
        bindings
            .iter()
            .map(|(name, binding)| vk::DescriptorSetLayoutBinding {
                binding: *binding,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::ALL,
                ..Default::default()
            })
            .collect()
    }
}
