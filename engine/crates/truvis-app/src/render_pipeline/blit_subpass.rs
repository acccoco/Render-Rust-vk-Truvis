use ash::vk;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_render_graph::compute_pass::ComputePass;
use truvis_render_graph::graph::node::ImageNode;
use truvis_render_graph::render_context::RenderContext;
use truvis_render_interface::bindless_manager::BindlessUavHandle;
use truvis_render_interface::global_descriptor_sets::GlobalDescriptorSets;
use truvis_render_interface::handles::GfxImageViewHandle;
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

pub struct BlitPassData {
    pub src_bindless_uav_handle: BindlessUavHandle,
    pub dst_bindless_uav_handle: BindlessUavHandle,

    pub src_image_size: vk::Extent2D,
    pub dst_image_size: vk::Extent2D,
}

pub struct BlitPass {
    blit_pass: ComputePass<truvisl::blit::PushConstant>,
}
impl BlitPass {
    pub fn new(render_descriptor_sets: &GlobalDescriptorSets) -> Self {
        let blit_pass = ComputePass::<truvisl::blit::PushConstant>::new(
            render_descriptor_sets,
            c"main",
            TruvisPath::shader_build_path_str("imgui/blit.slang").as_str(),
        );

        Self { blit_pass }
    }

    pub fn exec(&self, cmd: &GfxCommandBuffer, data: BlitPassData, render_context: &RenderContext) {
        self.blit_pass.exec(
            cmd,
            render_context,
            &truvisl::blit::PushConstant {
                src_image: data.src_bindless_uav_handle.0,
                dst_image: data.dst_bindless_uav_handle.0,
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
