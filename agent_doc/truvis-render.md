# truvis-render

## 概述
Truvis 的主渲染框架和应用层，构建在 truvis-rhi 之上，提供完整的渲染解决方案。包含渲染管线、场景管理、资源加载和演示应用。

## 架构组织

### 应用程序 (`src/bin/`)
独立的演示应用程序：
- **`triangle/`**: 基础三角形渲染演示，包含 `main.rs`、`triangle_pass.rs`、`triangle_pipeline.rs`
- **`rt-sponza/`**: 光线追踪 Sponza 场景演示，包含 `main.rs`
- **`rt_cornell.rs`**: Cornell Box 光线追踪演示
- **`shader_toy/`**: 着色器实验和原型环境

### 渲染管线 (`src/render_pipeline/`)
专门化的渲染通道实现：
- **`rt_pass.rs`**: 光线追踪渲染通道，包含 RayGen、Miss、ClosestHit 着色器的管理
- **`rt_pipeline.rs`**: 光线追踪管线协调器
- **`phong_pass.rs`**: Phong 光照模型渲染
- **`compute_pass.rs`**: 计算着色器通道
- **`pipeline_context.rs`**: 管线执行上下文定义，包含 RHI、GPU场景、无绑定管理器等

### 渲染器核心 (`src/renderer/`)
具体文件结构：
- **`renderer.rs`**: 主渲染器实现，包含帧控制、资源管理、渲染循环
- **`frame_controller.rs`**: 帧管理和渲染循环，支持多帧并行
- **`frame_buffers.rs`**: 帧缓冲区管理
- **`cmd_allocator.rs`**: 命令缓冲区分配器
- **`swapchain.rs`**: 交换链管理
- **`bindless.rs`**: 无绑定资源管理
- **`gpu_scene.rs`**: GPU 场景数据管理
- **`scene_manager.rs`**: 场景对象管理，支持模型加载和光源管理

### 平台层 (`src/platform/`)
包含具体文件：
- **`camera.rs`**: DrsCamera 相机实现，支持右手坐标系
- **`camera_controller.rs`**: 相机控制器，处理 WASD 移动和鼠标旋转
- **`input_manager.rs`**: 输入处理和响应
- **`timer.rs`**: 时间管理

### 窗口系统 (`src/window_system/`)
- **`main_window.rs`**: 主窗口实现，基于 winit
- 窗口创建和管理
- 事件循环处理

### GUI 系统 (`src/gui/`)
包含具体文件：
- **`gui.rs`**: ImGui 集成的主要实现
- **`gui_pass.rs`**: GUI 渲染通道
- **`mesh.rs`**: GUI 网格管理

### 其他核心文件
- **`app.rs`**: TruvisApp 主应用框架，实现 winit ApplicationHandler
- **`outer_app.rs`**: OuterApp trait 定义
- **`pipeline_settings.rs`**: 管线设置和帧设置
- **`gltf_loader.rs`**: GLTF 场景加载器

## 应用程序框架

### OuterApp Trait（定义在 `src/outer_app.rs`）
所有应用程序都需要实现 `OuterApp` trait：
```rust
// 文件：crates/truvis-render/src/outer_app.rs
pub trait OuterApp {
    fn init(renderer: &mut Renderer, camera: &mut DrsCamera) -> Self;
    fn draw_ui(&mut self, _ui: &imgui::Ui) {}
    fn update(&mut self, _renderer: &mut Renderer) {}
    /// 发生于 acquire_frame 之后，submit_frame 之前
    fn draw(&self, _pipeline_ctx: PipelineContext) {}
    /// window 发生改变后，重建
    fn rebuild(&mut self, _renderer: &mut Renderer) {}
}
```

### TruvisApp 框架（定义在 `src/app.rs`）
主应用框架实现了 winit 的 ApplicationHandler，管理整个应用生命周期：
```rust
pub struct TruvisApp<T: OuterApp> {
    renderer: Renderer,
    window_system: OnceCell<MainWindow>,
    input_manager: InputManager,
    camera_controller: CameraController,
    outer_app: OnceCell<T>,
}
```

### 应用程序启动
```rust
fn main() {
    TruvisApp::<YourApp>::run();
}
```

### 典型应用实现示例（基于 triangle 演示）
```rust
struct HelloTriangle {
    triangle_pipeline: TrianglePipeline,
    triangle: DrsGeometry<VertexPosColor>,
}

impl OuterApp for HelloTriangle {
    fn init(renderer: &mut Renderer, _camera: &mut DrsCamera) -> Self {
        Self {
            triangle_pipeline: TrianglePipeline::new(&renderer.rhi, &renderer.frame_settings()),
            triangle: VertexAosLayoutPosColor::triangle(&renderer.rhi),
        }
    }

    fn draw(&self, pipeline_ctx: PipelineContext) {
        self.triangle_pipeline.render(pipeline_ctx, &self.triangle);
    }
}
```

