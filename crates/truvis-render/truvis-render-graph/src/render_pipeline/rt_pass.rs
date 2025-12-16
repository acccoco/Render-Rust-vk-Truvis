use ash::vk;

use crate::apis::render_pass::RenderPass;
use crate::render_context::{RenderContext, RenderContextMut};
use crate::render_pipeline::blit_subpass::{BlitSubpass, BlitSubpassData, BlitSubpassDep};
use crate::render_pipeline::sdr_subpass::{SdrSubpass, SdrSubpassData, SdrSubpassDep};
use crate::render_pipeline::simple_rt_subpass::{SimpleRtPassData, SimpleRtPassDep};
use crate::render_pipeline::{compute_subpass::ComputeSubpass, simple_rt_subpass::SimpleRtSubpass};
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::{
    commands::{barrier::GfxImageBarrier, submit_info::GfxSubmitInfo},
    gfx::Gfx,
};
use truvis_render_base::bindless_manager::BindlessManager;
use truvis_shader_binding::truvisl;

/// 整个 RT 管线
pub struct RtRenderPass {
    simple_rt_subpass: SimpleRtSubpass,
    blit_subpass: BlitSubpass,
    sdr_subpass: SdrSubpass,
}

impl RtRenderPass {
    pub fn new(bindless_manager: &BindlessManager) -> Self {
        let rt_pass = SimpleRtSubpass::new(bindless_manager);
        let blit_subpass = BlitSubpass::new(bindless_manager);
        let sdr_subpass = SdrSubpass::new(bindless_manager);

        Self {
            simple_rt_subpass: rt_pass,
            blit_subpass,
            sdr_subpass,
        }
    }

    pub fn render(&self, render_context: &RenderContext, render_context_mut: &mut RenderContextMut) {
        let frame_label = render_context.frame_counter.frame_label();

        let fif_buffers = &render_context.fif_buffers;
        let bindless_manager = &render_context.bindless_manager;

        let color_image = render_context.gfx_resource_manager.get_image(fif_buffers.color_image_handle()).unwrap();
        let color_image_bindless_handle =
            bindless_manager.get_image_handle(fif_buffers.color_image_view_handle()).unwrap();
        let render_target_texture = render_context
            .gfx_resource_manager
            .get_texture(fif_buffers.render_target_texture_handle(frame_label))
            .unwrap();
        let render_target_image_bindless_handle = bindless_manager
            .get_image_handle_in_texture(fif_buffers.render_target_texture_handle(frame_label))
            .unwrap();

        let mut submit_cmds = Vec::new();
        // ray tracing
        {
            let cmd =
                render_context_mut.cmd_allocator.alloc_command_buffer(&render_context.frame_counter, "ray-tracing");
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
                SimpleRtPassData {
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
            let cmd = render_context_mut.cmd_allocator.alloc_command_buffer(&render_context.frame_counter, "blit");
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
                BlitSubpassData {
                    src_image: fif_buffers.color_image_view_handle(),
                    dst_image: fif_buffers.render_target_texture_handle(frame_label),
                    src_image_size: frame_settings.frame_extent,
                    dst_image_size: frame_settings.frame_extent,
                },
                render_context,
            );

            cmd.end();
            submit_cmds.push(cmd);
        }

        // TODO 上面的 blit 似乎没有任何作用，下面的 hdr -> sdr 也是在做同样的事情

        // hdr -> sdr
        {
            let cmd = render_context_mut.cmd_allocator.alloc_command_buffer(&render_context.frame_counter, "hdr2sdr");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "hdr2sdr");

            let dst_image_pre_usage = BlitSubpassDep::default().dst_image;
            let dst_image_crt_usage = SdrSubpassDep::default().dst_image;

            // 等待之前的 compute shader 执行完成
            let rt_barrier = GfxImageBarrier::new()
                .image(render_target_texture.image().handle())
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .src_mask(dst_image_pre_usage.stage, dst_image_pre_usage.src_access())
                .dst_mask(dst_image_crt_usage.stage, dst_image_pre_usage.dst_access());
            cmd.image_memory_barrier(vk::DependencyFlags::empty(), &[rt_barrier]);

            self.sdr_subpass.exec(
                &cmd,
                SdrSubpassData {
                    src_image: fif_buffers.color_image_view_handle(),
                    dst_image: fif_buffers.render_target_texture_handle(frame_label),
                    src_image_size: frame_settings.frame_extent,
                    dst_image_size: frame_settings.frame_extent,
                },
                render_context,
            );

            cmd.end();
            submit_cmds.push(cmd);
        }

        Gfx::get().gfx_queue().submit(vec![GfxSubmitInfo::new(&submit_cmds)], None);
    }
}
impl RenderPass for RtRenderPass {}
impl Drop for RtRenderPass {
    fn drop(&mut self) {
        log::info!("RtPipeline drop");
    }
}
