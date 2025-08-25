# App 架构重构方案

## 问题分析

### 当前架构的问题

1. **耦合问题**：
   - ImGui 依赖后续 Renderer 绘制的结果作为纹理显示
   - MainWindow 依赖 Renderer 的 BindlessManager 等组件
   - 组件间的依赖关系不直观

2. **循环依赖**：
   - UI 布局决定视口大小 → Renderer Framebuffer 尺寸 → 渲染结果 → UI 显示
   - 存在时序问题和一帧延迟

3. **职责不清**：
   - MainWindow 承担了窗口管理和 GUI 渲染的双重职责
   - Renderer 和 Window 看似解耦但实际存在隐式依赖

## 重构目标

1. **解耦 Window 和 Renderer**：让两者完全独立
2. **优化依赖关系**：让 ImGui 对渲染结果的依赖更直观
3. **消除时序问题**：避免 UI 设置变更的一帧延迟
4. **职责分离**：每个组件只负责自己的核心功能

## 重构方案

### 1. 整体架构设计

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MainWindow    │    │ RenderCoordinator│    │   GuiRenderer   │
│                 │    │                 │    │                 │
│ - 窗口管理      │    │ - 流程协调      │    │ - UI 逻辑       │
│ - Swapchain     │◄──►│ - 数据传递      │◄──►│ - UI 渲染       │
│ - 事件处理      │    │ - 依赖管理      │    │ - 布局计算      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌─────────────────┐
                       │    Renderer     │
                       │                 │
                       │ - 3D 场景渲染   │
                       │ - 资源管理      │
                       │ - 管线控制      │
                       └─────────────────┘
```

### 2. 核心组件设计

#### 2.1 RenderCoordinator（新增组件）

负责协调整个渲染流程，作为各组件间的中间层：

```rust
pub struct RenderCoordinator {
    renderer: Renderer,
    gui_renderer: GuiRenderer,
    frame_data: FrameData,
    current_framebuffer_extent: vk::Extent2D,
}

impl RenderCoordinator {
    pub fn begin_frame(&mut self) -> FrameData;
    pub fn render_scene(&mut self, frame_data: &FrameData) -> RenderResult;
    pub fn render_gui_final(&mut self, layout_result: &UiLayoutResult, render_result: &RenderResult) -> GuiResult;
    pub fn composite_to_swapchain(&mut self, swapchain_frame: SwapchainFrame, gui_result: &GuiResult);
    pub fn end_frame(&mut self);
    pub fn resize_framebuffer(&mut self, new_extent: vk::Extent2D);
    pub fn current_framebuffer_extent(&self) -> vk::Extent2D;
}
```

#### 2.2 FrameData（新增数据结构）

承载单帧渲染所需的所有数据：

```rust
pub struct FrameData {
    pub render_settings: RenderSettings,
    pub camera_info: CameraInfo,
    pub elapsed: Duration,
    pub frame_index: u64,
    // 保存上一帧的渲染纹理，用于UI占位显示
    pub last_render_texture: Option<String>,
}

pub struct RenderSettings {
    pub light_intensity: f32,
    pub enable_shadows: bool,
    pub background_color: [f32; 3],
    pub exposure: f32,
    pub gamma: f32,
    // 其他渲染参数...
}
```

#### 2.3 GuiRenderer（重构）

从 MainWindow 中提取 GUI 相关逻辑，支持两阶段处理：

```rust
pub struct GuiRenderer {
    gui: Gui,
    gui_pass: GuiPass,
    bindless_mgr: Rc<RefCell<BindlessManager>>,
    layout_cache: Option<UiLayoutResult>,
}

impl GuiRenderer {
    pub fn prepare_frame(&mut self, window: &Window, elapsed: Duration);
    
    // 第一阶段：布局计算和逻辑更新
    pub fn layout_pass<F>(&mut self, layout_fn: F) -> UiLayoutResult 
    where F: FnOnce(&imgui::Ui) -> UiLayoutResult;
    
    // 第二阶段：最终渲染
    pub fn render_final(&mut self, layout_result: &UiLayoutResult, render_result: &RenderResult) -> GuiResult;
}
```

#### 2.4 MainWindow（简化）

只负责窗口管理和 Swapchain 操作：

```rust
pub struct MainWindow {
    rhi: Rc<Rhi>,
    winit_window: Window,
    swapchain: Option<RenderSwapchain>,
    frame_ctrl: Rc<FrameController>,
    present_complete_semaphores: Vec<RhiSemaphore>,
    render_complete_semaphores: Vec<RhiSemaphore>,
}

