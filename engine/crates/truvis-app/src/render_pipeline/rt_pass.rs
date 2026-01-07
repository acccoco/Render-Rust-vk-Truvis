use ash::vk;

use crate::render_pipeline::blit_subpass::{BlitPass, BlitPassData, BlitSubpassDep};
use crate::render_pipeline::sdr_subpass::{SdrPass, SdrPassData, SdrSubpassDep};
use crate::render_pipeline::realtime_rt_subpass::{RealtimeRtPassData, SimpleRtPassDep, RealtimeRtPass};
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_gfx::{
    commands::{barrier::GfxImageBarrier, submit_info::GfxSubmitInfo},
    gfx::Gfx,
};
use truvis_render_graph::render_context::RenderContext;
use truvis_render_interface::cmd_allocator::CmdAllocator;
use truvis_render_interface::frame_counter::FrameCounter;
use truvis_render_interface::global_descriptor_sets::GlobalDescriptorSets;

/// 整个 RT 管线
pub struct RtRenderPass {
    simple_rt_subpass: RealtimeRtPass,
    blit_subpass: BlitPass,
    sdr_subpass: SdrPass,

    rt_cmds: [GfxCommandBuffer; FrameCounter::fif_count()],
    blit_cmds: [GfxCommandBuffer; FrameCounter::fif_count()],
    sdr_cmds: [GfxCommandBuffer; FrameCounter::fif_count()],
}

impl RtRenderPass {
    pub fn new(render_descriptor_sets: &GlobalDescriptorSets, cmd_allocator: &mut CmdAllocator) -> Self {
        let rt_pass = RealtimeRtPass::new(render_descriptor_sets);
        let blit_subpass = BlitPass::new(render_descriptor_sets);
        let sdr_subpass = SdrPass::new(render_descriptor_sets);

        let rt_cmds = FrameCounter::frame_labes()
            .map(|frame_label| cmd_allocator.alloc_command_buffer(frame_label, "ray-tracing"));
        let blit_cmds =
            FrameCounter::frame_labes().map(|frame_label| cmd_allocator.alloc_command_buffer(frame_label, "blit"));
        let sdr_cmds = FrameCounter::frame_labes()
            .map(|frame_label| cmd_allocator.alloc_command_buffer(frame_label, "hdr-to-sdr"));

        Self {
            simple_rt_subpass: rt_pass,
            blit_subpass,
            sdr_subpass,

            rt_cmds,
            blit_cmds,
            sdr_cmds,
        }
    }

    pub fn render(&self, render_context: &RenderContext) {
        let frame_label = render_context.frame_counter.frame_label();

        let fif_buffers = &render_context.fif_buffers;
        let _bindless_manager = &render_context.bindless_manager;

        let color_image = render_context.gfx_resource_manager.get_image(fif_buffers.color_image_handle()).unwrap();

        let (render_target_image_handle, render_target_view_handle) =
            render_context.fif_buffers.render_target_handle(frame_label);
        let render_target_image = render_context.gfx_resource_manager.get_image(render_target_image_handle).unwrap();

        let mut submit_cmds = Vec::new();
        // ray tracing
        {
            let cmd = self.rt_cmds[*frame_label].clone();
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "ray-tracing");

            let pre_usage = SimpleRtPassDep::default().accum_image;
            let crt_usage = SimpleRtPassDep::default().accum_image;

            // RT 的 accum image 在 fif 中只有一个， 因此需要确保之前的 rt 写入已经完成
            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[GfxImageBarrier::new()
                    .image(color_image.handle())
                    .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                    .layout_transfer(pre_usage.layout, crt_usage.layout)
                    .src_mask(pre_usage.stage, pre_usage.src_access())
                    .dst_mask(crt_usage.stage, crt_usage.dst_access())],
            );

            self.simple_rt_subpass.ray_trace(
                render_context,
                &cmd,
                RealtimeRtPassData {
                    accum_image_view: fif_buffers.color_image_view_handle(),
                    accum_image: fif_buffers.color_image_handle(),
                },
            );

            cmd.end();

            submit_cmds.push(cmd);
        }

        let frame_settings = &render_context.frame_settings;

        // blit
        {
            let cmd = self.blit_cmds[*frame_label].clone();
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "blit");

            let src_image_pre_usage = SimpleRtPassDep::default().accum_image;
            let src_image_crt_usage = BlitSubpassDep::default().src_image;

            // 等待 ray-tracing 执行完成
            let rt_barrier = GfxImageBarrier::new()
                .image(color_image.handle())
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .layout_transfer(src_image_pre_usage.layout, src_image_crt_usage.layout)
                .src_mask(src_image_pre_usage.stage, src_image_pre_usage.src_access())
                .dst_mask(src_image_crt_usage.stage, src_image_crt_usage.dst_access());
            cmd.image_memory_barrier(vk::DependencyFlags::empty(), &[rt_barrier]);

            self.blit_subpass.exec(
                &cmd,
                BlitPassData {
                    src_image: fif_buffers.color_image_view_handle(),
                    dst_image: render_target_view_handle,
                    src_image_size: frame_settings.frame_extent,
                    dst_image_size: frame_settings.frame_extent,
                },
                render_context,
            );

            cmd.end();
            submit_cmds.push(cmd.clone());
        }

        // TODO 上面的 blit 似乎没有任何作用，下面的 hdr -> sdr 也是在做同样的事情

        // hdr -> sdr
        {
            let cmd = self.sdr_cmds[*frame_label].clone();
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "hdr2sdr");

            let dst_image_pre_usage = BlitSubpassDep::default().dst_image;
            let dst_image_crt_usage = SdrSubpassDep::default().dst_image;

            // 等待之前的 compute shader 执行完成
            let rt_barrier = GfxImageBarrier::new()
                .image(render_target_image.handle())
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .src_mask(dst_image_pre_usage.stage, dst_image_pre_usage.src_access())
                .dst_mask(dst_image_crt_usage.stage, dst_image_pre_usage.dst_access());
            cmd.image_memory_barrier(vk::DependencyFlags::empty(), &[rt_barrier]);

            self.sdr_subpass.exec(
                &cmd,
                SdrPassData {
                    src_image: fif_buffers.color_image_view_handle(),
                    dst_image: render_target_view_handle,
                    src_image_size: frame_settings.frame_extent,
                    dst_image_size: frame_settings.frame_extent,
                },
                render_context,
            );

            cmd.end();
            submit_cmds.push(cmd.clone());
        }

        Gfx::get().gfx_queue().submit(vec![GfxSubmitInfo::new(&submit_cmds)], None);
    }
}
impl Drop for RtRenderPass {
    fn drop(&mut self) {
        log::info!("RtPipeline drop");
    }
}
