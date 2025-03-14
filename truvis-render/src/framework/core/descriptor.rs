use ash::vk;

use crate::framework::render_core::Core;

/// 将 descriptor set layout 的 bindings 抽象为一个 trait，通过类型系统来保证 bindings 的正确性
pub trait DescriptorBindings
{
    // FIXME 这个声明周期还是感觉不太安全
    fn bindings() -> Vec<vk::DescriptorSetLayoutBinding<'static>>;
}

/// brief：
///
/// 注：为什么要使用 <T>
/// 这样可以在每种 struct 内部存放一个 static 的 DescriptorSetLayout
pub struct DescriptorSetLayout<T>
where
    T: DescriptorBindings,
{
    pub layout: vk::DescriptorSetLayout,
    phantom_data: std::marker::PhantomData<T>,
}
impl<T> DescriptorSetLayout<T>
where
    T: DescriptorBindings,
{
    pub fn new(rhi: &Core, debug_name: &str) -> Self
    {
        let bindings = T::bindings();
        let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

        let layout = unsafe { rhi.vk_device().create_descriptor_set_layout(&create_info, None).unwrap() };
        rhi.set_debug_name(layout, debug_name);
        Self {
            layout,
            phantom_data: std::marker::PhantomData,
        }
    }
}


pub struct DescriptorSet<T>
where
    T: DescriptorBindings,
{
    pub descriptor_set: vk::DescriptorSet,
    phantom_data: std::marker::PhantomData<T>,
}
pub enum RhiDescriptorUpdateInfo
{
    Image(vk::DescriptorImageInfo),
    Buffer(vk::DescriptorBufferInfo),
}
impl<T> DescriptorSet<T>
where
    T: DescriptorBindings,
{
    pub fn new(rhi: &Core, layout: &DescriptorSetLayout<T>, debug_name: &str) -> Self
    {
        unsafe {
            let alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(rhi.descriptor_pool)
                .set_layouts(std::slice::from_ref(&layout.layout));
            let descriptor_set = rhi.vk_device().allocate_descriptor_sets(&alloc_info).unwrap()[0];
            rhi.set_debug_name(descriptor_set, debug_name);
            Self {
                descriptor_set,
                phantom_data: std::marker::PhantomData,
            }
        }
    }
}
