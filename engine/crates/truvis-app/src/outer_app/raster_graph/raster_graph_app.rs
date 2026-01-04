//! 基于 RenderGraph V2 的光栅化应用
//!
//! 演示如何使用声明式 RenderGraph 构建完整的渲染管线。

use crate::outer_app::OuterApp;
use crate::outer_app::raster_graph::bloom_pass::BloomPass;
use crate::outer_app::raster_graph::raster_pass::{RasterPass, RasterPipeline};
use crate::outer_app::raster_graph::ui_pass::UiPass;
use ash::vk;
use imgui::Ui;
use truvis_gfx::commands::barrier::GfxImageBarrier;
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_gfx::commands::submit_info::GfxSubmitInfo;
use truvis_gfx::gfx::Gfx;
use truvis_render_graph::render_context::RenderContext;
use truvis_render_graph::render_graph_v2::{RenderGraphBuilder, RgImageState};
use truvis_render_interface::frame_counter::FrameCounter;
use truvis_render_interface::geometry::RtGeometry;
use truvis_renderer::platform::camera::Camera;
use truvis_renderer::renderer::Renderer;
use truvis_scene::shapes::triangle::TriangleSoA;

/// 基于 RenderGraph V2 的光栅化管线应用
///
/// # 渲染流程
///
/// ```text
/// ┌─────────────────┐
/// │   RasterPass    │  场景光栅化渲染 -> render_target
/// └────────┬────────┘
///          │
///          ▼
/// ┌─────────────────┐
/// │   BloomPass     │  后处理 (可选) -> bloom_target
/// └────────┬────────┘
///          │
///          ▼
/// ┌─────────────────┐
/// │    UiPass       │  ImGui UI 叠加 -> final_target
/// └────────┬────────┘
///          │
///          ▼
///     [ Present ]     由 Renderer 处理 blit 到 swapchain
/// ```
///
/// # 关于 Present
///
/// Present 不作为 RenderGraph 的 Pass，原因：
/// 1. Swapchain image 的获取/提交由 Renderer 统一管理
/// 2. Present 涉及 WSI 同步，与 RenderGraph 的 barrier 系统不同
/// 3. 保持 RenderGraph 专注于渲染逻辑，分离 presentation 逻辑
pub struct RasterGraphApp {
    /// 光栅化管线（跨帧复用）
    raster_pipeline: Option<RasterPipeline>,

    /// 场景几何体（由 App 拥有，Pass 借用）
    geometry: Option<RtGeometry>,

    /// 预分配的命令缓冲区
    cmds: Option<[GfxCommandBuffer; FrameCounter::fif_count()]>,

    /// UI 控制参数
    bloom_enabled: bool,
}

impl Default for RasterGraphApp {
    fn default() -> Self {
        Self {
            raster_pipeline: None,
            geometry: None,
            cmds: None,
            bloom_enabled: true,
        }
    }
}

impl OuterApp for RasterGraphApp {
    fn init(&mut self, renderer: &mut Renderer, _camera: &mut Camera) {
        log::info!("RasterGraphApp: initializing with RenderGraph V2");

        // 创建管线
        self.raster_pipeline = Some(RasterPipeline::new(&renderer.render_context.frame_settings));

        // 创建测试几何体（由 App 拥有）
        self.geometry = Some(TriangleSoA::create_mesh());

        // 分配命令缓冲区
        self.cmds = Some(
            FrameCounter::frame_labes().map(|label| renderer.cmd_allocator.alloc_command_buffer(label, "raster-graph")),
        );
    }

    fn draw_ui(&mut self, ui: &Ui) {
        ui.window("RasterGraph Settings").build(|| {
            ui.checkbox("Enable Bloom", &mut self.bloom_enabled);

            ui.separator();
            ui.text("Render Pipeline:");
            ui.bullet_text("1. RasterPass - Scene rendering");
            if self.bloom_enabled {
                ui.bullet_text("2. BloomPass - Post-processing");
                ui.bullet_text("3. UiPass - ImGui overlay");
            } else {
                ui.bullet_text("2. UiPass - ImGui overlay");
            }
            ui.bullet_text("→ Present (handled by Renderer)");
        });
    }

    fn draw(&self, render_context: &RenderContext) {
        let frame_label = render_context.frame_counter.frame_label();
        let frame_settings = &render_context.frame_settings;

        // 获取资源
        let pipeline = self.raster_pipeline.as_ref().unwrap();
        let geometry = self.geometry.as_ref().unwrap();
        let cmd = &self.cmds.as_ref().unwrap()[*frame_label];

        // 获取 FIF 资源句柄
        let (render_target_img, render_target_view) = render_context.fif_buffers.render_target_handle(frame_label);
        let depth_img = render_context.fif_buffers.depth_image;
        let depth_view = render_context.fif_buffers.depth_image_view;

        // === 构建 RenderGraph ===
        let mut builder = RenderGraphBuilder::new();

        // 导入外部资源
        let rg_render_target = builder.import_image(
            "render_target",
            render_target_img,
            Some(render_target_view),
            frame_settings.color_format,
            RgImageState::UNDEFINED,
        );

        let rg_depth = builder.import_image(
            "depth",
            depth_img,
            Some(depth_view),
            frame_settings.depth_format,
            RgImageState::UNDEFINED,
        );

        // 1. Raster Pass - 场景渲染
        let raster_pass = RasterPass::new(rg_render_target, rg_depth, pipeline, geometry, frame_settings.frame_extent);
        builder.add_pass("raster", raster_pass);

        // 2. Bloom Pass - 后处理 (简化版：读写同一个 render_target)
        let bloom_pass = BloomPass::new(rg_render_target, rg_render_target, self.bloom_enabled);
        builder.add_pass("bloom", bloom_pass);

        // 3. UI Pass
        let ui_pass = UiPass::new(rg_render_target);
        builder.add_pass("ui", ui_pass);

        // === 编译并执行 ===
        let compiled = builder.compile();

        // debug
        {
            static PASS_DUMP: std::sync::Once = std::sync::Once::new();
            PASS_DUMP.call_once(|| {
                compiled.print_execution_plan();
            });
        }

        // 开始命令缓冲区
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "raster-graph");

        // 执行渲染图（传入资源管理器的引用）
        compiled.execute(cmd, &render_context.gfx_resource_manager);

        // 将 render target 转换为 GENERAL（供后续 present 使用）
        let render_target_image = render_context.gfx_resource_manager.get_image(render_target_img).unwrap();
        cmd.image_memory_barrier(
            vk::DependencyFlags::empty(),
            &[GfxImageBarrier::new()
                .image(render_target_image.handle())
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .layout_transfer(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::GENERAL)
                .src_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                .dst_mask(vk::PipelineStageFlags2::NONE, vk::AccessFlags2::NONE)],
        );

        cmd.end();

        // 提交
        Gfx::get().gfx_queue().submit(vec![GfxSubmitInfo::new(&[cmd.clone()])], None);
    }

    fn on_window_resized(&mut self, renderer: &mut Renderer) {
        log::info!("RasterGraphApp: window resized, rebuilding pipeline");
        self.raster_pipeline = Some(RasterPipeline::new(&renderer.render_context.frame_settings));
    }
}