## 渲染管线开发

### 管线上下文（定义在 `src/render_pipeline/pipeline_context.rs`）
PipelineContext 包含渲染所需的所有上下文信息：
```rust
pub struct PipelineContext<'a> {
    pub rhi: &'a Rhi,
    pub gpu_scene: &'a GpuScene,
    pub bindless_mgr: Rc<RefCell<BindlessManager>>,
    pub per_frame_data: &'a RhiStructuredBuffer<shader::PerFrameData>,
    pub frame_ctrl: &'a FrameController,
    pub cmd_allocator: &'a mut CmdAllocator,
    pub frame_settings: &'a FrameSettings,
    pub pipeline_settings: &'a PipelineSettings,
    pub timer: &'a Timer,
    pub frame_buffers: &'a FrameBuffers,
}
```

### 管线模式（基于 triangle 实现）
每个管线通常分为 Pass 和 Pipeline 两部分：

**Pass（`*_pass.rs`）**: 封装着色器、描述符布局和渲染状态
**Pipeline（`*_pipeline.rs`）**: 协调命令缓冲区、图像屏障、渲染调用

```rust
pub struct TrianglePipeline {
    triangle_pass: TrianglePass,
}

impl TrianglePipeline {
    pub fn render(&self, ctx: PipelineContext, shape: &DrsGeometry<VertexPosColor>) {
        let cmd = ctx.cmd_allocator.alloc_command_buffer("triangle");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "triangle");
        
        // 图像屏障转换
        cmd.image_memory_barrier(/* ... */);
        
        // 实际渲染
        self.triangle_pass.draw(&cmd, ctx.frame_ctrl.frame_label(), 
                               ctx.frame_buffers, ctx.frame_settings, shape);
        
        cmd.end();
        ctx.rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[cmd])], None);
    }
}
```

### 光线追踪管线（基于 rt_pass.rs）
光线追踪管线包含多个着色器阶段：
- **RayGen**: 主光线生成着色器
- **Miss**: 光线未命中着色器（天空、阴影）
- **ClosestHit**: 最近命中着色器

## 关键系统

### 相机系统（定义在 `src/platform/camera.rs`）
DrsCamera 实现右手坐标系相机：
```rust
pub struct DrsCamera {
    pub position: glam::Vec3,
    pub euler_yaw_deg: f32,
    pub euler_pitch_deg: f32,
    pub euler_roll_deg: f32,
    pub asp: f32,
    pub fov_deg_vertical: f32,
    pub near: f32,
}
```

相机约定：
- 上参考向量：`(0, 1, 0)`
- 默认朝向：`(0, 0, -1)` （-Z 方向）
- 旋转顺序：YXZ（Yaw-Pitch-Roll）
- 视图矩阵：使用 `glam::Mat4::look_to_rh` 生成右手坐标系视图矩阵

### 相机控制器（定义在 `src/platform/camera_controller.rs`）
- WASD 键控制移动
- 鼠标控制旋转
- Shift 键加速移动

### 场景管理（定义在 `src/renderer/scene_manager.rs`）
SceneManager 支持：
- 点光源注册：`register_point_light()`
- 3D 模型加载：`load_scene()` 方法加载 FBX 模型
- 场景数据自动同步到 GPU

### 渲染器（定义在 `src/renderer/renderer.rs`）
主渲染器包含：
- **Frame Controller**: 多帧并行渲染管理
- **Bindless Manager**: 无绑定资源管理
- **GPU Scene**: GPU 端场景数据管理
- **Command Allocator**: 命令缓冲区分配
- **Frame Buffers**: 渲染目标管理

渲染器主要方法：
- `new()`: 初始化渲染器
- `begin_frame()` / `end_frame()`: 帧开始/结束
- `before_render()` / `after_render()`: 渲染前后处理
- `collect_render_ctx()`: 构建 PipelineContext

### 输入管理（定义在 `src/platform/input_manager.rs`）
InputManager 处理：
- 窗口事件：`handle_window_event()`
- 设备事件：`handle_device_event()`
- 输入状态查询

## 坐标系统
基于代码中的相机实现：
- **模型/世界**: 右手坐标系，Y 向上
- **视图**: 右手坐标系，Y 向上，相机默认朝向 -Z
- **NDC**: Vulkan 标准左手坐标系，Y 向上
- **帧缓冲**: 原点在左上角

相机系统常量（来自 `camera.rs`）：
- `CAMERA_UP: (0, 1, 0)`
- `CAMERA_FORWARD: (0, 0, -1)` 
- `CAMERA_RIGHT: (1, 0, 0)`
- 旋转顺序：`EulerRot::YXZ`

## 设置和配置

