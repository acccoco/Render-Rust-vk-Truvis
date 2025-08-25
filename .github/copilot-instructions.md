# Render-Rust-vk-Truvis Copilot 指令

基于 Rust 和 Vulkan 的现代渲染引擎，支持自动化着色器绑定和光线追踪。

## 🏗️ 架构概览

### 核心 Workspace 结构
```
crates/
├── truvis-rhi/           # Vulkan RHI 抽象（设备、命令、内存管理）
├── truvis-render/        # 主渲染库和演示应用
│   └── src/bin/          # triangle/, rt-sponza/, rt_cornell.rs, shader_toy/
├── model-manager/        # 顶点数据和几何体管理
├── truvis-cxx/          # C++ 库绑定（Assimp + CMake）
├── shader-layout-*/     # 描述符布局宏和 trait
└── truvis-crate-tools/  # 工作区路径工具

shader/
├── src/                 # 按功能组织的 .slang/.glsl/.hlsl 源码
├── include/            # 共享头文件（.slangi）
├── shader-binding/     # 自动生成 Rust 绑定（bindgen）
└── shader-build/       # 着色器编译工具
```

## 🚀 必需的构建流程

```bash
# 1. 首次构建（自动处理 CMake + C++ 依赖）
cargo build --release

# 2. 编译着色器（运行前必需！）
cargo run --bin build_shader

# 3. 运行演示
cargo run --bin triangle     # 基础三角形
cargo run --bin rt-sponza   # 光线追踪 Sponza
cargo run --bin rt_cornell  # Cornell Box
cargo run --bin shader_toy  # 着色器实验
```

### 自动生成系统
- **着色器绑定**: `shader-binding/build.rs` 从 `.slangi` 头文件生成 Rust 结构体
- **C++ 集成**: `truvis-cxx/build.rs` 通过 CMake 构建 Assimp，复制 DLL 到 `target/`
- **路径管理**: `truvis-crate-tools::TruvisPath` 提供工作区相对路径

## 🎯 应用开发模式

### OuterApp Trait 模式（所有应用的标准模式）
```rust
// 文件: crates/truvis-render/src/bin/my_app/main.rs
use truvis_render::outer_app::OuterApp;

struct MyApp {
    pipeline: MyPipeline,
    geometry: DrsGeometry<VertexType>,
}

impl OuterApp for MyApp {
    fn init(renderer: &mut Renderer, camera: &mut DrsCamera) -> Self {
        Self {
            pipeline: MyPipeline::new(&renderer.rhi, &renderer.frame_settings()),
            geometry: VertexAosLayout::triangle(&renderer.rhi),
        }
    }
    
    fn draw(&self, pipeline_ctx: PipelineContext) {
        self.pipeline.render(pipeline_ctx, &self.geometry);
    }
    
    fn draw_ui(&mut self, ui: &imgui::Ui) { /* 可选 GUI */ }
}

fn main() { TruvisApp::<MyApp>::run(); }
```

### 渲染管线架构
- **Pass** (`*_pass.rs`): 封装着色器、描述符布局、渲染状态
- **Pipeline** (`*_pipeline.rs`): 协调命令记录、图像屏障、绘制调用

## 🎨 着色器开发工作流

### Slang 结构体自动绑定
```rust
// shader/include/frame_data.slangi
struct PerFrameData {
    float4x4 projection;
    float4x4 view;
    float3 camera_pos;
    uint time_ms;
};

// 自动生成到 shader_binding crate
use shader_binding::PerFrameData;
```

### 描述符布局简化（关键宏）
```rust
#[shader_layout]  // 来自 shader-layout-macro
struct MyLayout {
    #[binding = 0] uniforms: PerFrameData,
    #[texture(binding = 1)] diffuse: TextureHandle,
    #[sampler(binding = 2)] sampler: SamplerHandle,
}
```

### 多编译器支持
- **Slang**: `.slang` → `slangc` (主要使用)
- **GLSL**: `.vert/.frag` → `glslc`  
- **HLSL**: `.hlsl` → `dxc`
- 输出: `shader/.build/*.spv` (SPIR-V)

## 📁 资源管理模式

