//! 无光照材质系统


use ash::vk;
use shader_layout_macro::ShaderLayout;
use truvis_rhi::shader_cursor::ShaderCursorType;

#[derive(ShaderLayout)]
struct UnlitMatBindings
{
    #[binding = 0]
    #[stage = "FRAGMENT"]
    #[descriptor_type = "SAMPLED_IMAGE"]
    color_texture: ShaderCursorType,

    #[binding = 1]
    #[stage = "FRAGMENT"]
    #[descriptor_type = "STORAGE_BUFFER"]
    mateiral_params: ShaderCursorType,
}


#[repr(C)]
struct UnlitMat {}
