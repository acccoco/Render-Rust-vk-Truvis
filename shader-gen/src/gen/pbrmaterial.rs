// 自动生成的代码 - 请勿手动修改

use crate::prelude::*;
use shader_layout_macro::ShaderLayout;

#[derive(ShaderLayout)]
pub struct PBRMaterial {
    
    #[binding = 0]
    pub material_data: MaterialData,
    
    #[binding = 1]
    pub albedo_texture: texture2d,
    
    #[binding = 2]
    pub main_sampler: sampler,
    
}