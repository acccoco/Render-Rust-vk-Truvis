use ash::vk;

pub trait RHiDescriptorBindings
{
    fn get_bindings() -> &'static [vk::DescriptorSetLayoutBinding]
    {
        unsafe {
            static mut BINDINGS: Vec<vk::DescriptorSetLayoutBinding> = vec![];
            if BINDINGS.is_empty() {
                BINDINGS = Self::bindings();
            }
            &BINDINGS
        }
    }

    fn bindings() -> Vec<vk::DescriptorSetLayoutBinding>;
}

pub struct RayTracingBindings;
impl RHiDescriptorBindings for RayTracingBindings
{
    fn bindings() -> Vec<vk::DescriptorSetLayoutBinding>
    {
        vec![
            vk::DescriptorSetLayoutBinding {    // TLAS
                binding: 0,
                descriptor_type: vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::RAYGEN_KHR,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {    // Output image
                binding: 0,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::RAYGEN_KHR,
                ..Default::default()
            },
        ]
    }
}
