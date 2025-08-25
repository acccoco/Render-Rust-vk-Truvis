# shader-layout-trait

## 概述
为 `shader-layout-macro` 提供基础 trait 定义，定义着色器布局和描述符绑定的统一接口。

## 核心 Trait

### `ShaderLayout`
主要的着色器布局 trait：

```rust
pub trait ShaderLayout {
    /// 创建对应的 Vulkan 描述符集布局
    fn descriptor_set_layout(device: &Device) -> Result<DescriptorSetLayout>;
    
    /// 获取绑定信息列表
    fn binding_info() -> Vec<DescriptorSetLayoutBinding>;
    
    /// 获取描述符集的大小信息
    fn descriptor_counts() -> HashMap<DescriptorType, u32>;
    
    /// 验证布局的有效性
    fn validate_layout() -> Result<(), LayoutError> {
        Ok(())
    }
}
```

### `DescriptorBinding`
单个描述符绑定的 trait：

```rust
pub trait DescriptorBinding {
    /// 绑定点编号
    fn binding_index() -> u32;
    
    /// 描述符类型
    fn descriptor_type() -> DescriptorType;
    
    /// 描述符数量（用于数组）
    fn descriptor_count() -> u32 { 1 }
    
    /// 着色器阶段标志
    fn stage_flags() -> ShaderStageFlags;
    
    /// 绑定标志（可选特性）
    fn binding_flags() -> Option<DescriptorBindingFlags> { None }
}
```

### `ResourceBinding`
资源绑定的 trait：

```rust
pub trait ResourceBinding<T> {
    /// 将资源绑定到描述符集
    fn bind_to_set(&self, descriptor_set: &mut DescriptorSet, binding: u32) -> Result<()>;
    
    /// 更新描述符集中的资源
    fn update_binding(&self, descriptor_set: &mut DescriptorSet, binding: u32) -> Result<()>;
}
```

## 标准实现

### 基础类型的实现
为常用类型提供默认实现：

```rust
impl DescriptorBinding for Mat4 {
    fn binding_index() -> u32 { 0 } // 需要通过宏覆盖
    fn descriptor_type() -> DescriptorType { 
        DescriptorType::UNIFORM_BUFFER 
    }
    fn stage_flags() -> ShaderStageFlags { 
        ShaderStageFlags::ALL_GRAPHICS 
    }
}

impl DescriptorBinding for Vec3 {
    fn descriptor_type() -> DescriptorType { 
        DescriptorType::UNIFORM_BUFFER 
    }
    fn stage_flags() -> ShaderStageFlags { 
        ShaderStageFlags::ALL_GRAPHICS 
    }
}
```

### 纹理句柄实现
```rust
#[derive(Clone, Copy)]
pub struct TextureHandle(pub u32);

impl DescriptorBinding for TextureHandle {
    fn descriptor_type() -> DescriptorType { 
        DescriptorType::SAMPLED_IMAGE 
    }
    fn stage_flags() -> ShaderStageFlags { 
        ShaderStageFlags::FRAGMENT 
    }
}

impl ResourceBinding<TextureHandle> for TextureHandle {
    fn bind_to_set(&self, descriptor_set: &mut DescriptorSet, binding: u32) -> Result<()> {
        descriptor_set.update_image(binding, self.0)
    }
}
```

### 采样器句柄实现
```rust
#[derive(Clone, Copy)]
pub struct SamplerHandle(pub u32);

impl DescriptorBinding for SamplerHandle {
    fn descriptor_type() -> DescriptorType { 
        DescriptorType::SAMPLER 
    }
    fn stage_flags() -> ShaderStageFlags { 
        ShaderStageFlags::FRAGMENT 
    }
}
```

## 辅助类型

### `LayoutError`
布局相关的错误类型：

```rust
#[derive(Debug, Clone)]
pub enum LayoutError {
    /// 绑定点冲突
    DuplicateBinding(u32),
    /// 不支持的类型
    UnsupportedType(String),
    /// 无效的着色器阶段
    InvalidStageFlags(String),
    /// 数组大小无效
    InvalidArraySize(usize),
    /// 描述符创建失败
    DescriptorCreationFailed(String),
}
```

### `BindingInfo`
绑定信息的封装：

