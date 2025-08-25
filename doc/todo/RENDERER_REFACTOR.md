# Truvis-Render 结构体重构计划

## 概述

本文档详细分析了 truvis-render 模块中结构体设计存在的问题，并提供了系统性的重构方案。重构目标是优化 Rust 所有权模型的使用、简化依赖关系、提高代码可维护性和运行时性能。

## 当前问题分析

### 1. Rc 使用不当问题

#### 问题描述
- **过度使用 Rc 进行引用共享**：`Renderer` 中多个字段使用 `Rc` 包装，但实际上这些组件并不需要在多个所有者之间共享
- **Rc + RefCell 双重间接访问**：造成运行时借用检查开销和潜在的 panic 风险
- **违反 Rust 所有权原则**：使用 Rc 作为传值的便利手段，而非真正的共享所有权

#### 问题代码
```rust
// 文件：crates/truvis-render/src/renderer/renderer.rs
pub struct Renderer {
    pub rhi: Rc<Rhi>,                              // 不需要共享所有权
    pub frame_ctrl: Rc<FrameController>,           // 不需要共享所有权
    pub bindless_mgr: Rc<RefCell<BindlessManager>>, // 双重间接访问
    pub scene_mgr: Rc<RefCell<SceneManager>>,       // 双重间接访问
    // ...
}
```

#### 影响
- 运行时性能开销（引用计数和借用检查）
- 代码复杂性增加（频繁的 `.borrow()` 和 `.borrow_mut()` 调用）
- 潜在的运行时 panic（借用检查失败）

### 2. 依赖层级混乱问题

#### 问题描述
- **循环依赖**：`SceneManager` 依赖 `BindlessManager`，但两者都被 `Renderer` 直接管理
- **职责边界不清**：`Renderer` 既管理底层设备，又管理高层场景逻辑
- **组件间耦合度过高**：修改一个组件可能影响多个其他组件
- **Rc<RefCell<>> 滥用**：`GpuScene` 等组件中存储 `Rc<RefCell<SceneManager>>` 和 `Rc<RefCell<BindlessManager>>`，造成不必要的运行时开销

#### 当前依赖关系图
```
Renderer (管理所有)
├── Rhi (设备层)
├── FrameController (帧管理层)
├── BindlessManager (资源管理层)
├── SceneManager (场景层，依赖 BindlessManager)
├── GpuScene (GPU数据层，存储 Rc<RefCell<>> 引用)
├── CmdAllocator (命令层)
├── FrameBuffers (资源层)
└── 其他组件...
```

#### 具体问题案例
```rust
// 问题代码：crates/truvis-render/src/renderer/gpu_scene.rs
pub struct GpuScene {
    scene_mgr: Rc<RefCell<SceneManager>>,     // 不必要的 Rc 共享
    bindless_mgr: Rc<RefCell<BindlessManager>>, // 运行时借用检查开销
    // ... 其他字段
}

impl GpuScene {
    pub fn prepare_render_data(&mut self) {
        self.bindless_mgr.borrow_mut().prepare_render_data(...); // 运行时借用
        self.flatten_material_data(&self.scene_mgr.borrow());    // 多次借用
    }
}
```

#### 影响
- 难以单独测试各个组件
- 组件复用困难
- 代码修改风险高
- 运行时借用检查开销和潜在 panic 风险
- 依赖关系不透明，难以理解数据流

### 3. 结构体组织不合理问题

#### 问题描述
- **`Renderer` 过于庞大**：包含 15+ 个字段，职责过多
- **单一职责原则违反**：设备管理、资源管理、场景管理混在一起
- **`PipelineContext` 生命周期复杂**：包含 10+ 个借用，每帧重建

#### 当前 `Renderer` 结构分析
```rust
pub struct Renderer {
    // 设备层 (3 个字段)
    pub rhi: Rc<Rhi>,
    _descriptor_pool: RhiDescriptorPool,
    render_timeline_semaphore: RhiSemaphore,
    
    // 帧管理层 (2 个字段)
    pub frame_ctrl: Rc<FrameController>,
    framebuffers: FrameBuffers,
    
    // 配置层 (3 个字段)
    frame_settings: FrameSettings,
    pipeline_settings: PipelineSettings,
    accum_data: AccumData,
    
    // 资源管理层 (3 个字段)
    pub bindless_mgr: Rc<RefCell<BindlessManager>>,
    pub per_frame_data_buffers: Vec<RhiStructuredBuffer<shader::PerFrameData>>,
    cmd_allocator: CmdAllocator,
    
    // 场景层 (2 个字段)
    pub scene_mgr: Rc<RefCell<SceneManager>>,
    pub gpu_scene: GpuScene,
    
    // 其他 (2 个字段)
    timer: Timer,
    fps_limit: f32,
}
```

