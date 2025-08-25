# Agent Documentation Overview

## 概述
这个 agent_doc 目录包含了 Render-Rust-vk-Truvis 工作区内所有 crate 的详细文档，专门为 AI 辅助编程设计。每个文档都描述了对应 crate 的架构、功能、使用模式和最佳实践。

### 新应用开发
1. 在 `crates/truvis-render/src/bin/your_app/` 创建目录
2. 实现 `OuterApp` trait
3. 使用现有的渲染通道或创建新的
4. 集成 ImGui 调试界面

### 着色器开发
1. 在 `shader/include/` 中定义共享数据结构
2. 在 `shader/src/` 中编写着色器代码
3. 使用 `#[shader_layout]` 宏自动生成 Rust 绑定
4. 在渲染通道中使用生成的类型

### 资源管理
1. 使用 `truvis-crate-tools::TruvisPath` 管理路径
2. 通过 `truvis-cxx` 加载 3D 模型
3. 使用 `model-manager` 管理顶点数据
4. 通过 `truvis-rhi` 创建 GPU 资源

## 文档结构

### 核心渲染 Crates
- **[truvis-rhi.md](truvis-rhi.md)**: Vulkan RHI 抽象层，底层图形 API 封装
- **[truvis-render.md](truvis-render.md)**: 主渲染框架，应用层和演示程序
- **[model-manager.md](model-manager.md)**: 3D 模型和顶点数据管理

### 系统集成 Crates  
- **[truvis-cxx.md](truvis-cxx.md)**: C++ 库集成，主要是 Assimp 绑定
- **[truvis-crate-tools.md](truvis-crate-tools.md)**: 跨 crate 共享工具和路径管理

### 着色器系统 Crates
- **[shader-binding.md](shader-binding.md)**: Slang 到 Rust 的类型绑定生成
- **[shader-build.md](shader-build.md)**: 着色器编译和构建工具
- **[shader-layout-macro.md](shader-layout-macro.md)**: 着色器布局自动生成宏
- **[shader-layout-trait.md](shader-layout-trait.md)**: 着色器布局的基础 trait 定义

## 架构层次

```
应用层    │ truvis-render (演示应用, 渲染管线)
          │
集成层    │ truvis-cxx (Assimp) + model-manager (几何体)
          │
渲染层    │ truvis-rhi (Vulkan 抽象)
          │
着色器层  │ shader-binding + shader-build + shader-layout-*
          │
工具层    │ truvis-crate-tools (路径管理, 构建辅助)
```

## 依赖关系图

```
crates/truvis-render
├── crates/truvis-rhi (Vulkan 抽象)
├── crates/model-manager (几何体管理)
├── crates/truvis-cxx (Assimp 集成)
├── shader/shader-binding (着色器绑定)
└── crates/truvis-crate-tools (工具)

crates/model-manager
└── crates/truvis-rhi

crates/truvis-cxx  
├── crates/model-manager
└── crates/truvis-rhi

shader/shader-binding
└── (独立，通过 bindgen 生成)

shader/shader-build
└── crates/truvis-crate-tools

crates/shader-layout-macro
├── crates/shader-layout-trait
└── ash (Vulkan 绑定)

crates/shader-layout-trait
└── ash
```

## 开发工作流

### 1. 新功能开发
1. 如果需要新的着色器：在 `shader/src/` 中编写 Slang 代码
2. 运行 `cargo run --bin build_shader` 编译着色器
3. 在 `truvis-render/src/render_pipeline/` 中实现渲染通道
4. 在 `truvis-render/src/bin/` 中创建演示应用

### 2. 新应用开发  
1. 在 `truvis-render/src/bin/your_app/` 创建目录
2. 实现 `OuterApp` trait
3. 使用现有的渲染通道或创建新的
4. 集成 ImGui 调试界面

### 3. 着色器开发
1. 在 `shader/include/` 中定义共享数据结构
2. 在 `shader/src/` 中编写着色器代码
3. 使用 `#[shader_layout]` 宏自动生成 Rust 绑定
4. 在渲染通道中使用生成的类型

### 4. 资源管理
1. 使用 `truvis-crate-tools::TruvisPath` 管理路径
2. 通过 `truvis-cxx` 加载 3D 模型
3. 使用 `model-manager` 管理顶点数据
4. 通过 `truvis-rhi` 创建 GPU 资源

## AI 辅助编程指南

### 当需要...

#### 修改渲染管线
→ 查看 `truvis-render.md` 和 `truvis-rhi.md`

#### 添加新的着色器
→ 查看 `shader-binding.md`, `shader-build.md`, `shader-layout-macro.md`

#### 处理 3D 模型
→ 查看 `model-manager.md` 和 `truvis-cxx.md`

#### 管理资源路径
→ 查看 `truvis-crate-tools.md`

#### 创建描述符布局
→ 查看 `shader-layout-macro.md` 和 `shader-layout-trait.md`

#### 底层 Vulkan 操作
→ 查看 `truvis-rhi.md`

## 关键概念速查

### 坐标系统
- 模型/世界：右手坐标系，Y 向上
- 视图：右手坐标系，Y 向上，相机朝向 -Z  
- NDC：左手坐标系，Y 向上
- 帧缓冲：原点在左上角

### 资源路径（基于实际实现）
```rust
use truvis_crate_tools::TruvisPath;

let model = TruvisPath::assets_path("sponza.fbx");
let texture = TruvisPath::resources_path("uv_checker.png");  
let shader = TruvisPath::shader_path("rt/raygen.slang.spv");
```

### 着色器绑定（基于实际生成）
```rust
use shader_binding::shader;

let frame_data = shader::PerFrameData {
    projection: camera_projection.into(),
    view: camera_view.into(),
    camera_pos: camera_position.into(),
    time_ms: elapsed_time,
    // ... 其他字段
};
```

### 应用程序框架（基于实际 trait 定义）
```rust
// 文件：crates/truvis-render/src/outer_app.rs
impl OuterApp for MyApp {
    fn init(renderer: &mut Renderer, camera: &mut DrsCamera) -> Self;
    fn draw(&self, pipeline_ctx: PipelineContext);
    fn draw_ui(&mut self, ui: &imgui::Ui) {}
    fn update(&mut self, renderer: &mut Renderer) {}
    fn rebuild(&mut self, renderer: &mut Renderer) {}
}
```

## 常用命令
```bash
# 编译着色器
cargo run --bin build_shader

# 运行演示应用
cargo run --bin triangle
cargo run --bin rt-sponza  
cargo run --bin rt_cornell
cargo run --bin shader_toy

# 完整构建
cargo build --release
```

## 注意事项
- 运行应用前必须先编译着色器
- 所有路径使用 `TruvisPath` 管理
- 新的着色器数据结构需要添加到 `shader/include/`
- C++ 依赖通过 CMake 自动构建
- 使用 `#[shader_layout]` 简化描述符布局创建
