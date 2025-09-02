use crate::pipeline_settings::{FrameSettings, PipelineSettings};
use crate::platform::timer::Timer;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::cmd_allocator::CmdAllocator;
use crate::renderer::frame_buffers::FrameBuffers;
use crate::renderer::frame_controller::FrameController;
use crate::renderer::gpu_scene::GpuScene;
use shader_binding::shader;
use std::cell::RefCell;
use std::rc::Rc;
use truvis_rhi::resources::special_buffers::structured_buffer::StructuredBuffer;
use truvis_rhi::render_context::RenderContext;

/// Rt 管线上下文，每帧重建
pub struct PipelineContext<'a> {
    pub rhi: &'a RenderContext,
    pub gpu_scene: &'a GpuScene,
    pub bindless_mgr: Rc<RefCell<BindlessManager>>,
    pub per_frame_data: &'a StructuredBuffer<shader::PerFrameData>,
    pub frame_ctrl: &'a FrameController,
    pub cmd_allocator: &'a mut CmdAllocator,
    pub frame_settings: &'a FrameSettings,
    pub pipeline_settings: &'a PipelineSettings,
    pub timer: &'a Timer,
    pub frame_buffers: &'a FrameBuffers,
}
impl<'a> PipelineContext<'a> {}