#### 影响
- 难以理解和维护
- 测试困难
- 职责不清晰

### 4. 生命周期和数据传递问题

#### 问题描述
- **`PipelineContext` 借用过多**：生命周期复杂，难以扩展
- **频繁的运行时借用检查**：`borrow_mut()` 调用开销
- **数据传递链条过长**：`Renderer` → `PipelineContext` → `Pass` → 具体实现

#### 问题代码
```rust
// 文件：crates/truvis-render/src/render_pipeline/pipeline_context.rs
pub struct PipelineContext<'a> {
    pub rhi: &'a Rhi,
    pub gpu_scene: &'a GpuScene,
    pub bindless_mgr: Rc<RefCell<BindlessManager>>,  // 仍然使用 Rc<RefCell<>>
    pub per_frame_data: &'a RhiStructuredBuffer<shader::PerFrameData>,
    pub frame_ctrl: &'a FrameController,
    pub cmd_allocator: &'a mut CmdAllocator,
    pub frame_settings: &'a FrameSettings,
    pub pipeline_settings: &'a PipelineSettings,
    pub timer: &'a Timer,
    pub frame_buffers: &'a FrameBuffers,
}
```

#### 影响
- 编译时间增加（复杂的生命周期推导）
- 运行时性能开销
- 代码扩展困难

## 重构方案

### 重构目标

1. **消除不必要的 Rc 使用**：仅在真正需要共享所有权时使用 Rc
2. **清晰化依赖层级**：建立清晰的分层架构
3. **优化结构体组织**：按职责拆分大结构体
4. **简化生命周期**：减少借用依赖，提高编译和运行时性能
5. **提高代码可维护性**：降低耦合度，提高组件复用性

### 整体架构设计

#### 新的分层架构
```
应用层 (OuterApp)
    ↓
渲染器层 (Renderer)
    ↓
渲染上下文层 (RenderContext)
    ↓
核心组件层 (RenderCore, RenderResources, SceneContext)
    ↓
RHI 层 (Rhi)
```

#### 核心组件设计

##### 1. RenderCore - 核心设备和帧管理
```rust
// 新文件：crates/truvis-render/src/renderer/render_core.rs
pub struct RenderCore {
    pub rhi: Rhi,
    pub frame_ctrl: FrameController,
    pub cmd_allocator: CmdAllocator,
    pub descriptor_pool: RhiDescriptorPool,
    pub render_timeline_semaphore: RhiSemaphore,
}

impl RenderCore {
    pub fn new(extra_instance_ext: Vec<&'static CStr>) -> Self;
    pub fn begin_frame(&mut self);
    pub fn end_frame(&mut self);
    pub fn wait_idle(&self);
}
```

##### 2. RenderResources - 渲染资源管理
```rust
// 新文件：crates/truvis-render/src/renderer/render_resources.rs
pub struct RenderResources {
    pub framebuffers: FrameBuffers,
    pub bindless_mgr: BindlessManager,
    pub per_frame_data_buffers: Vec<RhiStructuredBuffer<shader::PerFrameData>>,
}

impl RenderResources {
    pub fn new(core: &RenderCore, settings: &RenderSettings) -> Self;
    pub fn resize_framebuffers(&mut self, core: &RenderCore, new_extent: vk::Extent2D);
    pub fn prepare_frame_data(&mut self, core: &RenderCore, frame_data: &FrameData);
}
```