### TruvisPath（统一路径管理）
```rust
use truvis_crate_tools::resource::TruvisPath;

// 所有路径基于工作区根目录
let model = TruvisPath::assets_path("sponza.fbx");           // assets/sponza.fbx
let texture = TruvisPath::resources_path("uv_checker.png");  // resources/uv_checker.png
let shader = TruvisPath::shader_path("rt/raygen.slang.spv"); // shader/.build/rt/raygen.slang.spv
```

### 顶点数据创建（model-manager）
```rust
use model_manager::vertex::vertex_pc::{VertexAosLayoutPosColor, VertexPosColor};

// 内置几何体
let triangle = VertexAosLayoutPosColor::triangle(&rhi);
let quad = VertexAosLayoutPosColor::quad(&rhi);

// 通过 truvis-cxx + Assimp 加载模型
// DLL 自动复制到 target/ 目录
```

## 📐 关键约定

### 坐标系统（严格遵循）
- **模型/世界**: 右手，Y-Up
- **视图**: 右手，Y-Up，相机朝向 -Z
- **NDC**: 左手，Y-Up
- **帧缓冲**: 原点左上角，视口 `height < 0`

### 调试命名规范
```rust
// Object name: [frame-A-id][pass]name
// Command label: [frame-A-id][pass]name
```

### 运行时控制
- **WASD**: 相机移动 | **鼠标**: 旋转 | **Shift**: 加速 | **F**: 切换 GUI

## 🔧 开发任务模板

### 添加新应用
```bash
# 1. 创建目录
mkdir crates/truvis-render/src/bin/my_app/

# 2. 实现 main.rs（见上述 OuterApp 模式）
# 3. 如需新着色器，在 shader/src/ 添加 .slang 文件
# 4. 运行构建流程
```

### 创建新渲染管线
```rust
// crates/truvis-render/src/render_pipeline/my_pass.rs
pub struct MyPass {
    pipeline: GraphicsPipeline,
    descriptor_sets: Vec<DescriptorSet>,
}

// crates/truvis-render/src/render_pipeline/my_pipeline.rs  
impl MyPipeline {
    pub fn render(&self, ctx: PipelineContext, geometry: &DrsGeometry<T>) {
        // ctx.command_buffer 记录命令
    }
}
```

### 集成新 C++ 库
参考 `crates/truvis-cxx/build.rs` 的 CMake + DLL 复制模式。

## 💡 关键实现细节

### Bindgen 着色器类型映射
```rust
// shader-binding/build.rs 自动转换
uint/uint2/uint3/uint4 → Uint/Uint2/Uint3/Uint4
float2/float3/float4 → Float2/Float3/Float4  
float4x4 → Float4x4
// 自动添加 bytemuck::Pod + Zeroable
```

### 并行着色器编译
`shader-build` 使用 `rayon::par_bridge()` 并行编译所有着色器。

### 工作区依赖管理
所有版本在根 `Cargo.toml` 的 `[workspace.dependencies]` 中统一管理。

## ⚠️ 关键限制和已知问题

### 构建依赖（必须按顺序执行）
```bash
# 错误：直接运行会失败，因为着色器未编译
cargo run --bin triangle  # ❌ 失败

# 正确：必须先编译着色器
cargo run --bin build_shader && cargo run --bin triangle  # ✅ 成功
```

### 平台特定要求
- **Windows**: 需要 Visual Studio 2019+，vcpkg 自动通过 `vcpkg.json` 管理 Assimp
- **DLL 自动复制**: `truvis-cxx/build.rs` 自动复制 Assimp DLL 到 `target/debug|release/`
- **Vulkan SDK**: 必需 1.3+，`tools/slang/` 包含 Slang 编译器

### 常见陷阱
```rust
// ❌ 错误：忘记使用 TruvisPath
let shader = "shader/src/triangle/triangle.slang.spv";

// ✅ 正确：使用 TruvisPath 获取正确路径
let shader = TruvisPath::shader_path("hello_triangle/triangle.slang.spv");

// ❌ 错误：viewport 设置错误
let viewport = vk::Viewport { height: extent.height as f32, .. };

// ✅ 正确：Y轴翻转 (height < 0)
let viewport = vk::Viewport { 
    y: extent.height as f32,
    height: -(extent.height as f32),
    ..
};
```

## 🔧 故障排除指南

### 编译失败常见原因
1. **Slang 编译器缺失**: 确保 `tools/slang/slangc.exe` 存在
2. **CMake 失败**: 检查 `VCPKG_ROOT` 环境变量
3. **DLL 缺失**: 运行 `cargo build` 触发 DLL 复制

