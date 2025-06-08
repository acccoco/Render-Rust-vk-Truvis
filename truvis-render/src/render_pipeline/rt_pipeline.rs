use crate::gui::ui::Gui;
use crate::pipeline_settings::FrameSettings;
use crate::render_context::RenderContext;
use crate::render_pipeline::compute::ComputePass;
use crate::render_pipeline::simple_rt::SimlpeRtPass;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::gpu_scene::GpuScene;
use crate::renderer::swapchain::RhiSwapchain;
use ash::vk;
use shader_binding::shader;
use std::cell::RefCell;
use std::rc::Rc;
use truvis_rhi::core::buffer::RhiStructuredBuffer;
use truvis_rhi::core::image::RhiImage2D;
use truvis_rhi::rhi::Rhi;

/// Rt 管线上下文
pub struct RtPipelineContext {
    pub present_image_handle: shader::ImageHandle,
    pub rt_image_handle: shader::ImageHandle,

    pub present_image: Rc<RhiImage2D>,

    pub per_frame_data: Rc<RhiStructuredBuffer<shader::PerFrameData>>,
    pub frame_settings: FrameSettings,
}

/// 整个 RT 管线
pub struct RtPipeline {
    /// 每一帧重新创建的管线上下文
    context: Option<RtPipelineContext>,

    gpu_scene: Rc<RefCell<GpuScene>>,
    bindless_mgr: Rc<RefCell<BindlessManager>>,

    rt_pass: SimlpeRtPass,
    blit_pass: ComputePass<shader::blit::PushConstant>,
}
impl RtPipeline {
    pub fn new(rhi: &Rhi, bindless_mgr: Rc<RefCell<BindlessManager>>, gpu_scene: Rc<RefCell<GpuScene>>) -> Self {
        let rt_pass = SimlpeRtPass::new(rhi, bindless_mgr.clone());
        let blit_pass = ComputePass::<shader::blit::PushConstant>::new(
            rhi,
            &bindless_mgr.borrow(),
            cstr::cstr!("main"),
            "shader/build/imgui/blit.slang.spv",
        );

        Self {
            context: None,
            rt_pass,
            blit_pass,
            gpu_scene,
            bindless_mgr,
        }
    }

    pub fn render(&self, rhi: &Rhi, render_ctx: &mut RenderContext, swapchain: &RhiSwapchain, gui: &mut Gui) {
        let ctx = self.context.as_ref().unwrap();
        let gpu_scene = self.gpu_scene.borrow();
        let bindless_mgr = self.bindless_mgr.borrow();

        let cmd = render_ctx.alloc_command_buffer("render-pipeline");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "render-pipeline");

        self.rt_pass.ray_trace(&cmd, render_ctx, &ctx.frame_settings, &ctx.per_frame_data, &gpu_scene);

        self.blit_pass.exec(
            &cmd,
            &self.bindless_mgr.borrow(),
            &shader::blit::PushConstant {
                src_image: ctx.rt_image_handle,
                dst_image: ctx.present_image_handle,
                src_image_size: glam::uvec2(ctx.frame_settings.rt_extent.width, ctx.frame_settings.rt_extent.height)
                    .into(),
                offset: glam::uvec2(ctx.frame_settings.rt_offset.x as u32, ctx.frame_settings.rt_offset.x as u32)
                    .into(),
            },
            glam::uvec3(
                ctx.frame_settings.rt_extent.width.div_ceil(shader::blit::SHADER_X as u32),
                ctx.frame_settings.rt_extent.height.div_ceil(shader::blit::SHADER_Y as u32),
                1,
            ),
        );

        gui.render(rhi, render_ctx, swapchain, &ctx.frame_settings);

        cmd.end();
    }
}
