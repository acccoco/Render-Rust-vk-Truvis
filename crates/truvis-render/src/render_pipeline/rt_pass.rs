use ash::vk;

use crate::apis::render_pass::RenderPass;
use crate::core::frame_context::FrameContext;
use crate::render_pipeline::{compute_subpass::ComputeSubpass, simple_rt_subpass::SimpleRtSubpass};
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::{
    commands::{barrier::GfxImageBarrier, submit_info::GfxSubmitInfo},
    gfx::Gfx,
};
use truvis_shader_binding::shader;

/// 整个 RT 管线
pub struct RtRenderPass {
    rt_pass: SimpleRtSubpass,
    blit_pass: ComputeSubpass<shader::blit::PushConstant>,
    sdr_pass: ComputeSubpass<shader::sdr::PushConstant>,
}
impl Default for RtRenderPass {
    fn default() -> Self {
        Self::new()
    }
}

impl RtRenderPass {
    pub fn new() -> Self {
        let rt_pass = SimpleRtSubpass::new();
        let bindless_manager = FrameContext::bindless_manager();
        let blit_pass = ComputeSubpass::<shader::blit::PushConstant>::new(
            &bindless_manager,
            c"main",
            TruvisPath::shader_path("imgui/blit.slang").as_str(),
        );
        let sdr_pass = ComputeSubpass::<shader::sdr::PushConstant>::new(
            &bindless_manager,
            c"main",
            TruvisPath::shader_path("pass/pp/sdr.slang").as_str(),
        );

        Self {
            rt_pass,
            blit_pass,
            sdr_pass,
        }
    }

    pub fn render(&self) {
        let frame_label = FrameContext::get().frame_label();
        let bindless_manager = FrameContext::bindless_manager();
        let frame_settings = FrameContext::get().frame_settings();

        let fif_buffers = FrameContext::get().fif_buffers.borrow();

        let color_image = fif_buffers.color_image();
        let color_image_handle = fif_buffers.color_image_bindless_handle(&bindless_manager);
        let render_target = fif_buffers.render_target_image(frame_label);
        let render_target_handle = fif_buffers.render_target_image_bindless_handle(&bindless_manager, frame_label);

        let mut submit_cmds = Vec::new();
        // ray tracing
        {
            let cmd = FrameContext::cmd_allocator_mut().alloc_command_buffer("ray-tracing");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "ray-tracing");

            // frams in flight 使用同一个 rt image，因此需要确保之前的 rt 写入已经完成
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
                &cmd,
                &frame_settings,
                &FrameContext::get().pipeline_settings(),
                color_image.handle(),
                color_image_handle,
                &FrameContext::get().per_frame_data_buffers[*frame_label],
            );

            cmd.end();

            submit_cmds.push(cmd);
        }

        // blit
        {
            let cmd = FrameContext::cmd_allocator_mut().alloc_command_buffer("blit");
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
                &bindless_manager,
                &shader::blit::PushConstant {
                    src_image: color_image_handle,
                    dst_image: render_target_handle,
                    src_image_size: glam::uvec2(frame_settings.frame_extent.width, frame_settings.frame_extent.height)
                        .into(),
                    offset: glam::uvec2(0, 0).into(),
                },
                glam::uvec3(
                    frame_settings.frame_extent.width.div_ceil(shader::blit::SHADER_X as u32),
                    frame_settings.frame_extent.height.div_ceil(shader::blit::SHADER_Y as u32),
                    1,
                ),
            );

            cmd.end();
            submit_cmds.push(cmd);
        }

        // hdr -> sdr
        {
            let cmd = FrameContext::cmd_allocator_mut().alloc_command_buffer("hdr2sdr");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "hdr2sdr");

            // 等待之前的 compute shader 执行完成
            let rt_barrier = GfxImageBarrier::new()
                .image(render_target)
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
                &bindless_manager,
                &shader::sdr::PushConstant {
                    src_image: color_image_handle,
                    dst_image: render_target_handle,
                    image_size: glam::uvec2(frame_settings.frame_extent.width, frame_settings.frame_extent.height)
                        .into(),
                    channel: FrameContext::get().pipeline_settings().channel,
                    _padding_1: Default::default(),
                },
                glam::uvec3(
                    frame_settings.frame_extent.width.div_ceil(shader::blit::SHADER_X as u32),
                    frame_settings.frame_extent.height.div_ceil(shader::blit::SHADER_Y as u32),
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
