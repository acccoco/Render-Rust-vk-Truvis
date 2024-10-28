use ash::vk;


/// 表示一张物理显卡
pub struct RhiPhysicalDevice
{
    pub(crate) vk_pdevice: vk::PhysicalDevice,
    pub(crate) pd_props: vk::PhysicalDeviceProperties,
    pub(crate) pd_mem_props: vk::PhysicalDeviceMemoryProperties,
    pub(crate) pd_features: vk::PhysicalDeviceFeatures,
    pub(crate) pd_rt_pipeline_props: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
    pub(crate) queue_family_props: Vec<vk::QueueFamilyProperties>,
}

impl RhiPhysicalDevice
{
    pub(crate) fn new(pdevice: vk::PhysicalDevice, instance: &ash::Instance) -> Self
    {
        unsafe {
            let mut pd_rt_props = vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::default();
            let mut pd_props2 = vk::PhysicalDeviceProperties2::builder().push_next(&mut pd_rt_props);
            instance.get_physical_device_properties2(pdevice, &mut pd_props2);

            Self {
                pd_mem_props: instance.get_physical_device_memory_properties(pdevice),
                pd_features: instance.get_physical_device_features(pdevice),
                vk_pdevice: pdevice,
                pd_props: pd_props2.properties,
                pd_rt_pipeline_props: pd_rt_props,
                queue_family_props: instance.get_physical_device_queue_family_properties(pdevice),
            }
        }
    }

    #[inline]
    pub(crate) fn is_descrete_gpu(&self) -> bool
    {
        self.pd_props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
    }


    pub(crate) fn find_queue_family_index(&self, queue_flags: vk::QueueFlags) -> Option<u32>
    {
        self.queue_family_props
            .iter()
            .enumerate()
            .find(|(_, prop)| prop.queue_flags.contains(queue_flags))
            .map(|(index, _)| index as u32)
    }
}
