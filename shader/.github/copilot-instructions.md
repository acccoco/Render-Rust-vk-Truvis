# Truvis 着色器管线 - AI 编程指南

## 架构概览

这是一个**Vulkan 光线追踪渲染器**，采用 Rust/Slang 混合着色器管线。项目使用 Slang 作为主要着色语言，通过 Rust FFI 绑定实现类型安全的 GPU 数据结构。

### 核心组件

- **shader-binding/**: 使用 `bindgen` 从 Slang 头文件自动生成 FFI 绑定的 Rust crate
- **shader-build/**: 基于 Rust 的着色器编译工具（替代传统构建脚本）
- **src/**: 按渲染通道组织的 Slang 着色器源文件
- **include/**: 共享的 Slang 头文件（.slangi），包含公共结构和工具
- **build/**: 编译后的 SPIR-V 着色器输出

## 开发工作流

### 着色器编译
```bash
# 手动着色器编译（在 shader-build/ 目录）
cargo run

# 生成 Rust 绑定（在 shader-binding/ 目录）
cargo build
```

### 关键构建依赖
- Slang 编译器必须在 PATH 中
- VS Code 使用 `include/` 作为 Slang IntelliSense 的附加搜索路径
- `shader-binding/ffi/` 中的 FFI 头文件定义了 bindgen 的 C++ 接口

## 项目特定模式

### Slang 类型约定
- 使用 `float3`、`float4x4` 等（而非 `vec3`、`mat4`）- 这些通过 FFI 映射到 Rust 类型
- 所有共享结构放在 `include/*.slangi` 文件中
- 着色器通道命名空间：`rt::PushConstants`、`raster::VertexInput`

### Rust-Slang FFI 桥接
```rust
// 自动生成的绑定重命名类型：
// slang "float3" -> rust "Float3" 
// slang "uint2" -> rust "Uint2"
// 自动派生 bytemuck::Pod 用于 GPU 上传
```

### 着色器组织
- **基于通道的结构**：`include/pass/rt.slangi`、`src/rt/rt.slang`
- **共享工具**：`include/sample/`、`include/common.slangi`
- **材质系统**：`include/pbr.slangi` 中的 PBR 材质

### GPU 内存管理
- 使用无绑定描述符（`ImageHandle`、`PTR()` 宏）
- 为 GPU 遍历结构化的场景数据：`Scene`、`Instance`、`Geometry`
- 帧数据模式：`PerFrameData` 包含相机、时间、分辨率

## 集成点

### 需要理解的关键头文件
- `frame_data.slangi`: 每帧相机/时间数据
- `scene.slangi`: 场景图和实例管理
- `bindless.slangi`: 描述符索引系统
- `ptr.slangi`: GPU 指针抽象

### 着色器编译链
1. Slang 源码 → SPIR-V（通过 shader-build 工具）
2. Slang 头文件 → C++ 头文件 → Rust FFI（通过 bindgen）
3. Slang 结构和 FFI 定义之间需要手动同步

### 光线追踪特性
- 使用 Vulkan 光线追踪扩展
- 负载结构：`HitPayload`、`ShadowMissPayload`
- 使用 `max_accum_samples` 限制的样本累积模式

## 常见任务

添加新着色器通道时：
1. 在相应的 `src/` 子目录中创建 `.slang` 文件
2. 在 `include/pass/` 中添加对应的 `.slangi` 头文件
3. 更新 `shader-binding/ffi/rust_ffi.hpp` 以包含新结构
4. 重新构建 shader-binding 以重新生成 Rust 类型
5. 运行 shader-build 编译新的 SPIR-V

修改共享结构时：
1. 首先更新 `.slangi` 头文件
2. 确保 C++ FFI 头文件完全匹配
3. 在编译着色器前重新生成 Rust 绑定
