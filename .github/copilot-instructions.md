# Render-Rust-vk-Truvis Copilot 指令

基于 Rust 和 Vulkan 的现代渲染引擎，支持自动化着色器绑定和光线追踪。

## 🏗️ 核心架构

```
crates/
├── truvis-gfx/              # Vulkan RHI 抽象
├── truvis-app/              # 应用框架（OuterApp trait）
├── truvis-render/           # 渲染管线、GPU 场景、FrameContext 单例
├── truvis-model-manager/    # 顶点数据和几何体
├── truvis-cxx/             # C++ 绑定（Assimp）
├── truvis-shader/          # 着色器系统
└── truvis-crate-tools/     # 工具（shader-build、路径管理）

shader/
├── src/        # .slang/.glsl/.hlsl 源码
├── include/   # 共享头文件（.slangi）
└── .build/   # 编译后 .spv 文件
```

**层次关系**: truvis-gfx → truvis-render → truvis-app → 应用 bin (`crates/truvis-app/src/bin/*/main.rs`)

## 🚀 构建流程（必需按顺序执行）

```powershell
# 1. 首次构建（自动处理 CMake + C++ 依赖）
cargo build --release

# 2. 编译着色器（运行前必需！）
cargo run --bin shader-build

# 3. 运行演示
cargo run --bin triangle
cargo run --bin rt-sponza    # 需要模型文件
cargo run --bin shader_toy
```

**关键**: `shader-build` 位于 `crates/truvis-crate-tools/src/bin/shader-build/`，使用 rayon 并行编译。

**自动生成系统**:
- 着色器绑定: `truvis-shader/binding/build.rs` 通过 `bindgen` 从 `.slangi` 生成 Rust 类型
- C++ 绑定: `truvis-cxx/build.rs` 通过 CMake 构建并复制 DLL
- 路径: `TruvisPath` 基于 `CARGO_MANIFEST_DIR` 推导工作区路径


## 🎯 OuterApp 开发模式

### 标准模板
```rust
// crates/truvis-app/src/bin/my_app/main.rs
use truvis_app::app::TruvisApp;
use truvis_app::outer_app::OuterApp;

struct MyApp {
    pipeline: MyPipeline,
    geometry: Geometry<VertexLayoutAoSPosColor>,
}

impl OuterApp for MyApp {
    fn init(_renderer: &mut Renderer, _camera: &mut Camera) -> Self {
        Self {
            pipeline: MyPipeline::new(&FrameContext::get().frame_settings()),
            geometry: VertexLayoutAoSPosColor::triangle(),
        }
    }
    
    fn draw(&self) {
        self.pipeline.render(&self.geometry);
    }
    
    // 可选方法
    fn draw_ui(&mut self, ui: &imgui::Ui) {}
    fn update(&mut self, renderer: &mut Renderer) {}
    fn rebuild(&mut self, renderer: &mut Renderer) {}
}

fn main() {
    TruvisApp::<MyApp>::run();
}
```


### FrameContext 单例（核心模式）
```rust
// 全局访问渲染状态，简化参数传递
let frame_label = FrameContext::frame_label();  // A/B/C
let cmd = FrameContext::cmd_allocator_mut().alloc_command_buffer("pass-name");

// 典型渲染管线
impl MyPipeline {
    pub fn render(&self, geometry: &Geometry) {
        let cmd = FrameContext::cmd_allocator_mut().alloc_command_buffer("my-pass");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "my-pass");
        // 绘制...
        cmd.end();
        Gfx::get().gfx_queue().submit(vec![SubmitInfo::new(&[cmd])], None);
    }
}
```

**常用方法**:
- `FrameContext::frame_label()` - 当前帧标签（A/B/C）
- `FrameContext::cmd_allocator_mut()` - 命令分配器
- `FrameContext::bindless_mgr_mut()` - Bindless 管理
- `FrameContext::gpu_scene_mut()` - GPU 场景
- `FrameContext::get().frame_settings()` - 帧设置


### 渲染管线架构
- **Pass** (`*_pass.rs`): 封装着色器、描述符布局，提供 `draw()` 或 `exec()` 方法
- **Pipeline** (`*_pipeline.rs`): 协调命令记录、图像屏障，提供 `render()` 方法

```rust
// Pass 示例
impl PhongPass {
    pub fn draw(&self, cmd: &CommandBuffer, /* ... */) {
        cmd.cmd_begin_rendering2(&rendering_info);
        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.handle());
        // 绘制...
        cmd.end_rendering();
    }
}
```


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

