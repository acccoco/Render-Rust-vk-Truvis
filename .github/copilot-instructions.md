# Render-Rust-vk-Truvis Copilot 指令

基于 Rust 和 Vulkan 1.3+ 的现代渲染引擎，支持 Slang 自动着色器绑定和硬件光线追踪。

## 🏗️ 核心架构

```
engine/
├── crates/
│   ├── truvis-gfx/              # Vulkan RHI 抽象（Gfx 单例）
│   ├── truvis-app/              # 应用框架核心（OuterApp trait + 内置 subpass 实现）
│   │   └── src/outer_app/       # 内置应用：triangle/, shader_toy/, sponza_app.rs
│   ├── truvis-render-interface/ # CmdAllocator, BindlessManager, FrameCounter, FrameLabel
│   ├── truvis-renderer/         # Renderer, Camera, Timer, 模型加载
│   ├── truvis-render-graph/     # RenderContext, Pass/Subpass, FifBuffers
│   ├── truvis-scene/            # GpuScene, SceneManager, RtGeometry, 几何体形状
│   ├── truvis-shader/           # 着色器系统
│   │   ├── truvis-shader-binding/   # Slang → Rust 自动绑定（build.rs + bindgen）
│   │   └── truvis-shader-build/     # 着色器编译工具（slangc/glslc/dxc）
│   ├── truvis-cxx/              # C++ FFI（Assimp 场景加载）
│   ├── truvis-asset/            # 异步资产加载
│   └── truvis-crate-tools/      # TruvisPath 路径工具
├── shader/
│   ├── src/                     # .slang 源码（按 pass 组织）
│   ├── include/                 # 共享头文件（.slangi）
│   └── .build/                  # 编译后 .spv（自动生成）
└── cxx/                         # C++ 源码 + CMakeLists.txt

truvis-winit-app/                # 独立 crate：可运行的演示应用
├── src/bin/                     # 应用入口：triangle_app.rs, sponza_app.rs, rt_cornell.rs...
└── src/app.rs                   # WinitApp 窗口管理

truvis-tauri-app/                # Tauri GUI 应用（可选）
```

**层次关系**: `truvis-gfx` → `truvis-render-*` → `truvis-app` → `truvis-winit-app/src/bin/*.rs`

## 🚀 构建流程（必须按顺序）

```powershell
# 1. 首次构建（自动处理 CMake + vcpkg 依赖）
cargo build --release

# 2. 编译着色器（运行前必需！）
cargo run --bin shader-build

# 3. 运行演示（从 truvis-winit-app 目录）
cd truvis-winit-app
cargo run --bin triangle_app       # 基础三角形
cargo run --bin sponza_app         # 光追 Sponza（需要模型）
cargo run --bin shader_toy_app     # 着色器实验场
cargo run --bin rt_cornell         # Cornell Box
cargo run --bin async_load_app     # 异步加载测试
```

**⚠️ 关键约束**:
- `shader-build` 必须在运行任何渲染应用前执行
- 位于 `engine/crates/truvis-shader/truvis-shader-build/src/bin/shader-build/`
- 使用 rayon 并行编译 `.slang` → `.spv`
- 输出目录：`engine/shader/.build/`

**自动生成系统**:
- 着色器绑定: `truvis-shader-binding/build.rs` 通过 bindgen 从 `.slangi` 生成 Rust 类型
- C++ 绑定: `truvis-cxx-binding/build.rs` 构建 CMake 并复制 DLL 到 `target/`


## 🎯 OuterApp 开发模式

### 标准模板
应用入口位于 `truvis-winit-app/src/bin/`，OuterApp 实现位于 `truvis-app/src/outer_app/`：

