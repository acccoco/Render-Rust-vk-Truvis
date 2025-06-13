use crate::gui::gui::Gui;
use crate::platform::timer::Timer;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_context::FrameContext;
use crate::renderer::gpu_scene::GpuScene;
use shader_binding::shader;
use std::cell::RefCell;
use std::rc::Rc;
use truvis_rhi::core::buffer::RhiStructuredBuffer;
use truvis_rhi::rhi::Rhi;

/// Rt 管线上下文，每帧重建
///
/// 由于来自各个位置，因此需要分步构建，所以需要 Option
#[derive(Default)]
pub struct TempPipelineCtx<'a> {
    pub rhi: Option<&'a Rhi>,
    pub gpu_scene: Option<&'a GpuScene>,
    pub bindless_mgr: Option<Rc<RefCell<BindlessManager>>>,

    pub per_frame_data: Option<&'a RhiStructuredBuffer<shader::PerFrameData>>,

    pub frame_ctx: Option<&'a mut FrameContext>,
    pub gui: Option<&'a mut Gui>,
    pub timer: Option<&'a Timer>,
}
impl<'a> TempPipelineCtx<'a> {
    #[inline]
    pub fn to_pipeline_context(self) -> PipelineContext<'a> {
        PipelineContext::new(self)
    }
}

/// Rt 管线上下文，每帧重建
pub struct PipelineContext<'a> {
    pub rhi: &'a Rhi,
    pub gpu_scene: &'a GpuScene,
    pub bindless_mgr: Rc<RefCell<BindlessManager>>,
    pub per_frame_data: &'a RhiStructuredBuffer<shader::PerFrameData>,
    pub frame_ctx: &'a mut FrameContext,
    pub gui: &'a mut Gui,
    pub timer: &'a Timer,
}
impl<'a> PipelineContext<'a> {
    #[inline]
    pub fn new(temp_ctx: TempPipelineCtx<'a>) -> Self {
        Self {
            rhi: temp_ctx.rhi.expect("Rhi must be set"),
            gpu_scene: temp_ctx.gpu_scene.expect("GpuScene must be set"),
            bindless_mgr: temp_ctx.bindless_mgr.expect("BindlessManager must be set"),
            per_frame_data: temp_ctx.per_frame_data.expect("PerFrameData must be set"),
            frame_ctx: temp_ctx.frame_ctx.expect("FrameContext must be set"),
            gui: temp_ctx.gui.expect("Gui must be set"),
            timer: temp_ctx.timer.expect("Timer must be set"),
        }
    }
}