##### 3. SceneContext - 场景和GPU数据
```rust
// 新文件：crates/truvis-render/src/renderer/scene_context.rs
pub struct SceneContext {
    pub scene_mgr: SceneManager,
    pub gpu_scene: GpuScene,
}

impl SceneContext {
    pub fn new(core: &RenderCore) -> Self {
        Self {
            scene_mgr: SceneManager::new(),
            gpu_scene: GpuScene::new(&core.rhi, core.frame_ctrl.clone()),
        }
    }
    
    pub fn update_gpu_data(&mut self, core: &RenderCore, resources: &mut RenderResources) {
        // 使用依赖注入而非存储引用
        self.gpu_scene.prepare_render_data(&self.scene_mgr, &mut resources.bindless_mgr);
        
        let cmd = core.cmd_allocator.alloc_command_buffer("gpu-scene-upload");
        self.gpu_scene.upload_to_buffer(
            &core.rhi, 
            &cmd, 
            RhiBarrierMask::all(),
            &self.scene_mgr,
            &resources.bindless_mgr
        );
    }
    
    pub fn register_default_assets(&mut self, core: &RenderCore, resources: &mut RenderResources) {
        // 注册默认纹理和材质
        self.scene_mgr.register_texture(&mut resources.bindless_mgr, "sky.jpg".to_string());
        self.scene_mgr.register_texture(&mut resources.bindless_mgr, "uv_checker.png".to_string());
    }
}
```

##### 4. RenderSettings - 统一设置管理
```rust
// 新文件：crates/truvis-render/src/renderer/render_settings.rs
#[derive(Clone, Copy)]
pub struct RenderSettings {
    pub frame: FrameSettings,
    pub pipeline: PipelineSettings,
    pub performance: PerformanceSettings,
}

#[derive(Clone, Copy)]
pub struct PerformanceSettings {
    pub fps_limit: f32,
    pub enable_vsync: bool,
}
```

##### 5. 重构后的主 Renderer
```rust
// 文件：crates/truvis-render/src/renderer/renderer.rs
pub struct Renderer {
    pub core: RenderCore,
    pub resources: RenderResources,
    pub scene: SceneContext,
    pub settings: RenderSettings,
    
    // 其他状态
    accum_data: AccumData,
    timer: Timer,
}

impl Renderer {
    pub fn new(extra_instance_ext: Vec<&'static CStr>) -> Self {
        let core = RenderCore::new(extra_instance_ext);
        let settings = RenderSettings::default();
        let mut resources = RenderResources::new(&core, &settings);
        let mut scene = SceneContext::new(&core);
        
        // 注册默认资产
        scene.register_default_assets(&core, &mut resources);
        
        Self {
            core,
            resources,
            scene,
            settings,
            accum_data: AccumData::default(),
            timer: Timer::default(),
        }
    }
    
    pub fn begin_frame(&mut self) {
        self.core.begin_frame();
        self.timer.tic();
    }
    
    pub fn before_render(&mut self, input_state: &InputState, camera: &DrsCamera) {
        // 更新累积数据
        let current_camera_dir = glam::vec3(camera.euler_yaw_deg, camera.euler_pitch_deg, camera.euler_roll_deg);
        self.accum_data.update_accum_frames(current_camera_dir, camera.position);
        
        // 更新 GPU 场景数据
        self.scene.update_gpu_data(&self.core, &mut self.resources);
        
        // 准备帧数据
        let frame_data = self.build_frame_data(input_state, camera);
        self.resources.prepare_frame_data(&self.core, &frame_data);
    }
    
    pub fn collect_render_ctx(&mut self) -> PipelineContext<'_> {
        PipelineContext::new(&self.core, &self.resources, &self.scene, &self.settings)
    }
}
```

##### 6. 简化的 PipelineContext
```rust
// 文件：crates/truvis-render/src/render_pipeline/pipeline_context.rs
pub struct PipelineContext<'a> {
    pub core: &'a RenderCore,
    pub resources: &'a RenderResources,
    pub scene: &'a SceneContext,
    pub settings: &'a RenderSettings,
}

impl<'a> PipelineContext<'a> {
    pub fn new(
        core: &'a RenderCore,
        resources: &'a RenderResources,
        scene: &'a SceneContext,
        settings: &'a RenderSettings,
    ) -> Self {
        Self { core, resources, scene, settings }
    }
    
    // 便利方法
    pub fn rhi(&self) -> &Rhi { &self.core.rhi }
    pub fn frame_ctrl(&self) -> &FrameController { &self.core.frame_ctrl }
    pub fn bindless_mgr(&self) -> &BindlessManager { &self.resources.bindless_mgr }
    pub fn gpu_scene(&self) -> &GpuScene { &self.scene.gpu_scene }
}
```

