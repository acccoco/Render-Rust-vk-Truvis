use crate::resource::ImageLoader;
use ash::vk;
use itertools::Itertools;
use shader_layout_macro::ShaderLayout;
use std::collections::HashMap;
use std::rc::Rc;
use truvis_rhi::core::descriptor::{RhiDescriptorSet, RhiDescriptorSetLayout};
use truvis_rhi::core::device::RhiDevice;
use truvis_rhi::core::texture::RhiTexture2D;
use truvis_rhi::rhi::Rhi;
use truvis_rhi::shader_cursor::ShaderCursor;

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
    pub bindless_sets: Vec<RhiDescriptorSet<BindlessTextureBindings>>,

    /// key: texture path
    ///
    /// value: bindless idx
    texture_map: HashMap<String, u32>,
    textures: HashMap<String, RhiTexture2D>,

    device: Rc<RhiDevice>,
}
impl BindlessManager {
    pub fn new(rhi: &Rhi, frames_in_flight: usize) -> Self {
        let bindless_layout = RhiDescriptorSetLayout::<BindlessTextureBindings>::new(
            rhi,
            vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
            "bindless-layout",
        );
        let bindless_descriptor_sets = (0..frames_in_flight)
            .into_iter()
            .map(|idx| {
                RhiDescriptorSet::<BindlessTextureBindings>::new(
                    rhi,
                    rhi.descriptor_pool(),
                    &bindless_layout,
                    &format!("bindless-descriptor-set-{idx}"),
                )
            })
            .collect_vec();

        Self {
            bindless_layout,
            bindless_sets: bindless_descriptor_sets,

            texture_map: HashMap::new(),
            textures: HashMap::new(),

            device: rhi.device.clone(),
        }
    }

    /// 在每一帧绘制之前，将纹理数据绑定到 descriptor set 中
    pub fn prepare_render_data(&mut self, frame_idx: usize) {
        let mut image_infos = vec![];
        image_infos.reserve(self.textures.iter().len());

        self.texture_map.clear();

        for (tex_idx, (tex_name, tex)) in self.textures.iter().enumerate() {
            image_infos.push(tex.descriptor_image_info(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL));
            self.texture_map.insert(tex_name.clone(), tex_idx as u32);
        }

        let write =
            BindlessTextureBindings::textures().write_image(self.bindless_sets[frame_idx].handle, 0, image_infos);
        self.device.write_descriptor_sets(std::slice::from_ref(&write));
    }

    pub fn register_texture(&mut self, rhi: &Rhi, texture_path: String) {
        if self.texture_map.contains_key(&texture_path) {
            return;
        }

        let texture = ImageLoader::load_image(rhi, std::path::Path::new(&texture_path));

        self.textures.insert(texture_path, texture);
    }

    pub fn get_texture_idx(&self, texture_path: &str) -> Option<u32> {
        self.texture_map.get(texture_path).map(|idx| *idx)
    }
}
