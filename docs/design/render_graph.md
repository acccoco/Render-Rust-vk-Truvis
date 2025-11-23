# Render Graph Design

## 1. 目标 (Goals)
- **声明式定义**: 将渲染流程描述为有向无环图 (DAG)，解耦资源管理与逻辑执行。
- **自动同步**: 自动推导 Image Layout 转换和 Memory Barriers，消除手动同步错误。
- **资源复用**: 支持 Transient Resources（瞬态资源），自动复用内存（Aliasing）。
- **多管线支持**: 统一支持 Graphics (Raster), Compute, Ray Tracing。

## 2. 核心概念 (Core Concepts)

### 2.1 资源 (Resources)
- **VirtualResource**: 图中定义的虚拟资源（Image/Buffer）。
- **ResourceHandle**: 资源的轻量级句柄，用于 Pass 之间传递依赖。
- **Imported Resource**: 外部传入的资源（如 Swapchain Image, History Buffer）。
- **Transient Resource**: 图内部创建的临时资源，生命周期仅限于当前帧。

### 2.2 节点 (PassNode)
- **Pass**: 渲染图的基本执行单元。
- **Dependencies**:
  - **Read**: 读取资源（作为 Texture, UBO, SRV 等）。
  - **Write**: 写入资源（作为 Color Attachment, Storage Image, UAV 等）。
- **Execute Closure**: 实际记录 Command Buffer 的回调函数。

### 2.3 图 (RenderGraph)
- **Builder**: 用于构建图，添加 Pass 和资源。
- **Compiler**: 拓扑排序，计算资源生命周期，插入 Barriers。
- **Executor**: 实际分配物理资源，执行回调。

## 3. API 设计 (API Design)

```rust
// 伪代码示例

// 1. 初始化 Graph
let mut graph = RenderGraph::new();

// 2. 导入外部资源
let backbuffer = graph.import_image(swapchain_image, "Backbuffer");
let depth_img = graph.import_image(depth_image, "Depth");

// 3. 创建瞬态资源 (可选，也可以在 Pass 中创建)
let gbuffer_desc = ImageDesc::new_2d(extent, format, usage);
let gbuffer_pos = graph.create_image(gbuffer_desc, "GBufferPos");

// 4. 定义 Pass: G-Buffer Pass
graph.add_pass("GBuffer Pass")
    .color_attachment(gbuffer_pos, LoadOp::Clear, StoreOp::Store)
    .depth_attachment(depth_img, LoadOp::Clear, StoreOp::Store)
    .execute(|cmd, resources| {
        // 渲染逻辑
        let pipeline = resources.get_pipeline("gbuffer");
        cmd.bind_pipeline(pipeline);
        cmd.draw(...);
    });

// 5. 定义 Pass: Lighting Pass (Compute)
graph.add_pass("Lighting Pass")
    .read(gbuffer_pos) // 自动转换为 ShaderReadOnlyOptimal
    .write(backbuffer) // 自动转换为 General 或 Storage
    .execute(|cmd, resources| {
        // Compute Dispatch
        cmd.dispatch(...);
    });

// 6. 编译与执行
graph.compile();
graph.execute(cmd_buffer);
```

## 4. 详细设计 (Detailed Design)

### 4.1 资源状态追踪 (Resource State Tracking)
每个资源在图中维护一个状态机：
- `initial_state`: 导入时的状态。
- `current_state`: 遍历 Pass 时的当前状态。
- Pass 声明需求 (`required_state`)：
  - `ShaderRead`: 需要 `SHADER_READ_ONLY_OPTIMAL` + `ACCESS_SHADER_READ`
  - `ColorAttachment`: 需要 `COLOR_ATTACHMENT_OPTIMAL` + `ACCESS_COLOR_ATTACHMENT_WRITE`
  - `StorageWrite`: 需要 `GENERAL` + `ACCESS_SHADER_WRITE`

**Barrier 插入逻辑**:
在 Pass 开始前，检查 `current_state` 是否满足 `required_state`。如果不满足，插入 `PipelineBarrier` 并更新 `current_state`。

### 4.2 物理资源管理 (Physical Resource Management)
- **ResourceCache**: 缓存实际的 `vk::Image` 和 `vk::Buffer`。
- **Aliasing**: 对于瞬态资源，如果两个资源生命周期不重叠，可以复用同一块 `vk::DeviceMemory` (暂不实现，作为进阶优化)。

### 4.3 执行流程 (Execution Flow)
1. **Setup**: 构建 DAG，解析依赖。
2. **Compile**:
   - 线性化 Pass (拓扑排序)。
   - 计算 Barrier 位置。
   - 分配物理资源 (对于 Transient)。
3. **Execute**:
   - 遍历线性化的 Pass。
   - 执行 Pre-Pass Barriers。
   - 开始 RenderPass (如果是 Raster)。
   - 调用 `execute` 回调。
   - 结束 RenderPass。

## 5. 演进路线 (Roadmap)

### Phase 1: 基础架构
- 实现 `RenderGraph` 结构体。
- 实现 `PassBuilder` 和基础的资源引用 (`ResourceHandle`)。
- 实现简单的 Barrier 插入 (基于全局状态，暂不考虑细粒度 Subresource)。
- 替换现有的 `RtRenderPass` 手动 Barrier 逻辑。

### Phase 2: 自动 RenderPass 合并
- 自动检测连续的 Raster Pass，如果兼容则合并为一个 Vulkan Render Pass (Subpasses)。

### Phase 3: 异步计算 (Async Compute)
- 识别独立的 Compute Pass 子图。
- 提交到 Compute Queue 执行。
- 处理 Queue Ownership Transfer Barriers。