## 依赖注入设计模式详解

### 核心原则

#### 1. 明确所有权原则
- **直接所有权优先**：组件应当直接拥有其管理的资源，而不是通过 `Rc` 共享
- **按需借用**：只在方法调用时通过参数借用所需的依赖
- **避免存储借用**：不在结构体中存储 `&` 或 `Rc<RefCell<>>` 引用

#### 2. 依赖传递模式
```rust
// ❌ 错误：在结构体中存储共享引用
pub struct ComponentA {
    dependency: Rc<RefCell<ComponentB>>,
}

// ✅ 正确：通过方法参数传递依赖
pub struct ComponentA {
    // 只包含自己的状态
}

impl ComponentA {
    pub fn do_something(&mut self, dependency: &mut ComponentB) {
        // 使用 dependency
    }
}
```

#### 3. 上下文传递模式
对于需要多个依赖的复杂操作，使用上下文对象：

```rust
pub struct RenderContext<'a> {
    pub scene_mgr: &'a SceneManager,
    pub bindless_mgr: &'a mut BindlessManager,
    pub rhi: &'a Rhi,
}

impl GpuScene {
    pub fn render(&mut self, ctx: RenderContext) {
        // 通过 ctx 访问所有依赖
        self.prepare_render_data(ctx.scene_mgr, ctx.bindless_mgr);
        self.upload_to_buffer(ctx.rhi, ctx.scene_mgr, ctx.bindless_mgr);
    }
}
```

### 实际应用案例

#### 案例1：GpuScene 重构
```rust
// 重构前：存储共享引用，运行时开销大
pub struct GpuScene {
    scene_mgr: Rc<RefCell<SceneManager>>,
    bindless_mgr: Rc<RefCell<BindlessManager>>,
    // ... 其他字段
}

impl GpuScene {
    pub fn update(&mut self) {
        let scene_mgr = self.scene_mgr.borrow();  // 运行时借用检查
        let mut bindless_mgr = self.bindless_mgr.borrow_mut();  // 可能 panic
        // ...
    }
}

// 重构后：依赖注入，编译时检查
pub struct GpuScene {
    // 只包含自己的状态
    flatten_instances: Vec<InsGuid>,
    flatten_materials: FlattenMap<MatGuid>,
    gpu_scene_buffers: Vec<GpuSceneBuffers>,
    // ... 其他自有状态
}

impl GpuScene {
    pub fn update(
        &mut self, 
        scene_mgr: &SceneManager,           // 明确的依赖
        bindless_mgr: &mut BindlessManager // 编译时借用检查
    ) {
        // 直接使用，无运行时开销
        self.flatten_material_data(scene_mgr);
        bindless_mgr.prepare_render_data(/*...*/);
    }
}
```

#### 案例2：外部调用模式
```rust
// 调用者负责协调依赖关系
impl Renderer {
    pub fn render_frame(&mut self) {
        // 明确的调用顺序和依赖关系
        self.scene.gpu_scene.prepare_render_data(
            &self.scene.scene_mgr,
            &mut self.resources.bindless_mgr
        );
        
        let cmd = self.core.cmd_allocator.alloc_command_buffer("frame");
        self.scene.gpu_scene.upload_to_buffer(
            &self.core.rhi,
            &cmd,
            RhiBarrierMask::all(),
            &self.scene.scene_mgr,
            &self.resources.bindless_mgr
        );
        
        self.scene.gpu_scene.draw(&cmd, &self.scene.scene_mgr, |instance_idx, submesh_idx| {
            // before_draw callback
        });
    }
}
```

### 设计优势

#### 1. 编译时安全
- **借用检查**：编译器确保借用安全，避免运行时 panic
- **生命周期明确**：通过函数签名清楚表达依赖关系
- **类型安全**：编译时发现类型不匹配问题

#### 2. 性能优势
- **零运行时开销**：无引用计数和动态借用检查
- **内存局部性**：减少间接访问，提高缓存命中率
- **编译优化**：编译器更容易内联和优化

#### 3. 代码质量
- **可测试性**：每个组件可独立测试，依赖可模拟
- **可维护性**：依赖关系明确，修改影响范围可控
- **可扩展性**：新功能更容易添加，不破坏现有结构

#### 4. 开发体验
- **清晰的错误信息**：编译错误更直观
- **更好的 IDE 支持**：代码补全和类型推导更准确
- **调试友好**：调用栈更清晰，变量查看更直观

