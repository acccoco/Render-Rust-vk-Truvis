use ash::vk;
use std::mem::swap;

use truvis_render_graph::render_context::RenderContext;
use truvis_render_graph::render_graph_v2::{
    CompiledGraph, RenderGraphBuilder, RgImageHandle, RgImageState, RgPassContext,
};

use crate::render_pipeline::blit_subpass::{BlitPass, BlitPassData};
use crate::render_pipeline::resolve_subpass::ResolvePass;
use crate::render_pipeline::sdr_subpass::{SdrPass, SdrSubpassData};
use crate::render_pipeline::simple_rt_subpass::{RealtimeRtPass, SimpleRtPassData};
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_gfx::commands::submit_info::GfxSubmitInfo;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::swapchain::swapchain::GfxSwapchain;
use truvis_gui_backend::gui_pass::GuiPass;
use truvis_render_interface::cmd_allocator::CmdAllocator;
use truvis_render_interface::frame_counter::FrameCounter;
use truvis_render_interface::global_descriptor_sets::GlobalDescriptorSets;
use truvis_render_interface::pipeline_settings::FrameLabel;

pub struct RtPipeline {
    /// 光追 pass
    realtime_rt_pass: RealtimeRtPass,
    /// Blit pass
    blit_pass: BlitPass,
    /// SDR pass
    sdr_pass: SdrPass,
    resolve_pass: ResolvePass,
    gui_pass: GuiPass,

    /// 每帧的命令缓冲区
    cmds: [GfxCommandBuffer; FrameCounter::fif_count()],
}

// new & init
impl RtPipeline {
    /// 创建新的 RT 渲染管线
    pub fn new(
        global_descriptor_sets: &GlobalDescriptorSets,
        swapchain: &GfxSwapchain,
        cmd_allocator: &mut CmdAllocator,
    ) -> Self {
        let realtime_rt_pass = RealtimeRtPass::new(global_descriptor_sets);
        let blit_pass = BlitPass::new(global_descriptor_sets);
        let sdr_pass = SdrPass::new(global_descriptor_sets);
        let resolve_pass = ResolvePass::new(global_descriptor_sets, swapchain.image_infos().image_format);
        let gui_pass = GuiPass::new(global_descriptor_sets, swapchain.image_infos().image_format);

        let cmds = FrameCounter::frame_labes()
            .map(|frame_label| cmd_allocator.alloc_command_buffer(frame_label, "rt-render-graph"));

        Self {
            realtime_rt_pass,
            blit_pass,
            sdr_pass,
            resolve_pass,
            gui_pass,
            cmds,
        }
    }
}

