# shader-binding

## 概述
通过 bindgen 自动生成 Slang 着色器对应的 Rust 数据结构，实现着色器和 Rust 代码之间的类型安全绑定。

## 架构组织

### 构建系统 (`build.rs`)
- 使用 bindgen 解析 Slang 头文件，具体实现在 `shader/shader-binding/build.rs`
- 自定义回调处理类型名称映射（uint→Uint, float4x4→Float4x4）
- 自动添加 `bytemuck::Pod` 和 `bytemuck::Zeroable` derive

### C++ 桥接文件
- **`rust_ffi.hpp`**: 包含所有需要绑定的 Slang 头文件
- **`slang_base.hpp`**: 定义基础 Slang 类型到 C++ 的映射

### 生成的绑定 (`src/`)
- **`_shader_bindings.rs`**: bindgen 自动生成的原始绑定
- **`lib.rs`**: 公开接口和 glam 类型转换实现

## 核心功能

### 自动类型生成
从 Slang 着色器代码：
```hlsl
// 在 common.slangi 中
struct FrameData {
    float4x4 view_matrix;
    float4x4 proj_matrix;
    float3 camera_pos;
    float time;
};

struct Material {
    float3 albedo;
    float metallic;
    float roughness;
    int diffuse_texture_id;
};
```

自动生成对应的 Rust 类型：
```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameData {
    pub view_matrix: Mat4,
    pub proj_matrix: Mat4,
    pub camera_pos: Vec3,
    pub time: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    pub albedo: Vec3,
    pub metallic: f32,
    pub roughness: f32,
    pub diffuse_texture_id: i32,
}
```

### 类型安全保证
- `#[repr(C)]` 确保内存布局与着色器一致
- bytemuck 支持安全的字节转换
- 编译时的类型检查防止数据不匹配

### 数学库集成
- 自动映射 Slang 向量/矩阵类型到 `glam` 类型
- `float4x4` → `Mat4`
- `float3` → `Vec3`
- `float2` → `Vec2`

## 构建流程

### bindgen 配置（来自 `shader/shader-binding/build.rs`）
```rust
// 文件：shader/shader-binding/build.rs
let bindings = bindgen::Builder::default()
    .header("rust_ffi.hpp")
    .clang_arg("-I../include")
    .derive_default(false)
    .raw_line("#![allow(clippy::all)]")
    .enable_cxx_namespaces()
    .parse_callbacks(Box::new(ModifyAdder))  // 自定义类型名称映射
    .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
    .generate()?;
```

### 类型映射实现（来自 `shader/shader-binding/build.rs`）
```rust
impl bindgen::callbacks::ParseCallbacks for ModifyAdder {
    fn item_name(&self, original_name: &str) -> Option<String> {
        match original_name {
            "uint" => Some("Uint".to_string()),
            "uint2" => Some("Uint2".to_string()),
            "float4x4" => Some("Float4x4".to_string()),
            // ... 其他映射
        }
    }
    
    fn add_derives(&self, info: &bindgen::callbacks::DeriveInfo) -> Vec<String> {
        if info.kind == bindgen::callbacks::TypeKind::Struct {
            vec!["bytemuck::Pod".into(), "bytemuck::Zeroable".into()]
        } else { vec![] }
    }
}
```

### 头文件处理
- 解析 `shader/include/` 中的 `.slangi` 文件
- 提取结构体和常量定义
- 处理 include 依赖关系

### 代码生成
- 生成标准的 Rust 结构体
- 添加必要的 derive 宏
- 集成 bytemuck 和 glam 特性

## 使用模式

### 访问生成的类型（来自 `shader/shader-binding/src/lib.rs`）
```rust
// 文件：shader/shader-binding/src/lib.rs
use shader_binding::shader;  // 公开所有生成的类型

// 使用自动生成的类型
let frame_data = shader::PerFrameData {
    projection: glam::Mat4::IDENTITY.into(),
    view: glam::Mat4::IDENTITY.into(),
    camera_pos: glam::Vec3::ZERO.into(),
    time_ms: 0.0,
    // ... 其他字段
};
```

### Glam 类型转换（来自 `shader/shader-binding/src/lib.rs`）
```rust
// 文件：shader/shader-binding/src/lib.rs
impl From<glam::Mat4> for Float4x4 {
    fn from(value: glam::Mat4) -> Self {
        Float4x4 {
            col0: Float4::from(value.x_axis),
            col1: Float4::from(value.y_axis),
            col2: Float4::from(value.z_axis),
            col3: Float4::from(value.w_axis),
        }
    }
}

impl From<glam::Vec3> for Float3 {
    fn from(value: glam::Vec3) -> Self {
        Float3 { x: value.x, y: value.y, z: value.z }
    }
}
```

## 与 Slang 着色器的集成

### 共享头文件（实际文件位置）
- **`shader/include/frame_data.slangi`**: 帧相关数据（PerFrameData）
- **`shader/include/bindless.slangi`**: 无绑定资源访问
- **`shader/include/scene.slangi`**: 场景数据结构
- **`shader/include/pass/`**: 各种渲染通道的数据结构
  - `blit.slangi`、`rt.slangi`、`imgui.slangi`、`raster.slangi`
  - `pp/sdr.slangi`: 后处理相关

### 包含关系（来自 `shader/shader-binding/rust_ffi.hpp`）
```cpp
// 文件：shader/shader-binding/rust_ffi.hpp
#include "./slang_base.hpp"
#include "bindless.slangi"
#include "frame_data.slangi"
#include "scene.slangi"
#include "pass/blit.slangi"
#include "pass/rt.slangi"
// ... 其他包含文件
```

### 类型一致性
- Rust 和 Slang 使用完全相同的数据布局
- 编译时验证数据结构匹配
- 自动同步类型更新

## 开发工作流

### 添加新的着色器类型
1. 在 `shader/include/` 中定义 Slang 结构体
2. 运行 `cargo build` 自动生成 Rust 绑定
3. 在 Rust 代码中使用生成的类型

### 类型更新
1. 修改 Slang 头文件中的结构体定义
2. 重新构建项目更新 Rust 绑定
3. 编译器会自动检测不匹配的使用

### 调试支持
- 生成的类型支持 `Debug` trait
- 可以直接打印着色器数据
- 类型不匹配会在编译时报错

## 最佳实践

### 数据对齐
- 遵循 Vulkan 统一缓冲区对齐要求
- 使用标准布局 (`#[repr(C)]`)
- 考虑填充字节的影响

### 版本管理
- 着色器接口变更时同步更新
- 使用语义版本控制
- 向后兼容性考虑

### 性能优化
- 最小化数据拷贝
- 批量更新缓冲区
- 缓存频繁使用的数据

## 限制和注意事项
- 仅支持 C 兼容的数据类型
- 不支持复杂的模板和泛型
- 需要手动处理某些 Slang 特性