### 运行时问题
```rust
// PipelineContext 使用模式
impl OuterApp for MyApp {
    fn draw(&self, ctx: PipelineContext) {
        // ✅ 正确：通过 ctx 访问所有组件
        let cmd = ctx.cmd_allocator.alloc_command_buffer("my-pass");
        // ❌ 避免：不要缓存 ctx 的组件引用
    }
}
```

### 着色器调试
- 使用 `-g2` 标志编译 Slang（已默认开启）
- Nsight Graphics 支持：通过 `dxc -fspv-debug=vulkan-with-source` 生成调试信息
- 输出位置：`shader/.build/` 目录下的 `.spv` 文件

## 🎯 架构决策记录

### 为什么选择 Slang？
- **跨平台**: 单一着色器语言编译到 HLSL/GLSL/SPIRV
- **自动绑定**: 通过 `bindgen` 自动生成 Rust 结构体
- **现代特性**: 支持 Generics、Interfaces、参数化类型

### OuterApp 模式的设计原因
```rust
// 简化应用开发：只需实现 3 个核心方法
trait OuterApp {
    fn init(renderer, camera) -> Self;  // 一次性初始化
    fn draw(&self, ctx: PipelineContext);  // 每帧渲染
    fn draw_ui(&mut self, ui: &imgui::Ui);  // 可选 GUI
}
```

### 内存管理策略
- **顶点数据**: AoS 布局通过 `model-manager` 管理
- **GPU 缓冲区**: 通过 `vk-mem` 分配器统一管理  
- **描述符**: Bindless 模式减少绑定切换开销

### 坐标系统设计原理
采用右手Y-Up世界坐标 + 左手Y-Up NDC 的混合系统：
- **优势**: 符合 Blender/Maya 等建模软件习惯
- **实现**: 通过视口 `height < 0` 实现 Y 轴翻转
- **兼容性**: 与 Vulkan NDC 坐标系统保持一致

## 🚧 当前重构状态与开发优先级

### 活跃重构项目（参考 `REFACTOR_PLAN.md`）
正在进行 `Renderer` 结构体重构，目标是消除过度的 `Rc<RefCell<>>` 使用：

```rust
// 当前问题模式 (正在重构)
pub struct Renderer {
    pub bindless_mgr: Rc<RefCell<BindlessManager>>,  // ❌ 过度使用 Rc
    pub scene_mgr: Rc<RefCell<SceneManager>>,         // ❌ 双重间接访问
}

// 目标架构模式
pub struct Renderer {
    pub core: RenderCore,           // 设备、帧控制、命令
    pub resources: RenderResources, // 资源、bindless、缓冲区  
    pub scene: SceneContext,        // 场景、GPU数据
    pub settings: RenderSettings,   // 统一配置
}
```

### 高优先级修复 (来自 `TODO.md`)
1. **ImGui 事件处理**: `truvis-render/src/app.rs:236` - 影响用户交互
2. **光照衰减计算**: `shader/include/light.slangi:29` - 影响渲染质量
3. **用户事件处理**: `truvis-render/src/app.rs:227` - 功能缺失(`todo!()`)

### 开发注意事项
- **重构期间**: 优先修复现有问题，避免大的架构变更
- **新功能**: 聚光灯支持、Hit Group 多样化正在规划中
- **性能**: 关注编译时间和运行时 RefCell 借用检查开销

## 💡 贡献指南

### 添加新着色器
```bash
# 1. 在 shader/src/ 创建 .slang 文件
# 2. 如需共享结构体，添加到 shader/include/*.slangi
# 3. 重新编译着色器
cargo run --bin build_shader
# 4. 使用自动生成的绑定
use shader_binding::MyStruct;
```

### 创建新演示应用
```bash
mkdir crates/truvis-render/src/bin/my_demo/
# 实现 OuterApp trait，参考 triangle/ 目录
# 在 Cargo.toml 中添加 [[bin]] 条目
```

### 性能分析
- **CPU**: 使用 `cargo build --release` + `perf`/`Tracy`
- **GPU**: Nsight Graphics 支持，确保使用 `-g2` 着色器调试标志
- **内存**: `vk-mem` 分配器提供统计信息