impl MainWindow {
    pub fn acquire_next_frame(&mut self) -> SwapchainFrame;
    pub fn present_frame(&self, frame: SwapchainFrame, wait_semaphores: &[RhiSemaphore]);
    pub fn window(&self) -> &Window;
    // 移除所有 GUI 相关方法
}
```

### 3. 数据流设计

#### 3.1 两阶段 UI 处理流程

```
阶段1: UI布局 → 确定视口大小 → 调整Framebuffer → 更新渲染设置
阶段2: 3D渲染 → 获取渲染结果 → UI最终渲染 → 合成显示
```

#### 3.2 详细数据流

```
Input Events → UI Layout Calculation → Viewport Size → Framebuffer Resize
     ↓                                                        ↓
Camera Update ← Render Settings Update ← UI Logic Update     3D Rendering
     ↓                                                        ↓
Scene Update → 3D Scene Rendering → Render Result → UI Final Rendering
     ↓                                                        ↓
Frame Data → Composite to Swapchain → Present → End Frame
```

### 4. 主循环重构

```rust
impl<T: OuterApp> TruvisApp<T> {
    fn update(&mut self) {
        if !self.render_coordinator.time_to_render() {
            return;
        }

        // === 第1阶段：获取帧和基础更新 ===
        let swapchain_frame = self.window_system.get_mut().unwrap().acquire_next_frame();
        let mut frame_data = self.render_coordinator.begin_frame();
        
        // === 第2阶段：UI布局和逻辑更新 ===
        self.update_input_and_camera(&mut frame_data);
        let ui_layout_result = self.update_ui_layout_and_logic(&mut frame_data);
        
        // === 第3阶段：根据UI布局调整renderer ===
        self.adjust_renderer_for_viewport(&ui_layout_result);
        
        // === 第4阶段：外部应用更新和3D渲染 ===
        self.outer_app.get_mut().unwrap().update(&mut frame_data);
        let render_result = self.render_coordinator.render_scene(&frame_data);
        
        // === 第5阶段：UI最终渲染 ===
        let gui_result = self.render_coordinator.render_gui_final(&ui_layout_result, &render_result);
        
        // === 第6阶段：合成和呈现 ===
        self.render_coordinator.composite_to_swapchain(swapchain_frame, &gui_result);
        self.window_system.get_mut().unwrap().present_frame(swapchain_frame, &gui_result.wait_semaphores);
        self.render_coordinator.end_frame();
    }
}
```

#### 4.1 UI布局和逻辑更新

```rust
fn update_ui_layout_and_logic(&mut self, frame_data: &mut FrameData) -> UiLayoutResult {
    let gui = &mut self.render_coordinator.gui_renderer;
    gui.prepare_frame(&self.window_system.get().unwrap().window(), frame_data.elapsed);
    
    let mut render_settings = frame_data.render_settings.clone();
    let mut viewport_info = ViewportInfo::default();
    
    gui.layout_pass(|ui| {
        // === 主视口区域布局 ===
        ui.window("Main Viewport")
            .size([800.0, 600.0], imgui::Condition::FirstUseEver)
            .resizable(true)
            .build(|| {
                let content_region = ui.content_region_avail();
                viewport_info.size = [content_region[0], content_region[1]];
                viewport_info.position = ui.cursor_screen_pos();
                
                // 显示占位符或上一帧的内容
                if let Some(last_texture) = &frame_data.last_render_texture {
                    ui.image(last_texture, viewport_info.size);
                } else {
                    ui.dummy(viewport_info.size);
                    ui.text("Preparing render...");
                }
            });
        
        // === 控制面板 ===
        ui.window("Controls")
            .size([300.0, 400.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.slider("Light Intensity", 0.0, 10.0, &mut render_settings.light_intensity);
                ui.checkbox("Enable Shadows", &mut render_settings.enable_shadows);
                ui.color_picker("Background Color", &mut render_settings.background_color);
                
                // 性能统计
                ui.separator();
                ui.text("Performance:");
                ui.text(format!("Frame Time: {:.2}ms", frame_data.elapsed.as_millis()));
                ui.text(format!("FPS: {:.1}", 1000.0 / frame_data.elapsed.as_millis() as f32));
            });
    });
    
    // 立即更新渲染设置，确保当前帧生效
    frame_data.render_settings = render_settings;
    
    UiLayoutResult {
        viewport_info,
        render_settings,
    }
}
```

#### 4.2 视口大小调整

```rust
fn adjust_renderer_for_viewport(&mut self, layout_result: &UiLayoutResult) {
    let new_extent = vk::Extent2D {
        width: layout_result.viewport_info.size[0].max(1.0) as u32,
        height: layout_result.viewport_info.size[1].max(1.0) as u32,
    };
    
    // 只有当视口大小真正改变时才重建framebuffer
    if self.render_coordinator.current_framebuffer_extent() != new_extent {
        log::debug!("Viewport size changed to: {}x{}", new_extent.width, new_extent.height);
        self.render_coordinator.resize_framebuffer(new_extent);
    }
}
```

### 5. 关键数据结构

#### 5.1 UI相关结构

```rust
#[derive(Clone, Debug)]
pub struct ViewportInfo {
    pub size: [f32; 2],
    pub position: [f32; 2],
}

