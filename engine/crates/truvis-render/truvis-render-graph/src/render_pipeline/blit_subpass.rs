use crate::graph::node::ImageNode;
use crate::render_context::RenderContext;
use crate::render_pipeline::compute_subpass::ComputeSubpass;
use ash::vk;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_render_base::global_descriptor_sets::GlobalDescriptorSets;
use truvis_resource::handles::{GfxImageViewHandle, GfxTextureHandle};
use truvis_shader_binding::truvisl;

pub struct BlitSubpassDep {
    pub src_image: ImageNode,
    pub dst_image: ImageNode,
}
impl Default for BlitSubpassDep {
    fn default() -> Self {
        Self {
            src_image: ImageNode {
                stage: vk::PipelineStageFlags2::COMPUTE_SHADER,
                access: vk::AccessFlags2::SHADER_READ,
                layout: vk::ImageLayout::GENERAL,
            },
            dst_image: ImageNode {
                stage: vk::PipelineStageFlags2::COMPUTE_SHADER,
                access: vk::AccessFlags2::SHADER_WRITE,
                layout: vk::ImageLayout::GENERAL,
            },
        }
    }
}

pub struct BlitSubpassData {
    pub src_image: GfxImageViewHandle,
    pub dst_image: GfxTextureHandle,

    pub src_image_size: vk::Extent2D,
    pub dst_image_size: vk::Extent2D,
}

pub struct BlitSubpass {
    blit_pass: ComputeSubpass<truvisl::blit::PushConstant>,
}
impl BlitSubpass {
    pub fn new(render_descriptor_sets: &GlobalDescriptorSets) -> Self {
        let blit_pass = ComputeSubpass::<truvisl::blit::PushConstant>::new(
            render_descriptor_sets,
            c"main",
            TruvisPath::shader_path("imgui/blit.slang").as_str(),
        );

        Self { blit_pass }
    }

    pub fn exec(&self, cmd: &GfxCommandBuffer, data: BlitSubpassData, render_context: &RenderContext) {
        let src_image_bindless_handle = render_context.bindless_manager.get_shader_uav_handle(data.src_image);
        let dst_image_bindless_handle =
            render_context.bindless_manager.get_shader_uav_handle_with_texture(data.dst_image);
        self.blit_pass.exec(
            cmd,
            render_context,
            &truvisl::blit::PushConstant {
                src_image: src_image_bindless_handle.0,
                dst_image: dst_image_bindless_handle.0,
                src_image_size: glam::uvec2(data.src_image_size.width, data.dst_image_size.height).into(),
                offset: glam::uvec2(0, 0).into(),
            },
            glam::uvec3(
                data.dst_image_size.width.div_ceil(truvisl::blit::SHADER_X as u32),
                data.dst_image_size.height.div_ceil(truvisl::blit::SHADER_Y as u32),
                1,
            ),
        );
    }
}
