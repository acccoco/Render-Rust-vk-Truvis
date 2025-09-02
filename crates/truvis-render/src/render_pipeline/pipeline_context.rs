use std::{cell::RefCell, rc::Rc};

use shader_binding::shader;
use truvis_rhi::{render_context::RenderContext, resources::special_buffers::structured_buffer::StructuredBuffer};

use crate::{
    pipeline_settings::{FrameSettings, PipelineSettings},
    platform::timer::Timer,
    renderer::{
        bindless::BindlessManager, cmd_allocator::CmdAllocator, frame_buffers::FrameBuffers,
        frame_controller::FrameController, gpu_scene::GpuScene,
    },
};

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
