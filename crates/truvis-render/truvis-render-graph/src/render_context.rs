use crate::resources::fif_buffer::FifBuffers;
use truvis_gfx::resources::special_buffers::structured_buffer::GfxStructuredBuffer;
use truvis_render_base::bindless_manager::BindlessManager;
use truvis_render_base::cmd_allocator::CmdAllocator;
use truvis_render_base::frame_counter::FrameCounter;
use truvis_render_base::pipeline_settings::{AccumData, FrameSettings, PipelineSettings};
use truvis_render_base::render_descriptor_sets::RenderDescriptorSets;
use truvis_render_base::stage_buffer_manager::StageBufferManager;
use truvis_render_scene::gpu_scene::GpuScene;
use truvis_render_scene::scene_manager::SceneManager;
use truvis_resource::gfx_resource_manager::GfxResourceManager;
use truvis_shader_binding::truvisl;

// Render 期间不可变
pub struct RenderContext {
    pub scene_manager: SceneManager,
    pub gpu_scene: GpuScene,
    pub fif_buffers: FifBuffers,
    pub bindless_manager: BindlessManager,
    pub per_frame_data_buffers: [GfxStructuredBuffer<truvisl::PerFrameData>; FrameCounter::fif_count()],
    pub gfx_resource_manager: GfxResourceManager,
    pub render_descriptor_sets: RenderDescriptorSets,
    pub delta_time_s: f32,
    pub total_time_s: f32,
    pub accum_data: AccumData,

    pub frame_counter: FrameCounter,
    pub frame_settings: FrameSettings,
    pub pipeline_settings: PipelineSettings,
}