pub struct UiLayoutResult {
    pub viewport_info: ViewportInfo,
    pub render_settings: RenderSettings,
}

pub struct UiContent {
    pub main_ui: Box<dyn FnOnce(&imgui::Ui, [f32; 2])>,
    pub sidebar_ui: Box<dyn FnOnce(&imgui::Ui)>,
}
```

#### 5.2 渲染相关结构

```rust
pub struct RenderResult {
    pub texture_handle: String,
    pub render_stats: RenderStats,
    pub wait_semaphores: Vec<RhiSemaphore>,
}

pub struct GuiResult {
    pub commands: Vec<RhiCommandBuffer>,
    pub wait_semaphores: Vec<RhiSemaphore>,
}

pub struct SwapchainFrame {
    pub image: RhiTexture2D,
    pub image_available_semaphore: RhiSemaphore,
    pub frame_index: u32,
}
```

### 6. 性能优化

#### 6.1 Framebuffer 重建优化

```rust
impl RenderCoordinator {
    pub fn resize_framebuffer(&mut self, new_extent: vk::Extent2D) {
        // 添加容差，避免频繁重建
        const SIZE_TOLERANCE: u32 = 4;
        
        let current = self.current_framebuffer_extent();
        let width_diff = (new_extent.width as i32 - current.width as i32).abs() as u32;
        let height_diff = (new_extent.height as i32 - current.height as i32).abs() as u32;
        
        if width_diff > SIZE_TOLERANCE || height_diff > SIZE_TOLERANCE {
            log::debug!("Resizing framebuffer: {:?} -> {:?}", current, new_extent);
            self.renderer.resize_frame_buffer(new_extent);
        }
    }
}
```

#### 6.2 UI布局缓存

```rust
pub struct GuiRenderer {
    // 缓存UI布局，避免重复计算
    layout_cache: Option<UiLayoutResult>,
    // 缓存渲染状态，减少状态切换
    last_render_state: Option<GuiRenderState>,
}
```

### 7. 边界情况处理

#### 7.1 初始化

- **首帧渲染**：使用默认视口大小，显示加载占位符
- **资源未就绪**：优雅降级，显示错误信息

#### 7.2 运行时异常

- **窗口最小化**：保持最小渲染尺寸（1x1），避免零尺寸
- **GPU设备丢失**：提供重建机制
- **内存不足**：降低渲染质量或尺寸

#### 7.3 用户交互

- **快速拖拽窗口**：使用容差减少频繁重建
- **极端视口尺寸**：设置合理的最小/最大限制

### 8. 迁移计划

#### 8.1 第一阶段：数据结构重构

1. 创建 `RenderCoordinator` 和相关数据结构
2. 定义新的 `FrameData` 和 `UiLayoutResult`
3. 修改现有接口以适配新结构

#### 8.2 第二阶段：GUI重构

1. 从 `MainWindow` 中提取 `GuiRenderer`
2. 实现两阶段UI处理逻辑
3. 更新UI相关的数据流

#### 8.3 第三阶段：主循环重构

1. 重写 `TruvisApp::update` 方法
2. 实现新的渲染流程
3. 测试和调试

#### 8.4 第四阶段：优化和完善

1. 性能优化
2. 错误处理完善
3. 文档更新

### 9. 预期收益

#### 9.1 架构收益

- **解耦**：Window、Renderer、GUI 完全独立
- **可测试性**：各组件可独立测试
- **可维护性**：职责清晰，易于理解和修改

#### 9.2 性能收益

- **零延迟**：UI设置变更立即生效
- **资源优化**：减少不必要的framebuffer重建
- **渲染效率**：更好的命令缓冲区管理

#### 9.3 用户体验收益

- **响应性**：实时参数调整体验
- **稳定性**：更好的错误处理和恢复
- **扩展性**：易于添加新的UI功能

### 10. 风险和注意事项

#### 10.1 技术风险

- **复杂性增加**：引入了更多的中间层
- **性能开销**：两阶段UI处理可能带来额外开销
- **状态同步**：需要确保各组件状态一致性

#### 10.2 迁移风险

- **兼容性**：现有代码需要大幅修改
- **调试难度**：新架构可能增加调试复杂度
- **时间成本**：重构工作量较大

#### 10.3 缓解措施

- **渐进式迁移**：分阶段进行，每个阶段都可独立测试
- **兼容性层**：保留旧接口一段时间，平滑过渡
- **充分测试**：为每个组件编写单元测试和集成测试

## 总结

这个重构方案通过引入 `RenderCoordinator` 作为协调层，实现了 Window、Renderer 和 GUI 的完全解耦。通过两阶段UI处理，解决了视口大小依赖和输入延迟问题。虽然增加了一些复杂性，但显著提升了架构的清晰度、可维护性和用户体验。

重构需要谨慎进行，建议采用渐进式迁移策略，确保每个阶段都经过充分测试验证。
