use crate::gui::gui_pass::GuiPass;
use crate::render_pipeline::compute_pass::ComputePass;
use crate::render_pipeline::pipeline_context::PipelineContext;
use crate::render_pipeline::rt_pass::SimlpeRtPass;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::pipeline_settings::PipelineSettings;
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
    gui_pass: GuiPass,
}
impl RtPipeline {
    pub fn new(rhi: &Rhi, pipeline_settings: &PipelineSettings, bindless_mgr: Rc<RefCell<BindlessManager>>) -> Self {
        let rt_pass = SimlpeRtPass::new(rhi, bindless_mgr.clone());
        let blit_pass = ComputePass::<shader::blit::PushConstant>::new(
            rhi,
            &bindless_mgr.borrow(),
            cstr::cstr!("main"),
            "shader/build/imgui/blit.slang.spv",
        );
        let gui_pass = GuiPass::new(rhi, pipeline_settings, bindless_mgr.clone());

        Self {
            rt_pass,
            blit_pass,
            gui_pass,
        }
    }

    pub fn render(&self, ctx: PipelineContext) {
        let PipelineContext {
            rhi,
            gpu_scene,
            bindless_mgr,
            frame_ctx,
            gui,
            timer: _,
            per_frame_data,
        } = ctx;
        let frame_settings = frame_ctx.frame_settings();
        let present_image = frame_ctx.crt_present_image();
        let present_image_handle = frame_ctx.crt_present_image_bindless_handle(&bindless_mgr.borrow());
        let rt_image_handle = frame_ctx.crt_rt_bindless_handle(&bindless_mgr.borrow());
        let rt_image = frame_ctx.crt_rt_image().handle();

        // ray tracing
        {
            let cmd = frame_ctx.alloc_command_buffer("ray-tracing");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "ray-tracing");
            // frams in flight 使用同一个 rt image，因此需要确保之前的 rt 写入已经完成
            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[RhiImageBarrier::new()
                    .image(rt_image)
                    .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                    .src_mask(vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR, vk::AccessFlags2::SHADER_STORAGE_WRITE)
                    .dst_mask(
                        vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                        vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                    )],
            );

            self.rt_pass.ray_trace(&cmd, frame_ctx, &frame_settings, per_frame_data, gpu_scene);

            cmd.end();

            rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[cmd]).wait_infos(&[])], None);
        }

        // blit
        {
            let cmd = frame_ctx.alloc_command_buffer("blit");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "blit");

            // 只需要建立起执行依赖即可，确保 present 完成后，再进行 layout trans
            // COLOR_ATTACHMENT_READ 对应 blend 等操作
            let present_image_barrier = RhiImageBarrier::new()
                .image(present_image)
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL)
                // 注：这里 bottom 是必须的
                .src_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                .dst_mask(vk::PipelineStageFlags2::COMPUTE_SHADER, vk::AccessFlags2::SHADER_WRITE);

            // 等待 ray-tracing 执行完成
            let rt_barrier = RhiImageBarrier::new()
                .image(rt_image)
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .src_mask(vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR, vk::AccessFlags2::SHADER_STORAGE_WRITE)
                .dst_mask(vk::PipelineStageFlags2::COMPUTE_SHADER, vk::AccessFlags2::SHADER_READ);
            cmd.image_memory_barrier(vk::DependencyFlags::empty(), &[present_image_barrier, rt_barrier]);

            self.blit_pass.exec(
                &cmd,
                &bindless_mgr.borrow(),
                &shader::blit::PushConstant {
                    src_image: rt_image_handle,
                    dst_image: present_image_handle,
                    src_image_size: glam::uvec2(frame_settings.rt_extent.width, frame_settings.rt_extent.height).into(),
                    offset: glam::uvec2(frame_settings.rt_offset.x as u32, frame_settings.rt_offset.x as u32).into(),
                },
                glam::uvec3(
                    frame_settings.rt_extent.width.div_ceil(shader::blit::SHADER_X as u32),
                    frame_settings.rt_extent.height.div_ceil(shader::blit::SHADER_Y as u32),
                    1,
                ),
            );

            cmd.end();
            rhi.graphics_queue.submit(
                vec![RhiSubmitInfo::new(&[cmd]).wait_infos(&[(
                    frame_ctx.current_present_complete_semaphore(),
                    vk::PipelineStageFlags2::COMPUTE_SHADER,
                )])],
                None,
            );
        }

        // gui
        {
            let cmd = frame_ctx.alloc_command_buffer("gui");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "gui");

            // 等待 blit 操作完成，并且将 image layout 转换为 COLOR_ATTACHMENT_OPTIMAL
            let blit_barrier = RhiImageBarrier::new()
                .image(present_image)
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .layout_transfer(vk::ImageLayout::GENERAL, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .src_mask(vk::PipelineStageFlags2::COMPUTE_SHADER, vk::AccessFlags2::SHADER_WRITE)
                // 可能有 blend 操作，因此需要 COLOR_ATTACHMENT_READ
                .dst_mask(
                    vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                    vk::AccessFlags2::COLOR_ATTACHMENT_WRITE | vk::AccessFlags2::COLOR_ATTACHMENT_READ,
                );
            cmd.image_memory_barrier(vk::DependencyFlags::empty(), &[blit_barrier]);

            self.gui_pass.draw(rhi, frame_ctx, &frame_settings, &cmd, gui);

            let present_layout_trans_barrier = RhiImageBarrier::new()
                .image(present_image)
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .layout_transfer(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR)
                .src_mask(
                    vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                    vk::AccessFlags2::COLOR_ATTACHMENT_WRITE | vk::AccessFlags2::COLOR_ATTACHMENT_READ,
                )
                .dst_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty());
            cmd.image_memory_barrier(vk::DependencyFlags::empty(), &[present_layout_trans_barrier]);

            cmd.end();

            rhi.graphics_queue.submit(
                vec![RhiSubmitInfo::new(&[cmd]).signal_infos(&[(
                    frame_ctx.crt_render_complete_semaphore(),
                    vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
                )])],
                Some(frame_ctx.crt_fence().clone()),
            );
        }
    }
}
