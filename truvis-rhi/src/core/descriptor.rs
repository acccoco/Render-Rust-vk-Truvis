use std::rc::Rc;

use super::device::RhiDevice;
use crate::core::descriptor_pool::RhiDescriptorPool;
use crate::rhi::Rhi;
use ash::vk;
use shader_layout_trait::ShaderBindingLayout;

/// 描述符集布局
///
/// 描述符集布局定义了描述符集的结构，包括：
/// - 绑定的数量
/// - 每个绑定的类型
/// - 每个绑定的着色器阶段
///
/// 使用泛型参数 T 来关联具体的绑定布局类型，
/// 这样可以保证类型安全，并且可以在编译时检查布局的正确性。
///
/// # 泛型参数
/// - T: 实现了 ShaderBindingLayout trait 的类型，定义了具体的绑定布局
pub struct RhiDescriptorSetLayout<T>
where
    T: ShaderBindingLayout,
{
    /// Vulkan 描述符集布局句柄
    pub layout: vk::DescriptorSetLayout,
    /// 用于在编译时关联泛型参数 T
    phantom_data: std::marker::PhantomData<T>,

    _device: Rc<RhiDevice>,
}

impl<T> RhiDescriptorSetLayout<T>
where
    T: ShaderBindingLayout,
{
    /// 创建新的描述符集布局
    ///
    /// # 参数
    /// - rhi: RHI 实例
    /// - debug_name: 用于调试的名称
    ///
    /// # 返回值
    /// 新的描述符集布局实例
    pub fn new(rhi: &Rhi, flags: vk::DescriptorSetLayoutCreateFlags, debug_name: &str) -> Self {
        // 从类型 T 获取绑定信息
        let (bindings, binding_flags) = T::get_vk_bindings();
        let mut bind_flags_ci = vk::DescriptorSetLayoutBindingFlagsCreateInfo::default().binding_flags(&binding_flags);

        let create_info =
            vk::DescriptorSetLayoutCreateInfo::default().flags(flags).bindings(&bindings).push_next(&mut bind_flags_ci);
        vk::DescriptorBindingFlags::empty();

        // 创建 Vulkan 描述符集布局
        let layout = unsafe { rhi.device().create_descriptor_set_layout(&create_info, None).unwrap() };
        rhi.device.debug_utils.set_object_debug_name(layout, debug_name);
        Self {
            layout,
            phantom_data: std::marker::PhantomData,
            _device: rhi.device.clone(),
        }
    }
}

/// 描述符集
///
/// 描述符集是描述符的集合，用于在着色器中访问资源。
/// 每个描述符集都关联一个描述符集布局，定义了其结构。
///
/// # 泛型参数
/// - T: 实现了 ShaderBindingLayout trait 的类型，定义了具体的绑定布局
pub struct RhiDescriptorSet<T>
where
    T: ShaderBindingLayout,
{
    /// Vulkan 描述符集句柄
    pub handle: vk::DescriptorSet,
    /// 用于在编译时关联泛型参数 T
    phantom_data: std::marker::PhantomData<T>,
}

/// 描述符更新信息
///
/// 用于更新描述符集的内容，可以是：
/// - 图像描述符：用于纹理和采样器
/// - 缓冲区描述符：用于统一缓冲区和存储缓冲区
pub enum RhiDescriptorUpdateInfo {
    /// 图像描述符信息
    Image(vk::DescriptorImageInfo),
    /// 缓冲区描述符信息
    Buffer(vk::DescriptorBufferInfo),
}

impl<T> RhiDescriptorSet<T>
where
    T: ShaderBindingLayout,
{
    /// 创建新的描述符集
    ///
    /// # 参数
    /// - rhi: RHI 实例
    /// - layout: 描述符集布局
    /// - debug_name: 用于调试的名称
    ///
    /// # 返回值
    /// 新的描述符集实例
    pub fn new(
        rhi: &Rhi,
        descriptor_pool: &RhiDescriptorPool,
        layout: &RhiDescriptorSetLayout<T>,
        debug_name: &str,
    ) -> Self {
        // 分配描述符集
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool.handle())
            .set_layouts(std::slice::from_ref(&layout.layout));
        let descriptor_set = unsafe { rhi.device.allocate_descriptor_sets(&alloc_info).unwrap()[0] };
        rhi.device.debug_utils.set_object_debug_name(descriptor_set, debug_name);
        Self {
            handle: descriptor_set,
            phantom_data: std::marker::PhantomData,
        }
    }
}