### 何时使用 Rc<RefCell<>>

依然有少数情况需要使用 `Rc<RefCell<>>`：

#### 1. 真正的共享所有权
```rust
// 多个线程需要共享同一个配置对象
let config = Rc::new(RefCell::new(GlobalConfig::new()));
let config_clone = config.clone();
```

#### 2. 递归或循环引用结构
```rust
// 树或图结构中的父子关系
pub struct Node {
    parent: Option<Weak<RefCell<Node>>>,
    children: Vec<Rc<RefCell<Node>>>,
}
```

#### 3. 回调和事件系统
```rust
// 事件处理器需要在多处注册
pub struct EventSystem {
    handlers: Vec<Rc<RefCell<dyn EventHandler>>>,
}
```

#### 判断标准
- **所有权不明确**：多个组件都需要"拥有"同一个对象
- **生命周期复杂**：对象的生命周期无法通过栈帧管理
- **动态配置**：需要在运行时动态添加/移除依赖关系

### 迁移指南

#### 步骤1：识别不必要的 Rc 使用
```rust
// 检查每个 Rc 使用是否真的需要共享所有权
// 如果只是为了方便传递，考虑改为函数参数
```

#### 步骤2：重构方法签名
```rust
// 将 Rc<RefCell<>> 字段改为方法参数
pub fn method(&mut self, dep1: &Dep1, dep2: &mut Dep2) {
    // ...
}
```

#### 步骤3：更新调用点
```rust
// 调用者负责提供依赖
component.method(&self.dep1, &mut self.dep2);
```

#### 步骤4：验证和测试
```rust
// 确保功能正确性和性能提升
// 运行基准测试和集成测试
```

## 重构实施计划

### 阶段 1：基础重构 (预计 2-3 天)

#### 目标
消除 `Renderer` 中不必要的 Rc 使用，为后续重构打基础。

#### 具体任务

##### 1.1 移除 `rhi` 的 Rc 包装
- **文件**：`crates/truvis-render/src/renderer/renderer.rs`
- **修改**：将 `pub rhi: Rc<Rhi>` 改为 `pub rhi: Rhi`
- **影响文件**：需要更新所有使用 `renderer.rhi.clone()` 的地方

##### 1.2 移除 `frame_ctrl` 的 Rc 包装
- **文件**：`crates/truvis-render/src/renderer/renderer.rs`
- **修改**：将 `pub frame_ctrl: Rc<FrameController>` 改为 `pub frame_ctrl: FrameController`
- **影响文件**：
  - `crates/truvis-render/src/renderer/bindless.rs`
  - `crates/truvis-render/src/renderer/gpu_scene.rs`
  - `crates/truvis-render/src/window_system/main_window.rs`

##### 1.3 创建 RenderSettings 结构体
- **新文件**：`crates/truvis-render/src/renderer/render_settings.rs`
- **内容**：合并 `FrameSettings`、`PipelineSettings` 和新的 `PerformanceSettings`

##### 1.4 更新构造函数
- **修改**：`Renderer::new()` 方法，使用直接所有权而非 Rc

#### 验证标准
- 所有现有的演示应用（triangle、rt-sponza、rt_cornell）正常编译和运行
- 性能基准测试显示无明显性能退化
- 代码中 Rc 使用数量减少 50%

### 阶段 2：依赖关系重构 (预计 3-4 天)

#### 目标
解决 `bindless_mgr` 和 `scene_mgr` 的 Rc<RefCell<>> 问题，建立清晰的依赖层级。

#### 具体任务

##### 2.1 重构 BindlessManager 依赖
- **问题**：当前 `SceneManager` 依赖 `Rc<RefCell<BindlessManager>>`，`GpuScene` 也存储相同的引用
- **解决方案**：使用依赖注入模式，通过方法参数传递而非存储引用

