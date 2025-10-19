use ash::vk;
use model_manager::{component::Geometry, vertex::vertex_pc::VertexPosColor};
use truvis_render::renderer::frame_context::FrameContext;
use truvis_render::{pipeline_settings::FrameSettings, render_pipeline::pipeline_context::PipelineContext};
use truvis_rhi::{
    commands::{barrier::ImageBarrier, submit_info::SubmitInfo},
    render_context::RenderContext,
};

use crate::triangle_pass::TrianglePass;

pub struct TrianglePipeline {
    triangle_pass: TrianglePass,
}
impl TrianglePipeline {
    pub fn new(frame_settings: &FrameSettings) -> Self {
        let triangle_pass = TrianglePass::new(frame_settings);
        Self { triangle_pass }
    }

    pub fn render(&self, ctx: PipelineContext, shape: &Geometry<VertexPosColor>) {
        let PipelineContext {
            gpu_scene: _,
            timer: _,
            per_frame_data: _,
            frame_settings,
            pipeline_settings: _,
            frame_buffers,
        } = ctx;
        let frame_label = FrameContext::get().frame_ctrl.frame_label();
        let render_target = frame_buffers.render_target_image(frame_label);

        // render triangle
        {
            let cmd = FrameContext::cmd_allocator_mut().alloc_command_buffer("triangle");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "triangle");

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

            self.triangle_pass.draw(&cmd, frame_label, frame_buffers, frame_settings, shape);

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
