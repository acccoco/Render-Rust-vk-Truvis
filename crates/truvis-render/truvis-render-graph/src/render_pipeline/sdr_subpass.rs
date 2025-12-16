use ash::vk;
use crate::graph::node::ImageNode;
use crate::render_pipeline::compute_subpass::ComputeSubpass;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_render_base::bindless_manager::BindlessManager;
use truvis_resource::handles::GfxTextureHandle;
use truvis_shader_binding::truvisl;
use crate::render_context::RenderContext;

pub struct SdrSubpassDep {
    pub image: ImageNode,
}

pub struct SdrSubpassData {
    pub image: GfxTextureHandle,
    pub image_size: vk::Extent2D,
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
        let image_bindless_handle =
            render_context.bindless_manager.get_image_handle_in_texture(data.image).unwrap();

        self.sdr_pass.exec(
            &cmd,
            &render_context.bindless_manager,
            &truvisl::sdr::PushConstant {
                src_image: color_image_bindless_handle.0,
                dst_image: render_target_image_bindless_handle.0,
                image_size: glam::uvec2(frame_settings.frame_extent.width, frame_settings.frame_extent.height).into(),
                channel: render_context.pipeline_settings.channel,
                _padding_1: Default::default(),
            },
            glam::uvec3(
                frame_settings.frame_extent.width.div_ceil(truvisl::blit::SHADER_X as u32),
                frame_settings.frame_extent.height.div_ceil(truvisl::blit::SHADER_Y as u32),
                1,
            ),
        );
    }
}
