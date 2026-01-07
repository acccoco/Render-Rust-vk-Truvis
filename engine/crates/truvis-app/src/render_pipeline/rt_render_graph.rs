use ash::vk;
use truvis_render_graph::render_context::RenderContext;
use truvis_render_graph::render_graph_v2::{
    CompiledGraph, RenderGraphBuilder, RgImageHandle, RgImageState, RgPassContext, RgSemaphoreInfo,
};

use crate::render_pipeline::blit_subpass::{BlitPass, BlitPassData, BlitRgPass};
use crate::render_pipeline::realtime_rt_subpass::{RealtimeRtPass, RealtimeRtPassData, RealtimeRtRgPass};
use crate::render_pipeline::resolve_subpass::{ResolvePass, ResolveRgPass};
use crate::render_pipeline::sdr_subpass::{SdrPass, SdrPassData, SdrRgPass};
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_gfx::commands::submit_info::GfxSubmitInfo;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::swapchain::swapchain::GfxSwapchain;
use truvis_gui_backend::gui_pass::GuiPass;
use truvis_render_interface::cmd_allocator::CmdAllocator;
use truvis_render_interface::frame_counter::FrameCounter;
use truvis_render_interface::global_descriptor_sets::GlobalDescriptorSets;
use truvis_render_interface::pipeline_settings::FrameLabel;
use truvis_renderer::present::render_present::RenderPresent;

pub struct RtPipeline {
    /// 光追 pass
    realtime_rt_pass: RealtimeRtPass,
    /// Blit pass
    blit_pass: BlitPass,
    /// SDR pass
    sdr_pass: SdrPass,
    resolve_pass: ResolvePass,
    gui_pass: GuiPass,

    compute_cmds: [GfxCommandBuffer; FrameCounter::fif_count()],
    present_cmds: [GfxCommandBuffer; FrameCounter::fif_count()],
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

        let compute_cmds = FrameCounter::frame_labes()
            .map(|frame_label| cmd_allocator.alloc_command_buffer(frame_label, "rt-compute-subgraph"));
        let present_cmds = FrameCounter::frame_labes()
            .map(|frame_label| cmd_allocator.alloc_command_buffer(frame_label, "rt-present-subgraph"));

        Self {
            realtime_rt_pass,
            blit_pass,
            sdr_pass,
            resolve_pass,
            gui_pass,
            compute_cmds,
            present_cmds,
        }
    }
}

// render
impl RtPipeline {
    pub fn render(&self, render_context: &RenderContext, render_present: &RenderPresent) {
        let frame_label = render_context.frame_counter.frame_label();

        // compute subgraph
        let compute_subgraph_submit = {
            let compute_cmd = self.compute_cmds[*frame_label].clone();
            let compute_graph = self.prepare_compute_graph(render_context);

            compute_cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "rt-render-graph");
            compute_graph.execute(&compute_cmd, &render_context.gfx_resource_manager);
            compute_cmd.end();

            compute_graph.build_submit_info(std::slice::from_ref(&compute_cmd))
        };

        // present subgraph
        let present_subgraph_submit = {
            let present_cmd = self.present_cmds[*frame_label].clone();
            let present_graph = self.prepare_present_graph(render_context, render_present);

            present_cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "rt-present-graph");
            present_graph.execute(&present_cmd, &render_context.gfx_resource_manager);
            present_cmd.end();

