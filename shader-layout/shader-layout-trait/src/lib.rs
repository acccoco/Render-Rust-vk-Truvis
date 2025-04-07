use ash::vk;

/// 描述符绑定的详细信息
///
/// 包含创建 Vulkan 描述符集布局所需的所有信息：
/// - 名称：用于调试和日志
/// - 绑定点：描述符在描述符集中的位置
/// - 描述符类型：如统一缓冲区、采样器等
/// - 着色器阶段：使用该描述符的着色器阶段
/// - 数量：描述符的数量
#[derive(Debug, Clone, Copy)]
pub struct ShaderBindingItem
{
    /// 绑定的名称，用于调试和日志
    pub name: &'static str,
    /// 绑定点编号，决定描述符在描述符集中的位置
    pub binding: u32,
    /// 描述符的类型，如 UNIFORM_BUFFER、COMBINED_IMAGE_SAMPLER 等
    pub descriptor_type: vk::DescriptorType,
    /// 使用该描述符的着色器阶段
    pub stage_flags: vk::ShaderStageFlags,
    /// 描述符的数量，通常为 1
    pub count: u32,
}

/// 着色器绑定布局 trait
///
/// 用于描述着色器需要的所有资源绑定。
/// 通过派生宏自动实现，不需要手动实现。
pub trait ShaderBindingLayout
{
    /// 获取所有绑定的详细信息
    ///
    /// 该函数通常由宏自动实现，返回一个包含所有绑定信息的数组。
    /// 每个绑定包含名称、绑定点、描述符类型、着色器阶段和数量。
    fn get_shader_bindings() -> Vec<ShaderBindingItem>;

    /// 获取 Vulkan 描述符集布局绑定
    ///
    /// 该函数不应该被覆盖，它使用 get_shader_bindings 的结果
    /// 生成 Vulkan 描述符集布局所需的绑定信息。
    fn get_bindings() -> Vec<vk::DescriptorSetLayoutBinding<'static>>
    {
        let bindings = Self::get_shader_bindings();
        bindings
            .iter()
            .map(|item| vk::DescriptorSetLayoutBinding {
                binding: item.binding,
                descriptor_type: item.descriptor_type,
                descriptor_count: item.count,
                stage_flags: item.stage_flags,
                ..Default::default()
            })
            .collect()
    }
}
