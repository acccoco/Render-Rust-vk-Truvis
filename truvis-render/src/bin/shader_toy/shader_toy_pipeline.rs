use crate::shader_toy_pass::ShaderToyPass;
use ash::vk;
use model_manager::component::DrsGeometry;
use model_manager::vertex::vertex_pc::VertexPosColor;
use std::cell::RefCell;
use std::rc::Rc;
use truvis_render::gui::gui_pass::GuiPass;
use truvis_render::render_pipeline::pipeline_context::PipelineContext;
use truvis_render::renderer::bindless::BindlessManager;
use truvis_render::renderer::pipeline_settings::PipelineSettings;
use truvis_rhi::core::command_queue::RhiSubmitInfo;
use truvis_rhi::core::synchronize::RhiImageBarrier;
use truvis_rhi::rhi::Rhi;

pub struct ShaderToyPipeline {
    shader_toy_pass: ShaderToyPass,
    gui_pass: GuiPass,
}
impl ShaderToyPipeline {
    pub fn new(rhi: &Rhi, pipeline_settings: &PipelineSettings, bindless_mgr: Rc<RefCell<BindlessManager>>) -> Self {
        let shader_toy_pass = ShaderToyPass::new(rhi, pipeline_settings);
        let gui_pass = GuiPass::new(rhi, pipeline_settings, bindless_mgr);
        Self {
            shader_toy_pass,
            gui_pass,
        }
    }

    pub fn render(&self, ctx: PipelineContext, shape: &DrsGeometry<VertexPosColor>) {
        let PipelineContext {
            rhi,
            gpu_scene: _,
            bindless_mgr: _,
            frame_ctx,
            gui,
            per_frame_data: _,
            timer,
        } = ctx;
        let frame_settings = frame_ctx.frame_settings();
        let present_image = frame_ctx.crt_present_image();

        // render shader toy
        {
            let cmd = frame_ctx.alloc_command_buffer("shader-toy");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "shader-toy");

            let present_image_barrier = RhiImageBarrier::new()
                .image(present_image)
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                // 注：这里 bottom 是必须的
                .src_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                .dst_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, vk::AccessFlags2::COLOR_ATTACHMENT_WRITE);
            cmd.image_memory_barrier(vk::DependencyFlags::empty(), &[present_image_barrier]);
            self.shader_toy_pass.draw(&cmd, frame_ctx, timer, shape);

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

            let draw_barrier = RhiImageBarrier::new()
                .image(present_image)
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .src_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                // 可能有 blend 操作，因此需要 COLOR_ATTACHMENT_READ
                .dst_mask(
                    vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                    vk::AccessFlags2::COLOR_ATTACHMENT_WRITE | vk::AccessFlags2::COLOR_ATTACHMENT_READ,
                );
            cmd.image_memory_barrier(vk::DependencyFlags::empty(), &[draw_barrier]);

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