```rust
// truvis-winit-app/src/bin/my_app.rs
use truvis_app::outer_app::my_app::MyAppImpl;
use truvis_winit_app::app::WinitApp;

fn main() {
    let outer_app = Box::new(MyAppImpl::default());
    WinitApp::run(outer_app);
}

// engine/crates/truvis-app/src/outer_app/my_app.rs
use truvis_app::outer_app::OuterApp;
use truvis_renderer::renderer::Renderer;
use truvis_renderer::platform::camera::Camera;
use truvis_render_graph::render_context::RenderContext;

#[derive(Default)]
pub struct MyAppImpl {
    pipeline: Option<MyPass>,
    geometry: Option<RtGeometry>,
}

impl OuterApp for MyAppImpl {
    fn init(&mut self, renderer: &mut Renderer, _camera: &mut Camera) {
        self.pipeline = Some(MyPass::new(&renderer.render_context.frame_settings, &mut renderer.cmd_allocator));
        self.geometry = Some(TriangleSoA::create_mesh());
    }
    
    fn draw(&self, render_context: &RenderContext) {
        self.pipeline.as_ref().unwrap().render(render_context, self.geometry.as_ref().unwrap());
    }
    
    // 可选方法
    fn draw_ui(&mut self, _ui: &imgui::Ui) {}
    fn update(&mut self, _renderer: &mut Renderer) {}
    fn on_window_resized(&mut self, _renderer: &mut Renderer) {}
}
```


### RenderContext（核心渲染状态）

通过 `renderer.render_context` 访问，`OuterApp::draw()` 参数传入：
- `frame_counter.frame_label()` → 当前帧标签（A/B/C）
- `fif_buffers` → 管理 render target、depth images
- `bindless_manager` → Bindless 资源管理
- `gpu_scene` → GPU 场景数据
- `frame_settings` → 分辨率、格式等

**Pass 模式**（命令缓冲区预分配）：
```rust
impl MyPass {
    pub fn new(frame_settings: &FrameSettings, cmd_allocator: &mut CmdAllocator) -> Self {
        let cmds = FrameCounter::frame_labes()
            .map(|label| cmd_allocator.alloc_command_buffer(label, "my-pass"));
        Self { subpass: MySubpass::new(frame_settings), cmds }
    }
    
    pub fn render(&self, render_context: &RenderContext, geometry: &RtGeometry) {
        let frame_label = render_context.frame_counter.frame_label();
        let cmd = self.cmds[*frame_label].clone();
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "my-pass");
        self.subpass.draw(&cmd, /* ... */);
        cmd.end();
        Gfx::get().gfx_queue().submit(vec![GfxSubmitInfo::new(&[cmd])], None);
    }
}
```


### 渲染管线架构

| 层级 | 文件模式 | 职责 |
|------|----------|------|
| Subpass | `*_subpass.rs` | 封装着色器、描述符布局，实现 `RenderSubpass` trait |
| Pass | `*_pass.rs` | 命令记录、图像屏障、调用 subpass.draw() |


## 🎨 着色器开发（Slang 优先）

### 目录结构
| 目录 | 用途 |
|------|------|
| `engine/shader/include/` | 共享头文件（`.slangi`）：结构体、全局绑定 |
| `engine/shader/src/<pass>/` | 按渲染通道组织的着色器源码 |
| `engine/shader/.build/` | 编译输出（SPIR-V） |

### 全局描述符布局（三层绑定）
定义于 [global_binding_sets.slangi](engine/shader/include/global_binding_sets.slangi)：
```slang
// set 0: 全局采样器
[[vk::binding(0, 0)]] SamplerState global_samplers[];
// set 1: Bindless 资源（需 NonUniformResourceIndex）
[[vk::binding(0, 1)]] Sampler2D<float4> bindless_textures[];
// set 2: 每帧数据
[[vk::binding(0, 2)]] ConstantBuffer<PerFrameData> per_frame_data;
```

### Slang → Rust 自动绑定
```slang
// engine/shader/include/frame_data.slangi
struct PerFrameData { float4x4 projection; float4x4 view; float3 camera_pos; uint time_ms; };
```
```rust
// 自动生成到 truvis-shader-binding crate
use truvis_shader_binding::truvisl::PerFrameData;
```

### 描述符布局宏
```rust
#[shader_layout]  // 来自 truvis-descriptor-layout-macro
struct MyLayout {
    #[binding = 0] uniforms: PerFrameData,
    #[texture(binding = 1)] diffuse: TextureHandle,
}
```


## 📁 资源管理

### TruvisPath
```rust
use truvis_crate_tools::resource::TruvisPath;

let model = TruvisPath::assets_path("sponza.fbx");                      // assets/...
let texture = TruvisPath::resources_path("uv_checker.png");             // resources/...
let shader = TruvisPath::shader_build_path_str("rt/raygen.slang");      // shader/.build/...spv
// 注意：shader_build_path_str 自动添加 .spv 后缀
```