```rust
// 修改前：SceneManager 构造时需要 BindlessManager 引用
impl SceneManager {
    pub fn new(bindless_mgr: Rc<RefCell<BindlessManager>>) -> Self;
}

// 修改后：依赖注入模式
impl SceneManager {
    pub fn new() -> Self;
    pub fn register_texture(&mut self, bindless_mgr: &mut BindlessManager, texture_path: String);
    pub fn load_material(&mut self, bindless_mgr: &mut BindlessManager, material_data: MaterialData);
}

// 修改前：GpuScene 存储 Rc<RefCell<>> 引用
pub struct GpuScene {
    scene_mgr: Rc<RefCell<SceneManager>>,
    bindless_mgr: Rc<RefCell<BindlessManager>>,
    // ...
}

// 修改后：通过方法参数传递依赖
pub struct GpuScene {
    // 移除 Rc<RefCell<>> 字段
    flatten_instances: Vec<InsGuid>,
    flatten_materials: FlattenMap<MatGuid>,
    // ...
}

impl GpuScene {
    pub fn prepare_render_data(
        &mut self, 
        scene_mgr: &SceneManager,
        bindless_mgr: &mut BindlessManager
    ) {
        bindless_mgr.prepare_render_data(self.frame_ctrl.frame_label());
        
        self.flatten_material_data(scene_mgr);
        self.flatten_mesh_data(scene_mgr);
        self.flatten_instance_data(scene_mgr);
    }

    pub fn upload_to_buffer(
        &mut self, 
        rhi: &Rhi, 
        cmd: &RhiCommandBuffer, 
        barrier_mask: RhiBarrierMask,
        scene_mgr: &SceneManager,
        bindless_mgr: &BindlessManager
    ) {
        self.upload_mesh_buffer(cmd, barrier_mask, scene_mgr);
        self.upload_instance_buffer(cmd, barrier_mask, scene_mgr);
        self.upload_material_buffer(cmd, barrier_mask, scene_mgr, bindless_mgr);
        self.upload_light_buffer(cmd, barrier_mask, scene_mgr);
        
        self.build_tlas(rhi, scene_mgr);
        self.upload_scene_buffer(cmd, barrier_mask, scene_mgr, bindless_mgr);
    }

    pub fn draw(&self, cmd: &RhiCommandBuffer, scene_mgr: &SceneManager, mut before_draw: impl FnMut(u32, u32)) {
        for (instance_idx, instance_uuid) in self.flatten_instances.iter().enumerate() {
            let instance = scene_mgr.get_instance(instance_uuid).unwrap();
            let mesh = scene_mgr.get_mesh(&instance.mesh).unwrap();
            for (submesh_idx, geometry) in mesh.geometries.iter().enumerate() {
                cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&geometry.vertex_buffer), &[0]);
                cmd.cmd_bind_index_buffer(&geometry.index_buffer, 0, DrsGeometry3D::index_type());

                before_draw(instance_idx as u32, submesh_idx as u32);
                cmd.draw_indexed(geometry.index_cnt(), 0, 1, 0, 0);
            }
        }
    }
}
```

##### 2.2 移除 Renderer 中的 RefCell 包装
- **修改**：
  ```rust
  // 修改前
  pub bindless_mgr: Rc<RefCell<BindlessManager>>,
  pub scene_mgr: Rc<RefCell<SceneManager>>,
  
  // 修改后
  pub bindless_mgr: BindlessManager,
  pub scene_mgr: SceneManager,
  ```

##### 2.3 更新方法调用
- **影响**：需要更新所有 `.borrow()` 和 `.borrow_mut()` 调用
- **示例**：
  ```rust
  // 修改前
  self.scene_mgr.borrow_mut().load_scene(...)
  
  // 修改后
  self.scene_mgr.load_scene(...)
  ```

##### 2.4 更新 PipelineContext
- **修改**：移除 `Rc<RefCell<BindlessManager>>` 字段
- **影响文件**：所有使用 `PipelineContext` 的渲染管线

#### 验证标准
- 消除所有 `Rc<RefCell<>>` 组合使用
- 编译时间减少 10-20%
- 运行时借用检查开销消除
- 依赖关系明确，通过方法签名可见
- 各组件可独立测试和模拟
- 内存使用更加高效（减少引用计数开销）

### 阶段 3：结构体拆分 (预计 4-5 天)

#### 目标
将庞大的 `Renderer` 按职责拆分为多个更小、更专注的结构体。

#### 具体任务

##### 3.1 创建 RenderCore
- **新文件**：`crates/truvis-render/src/renderer/render_core.rs`
- **职责**：设备管理、帧控制、命令分配
- **包含字段**：
  - `rhi: Rhi`
  - `frame_ctrl: FrameController`
  - `cmd_allocator: CmdAllocator`
  - `descriptor_pool: RhiDescriptorPool`
  - `render_timeline_semaphore: RhiSemaphore`