            present_graph.build_submit_info(std::slice::from_ref(&present_cmd))
        };

        // TODO 在 RenderGraph 的提交时插入 fence
        Gfx::get().gfx_queue().submit(vec![compute_subgraph_submit, present_subgraph_submit], None);
    }

    pub fn prepare_compute_graph(&self, render_context: &RenderContext) -> CompiledGraph {
        let frame_label = render_context.frame_counter.frame_label();
        let fif_buffers = &render_context.fif_buffers;

        // 构建 RenderGraph
        let mut rg_builder = RenderGraphBuilder::new();

        // 导入外部资源
        let accum_image = rg_builder.import_image(
            "accum-image",
            fif_buffers.color_image_handle(),
            Some(fif_buffers.color_image_view_handle()),
            fif_buffers.color_image_format(),
            RgImageState::STORAGE_READ_WRITE_RAY_TRACING,
            None,
        );

        let (render_target_image_handle, render_target_view_handle) = fif_buffers.render_target_handle(frame_label);
        let render_target = rg_builder.import_image(
            "render-target",
            render_target_image_handle,
            Some(render_target_view_handle),
            fif_buffers.render_target_format(),
            RgImageState::UNDEFINED_TOP,
            None,
        );

        // 导出渲染目标（用于后续绘制）
        rg_builder.export_image(render_target, RgImageState::SHADER_READ_FRAGMENT, None);

        // 添加 pass
        rg_builder
            .add_pass(
                "ray-tracing",
                RealtimeRtRgPass {
                    rt_pass: &self.realtime_rt_pass,
                    render_context,
                    accum_image,
                    accum_image_extent: render_context.frame_settings.frame_extent,
                },
            )
            .add_pass(
                "blit",
                BlitRgPass {
                    blit_pass: &self.blit_pass,
                    render_context,
                    src_image: accum_image,
                    dst_image: render_target,
                    src_image_extent: render_context.frame_settings.frame_extent,
                    dst_image_extent: render_context.frame_settings.frame_extent,
                },
            )
            .add_pass(
                "hdr-to-sdr",
                SdrRgPass {
                    sdr_pass: &self.sdr_pass,
                    render_context,
                    src_image: accum_image,
                    dst_image: render_target,
                    src_image_extent: render_context.frame_settings.frame_extent,
                    dst_image_extent: render_context.frame_settings.frame_extent,
                },
            );

        // 编译 RenderGraph
        let compiled_graph = rg_builder.compile();

        // 调试输出执行计划
        if log::log_enabled!(log::Level::Debug) {
            static PRINT_DEBUG_INFO: std::sync::Once = std::sync::Once::new();
            PRINT_DEBUG_INFO.call_once(|| {
                compiled_graph.print_execution_plan();
            });
        }

        compiled_graph
    }

    pub fn prepare_present_graph(
        &self,
        render_context: &RenderContext,
        render_present: &RenderPresent,
    ) -> CompiledGraph {
        let frame_label = render_context.frame_counter.frame_label();
        let fif_buffers = &render_context.fif_buffers;

        // 构建 RenderGraph
        let mut rg_builder = RenderGraphBuilder::new();

        // 导入外部资源
        let (render_target_image_handle, render_target_view_handle) = fif_buffers.render_target_handle(frame_label);
        let render_target = rg_builder.import_image(
            "render-target",
            render_target_image_handle,
            Some(render_target_view_handle),
            fif_buffers.render_target_format(),
            RgImageState::SHADER_READ_FRAGMENT,
            None,
        );

        let (present_image, present_view) = render_present.current_image_and_view();
        let present_image = rg_builder.import_image(
            "present-image",
            present_image,
            Some(present_view),
            render_present.swapchain_image_info().image_format,
            RgImageState::UNDEFINED_BOTTOM,
            Some(RgSemaphoreInfo::binary(
                render_present.current_present_complete_semaphore(frame_label).handle(),
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            )),
        );

        // 导出渲染目标（用于后续呈现）
        rg_builder.export_image(
            present_image,
            RgImageState::PRESENT,
            Some(RgSemaphoreInfo::binary(
                render_present.current_render_compute_semaphore().handle(),
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            )),
        );

        // 添加 Pass
        rg_builder
            .add_pass(
                "resolve",
                ResolveRgPass {
                    resolve_pass: &self.resolve_pass,
                    render_context,
                    render_target,
                    swapchain_image: present_image,
                    swapchain_extent: render_present.swapchain_image_info().image_extent,
                },
            )
            .add_pass("gui", ());

        // 编译 RenderGraph
        let compiled_graph = rg_builder.compile();

        // 调试输出执行计划
        if log::log_enabled!(log::Level::Debug) {
            static PRINT_DEBUG_INFO: std::sync::Once = std::sync::Once::new();
            PRINT_DEBUG_INFO.call_once(|| {
                compiled_graph.print_execution_plan();
            });
        }

        compiled_graph
    }
}

impl Drop for RtPipeline {
    fn drop(&mut self) {
        log::info!("RtRenderGraph drop");
    }
}
