//! RenderGraph V2 - 声明式渲染图系统
//!
//! 提供自动依赖分析和 barrier 生成的渲染图抽象。
//!
//! # 核心概念
//!
//! - **RgImageHandle / RgBufferHandle**: 虚拟资源句柄，在 graph 内部标识资源
//! - **ImageState / BufferState**: 资源状态描述，包含 stage、access、layout
//! - **RgPass**: 渲染 Pass trait，声明资源依赖和执行逻辑
//! - **RenderGraphBuilder**: 构建器，用于注册资源和 Pass
//! - **CompiledGraph**: 编译结果，包含执行顺序和预计算的 barriers
//!
//! # 使用示例
//!
//! ```ignore
//! use truvis_render_graph::render_graph_v2::*;
//!
//! // 1. 定义 Pass
//! struct MyComputePass {
//!     input: RgImageHandle,
//!     output: RgImageHandle,
//! }
//!
//! impl RgPass for MyComputePass {
//!     fn setup(&mut self, builder: &mut PassBuilder) {
//!         builder.read_image(self.input, ImageState::SHADER_READ_COMPUTE);
//!         self.output = builder.write_image(self.output, ImageState::STORAGE_WRITE_COMPUTE);
//!     }
//!
//!     fn execute(&self, ctx: &PassContext) {
//!         let input_view = ctx.get_image_view_handle(self.input);
//!         let output_view = ctx.get_image_view_handle(self.output);
//!         // 绑定 descriptor sets, dispatch...
//!     }
//! }
//!
//! // 2. 构建渲染图
//! let mut builder = RenderGraphBuilder::new();
//!
//! // 导入外部资源
//! let input = builder.import_image("input", input_handle, Some(input_view), vk::Format::R8G8B8A8_UNORM, ImageState::UNDEFINED);
//! let output = builder.import_image("output", output_handle, Some(output_view), vk::Format::R8G8B8A8_UNORM, ImageState::UNDEFINED);
//!
//! // 添加 Pass
//! builder.add_pass("compute", MyComputePass { input, output });
//!
//! // 3. 编译
//! let graph = builder.compile();
//!
//! // 4. 执行
//! cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "render-graph");
//! graph.execute(&cmd, &resource_manager);
//! cmd.end();
//! ```
//!
//! # 与现有系统的关系
//!
//! 此模块与现有的 `OuterApp::draw()` 模式**完全兼容**：
//!
//! - 现有 Pass 可以继续使用手动 barrier 管理
//! - 新 Pass 可以选择性采用 RenderGraph
//! - 两种模式可以在同一应用中混合使用
//!
//! # 模块结构
//!
//! - `handle`: 虚拟资源句柄定义
//! - `state`: 资源状态（stage/access/layout）封装
//! - `resource`: 资源注册表
//! - `pass`: Pass trait 和 builder
//! - `graph`: 依赖图和拓扑排序
//! - `barrier`: 自动 barrier 计算
//! - `executor`: 构建器和执行器

mod barrier;
mod executor;
mod graph;
mod handle;
mod pass;
mod resource;
mod state;

// Re-exports
pub use barrier::{BufferBarrierDesc, ImageBarrierDesc, PassBarriers};
pub use executor::{CompiledGraph, RenderGraphBuilder};
pub use graph::{DependencyAnalyzer, DependencyEdge, DependencyGraph};
pub use handle::{RgBufferHandle, RgImageHandle};
pub use pass::{BufferRead, BufferWrite, ImageRead, ImageWrite, PassBuilder, PassContext, PassNode, RgPass};
pub use resource::{
    BufferResource, BufferSource, ImageResource, ImageSource, ResourceRegistry, RgBufferDesc, RgImageDesc,
};
pub use state::{BufferState, ImageState};
