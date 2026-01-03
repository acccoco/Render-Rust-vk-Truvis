//! RasterGraph 应用入口
//!
//! 演示基于 RenderGraph V2 的光栅化渲染管线。

use truvis_app::outer_app::raster_graph::RasterGraphApp;
use truvis_winit_app::app::WinitApp;

fn main() {
    let outer_app = Box::<RasterGraphApp>::default();
    WinitApp::run(outer_app);
}
