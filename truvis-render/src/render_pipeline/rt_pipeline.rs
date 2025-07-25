use crate::render_pipeline::compute_pass::ComputePass;
use crate::render_pipeline::pipeline_context::PipelineContext;
use crate::render_pipeline::rt_pass::SimlpeRtPass;
use crate::renderer::bindless::BindlessManager;
use ash::vk;
use shader_binding::shader;
use std::cell::RefCell;
use std::rc::Rc;
use truvis_rhi::core::command_queue::RhiSubmitInfo;
use truvis_rhi::core::synchronize::RhiImageBarrier;
use truvis_rhi::rhi::Rhi;

/// 整个 RT 管线
pub struct RtPipeline {
    rt_pass: SimlpeRtPass,
    blit_pass: ComputePass<shader::blit::PushConstant>,
    sdr_pass: ComputePass<shader::sdr::PushConstant>,
}
impl RtPipeline {
    pub fn new(rhi: &Rhi, bindless_mgr: Rc<RefCell<BindlessManager>>) -> Self {
        let rt_pass = SimlpeRtPass::new(rhi, bindless_mgr.clone());
        let blit_pass = ComputePass::<shader::blit::PushConstant>::new(
            rhi,
            &bindless_mgr.borrow(),
            cstr::cstr!("main"),
            "shader/build/imgui/blit.slang.spv",
        );
        let sdr_pass = ComputePass::<shader::sdr::PushConstant>::new(
            rhi,
            &bindless_mgr.borrow(),
            c"main",
            "shader/build/pass/pp/sdr.slang.spv",
        );

        Self {
            rt_pass,
            blit_pass,
            sdr_pass,
        }
    }

    pub fn render(&self, ctx: PipelineContext) {
        let PipelineContext {
            rhi,
            gpu_scene,
            bindless_mgr,
            frame_ctrl,
            timer: _,
            per_frame_data,
            frame_settings,
            frame_buffers,
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
            let cmd = frame_ctrl.alloc_command_buffer("ray-tracing");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "ray-tracing");

            // frams in flight 使用同一个 rt image，因此需要确保之前的 rt 写入已经完成
            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[RhiImageBarrier::new()
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
            let cmd = frame_ctrl.alloc_command_buffer("blit");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "blit");

            // 等待 ray-tracing 执行完成
            let rt_barrier = RhiImageBarrier::new()
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
            let cmd = frame_ctrl.alloc_command_buffer("hdr2sdr");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "hdr2sdr");

            // 等待之前的 compute shader 执行完成
            let rt_barrier = RhiImageBarrier::new()
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

        rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&submit_cmds)], None);
    }
}