##### 3.2 创建 RenderResources
- **新文件**：`crates/truvis-render/src/renderer/render_resources.rs`
- **职责**：渲染资源管理
- **包含字段**：
  - `framebuffers: FrameBuffers`
  - `bindless_mgr: BindlessManager`
  - `per_frame_data_buffers: Vec<RhiStructuredBuffer<shader::PerFrameData>>`

##### 3.3 创建 SceneContext
- **新文件**：`crates/truvis-render/src/renderer/scene_context.rs`
- **职责**：场景和 GPU 数据管理
- **包含字段**：
  - `scene_mgr: SceneManager`
  - `gpu_scene: GpuScene`

##### 3.4 重构主 Renderer
- **修改**：使用组合而非直接包含所有组件
- **新结构**：
  ```rust
  pub struct Renderer {
      pub core: RenderCore,
      pub resources: RenderResources,
      pub scene: SceneContext,
      pub settings: RenderSettings,
      // 其他状态
  }
  ```

##### 3.5 更新模块导出
- **文件**：`crates/truvis-render/src/renderer/mod.rs`
- **添加**：新的子模块导出

#### 验证标准
- 每个结构体的字段数量不超过 8 个
- 职责划分清晰，没有跨层访问
- 所有组件可以独立测试

### 阶段 4：PipelineContext 简化 (预计 2-3 天)

#### 目标
简化 `PipelineContext`，减少借用数量，提高生命周期清晰度。

#### 具体任务

##### 4.1 重新设计 PipelineContext
- **当前问题**：包含 10+ 个借用字段
- **新设计**：只包含组件的引用，通过便利方法访问具体字段

```rust
// 新设计
pub struct PipelineContext<'a> {
    pub core: &'a RenderCore,
    pub resources: &'a RenderResources,
    pub scene: &'a SceneContext,
    pub settings: &'a RenderSettings,
}

impl<'a> PipelineContext<'a> {
    // 便利方法，按需访问
    pub fn rhi(&self) -> &Rhi { &self.core.rhi }
    pub fn frame_ctrl(&self) -> &FrameController { &self.core.frame_ctrl }
    pub fn bindless_mgr(&self) -> &BindlessManager { &self.resources.bindless_mgr }
    // ...
}
```

##### 4.2 更新所有 Pass 实现
- **影响文件**：
  - `crates/truvis-render/src/render_pipeline/rt_pass.rs`
  - `crates/truvis-render/src/render_pipeline/phong_pass.rs`
  - `crates/truvis-render/src/render_pipeline/compute_pass.rs`
  - `crates/truvis-render/src/bin/triangle/triangle_pass.rs`

##### 4.3 优化数据预准备
- **目标**：减少运行时数据获取
- **方法**：在 `before_render` 阶段预先准备所有帧相关数据

#### 验证标准
- `PipelineContext` 字段数量不超过 5 个
- 编译时间进一步减少
- 代码可读性显著提高

### 阶段 5：生命周期优化和收尾 (预计 2-3 天)

#### 目标
最终优化生命周期注解，完善文档和测试，确保重构质量。

#### 具体任务

##### 5.1 生命周期优化
- **检查**：所有生命周期注解是否必要和正确
- **简化**：移除不必要的生命周期约束
- **文档**：为复杂的生命周期添加说明注释

##### 5.2 性能测试
- **基准测试**：与重构前的性能对比
- **内存使用**：确认内存使用量没有显著增加
- **编译时间**：确认编译时间有所改善

##### 5.3 代码清理
- **移除**：不再使用的旧代码
- **整理**：导入语句和模块组织
- **格式化**：统一代码风格

##### 5.4 文档更新
- **更新**：`agent_doc/truvis-render.md`
- **添加**：新架构的说明和使用示例
- **修改**：README 中的相关说明

##### 5.5 测试完善
- **单元测试**：为新的组件添加单元测试
- **集成测试**：确保所有演示应用正常工作
- **边界情况**：测试错误处理和边界情况

#### 验证标准
- 所有测试通过
- 性能不低于重构前
- 代码复杂度指标改善
- 文档完整且准确

## 风险评估和缓解策略

