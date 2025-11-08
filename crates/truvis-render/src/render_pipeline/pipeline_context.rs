use truvis_gfx::resources::special_buffers::structured_buffer::StructuredBuffer;
use truvis_shader_binding::shader;

use crate::renderer::frame_buffers::FrameBuffers;
use crate::{
    pipeline_settings::{FrameSettings, PipelineSettings},
    platform::timer::Timer,
    renderer::gpu_scene::GpuScene,
};

/// Rt 管线上下文，每帧重建
pub struct PipelineContext<'a> {
    pub gpu_scene: &'a GpuScene,
    pub per_frame_data: &'a StructuredBuffer<shader::PerFrameData>,
    pub frame_settings: &'a FrameSettings,
    pub pipeline_settings: &'a PipelineSettings,
    pub timer: &'a Timer,
    pub frame_buffers: &'a FrameBuffers,
}
impl<'a> PipelineContext<'a> {}