// 自动生成到 truvis-shader-binding crate
use truvis_shader_binding::shader::PerFrameData;
```

### 描述符布局简化（关键宏）
```rust
#[shader_layout]  // 来自 truvis-shader-layout-macro
struct MyLayout {
    #[binding = 0] uniforms: PerFrameData,
    #[texture(binding = 1)] diffuse: TextureHandle,
    #[sampler(binding = 2)] sampler: SamplerHandle,
}
```

### 多编译器支持
- **Slang**: `.slang` → `slangc` (主要使用，位于 `tools/slang/slangc.exe`)
- **GLSL**: `.vert/.frag` → `glslc`  
- **HLSL**: `.hlsl` → `dxc`
- 输出: `shader/.build/*.spv` (SPIR-V)

## 📁 资源管理模式

### TruvisPath（统一路径管理）
```rust
use truvis_crate_tools::resource::TruvisPath;

// 所有路径基于工作区根目录（通过 CARGO_MANIFEST_DIR 推导）
let model = TruvisPath::assets_path("sponza.fbx");           // assets/sponza.fbx
let texture = TruvisPath::resources_path("uv_checker.png");  // resources/uv_checker.png
let shader = TruvisPath::shader_path("rt/raygen.slang.spv"); // shader/.build/rt/raygen.slang.spv
```

### 顶点数据创建（model-manager）
```rust
use truvis_model_manager::vertex::aos_pos_color::VertexLayoutAoSPosColor;
use truvis_model_manager::components::geometry::Geometry;

// 内置几何体（已包含 GPU 缓冲区）
let triangle: Geometry<VertexLayoutAoSPosColor> = VertexLayoutAoSPosColor::triangle();
let quad = VertexLayoutAoSPosColor::quad();

// 通过 truvis-cxx + Assimp 加载模型（DLL 自动复制到 target/）
```

## 📐 关键约定

### 坐标系统（严格遵循）
- **模型/世界**: 右手，Y-Up
- **视图**: 右手，Y-Up，相机朝向 -Z
- **NDC**: 左手，Y-Up（Vulkan 标准）
- **帧缓冲**: 原点左上角，视口 `height < 0`（Y 轴翻转）

**Blender 导出设置**: Forward=Y, Up=Z

### 调试命名规范
```rust
// 格式: [frame-label]name
cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "ray-tracing");
FrameContext::frame_name()  // 返回 "[F42A]"
```

### 运行时控制
- **WASD**: 相机移动 | **鼠标**: 旋转 | **Shift**: 加速 | **F**: 切换 GUI

## 🔧 开发任务模板

### 添加新应用
```powershell
# 1. 创建目录（位于 truvis-app/src/bin/）
mkdir crates/truvis-app/src/bin/my_app/

# 2. 创建 main.rs，实现 OuterApp trait（参考上述模式）
# 3. 如需新着色器，在 shader/src/ 添加 .slang 文件
# 4. 运行构建流程
cargo run --bin shader-build
cargo run --bin my_app
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
    pub fn render(&self, geometry: &Geometry<T>) {
        let cmd = FrameContext::cmd_allocator_mut().alloc_command_buffer("my-pass");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "my-pass");
        
        // 图像屏障
        cmd.image_memory_barrier(vk::DependencyFlags::empty(), &[/* barriers */]);
        
        // 绘制
        self.my_pass.draw(&cmd, /* params */);
        
        cmd.end();
        Gfx::get().gfx_queue().submit(vec![SubmitInfo::new(&[cmd])], None);
    }
}
```

### 集成新 C++ 库
参考 `crates/truvis-cxx/build.rs` 的 CMake + DLL 复制模式：
```rust
// build.rs
println!("cargo:rustc-link-search=native={}", cargo_build_dir.display());
println!("cargo:rustc-link-lib=static=my-lib");
```

## 💡 关键实现细节

### Gfx 和 FrameContext 单例模式
```rust
// Gfx: 底层 Vulkan 抽象单例
Gfx::init("Truvis".to_string(), extra_instance_ext);
Gfx::get().gfx_device()  // 访问设备
Gfx::get().gfx_queue()   // 访问队列

// FrameContext: 渲染状态单例
FrameContext::init();
FrameContext::get()      // 访问完整上下文

// 销毁顺序（在 Renderer::destroy() 中）
FrameContext::destroy();
Gfx::destroy();
```

### Frames in Flight (FIF) 模式
- **固定 3 帧**: FrameLabel::A/B/C（`fif_count = 3`）
- **Timeline Semaphore**: 同步 GPU 进度（`frame_id` 与 semaphore value 对应）
- **FifBuffers**: 管理 render target、depth、color images

```rust
let frame_label = FrameContext::frame_label();  // A/B/C
let render_target = fif_buffers.render_target_image(frame_label);
```


## ⚠️ 关键限制和已知问题

### 构建依赖（必须按顺序执行）
```powershell
# ❌ 错误：直接运行会失败，因为着色器未编译
cargo run --bin triangle

# ✅ 正确：必须先编译着色器
cargo run --bin shader-build
cargo run --bin triangle
```

### 平台特定要求
- **Windows**: 需要 Visual Studio 2019+，vcpkg 自动通过 `vcpkg.json` 管理 Assimp
- **DLL 自动复制**: `truvis-cxx/build.rs` 自动复制 Assimp DLL 到 `target/debug|release/`
- **Vulkan SDK**: 必需 1.3+，`tools/slang/` 包含 Slang 编译器


## ⚠️ 常见陷阱

```rust
// ❌ 错误：忘记使用 TruvisPath
let shader = "shader/src/triangle/triangle.slang.spv";
// ✅ 正确
let shader = TruvisPath::shader_path("hello_triangle/triangle.slang.spv");

// ❌ 错误：viewport 设置
let viewport = vk::Viewport { height: extent.height as f32, .. };
// ✅ 正确：Y轴翻转
let viewport = vk::Viewport { 
    y: extent.height as f32,
    height: -(extent.height as f32),
    ..
};

// ❌ 错误：OuterApp::draw() 签名（旧版本）
fn draw(&self, ctx: PipelineContext) { }
// ✅ 正确：当前版本无参数
fn draw(&self) { /* 通过 FrameContext 访问 */ }

// ❌ 避免：缓存 RefCell 引用会 panic
let cmd_allocator = FrameContext::cmd_allocator_mut();
let bindless = FrameContext::bindless_mgr_mut();  // panic!
```