### 主要风险

#### 1. 编译错误风险
- **风险**：大规模重构可能导致大量编译错误
- **缓解**：逐步进行，每个阶段确保编译通过后再进行下一阶段

#### 2. 性能回退风险
- **风险**：重构可能意外影响性能
- **缓解**：在每个阶段进行性能基准测试，及时发现问题

#### 3. 功能破坏风险
- **风险**：重构可能破坏现有功能
- **缓解**：保持所有演示应用正常运行作为验证标准

#### 4. 时间超期风险
- **风险**：重构可能花费超出预期的时间
- **缓解**：每个阶段设定明确的时间限制和验证标准

### 缓解策略

#### 1. 分支策略
- 在 `refactor/renderer-restructure` 分支进行重构
- 每个阶段完成后合并到主分支
- 保持主分支始终可用

#### 2. 备份策略
- 重构前创建完整的代码备份
- 每个阶段完成后创建快照
- 出现问题时能够快速回滚

#### 3. 测试策略
- 重构前运行完整的测试套件，记录基准
- 每个阶段都要确保所有测试通过
- 添加必要的新测试覆盖新代码

## 预期收益

### 代码质量提升
- **可维护性**：组件职责清晰，依赖关系透明，易于理解和修改
- **可测试性**：各组件可独立测试，依赖可轻松模拟，提高测试覆盖率
- **可扩展性**：新功能更容易添加，不会影响现有组件
- **类型安全**：编译时发现更多问题，减少运行时错误

### 性能改善
- **编译时间**：减少复杂的生命周期推导和借用检查，提高编译速度
- **运行时性能**：消除 Rc 引用计数和 RefCell 借用检查开销（预计提升 5-15%）
- **内存使用**：减少间接访问和引用计数开销，提高内存局部性
- **CPU 缓存效率**：更好的数据局部性，减少缓存未命中

### 开发体验
- **错误信息**：更清晰的编译错误信息，借用检查在编译时完成
- **IDE 支持**：更好的代码补全、类型推导和重构支持
- **调试体验**：更直观的调用栈和变量查看，减少间接引用
- **代码可读性**：依赖关系通过方法签名明确表达，易于理解数据流

## 后续维护建议

### 代码规范
1. **所有权原则**：优先使用直接所有权，仅在真正需要共享时使用 Rc
2. **借用原则**：通过函数参数传递借用，避免在结构体中存储借用
3. **生命周期原则**：保持生命周期简单明确，避免过度复杂的约束
4. **依赖注入原则**：使用方法参数传递依赖，而不是在结构体中存储引用

### 架构原则
1. **单一职责**：每个结构体只负责一个明确的功能域
2. **依赖倒置**：高层组件不应直接依赖低层实现细节  
3. **开闭原则**：对扩展开放，对修改封闭
4. **明确依赖**：依赖关系应通过方法签名明确表达，而非隐藏在 Rc 中

### 性能原则
1. **零成本抽象**：避免运行时开销，优先使用编译时检查
2. **内存局部性**：减少间接引用，提高缓存命中率
3. **编译时优化**：设计有利于编译器优化的代码结构

### 持续改进
1. 定期审查组件设计，及时发现和解决架构问题
2. 监控性能指标，确保架构改进不会带来性能退化
3. 收集开发者反馈，持续优化开发体验
4. 建立代码审查流程，确保新代码遵循设计原则

---

*本文档创建于 2025年8月26日，更新于 2025年8月26日，用于指导 Truvis-Render 模块的结构体重构工作。*

## 重构总结

本次重构的核心是从**共享引用模式**向**依赖注入模式**的转变：

### 关键改变
1. **移除 Rc<RefCell<>> 滥用**：从运行时借用检查转向编译时借用检查
2. **明确依赖关系**：通过方法签名表达依赖，提高代码可读性
3. **提高性能**：消除引用计数和动态借用检查的开销
4. **增强安全性**：编译时发现更多借用相关问题

### 设计哲学
- **明确胜于隐式**：依赖关系应当明确可见
- **编译时胜于运行时**：尽可能在编译时解决问题
- **直接胜于间接**：优先使用直接所有权和借用
- **简单胜于复杂**：保持结构和生命周期的简单性

这一重构不仅解决了当前的性能和维护性问题，更为项目未来的扩展奠定了坚实的架构基础。