### Phase 4: 瞬态资源优化
- 实现内存复用 (Memory Aliasing)。

## 6. 现有系统集成 (Integration)
- `RenderGraph` 将位于 `truvis-render` crate 中。
- `FrameContext` 将作为 `RenderGraph` 的上下文提供者。
- 现有的 `OuterApp` 可以在 `draw` 方法中构建并执行 Graph。

### 6.1 案例分析与覆盖 (Use Case Coverage)

#### Case 1: ShaderToy / Triangle (Raster)
- **需求**: 简单的光栅化 Pass，写入 `render_target`。
- **实现**:
  - 导入 `render_target` (Initial: `Undefined` / `General`).
  - 定义 Raster Pass，声明 `color_attachment(render_target)`.
  - 在 `execute` 中绑定 Pipeline, PushConstants (Time, Mouse), Draw.
  - Graph 自动插入 Barrier: `Undefined` -> `ColorAttachment`.

#### Case 2: RT-Cornell (RayTracing + Compute)
- **需求**: RT Pass (Write Storage) -> Barrier -> Compute Pass (Read Storage, Write Target).
- **实现**:
  - 创建瞬态资源 `rt_output` (Storage Image).
  - 导入 `render_target`.
  - **Pass 1 (RT)**: `write(rt_output)`.
  - **Pass 2 (ToneMapping)**: `read(rt_output)`, `write(render_target)`.
  - Graph 自动插入 Barrier: `rt_output` (Storage Write -> Shader Read)。

### 6.2 关键细节补充 (Refinements)

#### 资源状态转换 (State Transitions)
- **Imported Resources**: 需指定 `initial_state` (当前实际状态) 和 `final_state` (图执行完后的期望状态)。
  - 例如 `render_target`: Initial=`General`, Final=`General` (供 ImGui 使用).
- **Pipeline Data**: `PushConstants` 和 `DescriptorSets` (如 Bindless) 继续在 `execute` 闭包中通过 `cmd` 绑定，Graph 仅负责 Resource Barrier。

#### ImGui 与 Present
- ImGui 通常在 RenderGraph 执行之后运行。
- RenderGraph 的职责是生成场景图像并写入 `render_target`。
- 确保 `render_target` 在 Graph 结束时处于 `General` 或 `ColorAttachment` 状态，以便后续 ImGui Pass 使用。

## 7. 实现规范 (Implementation Specifications)

为了指导具体编码，以下定义核心数据结构和接口。

### 7.1 句柄与资源 (Handles & Resources)

使用索引作为句柄，避免生命周期复杂性。

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResourceHandle(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PassHandle(u32);

pub enum ResourceType {
    Image(ImageDesc),
    Buffer(BufferDesc),
}

pub struct VirtualResource {
    pub name: String,
    pub ty: ResourceType,
    pub initial_state: ResourceState, // 导入时的状态
    // 物理资源引用 (仅在 Execute 阶段有效)
    pub physical_index: Option<usize>, 
}
```

### 7.2 节点定义 (Pass Node)

```rust
pub struct PassNode {
    pub name: String,
    pub inputs: Vec<ResourceAccess>,
    pub outputs: Vec<ResourceAccess>,
    // 使用 Box<dyn Fn> 存储回调，需处理生命周期
    pub execute: Box<dyn Fn(&CommandBuffer, &RenderContext) + 'static>, 
}

pub struct ResourceAccess {
    pub handle: ResourceHandle,
    pub access_type: AccessType, // Read/Write/ReadWrite
    pub layout: vk::ImageLayout,
    pub stage: vk::PipelineStageFlags2,
    pub access: vk::AccessFlags2,
}
```

### 7.3 执行上下文 (RenderContext)

这是 Pass 闭包与物理资源交互的桥梁。

```rust
pub struct RenderContext<'a> {
    // 引用物理资源池
    resource_cache: &'a ResourceCache,
    // 引用当前帧的资源映射
    resource_map: &'a HashMap<ResourceHandle, PhysicalResourceId>,
}

impl<'a> RenderContext<'a> {
    pub fn get_image(&self, handle: ResourceHandle) -> vk::Image { ... }
    pub fn get_image_view(&self, handle: ResourceHandle) -> vk::ImageView { ... }
    pub fn get_buffer(&self, handle: ResourceHandle) -> vk::Buffer { ... }
}
```

### 7.4 编译器逻辑 (Compiler Logic)

1.  **拓扑排序**: 确定 Pass 执行顺序。
2.  **资源生命周期分析**:
    *   遍历排序后的 Pass。
    *   记录每个资源最后一次被使用的 Pass Index。
    *   记录每个资源的当前状态 (`current_state`)。
3.  **Barrier 插入**:
    *   在 Pass `i` 之前，检查其所有输入/输出资源。
    *   如果 `resource.current_state != required_state`:
        *   生成 `ImageMemoryBarrier`。
        *   更新 `resource.current_state = required_state`。
    *   将生成的 Barriers 存储在 `barrier_points: HashMap<PassHandle, Vec<Barrier>>` 中。

### 7.5 执行器逻辑 (Executor Logic)

```rust
pub fn execute(&mut self, cmd: &CommandBuffer) {
    for pass in &self.sorted_passes {
        // 1. 插入 Pre-Pass Barriers
        if let Some(barriers) = self.barriers.get(&pass.id) {
            cmd.pipeline_barrier(barriers);
        }

        // 2. 准备上下文
        let ctx = RenderContext::new(self.resources, ...);

        // 3. 执行用户回调
        (pass.execute)(cmd, &ctx);
    }
}
```
