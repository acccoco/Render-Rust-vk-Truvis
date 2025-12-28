use crate::resources::fif_buffer::FifBuffers;
use truvis_gfx::resources::special_buffers::structured_buffer::GfxStructuredBuffer;
use truvis_render_interface::bindless_manager::BindlessManager;
use truvis_render_interface::frame_counter::FrameCounter;
use truvis_render_interface::gfx_resource_manager::GfxResourceManager;
use truvis_render_interface::global_descriptor_sets::GlobalDescriptorSets;
use truvis_render_interface::pipeline_settings::{AccumData, FrameSettings, PipelineSettings};
use truvis_render_interface::sampler_manager::RenderSamplerManager;
use truvis_scene::gpu_scene::GpuScene;
use truvis_scene::scene_manager::SceneManager;
use truvis_shader_binding::truvisl;

// Render 期间不可变
pub struct RenderContext {
    pub scene_manager: SceneManager,
    pub gpu_scene: GpuScene,
    pub fif_buffers: FifBuffers,
    pub bindless_manager: BindlessManager,
    pub per_frame_data_buffers: [GfxStructuredBuffer<truvisl::PerFrameData>; FrameCounter::fif_count()],
    pub gfx_resource_manager: GfxResourceManager,
    pub sampler_manager: RenderSamplerManager,

    pub global_descriptor_sets: GlobalDescriptorSets,

    pub delta_time_s: f32,
    pub total_time_s: f32,
    pub accum_data: AccumData,

    pub frame_counter: FrameCounter,
    pub frame_settings: FrameSettings,
    pub pipeline_settings: PipelineSettings,
}
