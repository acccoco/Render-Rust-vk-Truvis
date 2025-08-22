# Copilot Instructions for Render-Rust-vk-Truvis

## 项目架构与核心模块
**crates/**：核心 Rust 库，模块化设计，主要子模块：
  - `truvis-rhi/`：Vulkan 渲染硬件接口（RHI），**底层资源与命令抽象的核心**。
    - 结构：`src/core/` 细分为 device、command、pipeline、descriptor、synchronize、allocator、buffer/image 等子模块，`resources/` 进一步细化 buffer/image/special_buffers。
    - 主要职责：
      - 设备与物理设备管理（device/physical_device.rs）
      - 队列与命令池/缓冲区（command_queue/pool/buffer.rs）
      - 内存分配与资源生命周期（allocator.rs、buffer.rs、image.rs）
      - 管线与渲染状态（graphics_pipeline.rs、shader.rs）
      - 描述符与同步（descriptor.rs、synchronize.rs）
      - 特殊缓冲区（special_buffers/vertex_buffer.rs、index_buffer.rs、stage_buffer.rs等）
    - 设计理念：**高内聚、低耦合**，为上层渲染/场景/管线提供稳定高效的 Vulkan 封装，便于扩展和跨平台。
  - `truvis-render/`：主渲染库，负责渲染器、场景、相机、GUI、渲染管线。
    - 结构：`src/` 细分为 renderer、scene、camera、gui、render_pipeline、outer_app 等模块。
    - 主要职责：
      - 渲染器生命周期与主循环管理（renderer.rs）
      - 场景/实体/相机系统（scene/、camera/）
      - GUI 集成（imgui/）
      - 渲染管线扩展点：`render_pipeline/` 目录下实现 `RenderPipeline` trait，可插拔多种渲染流。
      - 应用扩展点：实现 `OuterApp` trait，定制 init/draw_ui/draw 等生命周期。
    - 推荐模式：每种渲染管线配套一个 demo（`src/bin/`），便于隔离测试和演示。
  - `shader-binding/`：Slang 着色器与 Rust 自动绑定，管理描述符布局。
    - 结构：`src/` 生成 Rust 端绑定，`build.rs` 负责自动化 Slang 头文件/布局同步。
    - 主要职责：
      - 解析 Slang shader 头文件，自动生成 Rust 侧的 descriptor/struct 绑定
      - 保证 shader/Rust 端布局一致性，减少手动同步错误
      - 支持多 pipeline、bindless、push constant 等高级特性
  - `shader/`：Slang shader 源码、头文件、构建工具。
    - 结构：
      - `src/`：各渲染管线/模块的 Slang 源码（如 hello_triangle/、phong/、rt/ 等）
      - `include/`：Slang 公共头文件（如 common.slangi、frame_data.slangi 等）
      - `shader-binding/`、`shader-build/`：绑定生成与构建工具
    - 典型流程：
      1. 在 `shader/src/` 新建 Slang 源码，复用 `include/` 公共头
      2. 运行 `cargo run --bin build_shader`，自动生成 Rust 绑定
      3. Rust 端通过 `shader-binding` crate 直接使用
    - 约定：Slang 头文件以 `.slangi` 结尾，支持 include 复用；shader 变量布局需与 Rust 端严格一致
  - `model-manager/`：3D 模型/场景/材质/几何体管理，支持多格式
  - `truvis-cxx/`：C++集成（如Assimp），通过CMake和build.rs自动链接
  - 其它如`shader-layout-*`、`truvis-crate-tools/`为工具/trait支持
- **shader/**：Slang着色器源码、头文件、绑定生成、构建工具
- **assets/**、**resources/**：运行时资源
- **tools/**：开发辅助工具

## 构建与运行
- **完整构建**：
  ```bash
  cargo build --release
  ```
- **Shader 编译**：
  ```bash
  cargo run --bin build_shader
  ```
- **运行示例**：
  ```bash
  cargo run --bin triangle         # 基础三角形
  cargo run --bin rt-sponza       # 光线追踪 Sponza
  cargo run --bin rt_cornell      # Cornell Box
  cargo run --bin shader_toy      # Shader Toy
  ```
- **C++依赖**：CMake自动编译，dll自动复制到`target/debug/`，无需手动配置link属性

## 约定与模式
- **渲染管线扩展**：在`truvis-render/src/render_pipeline/`新建模块，实现`RenderPipeline` trait，并在`src/bin/`添加示例
- **Shader开发**：在`shader/src/`下用Slang语法编写，运行`build_shader`自动生成绑定
- **自定义应用**：实现`OuterApp` trait，参考`truvis_render::outer_app::OuterApp`，重写`init`/`draw_ui`/`draw`方法
- **模型/顶点扩展**：在`model-manager/src/vertex/`添加新顶点类型，配合trait实现

## 重要文件/目录
- `crates/truvis-rhi/src/core/`：Vulkan资源/命令/同步等底层实现
- `shader/include/`：Slang头文件，供shader复用
- `shader/shader-binding/`：Rust与Shader绑定生成逻辑
- `truvis-cxx/cxx/`：C++源文件与CMake集成

## 典型开发流程
1. 新增shader：`shader/src/`编写 → `cargo run --bin build_shader` → Rust端自动可用
2. 新增渲染管线：`truvis-render/src/render_pipeline/`实现trait → `src/bin/`写demo
3. 集成C++库：CMakeLists配置 → build.rs自动链接 → dll自动复制

## 其他
- 运行时快捷键：WASD/鼠标/Shift/F 控制相机与GUI
- 依赖：Rust 1.75+、Vulkan SDK 1.3+、CMake 3.20+、VS2019+

---
如需扩展/修改此说明，请补充具体场景或约定。
