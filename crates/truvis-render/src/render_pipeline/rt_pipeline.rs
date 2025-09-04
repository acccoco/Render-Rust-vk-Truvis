use std::{cell::RefCell, rc::Rc};

use ash::vk;
use shader_binding::shader;
use truvis_crate_tools::resource::TruvisPath;
use truvis_rhi::{
    commands::{barrier::ImageBarrier, submit_info::SubmitInfo},
    render_context::RenderContext,
};

use crate::{
    render_pipeline::{compute_pass::ComputePass, pipeline_context::PipelineContext, rt_pass::SimlpeRtPass},
    renderer::bindless::BindlessManager,
};

/// 整个 RT 管线
pub struct RtPipeline {
    rt_pass: SimlpeRtPass,
    blit_pass: ComputePass<shader::blit::PushConstant>,
    sdr_pass: ComputePass<shader::sdr::PushConstant>,
}
impl RtPipeline {
    pub fn new(bindless_mgr: Rc<RefCell<BindlessManager>>) -> Self {
        let rt_pass = SimlpeRtPass::new(bindless_mgr.clone());
        let blit_pass = ComputePass::<shader::blit::PushConstant>::new(
            &bindless_mgr.borrow(),
            c"main",
            TruvisPath::shader_path("imgui/blit.slang.spv").as_str(),
        );
        let sdr_pass = ComputePass::<shader::sdr::PushConstant>::new(
            &bindless_mgr.borrow(),
            c"main",
            TruvisPath::shader_path("pass/pp/sdr.slang.spv").as_str(),
        );

        Self {
            rt_pass,
            blit_pass,
            sdr_pass,
        }
    }

    pub fn render(&self, ctx: PipelineContext) {
        let PipelineContext {
            gpu_scene,
            bindless_mgr,
            frame_ctrl,
            timer: _,
            per_frame_data,
            frame_settings,
            pipeline_settings,
            frame_buffers,
            cmd_allocator,
        } = ctx;
        let frame_label = frame_ctrl.frame_label();

        let color_image = frame_buffers.color_image();
        let color_image_handle = frame_buffers.color_image_bindless_handle(&bindless_mgr.borrow());
        let render_target = frame_buffers.render_target_image(frame_label);
        let render_target_handle =
            frame_buffers.render_target_image_bindless_handle(&bindless_mgr.borrow(), frame_label);

        let mut submit_cmds = Vec::new();
        // ray tracing
        {
            let cmd = cmd_allocator.alloc_command_buffer("ray-tracing");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "ray-tracing");

            // frams in flight 使用同一个 rt image，因此需要确保之前的 rt 写入已经完成
            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[ImageBarrier::new()
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
                frame_ctrl,
                frame_settings,
                pipeline_settings,
                color_image.handle(),
                color_image_handle,
                per_frame_data,
                gpu_scene,
            );

            cmd.end();

            submit_cmds.push(cmd);
        }

        // blit
        {
            let cmd = cmd_allocator.alloc_command_buffer("blit");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "blit");

            // 等待 ray-tracing 执行完成
            let rt_barrier = ImageBarrier::new()
                .image(color_image.handle())
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .src_mask(vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR, vk::AccessFlags2::SHADER_STORAGE_WRITE)
                .dst_mask(vk::PipelineStageFlags2::COMPUTE_SHADER, vk::AccessFlags2::SHADER_READ);
            cmd.image_memory_barrier(vk::DependencyFlags::empty(), &[rt_barrier]);

            self.blit_pass.exec(
                &cmd,
                &bindless_mgr.borrow(),
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
            let cmd = cmd_allocator.alloc_command_buffer("hdr2sdr");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "hdr2sdr");

            // 等待之前的 compute shader 执行完成
            let rt_barrier = ImageBarrier::new()
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
                &bindless_mgr.borrow(),
                &shader::sdr::PushConstant {
                    src_image: color_image_handle,
                    dst_image: render_target_handle,
                    image_size: glam::uvec2(frame_settings.frame_extent.width, frame_settings.frame_extent.height)
                        .into(),
                    channel: pipeline_settings.channel,
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

        RenderContext::get().graphics_queue().submit(vec![SubmitInfo::new(&submit_cmds)], None);
    }
}
