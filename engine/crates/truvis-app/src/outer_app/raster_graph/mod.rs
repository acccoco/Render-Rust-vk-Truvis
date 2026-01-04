//! 基于 RenderGraph V2 的光栅化管线示例
//!
//! 包含以下 Pass：
//! - **RasterPass**: 场景光栅化渲染
//! - **BloomPass**: Bloom 后处理效果
//! - **UiPass**: ImGui UI 渲染
//!
//! Present 不作为独立 Pass，而是在 Renderer 层面处理（blit 到 swapchain）。

mod bloom_pass;
mod raster_graph_app;
mod raster_pass;
mod ui_pass;

pub use raster_graph_app::RasterGraphApp;
