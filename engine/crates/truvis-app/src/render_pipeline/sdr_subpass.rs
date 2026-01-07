use ash::vk;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_render_graph::compute_pass::ComputePass;
use truvis_render_graph::render_context::RenderContext;
use truvis_render_graph::render_graph_v2::{RgImageHandle, RgImageState, RgPass, RgPassBuilder, RgPassContext};
use truvis_render_interface::global_descriptor_sets::GlobalDescriptorSets;
use truvis_render_interface::handles::GfxImageViewHandle;
use truvis_shader_binding::truvisl;

pub struct SdrPassData {
    pub src_image: GfxImageViewHandle,
    pub src_image_size: vk::Extent2D,

    pub dst_image: GfxImageViewHandle,
    pub dst_image_size: vk::Extent2D,
}

pub struct SdrPass {
    sdr_pass: ComputePass<truvisl::sdr::PushConstant>,
}
impl SdrPass {
    pub fn new(render_descriptor_sets: &GlobalDescriptorSets) -> Self {
        let sdr_pass = ComputePass::<truvisl::sdr::PushConstant>::new(
            render_descriptor_sets,
            c"main",
            TruvisPath::shader_build_path_str("pass/pp/sdr.slang").as_str(),
        );

        Self { sdr_pass }
    }

    pub fn exec(&self, cmd: &GfxCommandBuffer, data: SdrPassData, render_context: &RenderContext) {
        let src_image_bindless_handle = render_context.bindless_manager.get_shader_uav_handle(data.src_image);
        let dst_image_bindless_handle = render_context.bindless_manager.get_shader_uav_handle(data.dst_image);

        self.sdr_pass.exec(
            cmd,
            render_context,
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

pub struct SdrRgPass<'a> {
    pub sdr_pass: &'a SdrPass,

    // TODO 暂时使用这个肮脏的实现
    pub render_context: &'a RenderContext,

    pub src_image: RgImageHandle,
    pub dst_image: RgImageHandle,

    pub src_image_extent: vk::Extent2D,
    pub dst_image_extent: vk::Extent2D,
}
impl<'a> RgPass for SdrRgPass<'a> {
    fn setup(&mut self, builder: &mut RgPassBuilder) {
        builder.read_image(self.src_image, RgImageState::STORAGE_READ_COMPUTE);
        builder.write_image(self.dst_image, RgImageState::STORAGE_WRITE_COMPUTE);
    }

    fn execute(&self, ctx: &RgPassContext<'_>) {
        let src_image = ctx.get_image_view(self.src_image).unwrap();
        let dst_image = ctx.get_image_view(self.dst_image).unwrap();

        self.exec(
            ctx.cmd,
            SdrPassData {
                src_image: src_image.handle(),
                dst_image: dst_image.handle(),
                src_image_size: self.src_image_extent,
                dst_image_size: self.dst_image_extent,
            },
            self.render_context,
        );
    }
}
