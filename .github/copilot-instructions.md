# Render-Rust-vk-Truvis Copilot 指令

基于 Rust 和 Vulkan 的现代渲染引擎，支持自动化着色器绑定和光线追踪。

## 架构概览

### 核心工作区结构
- `crates/truvis-rhi/`: 底层 Vulkan RHI 抽象层
  - `src/core/`: 设备、命令缓冲区、管线、描述符、同步
  - `src/resources/`: 缓冲区/图像管理，集成 VMA
- `crates/truvis-render/`: 主渲染框架和应用层
  - `src/bin/`: 独立演示应用（triangle/、rt-sponza/、rt_cornell.rs、shader_toy/）
  - `src/render_pipeline/`: 管线实现（rt_pass.rs、phong_pass.rs、compute_pass.rs）
  - `src/renderer/`: 核心渲染器和帧管理
  - `src/platform/`: 相机系统和输入处理
- `shader/`: Slang 着色器生态系统
  - `src/`: 按功能组织的着色器源码（hello_triangle/、phong/、rt/）
  - `include/`: 共享 Slang 头文件（.slangi 文件）
  - `shader-binding/`: 自动生成 Rust 绑定
- `crates/model-manager/`: 3D 场景加载和顶点管理
- `crates/truvis-cxx/`: C++ 集成（Assimp）通过 CMake

## 关键构建依赖和设置

### 构建命令
```bash
# 完整构建（通过 CMake 自动构建 C++ 依赖）
cargo build --release

# 编译着色器（运行应用前必需）
cargo run --bin build_shader

# 运行特定演示
cargo run --bin triangle         # 基础三角形渲染
cargo run --bin rt-sponza       # 光线追踪 Sponza 场景  
cargo run --bin rt_cornell      # Cornell Box 光线追踪
cargo run --bin shader_toy      # 着色器实验环境
```

### 自动生成的依赖
- **C++ 库**: CMake 自动构建 Assimp，将 DLL 复制到 `target/debug/`
- **着色器绑定**: `shader-binding/build.rs` 使用 bindgen 从 Slang 头文件生成 Rust 结构体
- **资源路径**: `truvis-crate-tools` 提供工作区相对路径辅助工具

## 应用程序模式

### OuterApp Trait 实现
所有应用程序都遵循这一模式：
```rust
use truvis_render::outer_app::OuterApp;

struct MyApp {
    my_pipeline: MyPipeline,
    geometry: DrsGeometry<MyVertexType>,
}

impl OuterApp for MyApp {
    fn init(renderer: &mut Renderer, camera: &mut DrsCamera) -> Self {
        Self {
            my_pipeline: MyPipeline::new(&renderer.rhi, &renderer.frame_settings()),
            geometry: VertexAosLayout::some_shape(&renderer.rhi),
        }
    }
    
    fn draw(&self, pipeline_ctx: PipelineContext) {
        self.my_pipeline.render(pipeline_ctx, &self.geometry);
    }
    
    fn draw_ui(&mut self, ui: &imgui::Ui) { /* 可选 GUI 控制 */ }
}

fn main() { TruvisApp::<MyApp>::run(); }
```

### 渲染管线模式
每个管线包含一个"Pass"和一个"Pipeline"：
- **Pass** (`*_pass.rs`): 封装渲染状态、着色器、描述符布局
- **Pipeline** (`*_pipeline.rs`): 协调命令缓冲区、图像屏障、渲染调用

## 着色器工作流

### Slang 结构绑定
在 `shader/include/` 中定义的 Slang 结构体自动生成 Rust 绑定：
```rust
// shader/include/my_uniforms.slangi
struct MyUniforms {
    float4x4 mvp_matrix;
    float3 light_pos;
};

// 自动生成的 Rust 绑定可在代码中使用
use shader_binding::MyUniforms;
```

### 描述符布局生成
使用 `#[shader_layout]` 宏简化描述符布局：
```rust
#[shader_layout]
struct MyLayoutBinding {
    #[binding = 0] uniforms: MyUniforms,
    #[texture(binding = 1)] diffuse: TextureHandle,
    #[sampler(binding = 2)] sampler: SamplerHandle,
}
```

### 着色器编译流程
1. 在 `shader/src/` 中编写 `.slang` 文件
2. 运行 `cargo run --bin build_shader` 编译为 SPIR-V
3. 管线在运行时加载编译后的着色器

## 资源管理模式

### 路径管理
使用 `TruvisPath` 进行一致的资源访问：
```rust
use truvis_crate_tools::TruvisPath;

let model_path = TruvisPath::assets_path("models/sponza.fbx");
let texture_path = TruvisPath::resources_path("textures/uv_checker.png");
let shader_path = TruvisPath::shader_path("rt/raygen.slang.spv");
```

