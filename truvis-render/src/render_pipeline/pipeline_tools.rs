use ash::vk;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::synchronize::RhiImageBarrier;

pub struct PipelineTools {}
impl PipelineTools {
    /// 将 present image 的 layout 从 `vk::ImageLayout::UNDEFINED` 转换为其他
    pub fn present_image_layout_trans_to(
        cmd: &RhiCommandBuffer,
        present_image: vk::Image,
        new_layout: vk::ImageLayout,
        dst_stage: vk::PipelineStageFlags2,
        dst_access: vk::AccessFlags2,
    ) {
        cmd.image_memory_barrier(
            vk::DependencyFlags::empty(),
            &[RhiImageBarrier::new()
                .image(present_image)
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .layout_transfer(vk::ImageLayout::UNDEFINED, new_layout)
                // 注：这里 bottom 是必须的
                .src_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                .dst_mask(dst_stage, dst_access)],
        );
    }
}