```rust
#[derive(Debug, Clone)]
pub struct BindingInfo {
    pub binding: u32,
    pub descriptor_type: DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: ShaderStageFlags,
    pub binding_flags: Option<DescriptorBindingFlags>,
}

impl BindingInfo {
    pub fn to_vulkan_binding(&self) -> DescriptorSetLayoutBinding {
        DescriptorSetLayoutBinding {
            binding: self.binding,
            descriptor_type: self.descriptor_type,
            descriptor_count: self.descriptor_count,
            stage_flags: self.stage_flags,
            p_immutable_samplers: ptr::null(),
        }
    }
}
```

## 高级特性

### 动态描述符支持
```rust
pub trait DynamicBinding: DescriptorBinding {
    /// 是否为动态描述符
    fn is_dynamic() -> bool { true }
    
    /// 动态偏移计算
    fn dynamic_offset(&self) -> u32;
}
```

### 绑定验证
```rust
pub trait BindingValidator {
    /// 验证绑定配置
    fn validate_bindings(bindings: &[BindingInfo]) -> Result<(), LayoutError> {
        // 检查绑定点冲突
        let mut used_bindings = HashSet::new();
        for binding_info in bindings {
            if !used_bindings.insert(binding_info.binding) {
                return Err(LayoutError::DuplicateBinding(binding_info.binding));
            }
        }
        Ok(())
    }
}
```

### 着色器反射集成
```rust
pub trait ShaderReflection {
    /// 从着色器反射信息创建布局
    fn from_reflection(reflection: &SpvReflection) -> Result<Vec<BindingInfo>>;
    
    /// 验证布局与着色器的兼容性
    fn validate_with_shader(&self, shader: &ShaderModule) -> Result<()>;
}
```

## 使用模式

### 手动实现 ShaderLayout
```rust
struct CustomLayout {
    uniform_data: Mat4,
    texture: TextureHandle,
}

impl ShaderLayout for CustomLayout {
    fn descriptor_set_layout(device: &Device) -> Result<DescriptorSetLayout> {
        let bindings = vec![
            BindingInfo {
                binding: 0,
                descriptor_type: DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: ShaderStageFlags::VERTEX,
                binding_flags: None,
            }.to_vulkan_binding(),
            BindingInfo {
                binding: 1,
                descriptor_type: DescriptorType::SAMPLED_IMAGE,
                descriptor_count: 1,
                stage_flags: ShaderStageFlags::FRAGMENT,
                binding_flags: None,
            }.to_vulkan_binding(),
        ];
        
        DescriptorSetLayout::new(device, &bindings)
    }
    
    fn binding_info() -> Vec<DescriptorSetLayoutBinding> {
        // 返回绑定信息
    }
}
```

### 与宏配合使用
```rust
// 宏会自动实现这个 trait
#[shader_layout]
struct AutoLayout {
    #[binding = 0]
    uniforms: MyUniforms,
    
    #[texture(binding = 1)]
    diffuse: TextureHandle,
}

// 生成的实现会调用 trait 中的方法
```

## 扩展点

### 自定义描述符类型
```rust
pub trait CustomDescriptorType: DescriptorBinding {
    /// 自定义的描述符类型处理
    fn custom_descriptor_info() -> CustomDescriptorInfo;
}
```

### 布局优化
```rust
pub trait LayoutOptimizer {
    /// 优化描述符布局以提高性能
    fn optimize_layout(bindings: &mut [BindingInfo]);
    
    /// 计算布局的性能得分
    fn layout_score(bindings: &[BindingInfo]) -> f32;
}
```

## 与 Vulkan 的集成

### 描述符池管理
```rust
impl ShaderLayout for dyn ShaderLayout {
    fn required_pool_sizes(&self) -> Vec<DescriptorPoolSize> {
        let counts = self.descriptor_counts();
        counts.into_iter()
            .map(|(desc_type, count)| DescriptorPoolSize {
                ty: desc_type,
                descriptor_count: count,
            })
            .collect()
    }
}
```

### 管线布局集成
```rust
pub trait PipelineLayoutBuilder {
    fn add_descriptor_set_layout<T: ShaderLayout>(&mut self) -> &mut Self;
    fn build(self, device: &Device) -> Result<PipelineLayout>;
}
```

## 最佳实践
- 为自定义类型实现 `DescriptorBinding`
- 使用 `ResourceBinding` 简化资源更新
- 通过 `validate_layout` 提供编译时检查
- 保持 trait 实现的一致性和可预测性