// 各种 pass
impl RtPipeline {
    /// 添加光线追踪 Pass
    ///
    /// # 参数
    /// - `builder`: RenderGraph 构建器
    /// - `accum_image`: 累积图像句柄
    /// - `render_context`: 渲染上下文
    fn add_rt_pass<'a>(
        &'a self,
        builder: &mut RenderGraphBuilder<'a>,
        accum_image: RgImageHandle,
        render_context: &'a RenderContext,
    ) {
        let subpass = &self.realtime_rt_pass;
        let fif_buffers = &render_context.fif_buffers;

        builder.add_pass_lambda(
            "ray-tracing",
            move |b| {
                // RT 需要读写累积图像（每次采样累积）
                b.read_write_image(accum_image, RgImageState::STORAGE_READ_WRITE_RAY_TRACING);
            },
            move |ctx| {
                subpass.ray_trace(
                    render_context,
                    ctx.cmd,
                    SimpleRtPassData {
                        accum_image: fif_buffers.color_image_handle(),
                        accum_image_view: fif_buffers.color_image_view_handle(),
                    },
                );
            },
        );
    }

    /// 添加 Blit Pass
    ///
    /// 将 HDR 累积图像 blit 到渲染目标
    ///
    /// # 参数
    /// - `builder`: RenderGraph 构建器
    /// - `src_image`: 源图像句柄（累积图像）
    /// - `dst_image`: 目标图像句柄（渲染目标）
    /// - `render_context`: 渲染上下文
    /// - `frame_label`: 当前帧标签
    fn add_blit_pass<'a>(
        &'a self,
        builder: &mut RenderGraphBuilder<'a>,
        src_image: RgImageHandle,
        dst_image: RgImageHandle,
        render_context: &'a RenderContext,
        frame_label: FrameLabel,
    ) {
        let subpass = &self.blit_pass;
        let fif_buffers = &render_context.fif_buffers;
        let frame_extent = render_context.frame_settings.frame_extent;

        builder.add_pass_lambda(
            "blit",
            move |b| {
                b.read_image(src_image, RgImageState::SHADER_READ_COMPUTE);
                b.write_image(dst_image, RgImageState::STORAGE_WRITE_COMPUTE);
            },
            move |ctx| {
                let (_, render_target_view_handle) = fif_buffers.render_target_handle(frame_label);

                let src_bindless_handle =
                    render_context.bindless_manager.get_shader_uav_handle(fif_buffers.color_image_view_handle());
                let dst_bindless_handle =
                    render_context.bindless_manager.get_shader_uav_handle(render_target_view_handle);

                subpass.exec(
                    ctx.cmd,
                    BlitPassData {
                        src_bindless_uav_handle: src_bindless_handle,
                        dst_bindless_uav_handle: dst_bindless_handle,
                        src_image_size: frame_extent,
                        dst_image_size: frame_extent,
                    },
                    render_context,
                );
            },
        );
    }

    /// 添加 SDR 转换 Pass
    ///
    /// 将 HDR 图像转换为 SDR（色调映射）
    ///
    /// # 参数
    /// - `builder`: RenderGraph 构建器
    /// - `src_image`: 源图像句柄（HDR 累积图像）
    /// - `dst_image`: 目标图像句柄（渲染目标）
    /// - `render_context`: 渲染上下文
    /// - `frame_label`: 当前帧标签
    fn add_sdr_pass<'a>(
        &'a self,
        builder: &mut RenderGraphBuilder<'a>,
        src_image: RgImageHandle,
        dst_image: RgImageHandle,
        render_context: &'a RenderContext,
        frame_label: FrameLabel,
    ) {
        let subpass = &self.sdr_pass;
        let fif_buffers = &render_context.fif_buffers;
        let frame_extent = render_context.frame_settings.frame_extent;

        builder.add_pass_lambda(
            "hdr-to-sdr",
            move |b| {
                b.read_image(src_image, RgImageState::SHADER_READ_COMPUTE);
                b.write_image(dst_image, RgImageState::STORAGE_WRITE_COMPUTE);
            },
            move |ctx| {
                let (_, render_target_view_handle) = fif_buffers.render_target_handle(frame_label);

                subpass.exec(
                    ctx.cmd,
                    SdrSubpassData {
                        src_image: fif_buffers.color_image_view_handle(),
                        dst_image: render_target_view_handle,
                        src_image_size: frame_extent,
                        dst_image_size: frame_extent,
                    },
                    render_context,
                );
            },
        );
    }

    fn add_resolve_pass<'a>(
        &'a self,
        builder: &mut RenderGraphBuilder<'a>,
        src_image: RgImageHandle,
        dst_image: RgImageHandle,
        render_context: &'a RenderContext,
        frame_label: FrameLabel,
    ) {
        builder.add_pass_lambda(
            "resolve",
            move |b| {},
            move |ctx| {},
        );
    }
}

// render
impl RtPipeline {
    /// 执行渲染（不包含 UI）
    ///
    /// # 参数
    /// - `render_context`: 渲染上下文
    pub fn render(&self, render_context: &RenderContext) {
        let frame_label = render_context.frame_counter.frame_label();
        let fif_buffers = &render_context.fif_buffers;

        // 构建 RenderGraph
        let mut rg_builder = RenderGraphBuilder::new();

        // 导入外部资源
        let accum_image = rg_builder.import_image(
            "accum-image",
            fif_buffers.color_image_handle(),
            Some(fif_buffers.color_image_view_handle()),
            vk::Format::R32G32B32A32_SFLOAT,
            RgImageState::STORAGE_READ_WRITE_RAY_TRACING,
        );

        let (render_target_image_handle, render_target_view_handle) = fif_buffers.render_target_handle(frame_label);
        let render_target = rg_builder.import_image(
            "render-target",
            render_target_image_handle,
            Some(render_target_view_handle),
            vk::Format::R8G8B8A8_UNORM,
            RgImageState::UNDEFINED,
        );

        // 添加 Pass
        self.add_rt_pass(&mut rg_builder, accum_image, render_context);
        self.add_blit_pass(&mut rg_builder, accum_image, render_target, render_context, frame_label);
        self.add_sdr_pass(&mut rg_builder, accum_image, render_target, render_context, frame_label);

        // 编译 RenderGraph
        let compiled_graph = rg_builder.compile();

        // 调试输出执行计划
        if log::log_enabled!(log::Level::Debug) {
            compiled_graph.print_execution_plan();
        }

        // 执行
        self.execute_graph(render_context, &compiled_graph);
    }

    /// 执行编译后的 RenderGraph
    pub fn execute_graph(&self, render_context: &RenderContext, compiled_graph: &CompiledGraph<'_>) {
        let frame_label = render_context.frame_counter.frame_label();
        let cmd = self.cmds[*frame_label].clone();

        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "rt-render-graph");

        compiled_graph.execute(&cmd, &render_context.gfx_resource_manager);

        cmd.end();

        Gfx::get().gfx_queue().submit(vec![GfxSubmitInfo::new(&[cmd])], None);
    }

    /// 获取命令缓冲区（用于外部执行）
    pub fn cmd(&self, frame_label: FrameLabel) -> &GfxCommandBuffer {
        &self.cmds[*frame_label]
    }
}

impl Drop for RtPipeline {
    fn drop(&mut self) {
        log::info!("RtRenderGraph drop");
    }
}
