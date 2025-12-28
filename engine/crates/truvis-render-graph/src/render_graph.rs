use ash::vk;
use std::collections::HashMap;
use truvis_render_interface::handles::GfxImageViewHandle;

pub struct GraphImage {
    view: GfxImageViewHandle,
    layout: vk::ImageLayout,

    stage: vk::PipelineStageFlags2,
    usage: vk::AccessFlags2,
}

#[derive(Default)]
pub struct RenderGraph {
    pub maps: HashMap<String, GraphImage>,
}
