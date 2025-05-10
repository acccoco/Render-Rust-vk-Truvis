use ash::vk;
use shader_layout_macro::ShaderLayout;
use std::collections::HashMap;
use truvis_rhi::core::descriptor::{RhiDescriptorSet, RhiDescriptorSetLayout};
use truvis_rhi::rhi::Rhi;

#[derive(ShaderLayout)]
pub struct BindlessTextureBindings {
    #[binding = 0]
    #[descriptor_type = "COMBINED_IMAGE_SAMPLER"]
    #[stage = "FRAGMENT"]
    #[count = 128]
    #[flags = "PARTIALLY_BOUND | UPDATE_AFTER_BIND"]
    _textures: (),

    #[binding = 1]
    #[descriptor_type = "STORAGE_IMAGE"]
    #[stage = "FRAGMENT"]
    #[count = 128]
    #[flags = "PARTIALLY_BOUND | UPDATE_AFTER_BIND"]
    _images: (),
}

pub struct BindlessManager {
    pub bindless_layout: RhiDescriptorSetLayout<BindlessTextureBindings>,
    pub bindless_set: RhiDescriptorSet<BindlessTextureBindings>,

    /// key: texture path
    ///
    /// value: bindless idx
    pub texture_map: HashMap<String, u32>,
}
impl BindlessManager {
    pub fn new(rhi: &Rhi) -> Self {
        let bindless_layout = RhiDescriptorSetLayout::<BindlessTextureBindings>::new(
            rhi,
            vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
            "bindless-layout",
        );
        let bindless_descriptor_set = RhiDescriptorSet::<BindlessTextureBindings>::new(
            rhi,
            rhi.descriptor_pool(),
            &bindless_layout,
            "bindless-descriptor-set",
        );

        Self {
            bindless_layout,
            bindless_set: bindless_descriptor_set,

            texture_map: HashMap::new(),
        }
    }

    pub fn register_texture(_texture_path: String) {
        todo!()
    }

    pub fn get_texture_idx(&self, _texture_path: &str) -> Option<u32> {
        // TODO
        Some(0)
    }
}