### 顶点数据管理
使用 `model-manager` 创建和管理顶点：
```rust
use model_manager::vertex::vertex_pc::{VertexAosLayoutPosColor, VertexPosColor};

// 创建几何体
let triangle = VertexAosLayoutPosColor::triangle(&rhi);
let quad = VertexAosLayoutPosColor::quad(&rhi);
```

## 坐标系统约定

- **模型/世界空间**: 右手坐标系，Y 向上
- **视图空间**: 右手坐标系，Y 向上，相机朝向 -Z
- **NDC**: 左手坐标系，Y 向上  
- **帧缓冲**: 原点在左上角，确保视口 `height < 0`

从 Blender 导出：Forward = Y，Up = Z 以匹配渲染器约定。

## 调试和诊断

### 对象命名约定
- **Object name**: `[frame-A-id][pass]name`
- **Queue/Command label**: `[frame-A-id][pass]name`

### ImGui 集成
每个应用程序通过 `draw_ui()` 集成调试控制：
- 按 `F` 切换 GUI 显示/隐藏
- 通过 WASD 移动相机，鼠标控制旋转
- Shift 键加速移动

## 常见任务

### 添加新应用程序
1. 在 `crates/truvis-render/src/bin/my_app/` 创建目录
2. 实现 `OuterApp` trait 如上述模式
3. 如需新着色器，在 `shader/src/` 中添加 `.slang` 文件

### 创建新渲染管线
1. 在 `crates/truvis-render/src/render_pipeline/` 创建 `my_pass.rs` 和 `my_pipeline.rs`
2. 在 Pass 中设置着色器和描述符布局
3. 在 Pipeline 中协调命令缓冲区和屏障

### 集成 C++ 库
参考 `crates/truvis-cxx/` 的 CMake 集成模式，通过 `build.rs` 调用 CMake 并复制输出。

## 开发模式

### 创建新应用
在 `truvis-render/src/bin/your_app/` 中实现 `OuterApp` trait：
```rust
use truvis_render::outer_app::OuterApp;

struct MyApp { /* 你的状态 */ }

impl OuterApp for MyApp {
    fn init(renderer: &mut Renderer, camera: &mut DrsCamera) -> Self {
        // 在这里初始化你的渲染管线
    }
    
    fn draw(&self, pipeline_ctx: PipelineContext) {
        // 使用 pipeline_ctx 提交绘制命令
    }
    
    fn draw_ui(&mut self, ui: &imgui::Ui) {
        // ImGui 界面用于调试/控制
    }
}

fn main() {
    TruvisApp::<MyApp>::run();
}
```

### 着色器开发工作流
1. 在 `shader/src/your_feature/` 中编写 Slang 着色器
2. 包含共享头文件：`#include "../../include/common.slangi"`
3. 运行 `cargo run --bin build_shader` 生成 Rust 绑定
4. 在 Rust 代码中使用 `shader_binding` crate 的生成类型

### 渲染管线创建
在 `truvis-render/src/render_pipeline/` 中添加新通道：
```rust
pub struct YourPass {
    pipeline: GraphicsPipeline,
    descriptor_sets: Vec<DescriptorSet>,
}

impl YourPass {
    pub fn render(&self, ctx: PipelineContext, geometry: &DrsGeometry<VertexType>) {
        // 使用 ctx.command_buffer 记录命令
    }
}
```

## 关键集成点

### Vulkan RHI 使用
- 设备创建：`truvis_rhi::core::device::Device`
- 内存管理：通过 `truvis_rhi::core::allocator` 使用 VMA
- 命令记录：`truvis_rhi::core::command_buffer::CommandBuffer`

### 坐标系统（3D 工作的关键）
- 模型/世界：右手坐标系，Y 向上
- 视图：右手坐标系，Y 向上，相机朝向 -Z
- NDC：左手坐标系，Y 向上
- 帧缓冲：原点在左上角，确保视口高度 < 0

### 资源加载
```rust
use truvis_crate_tools::resource::TruvisPath;

// 资产路径是工作区相对路径
let model_path = TruvisPath::assets_path("models/sponza.fbx");
let texture_path = TruvisPath::resources_path("uv_checker.png");
```

## 运行时控制
- **WASD**: 相机移动
- **鼠标**: 相机旋转  
- **Shift**: 快速移动
- **F**: 切换 GUI 可见性

## 其他
- 所有文档使用中文
- 详细的模块文档请参考 `agent_doc` 目录
