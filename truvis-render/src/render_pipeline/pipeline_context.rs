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
pub struct PipelineContext<'a> {
    pub rhi: &'a Rhi,
    pub gpu_scene: &'a GpuScene,
    pub bindless_mgr: Rc<RefCell<BindlessManager>>,
    pub per_frame_data: &'a RhiStructuredBuffer<shader::PerFrameData>,
    pub frame_ctx: &'a mut FrameContext,
    pub timer: &'a Timer,
}
impl<'a> PipelineContext<'a> {}
