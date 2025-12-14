use ash::vk;

use crate::apis::render_pass::RenderPass;
use crate::core::frame_context::FrameContext;
use crate::core::renderer::{FrameContext2, FrameContext3};
use crate::render_pipeline::{compute_subpass::ComputeSubpass, simple_rt_subpass::SimpleRtSubpass};
use crate::subsystems::bindless_manager::BindlessManager;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::{
    commands::{barrier::GfxImageBarrier, submit_info::GfxSubmitInfo},
    gfx::Gfx,
};
use truvis_shader_binding::truvisl;

/// 整个 RT 管线
pub struct RtRenderPass {
    rt_pass: SimpleRtSubpass,
    blit_pass: ComputeSubpass<truvisl::blit::PushConstant>,
    sdr_pass: ComputeSubpass<truvisl::sdr::PushConstant>,
}

impl RtRenderPass {
    pub fn new(bindless_manager: &BindlessManager) -> Self {
        let rt_pass = SimpleRtSubpass::new(bindless_manager);
        let blit_pass = ComputeSubpass::<truvisl::blit::PushConstant>::new(
            bindless_manager,
            c"main",
            TruvisPath::shader_path("imgui/blit.slang").as_str(),
        );
        let sdr_pass = ComputeSubpass::<truvisl::sdr::PushConstant>::new(
            bindless_manager,
            c"main",
            TruvisPath::shader_path("pass/pp/sdr.slang").as_str(),
        );

        Self {
            rt_pass,
            blit_pass,
            sdr_pass,
        }
    }

    pub fn render(&self, frame_context2: &FrameContext2, frame_context3: &mut FrameContext3) {
        let frame_label = FrameContext::get().frame_label();
        let frame_settings = FrameContext::get().frame_settings();

        let fif_buffers = &frame_context2.fif_buffers;
        let bindless_manager = &frame_context2.bindless_manager;

        let color_image = frame_context2.gfx_resource_manager.get_image(fif_buffers.color_image_handle()).unwrap();
        let color_image_bindless_handle =
            bindless_manager.get_image_handle2(fif_buffers.color_image_view_handle()).unwrap();
        let render_target_texture = frame_context2
            .gfx_resource_manager
            .get_texture(fif_buffers.render_target_texture_handle(frame_label))
            .unwrap();
        let render_target_image_bindless_handle = bindless_manager
            .get_image_handle_in_texture(fif_buffers.render_target_texture_handle(frame_label))
            .unwrap();

        let mut submit_cmds = Vec::new();
        // ray tracing
        {
            let cmd = frame_context3.cmd_allocator.alloc_command_buffer("ray-tracing");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "ray-tracing");

            // RT 的 accum image 在 fif 中只有一个， 因此需要确保之前的 rt 写入已经完成
            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[GfxImageBarrier::new()
                    .image(color_image.handle())
                    .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                    .src_mask(vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR, vk::AccessFlags2::SHADER_STORAGE_WRITE)
                    .dst_mask(
                        vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                        vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                    )],
            );

            self.rt_pass.ray_trace(
                frame_context2,
                &cmd,
                &frame_settings,
                &FrameContext::get().pipeline_settings(),
                color_image.handle(),
                color_image_bindless_handle,
                &frame_context2.per_frame_data_buffers[*frame_label],
            );

            cmd.end();

            submit_cmds.push(cmd);
        }

        // blit
        {
            let cmd = frame_context3.cmd_allocator.alloc_command_buffer("blit");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "blit");

            // 等待 ray-tracing 执行完成
            let rt_barrier = GfxImageBarrier::new()
                .image(color_image.handle())
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .src_mask(vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR, vk::AccessFlags2::SHADER_STORAGE_WRITE)
                .dst_mask(vk::PipelineStageFlags2::COMPUTE_SHADER, vk::AccessFlags2::SHADER_READ);
            cmd.image_memory_barrier(vk::DependencyFlags::empty(), &[rt_barrier]);

            self.blit_pass.exec(
                &cmd,
                bindless_manager,
                &truvisl::blit::PushConstant {
                    src_image: color_image_bindless_handle.0,
                    dst_image: render_target_image_bindless_handle.0,
                    src_image_size: glam::uvec2(frame_settings.frame_extent.width, frame_settings.frame_extent.height)
                        .into(),
                    offset: glam::uvec2(0, 0).into(),
                },
                glam::uvec3(
                    frame_settings.frame_extent.width.div_ceil(truvisl::blit::SHADER_X as u32),
                    frame_settings.frame_extent.height.div_ceil(truvisl::blit::SHADER_Y as u32),
                    1,
                ),
            );

            cmd.end();
            submit_cmds.push(cmd);
        }

        // hdr -> sdr
        {
            let cmd = frame_context3.cmd_allocator.alloc_command_buffer("hdr2sdr");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "hdr2sdr");

            // 等待之前的 compute shader 执行完成
            let rt_barrier = GfxImageBarrier::new()
                .image(render_target_texture.image().handle())
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .src_mask(
                    vk::PipelineStageFlags2::COMPUTE_SHADER,
                    vk::AccessFlags2::SHADER_WRITE | vk::AccessFlags2::SHADER_READ,
                )
                .dst_mask(
                    vk::PipelineStageFlags2::COMPUTE_SHADER,
                    vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                );
            cmd.image_memory_barrier(vk::DependencyFlags::empty(), &[rt_barrier]);

            self.sdr_pass.exec(
                &cmd,
                bindless_manager,
                &truvisl::sdr::PushConstant {
                    src_image: color_image_bindless_handle.0,
                    dst_image: render_target_image_bindless_handle.0,
                    image_size: glam::uvec2(frame_settings.frame_extent.width, frame_settings.frame_extent.height)
                        .into(),
                    channel: FrameContext::get().pipeline_settings().channel,
                    _padding_1: Default::default(),
                },
                glam::uvec3(
                    frame_settings.frame_extent.width.div_ceil(truvisl::blit::SHADER_X as u32),
                    frame_settings.frame_extent.height.div_ceil(truvisl::blit::SHADER_Y as u32),
                    1,
                ),
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
