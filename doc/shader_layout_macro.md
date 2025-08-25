# shader-layout-macro

## 概述
过程宏 crate，通过属性宏简化着色器绑定的生成。自动为 Rust 结构体生成对应的 Vulkan 描述符布局和绑定信息。

## 架构组织

### 宏处理器 (`src/lib.rs`)
- 主要的过程宏入口点
- AST 解析和变换
- 代码生成逻辑

### 布局分析 (`src/layout_analyzer.rs`)
- 结构体字段的着色器布局分析
- 类型到描述符类型的映射
- 绑定点和集合的自动分配

### 代码生成 (`src/codegen.rs`)
- Vulkan 描述符布局代码生成
- 绑定信息的实现生成
- 错误处理代码的生成

## 核心宏

### `#[shader_layout]`
为结构体自动生成着色器布局信息：

```rust
use shader_layout_macro::shader_layout;

#[shader_layout]
pub struct FrameUniforms {
    #[binding = 0]
    pub view_proj_matrix: Mat4,
    
    #[binding = 1] 
    pub light_data: LightData,
    
    #[texture(binding = 2)]
    pub diffuse_texture: u32,
    
    #[sampler(binding = 3)]
    pub linear_sampler: u32,
}
```

自动生成：
```rust
impl ShaderLayout for FrameUniforms {
    fn descriptor_set_layout(device: &Device) -> DescriptorSetLayout {
        // 自动生成的布局创建代码
    }
    
    fn binding_info() -> Vec<DescriptorSetLayoutBinding> {
        // 自动生成的绑定信息
    }
}
```

### 支持的属性

#### `#[binding = N]`
指定绑定点：
```rust
#[shader_layout]
struct Uniforms {
    #[binding = 0]
    mvp_matrix: Mat4,           // binding 0, uniform buffer
    
    #[binding = 1]
    material_data: Material,    // binding 1, uniform buffer
}
```

#### `#[texture(binding = N)]`
纹理绑定：
```rust
#[shader_layout]
struct TextureSet {
    #[texture(binding = 0)]
    diffuse: u32,               // 纹理句柄
    
    #[texture(binding = 1, array_size = 16)]
    material_textures: [u32; 16], // 纹理数组
}
```

#### `#[sampler(binding = N)]`
采样器绑定：
```rust
#[shader_layout]
struct SamplerSet {
    #[sampler(binding = 0)]
    linear: u32,
    
    #[sampler(binding = 1)]
    nearest: u32,
}
```

#### `#[storage(binding = N)]`
存储缓冲区绑定：
```rust
#[shader_layout]
struct ComputeData {
    #[storage(binding = 0, access = "read")]
    input_buffer: BufferHandle,
    
    #[storage(binding = 1, access = "write")]
    output_buffer: BufferHandle,
}
```

## 类型映射

### 自动类型推断
宏会自动推断适当的描述符类型：

```rust
Mat4 → VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER
u32 (with #[texture]) → VK_DESCRIPTOR_TYPE_SAMPLED_IMAGE
u32 (with #[sampler]) → VK_DESCRIPTOR_TYPE_SAMPLER
BufferHandle (with #[storage]) → VK_DESCRIPTOR_TYPE_STORAGE_BUFFER
```

### 数组支持
```rust
#[shader_layout]
struct ArrayBindings {
    #[texture(binding = 0, array_size = 8)]
    textures: [u32; 8],         // 纹理数组
    
    #[binding = 1]
    matrices: [Mat4; 4],        // 统一缓冲区数组
}
```

### 嵌套结构体
```rust
#[shader_layout]
struct Material {
    albedo: Vec3,
    metallic: f32,
    roughness: f32,
}

#[shader_layout]  
struct SceneData {
    #[binding = 0]
    camera: CameraData,
    
    #[binding = 1]
    materials: [Material; 32],  // 嵌套结构体数组
}
```

## 生成的代码

### 描述符集布局
```rust
impl ShaderLayout for FrameUniforms {
    fn descriptor_set_layout(device: &Device) -> Result<DescriptorSetLayout> {
        let bindings = vec![
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::ALL_GRAPHICS,
                ..Default::default()
            },
            // ... 更多绑定
        ];
        
        DescriptorSetLayout::new(device, &bindings)
    }
}
```

### 绑定辅助方法
```rust
impl FrameUniforms {
    pub fn bind_uniforms(&self, descriptor_set: &mut DescriptorSet) {
        descriptor_set.update_buffer(0, &self.view_proj_matrix);
        descriptor_set.update_buffer(1, &self.light_data);
        descriptor_set.update_texture(2, self.diffuse_texture);
        descriptor_set.update_sampler(3, self.linear_sampler);
    }
}
```

## 高级特性

### 着色器阶段指定
```rust
#[shader_layout]
struct VertexUniforms {
    #[binding = 0, stages = "vertex"]
    mvp_matrix: Mat4,
    
    #[binding = 1, stages = "vertex|fragment"]
    light_data: LightData,
}
```

### 动态描述符
```rust
#[shader_layout]
struct DynamicSet {
    #[binding = 0, dynamic]
    per_object_data: ObjectData,    // 动态统一缓冲区
}
```

### 绑定标志
```rust
#[shader_layout]
struct AdvancedBindings {
    #[texture(binding = 0, flags = "partially_bound")]
    optional_textures: [u32; 64],   // 部分绑定纹理数组
    
    #[storage(binding = 1, flags = "update_after_bind")]
    dynamic_storage: BufferHandle,  // 绑定后更新
}
```

## 使用工作流

### 1. 定义着色器接口
```rust
#[shader_layout]
pub struct MyShaderData {
    #[binding = 0]
    pub uniforms: MyUniforms,
    
    #[texture(binding = 1)]
    pub diffuse: u32,
}
```

### 2. 创建描述符集
```rust
let layout = MyShaderData::descriptor_set_layout(&device)?;
let descriptor_set = DescriptorSet::new(&device, &layout)?;
```

### 3. 绑定资源
```rust
let shader_data = MyShaderData {
    uniforms: my_uniforms,
    diffuse: texture_id,
};

shader_data.bind_to_set(&mut descriptor_set)?;
```

## 错误处理

### 编译时验证
- 绑定点冲突检测
- 类型兼容性检查
- 属性参数验证

### 运行时错误
```rust
pub enum LayoutError {
    DuplicateBinding(u32),
    UnsupportedType(String),
    InvalidStageFlags(String),
    DescriptorCreationFailed,
}
```

## 调试支持

### 布局信息输出
编译时可以输出详细的布局信息：
```rust
#[shader_layout(debug)]
struct DebugLayout {
    // ... 字段定义
}
```

### 运行时验证
```rust
#[shader_layout(validate)]
struct ValidatedLayout {
    // 会生成额外的运行时检查
}
```

## 限制和考虑

### 支持的类型
- 基础数学类型 (Vec2, Vec3, Vec4, Mat4)
- 自定义结构体 (必须也有 #[shader_layout])
- 数组类型
- 句柄类型 (u32, BufferHandle 等)

### 绑定限制
- 绑定点必须唯一
- 数组大小必须在编译时确定
- 某些 Vulkan 特性需要显式指定

## 与其他组件的集成
- `shader-layout-trait`: 提供基础 trait 定义
- `truvis-rhi`: 与 Vulkan 对象集成
- `shader-binding`: 配合自动生成的类型使用
