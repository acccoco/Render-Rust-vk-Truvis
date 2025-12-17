use crate::graph::node::ImageNode;
use crate::render_context::RenderContext;
use crate::render_pipeline::compute_subpass::ComputeSubpass;
use ash::vk;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_render_base::bindless_manager::BindlessManager;
use truvis_resource::handles::{GfxImageViewHandle, GfxTextureHandle};
use truvis_shader_binding::truvisl;

pub struct SdrSubpassDep {
    pub src_iamge: ImageNode,
    pub dst_image: ImageNode,
}
impl Default for SdrSubpassDep {
    fn default() -> Self {
        Self {
            src_iamge: ImageNode {
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

pub struct SdrSubpassData {
    pub src_image: GfxImageViewHandle,
    pub src_image_size: vk::Extent2D,

    pub dst_image: GfxTextureHandle,
    pub dst_image_size: vk::Extent2D,
}

pub struct SdrSubpass {
    sdr_pass: ComputeSubpass<truvisl::sdr::PushConstant>,
}
impl SdrSubpass {
    pub fn new(bindless_manager: &BindlessManager) -> Self {
        let sdr_pass = ComputeSubpass::<truvisl::sdr::PushConstant>::new(
            bindless_manager,
            c"main",
            TruvisPath::shader_path("pass/pp/sdr.slang").as_str(),
        );

        Self { sdr_pass }
    }

    pub fn exec(&self, cmd: &GfxCommandBuffer, data: SdrSubpassData, render_context: &RenderContext) {
        let src_image_bindless_handle = render_context.bindless_manager.get_image_handle(data.src_image).unwrap();
        let dst_image_bindless_handle =
            render_context.bindless_manager.get_image_handle_in_texture(data.dst_image).unwrap();

        self.sdr_pass.exec(
            &cmd,
            &render_context.bindless_manager,
            &truvisl::sdr::PushConstant {
                src_image: src_image_bindless_handle.0,
                dst_image: dst_image_bindless_handle.0,
                image_size: glam::uvec2(data.src_image_size.width, data.src_image_size.height).into(),
                channel: render_context.pipeline_settings.channel,
                _padding_1: Default::default(),
            },
            glam::uvec3(
                data.dst_image_size.width.div_ceil(truvisl::blit::SHADER_X as u32),
                data.dst_image_size.height.div_ceil(truvisl::blit::SHADER_Y as u32),
                1,
            ),
        );
    }
}