### 顶点数据
```rust
use truvis_scene::shapes::triangle::TriangleSoA;
use truvis_scene::components::geometry::RtGeometry;
let triangle: RtGeometry = TriangleSoA::create_mesh();  // 内置几何体
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
// 帧计数器：render_context.frame_counter.frame_id
```

### 运行时控制
- **WASD**: 相机移动 | **鼠标**: 旋转 | **Shift**: 加速 | **F**: 切换 GUI

## 🔧 开发任务模板

### 添加新应用
```powershell
# 1. 在 truvis-app/src/outer_app/ 创建 OuterApp 实现
# 2. 在 truvis-winit-app/src/bin/ 创建入口文件 my_app.rs
# 3. 如需新着色器，在 engine/shader/src/ 添加 .slang 文件
# 4. 运行构建流程
cargo run --bin shader-build
cd truvis-winit-app && cargo run --bin my_app
```

参考示例：[triangle_app.rs](truvis-winit-app/src/bin/triangle_app.rs) + [triangle/](engine/crates/truvis-app/src/outer_app/triangle/)

### 创建新渲染管线
```rust
// engine/crates/truvis-app/src/outer_app/my_app/my_subpass.rs
pub struct MySubpass {
    pipeline: GfxGraphicsPipeline,
    pipeline_layout: Rc<GfxPipelineLayout>,
}
impl RenderSubpass for MySubpass {}

// engine/crates/truvis-app/src/outer_app/my_app/my_pass.rs
impl MyPass {
    pub fn render(&self, render_context: &RenderContext, geometry: &RtGeometry) {
        let frame_label = render_context.frame_counter.frame_label();
        let cmd = self.cmds[*frame_label].clone();
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "my-pass");
        
        // 图像屏障
        cmd.image_memory_barrier(vk::DependencyFlags::empty(), &[/* barriers */]);
        
        // 绘制
        self.subpass.draw(&cmd, /* params */);
        
        cmd.end();
        Gfx::get().gfx_queue().submit(vec![GfxSubmitInfo::new(&[cmd])], None);
    }
}
```

### 集成新 C++ 库
参考 `engine/crates/truvis-cxx/build.rs` 的 CMake + DLL 复制模式：
```rust
// build.rs
println!("cargo:rustc-link-search=native={}", cargo_build_dir.display());
println!("cargo:rustc-link-lib=static=my-lib");
```

## 💡 关键实现细节

### Gfx 单例模式
```rust
// Gfx: 底层 Vulkan 抽象单例
Gfx::init("Truvis".to_string(), extra_instance_ext);
Gfx::get().gfx_device()  // 访问设备
Gfx::get().gfx_queue()   // 访问队列

// Renderer 管理整个渲染流程
// RenderContext 包含渲染状态，通过 renderer.render_context 访问

// 销毁顺序（在 Renderer::destroy() 中自动处理）
```

### Frames in Flight (FIF) 模式
- **固定 3 帧**: FrameLabel::A/B/C（`fif_count = 3`）
- **Timeline Semaphore**: 同步 GPU 进度（`frame_id` 与 semaphore value 对应）
- **FifBuffers**: 管理 render target、depth、color images

```rust
let frame_label = render_context.frame_counter.frame_label();  // A/B/C
let render_target = render_context.fif_buffers.render_target_image(frame_label);
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
// ✅ 正确：使用 shader_build_path_str（自动添加 .spv 后缀）
let shader = TruvisPath::shader_build_path_str("hello_triangle/triangle.slang");

// ❌ 错误：viewport 设置
let viewport = vk::Viewport { height: extent.height as f32, .. };
// ✅ 正确：Y轴翻转（Vulkan Y-down → 右手坐标系 Y-up）
let viewport = vk::Viewport { 
    y: extent.height as f32,
    height: -(extent.height as f32),
    ..
};

// ❌ 错误：OuterApp::draw() 签名（旧版本无参数）
fn draw(&self) { }
// ✅ 正确：当前版本接收 RenderContext
fn draw(&self, render_context: &RenderContext) { /* 通过 render_context 访问状态 */ }
```