use crate::shader_toy_pass::ShaderToyPass;
use ash::vk;
use model_manager::component::DrsGeometry;
use model_manager::vertex::vertex_pc::VertexPosColor;
use truvis_render::render_pipeline::pipeline_context::PipelineContext;
use truvis_rhi::core::command_queue::RhiSubmitInfo;
use truvis_rhi::core::synchronize::RhiImageBarrier;
use truvis_rhi::rhi::Rhi;

pub struct ShaderToyPipeline {
    shader_toy_pass: ShaderToyPass,
}
impl ShaderToyPipeline {
    pub fn new(rhi: &Rhi, color_format: vk::Format) -> Self {
        let shader_toy_pass = ShaderToyPass::new(rhi, color_format);
        Self { shader_toy_pass }
    }

    pub fn render(&self, ctx: PipelineContext, shape: &DrsGeometry<VertexPosColor>) {
        let PipelineContext {
            rhi,
            gpu_scene: _,
            bindless_mgr: _,
            frame_ctrl,
            timer,
            per_frame_data: _,
            frame_settings,
            frame_buffers,
        } = ctx;
        let frame_label = frame_ctrl.frame_label();
        let render_target = frame_buffers.render_target_image(frame_label);
        let render_target_view = frame_buffers.render_target_image_view(frame_label);

        // render shader toy
        {
            let cmd = frame_ctrl.alloc_command_buffer("shader-toy");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "shader-toy");

            // 将 render target 从 general -> color attachment
            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[RhiImageBarrier::new()
                    .image(render_target)
                    .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                    .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .src_mask(
                        vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        vk::AccessFlags2::COLOR_ATTACHMENT_READ | vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                    )
                    .dst_mask(
                        vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        vk::AccessFlags2::COLOR_ATTACHMENT_READ | vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                    )],
            );

            self.shader_toy_pass.draw(&cmd, frame_ctrl, frame_settings, render_target_view.handle(), timer, shape);

            // 将 render target 从 color attachment -> general
            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[RhiImageBarrier::new()
                    .image(render_target)
                    .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                    .layout_transfer(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::GENERAL)
                    .src_mask(
                        vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        vk::AccessFlags2::COLOR_ATTACHMENT_READ | vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                    )
                    .dst_mask(vk::PipelineStageFlags2::NONE, vk::AccessFlags2::NONE)],
            );

            cmd.end();
            rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[cmd])], None);
        }
    }
}
