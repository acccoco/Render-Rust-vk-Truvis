use ash::vk;

use truvis_model_manager::components::geometry::Geometry;
use truvis_model_manager::vertex::aos_pos_color::VertexLayoutAoSPosColor;
use truvis_render::render_pipeline::pipeline_context::PipelineContext;
use truvis_render::renderer::frame_context::FrameContext;
use truvis_rhi::{
    commands::{barrier::ImageBarrier, submit_info::SubmitInfo},
    render_context::RenderContext,
};

use crate::shader_toy_pass::ShaderToyPass;

pub struct ShaderToyPipeline {
    shader_toy_pass: ShaderToyPass,
}
impl ShaderToyPipeline {
    pub fn new(color_format: vk::Format) -> Self {
        let shader_toy_pass = ShaderToyPass::new(color_format);
        Self { shader_toy_pass }
    }

    pub fn render(&self, ctx: PipelineContext, shape: &Geometry<VertexLayoutAoSPosColor>) {
        let PipelineContext {
            gpu_scene: _,
            timer,
            per_frame_data: _,
            frame_settings,
            pipeline_settings: _,
            frame_buffers,
        } = ctx;
        let frame_ctrl = FrameContext::get().frame_ctrl.clone();
        let frame_label = frame_ctrl.frame_label();
        let render_target = frame_buffers.render_target_image(frame_label);
        let render_target_view = frame_buffers.render_target_image_view(frame_label);

        // render shader toy
        {
            let cmd = FrameContext::cmd_allocator_mut().alloc_command_buffer("shader-toy");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "shader-toy");

            // 将 render target 从 general -> color attachment
            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[ImageBarrier::new()
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

            self.shader_toy_pass.draw(&cmd, &frame_ctrl, frame_settings, render_target_view.handle(), timer, shape);

            // 将 render target 从 color attachment -> general
            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[ImageBarrier::new()
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
            RenderContext::get().graphics_queue().submit(vec![SubmitInfo::new(&[cmd])], None);
        }
    }
}
