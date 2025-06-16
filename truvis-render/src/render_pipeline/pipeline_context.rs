use crate::pipeline_settings::FrameSettings;
use crate::platform::timer::Timer;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_buffers::FrameBuffers;
use crate::renderer::frame_controller::FrameController;
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
    pub frame_ctrl: &'a mut FrameController,
    pub frame_settings: &'a FrameSettings,
    pub timer: &'a Timer,
    pub frame_buffers: &'a FrameBuffers,
}
impl<'a> PipelineContext<'a> {}
