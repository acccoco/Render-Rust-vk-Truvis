use ash::vk;

use truvis_gfx::{
    commands::{barrier::ImageBarrier, submit_info::SubmitInfo},
    gfx::Gfx,
};
use truvis_model_manager::components::geometry::Geometry;
use truvis_model_manager::vertex::aos_pos_color::VertexLayoutAoSPosColor;
use truvis_render::renderer::frame_context::FrameContext;

use crate::shader_toy_pass::ShaderToyPass;

pub struct ShaderToyPipeline {
    shader_toy_pass: ShaderToyPass,
}
impl ShaderToyPipeline {
    pub fn new(color_format: vk::Format) -> Self {
        let shader_toy_pass = ShaderToyPass::new(color_format);
        Self { shader_toy_pass }
    }

    pub fn render(&self, shape: &Geometry<VertexLayoutAoSPosColor>) {
        let fif_buffers = FrameContext::get().fif_buffers.borrow();

        let frame_label = FrameContext::frame_label();
        let render_target = fif_buffers.render_target_image(frame_label);
        let render_target_view = fif_buffers.render_target_image_view(frame_label);
        let frame_settings = FrameContext::get().frame_settings();

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

            self.shader_toy_pass.draw(&cmd, &frame_settings, render_target_view.handle(), shape);

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
            Gfx::get().gfx_queue().submit(vec![SubmitInfo::new(&[cmd])], None);
        }
    }
}
