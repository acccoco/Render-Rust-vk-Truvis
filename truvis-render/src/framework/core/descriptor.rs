use ash::vk;
use itertools::Itertools;

use crate::framework::rhi::Rhi;

pub trait RHiDescriptorBindings
{
    // FIXME 这个声明周期还是感觉不太安全
    fn bindings() -> Vec<vk::DescriptorSetLayoutBinding<'static>>;
}

/// brief：
///
/// 注：为什么要使用 <T>
/// 这样可以在每种 struct 内部存放一个 static 的 DescriptorSetLayout
pub struct RhiDescriptorLayout<T>
where
    T: RHiDescriptorBindings,
{
    phantom_data: std::marker::PhantomData<T>,
}

impl<T> RhiDescriptorLayout<T>
where
    T: RHiDescriptorBindings,
{
    fn create_layout(rhi: &Rhi) -> vk::DescriptorSetLayout
    {
        let bindings = T::bindings();
        let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

        unsafe { rhi.vk_device().create_descriptor_set_layout(&create_info, None).unwrap() }
    }
}


pub struct RhiDescriptorSet
{
    descriptor_set: vk::DescriptorSet,
    // FIXME
    bindings: Vec<vk::DescriptorSetLayoutBinding<'static>>,
    rhi: &'static Rhi,
}


pub enum RhiDescriptorUpdateInfo
{
    Image(vk::DescriptorImageInfo),
    Buffer(vk::DescriptorBufferInfo),
}
impl RhiDescriptorSet
{
    pub fn new<T>(rhi: &'static Rhi) -> Self
    where
        T: RHiDescriptorBindings,
    {
        let layout = RhiDescriptorLayout::<T>::create_layout(rhi);
        unsafe {
            let alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(rhi.descriptor_pool)
                .set_layouts(std::slice::from_ref(&layout));
            let descriptor_set = rhi.vk_device().allocate_descriptor_sets(&alloc_info).unwrap()[0];
            Self {
                descriptor_set,
                bindings: T::bindings(),
                rhi,
            }
        }
    }

    pub fn get_descriptor_type(&self, binding_index: u32) -> vk::DescriptorType
    {
        self.bindings.get(binding_index as usize).unwrap().descriptor_type
    }

    pub fn write(&mut self, write_datas: Vec<(u32, RhiDescriptorUpdateInfo)>)
    {
        let writes = write_datas
            .iter()
            .map(|(binding_index, info)| {
                let mut write = vk::WriteDescriptorSet::default()
                    .dst_set(self.descriptor_set)
                    .dst_binding(*binding_index)
                    .dst_array_element(1)
                    .descriptor_type(self.bindings.get(*binding_index as usize).unwrap().descriptor_type);

                match info {
                    RhiDescriptorUpdateInfo::Buffer(info) => {
                        write = write.buffer_info(std::slice::from_ref(info));
                    }
                    RhiDescriptorUpdateInfo::Image(info) => {
                        write = write.image_info(std::slice::from_ref(info));
                    }
                }

                write
            })
            .collect_vec();

        unsafe {
            self.rhi.vk_device().update_descriptor_sets(&writes, &[]);
        }
        //
    }
}
