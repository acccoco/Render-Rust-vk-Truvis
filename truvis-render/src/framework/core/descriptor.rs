use std::{cell::OnceCell, sync::OnceLock};

use ash::vk;
use itertools::Itertools;

use crate::framework::rhi::Rhi;

pub trait RHiDescriptorBindings
{
    fn bindings() -> Vec<vk::DescriptorSetLayoutBinding>;
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
    fn get_bindings() -> &'static [vk::DescriptorSetLayoutBinding]
    {
        unsafe {
            static mut BINDINGS: Vec<vk::DescriptorSetLayoutBinding> = vec![];
            if BINDINGS.is_empty() {
                BINDINGS = T::bindings();
            }
            &BINDINGS
        }
    }

    fn create_layout(rhi: &Rhi) -> vk::DescriptorSetLayout
    {
        let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(Self::get_bindings());

        unsafe { rhi.device().create_descriptor_set_layout(&create_info, None).unwrap() }
    }

    /// 可以确保每种类型的 layout 在内存中只有 1 份
    fn get_layout(rhi: &Rhi) -> vk::DescriptorSetLayout
    {
        static LAYOUT: OnceLock<vk::DescriptorSetLayout> = OnceLock::new();
        *LAYOUT.get_or_init(|| Self::create_layout(rhi))
    }
}


struct RhiDescriptorSet
{
    descriptor_set: vk::DescriptorSet,
    bindings: &'static [vk::DescriptorSetLayoutBinding],
    rhi: &'static Rhi,
}
enum RhiDescriptorUpdateInfo
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
        let layout = [RhiDescriptorLayout::<T>::get_layout(rhi)];
        unsafe {
            let alloc_info =
                vk::DescriptorSetAllocateInfo::builder().descriptor_pool(rhi.descriptor_pool()).set_layouts(&layout);
            let descriptor_set = rhi.device().allocate_descriptor_sets(&alloc_info).unwrap()[0];
            Self {
                descriptor_set,
                bindings: RhiDescriptorLayout::<T>::get_bindings(),
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
                let mut write = vk::WriteDescriptorSet::builder()
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

                write.build()
            })
            .collect_vec();

        unsafe {
            self.rhi.device().update_descriptor_sets(&writes, &[]);
        }
        //
    }
}

pub struct RayTracingBindings;
impl RHiDescriptorBindings for RayTracingBindings
{
    fn bindings() -> Vec<vk::DescriptorSetLayoutBinding>
    {
        vec![
            vk::DescriptorSetLayoutBinding {
                // TLAS
                binding: 0,
                descriptor_type: vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::RAYGEN_KHR,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                // Output image
                binding: 0,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::RAYGEN_KHR,
                ..Default::default()
            },
        ]
    }
}