### 帧设置（定义在 `src/pipeline_settings.rs`）
```rust
pub struct FrameSettings {
    pub color_format: vk::Format,      // 默认 R32G32B32A32_SFLOAT
    pub depth_format: vk::Format,      // 自动选择最佳深度格式
    pub frame_extent: vk::Extent2D,    // 渲染分辨率
}
```

### 管线设置
```rust
pub struct PipelineSettings {
    pub channel: u32,  // 0=光线追踪，1=正常渲染
}
```

### 默认渲染器设置
```rust
impl DefaultRendererSettings {
    pub const DEFAULT_SURFACE_FORMAT: vk::SurfaceFormatKHR = /* R8G8B8A8_UNORM + SRGB_NONLINEAR */;
    pub const DEFAULT_PRESENT_MODE: vk::PresentModeKHR = vk::PresentModeKHR::MAILBOX;
    pub const DEPTH_FORMAT_CANDIDATES: &'static [vk::Format] = /* D32_SFLOAT 等候选格式 */;
}
```

## 运行时控制
基于 `input_manager.rs` 和 `camera_controller.rs` 的实现：
- **WASD**: 相机移动（前后左右）
- **鼠标移动**: 相机旋转（Yaw/Pitch）
- **Shift**: 快速移动模式
- **F**: 切换 GUI 可见性（通过 ImGui）

### 演示应用控制
在 GUI 界面中显示：
- 相机位置和欧拉角信息
- 相机朝向向量
- 渲染通道选择（channel 滑块：0-3）
- 累积帧数显示

## 典型应用示例

### 光线追踪应用（rt-sponza、rt_cornell）
```rust
impl OuterApp for RtApp {
    fn init(renderer: &mut Renderer, camera: &mut DrsCamera) -> Self {
        // 设置相机初始位置
        camera.position = glam::vec3(270.0, 194.0, -64.0);
        camera.euler_yaw_deg = 90.0;
        
        // 创建光线追踪管线
        let rt_pipeline = RtPipeline::new(&renderer.rhi, renderer.bindless_mgr.clone());
        
        // 注册点光源
        let mut scene_mgr = renderer.scene_mgr.borrow_mut();
        scene_mgr.register_point_light(shader::PointLight {
            pos: glam::vec3(-20.0, 40.0, 0.0).into(),
            color: (glam::vec3(5.0, 6.0, 1.0) * 2.0).into(),
            // ...
        });
        
        // 加载场景模型
        scene_mgr.load_scene(&renderer.rhi, 
                           std::path::Path::new("assets/blender/sponza.fbx"), 
                           &glam::Mat4::IDENTITY);
        
        Self { rt_pipeline }
    }
    
    fn draw(&self, pipeline_ctx: PipelineContext) {
        self.rt_pipeline.render(pipeline_ctx);
    }
}
```

## 依赖关系
基于 Cargo.toml 和代码导入分析：
- **`truvis-rhi`**: 底层图形 API 抽象（Vulkan）
- **`model-manager`**: 3D 模型和顶点管理，提供 DrsGeometry 等类型
- **`truvis-cxx`**: Assimp 集成用于 FBX 模型加载
- **`shader-binding`**: 着色器绑定，提供 `shader::PerFrameData`、`shader::PointLight` 等类型
- **`imgui`**: 调试和控制界面
- **`winit`**: 跨平台窗口管理
- **`ash`**: Vulkan API 绑定
- **`glam`**: 数学库（向量、矩阵）

## 开发工作流
1. 在 `src/bin/your_app/` 创建新应用目录
2. 实现 `OuterApp` trait，定义 `init()`、`draw()` 等方法
3. 在 `render_pipeline/` 中添加专门的渲染通道（Pass + Pipeline 模式）
4. 使用 `shader-binding` 集成着色器和统一缓冲区
5. 通过 ImGui 在 `draw_ui()` 中添加调试控制
6. 使用 `scene_mgr.load_scene()` 加载 FBX 模型
7. 通过 `TruvisApp::<YourApp>::run()` 启动应用

## 帧渲染流程
基于 `app.rs` 中的 `update()` 方法：
1. **Begin Frame**: `renderer.begin_frame()`
2. **Acquire Image**: `window_system.acquire_image()`
3. **Update GUI**: 更新 ImGui 界面，显示相机信息和调试控制
4. **Update Input**: 处理输入事件，更新相机位置
5. **Outer App Update**: 调用 `outer_app.update()`
6. **Before Render**: 准备渲染数据 `renderer.before_render()`
7. **Render**: 构建 PipelineContext，调用 `outer_app.draw()`
8. **After Render**: 完成渲染 `renderer.after_render()`
9. **Draw GUI**: 渲染 GUI 到屏幕
10. **Present**: 呈现最终图像
11. **End Frame**: `renderer.end_frame()`
