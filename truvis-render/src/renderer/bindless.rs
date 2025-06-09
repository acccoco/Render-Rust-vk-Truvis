use crate::renderer::pipeline_settings::FifLabel;
use crate::resource::ImageLoader;
use ash::vk;
use itertools::Itertools;
use shader_binding::shader;
use shader_layout_macro::ShaderLayout;
use std::collections::HashMap;
use std::rc::Rc;
use truvis_rhi::core::descriptor::{RhiDescriptorSet, RhiDescriptorSetLayout};
use truvis_rhi::core::device::RhiDevice;
use truvis_rhi::core::image::RhiImage2DView;
use truvis_rhi::core::texture::RhiTexture2D;
use truvis_rhi::rhi::Rhi;
use truvis_rhi::shader_cursor::ShaderCursor;

#[derive(ShaderLayout)]
pub struct BindlessTextureBindings {
    #[binding = 0]
    #[descriptor_type = "COMBINED_IMAGE_SAMPLER"]
    #[stage = "FRAGMENT | RAYGEN_KHR | CLOSEST_HIT_KHR | ANY_HIT_KHR | CALLABLE_KHR | MISS_KHR | COMPUTE"]
    #[count = 128]
    #[flags = "PARTIALLY_BOUND | UPDATE_AFTER_BIND"]
    _textures: (),

    #[binding = 1]
    #[descriptor_type = "STORAGE_IMAGE"]
    #[stage = "FRAGMENT | RAYGEN_KHR | CLOSEST_HIT_KHR | ANY_HIT_KHR | CALLABLE_KHR | MISS_KHR | COMPUTE"]
    #[count = 128]
    #[flags = "PARTIALLY_BOUND | UPDATE_AFTER_BIND"]
    _images: (),
}

pub struct BindlessManager {
    pub bindless_layout: RhiDescriptorSetLayout<BindlessTextureBindings>,

    /// 每一个 frame in flights 都有一个 descriptor set
    pub bindless_sets: Vec<RhiDescriptorSet<BindlessTextureBindings>>,

    /// 每一帧都需要重新构建的映射
    ///
    /// key: texture path
    ///
    /// value: bindless idx
    texture_map: HashMap<String, u32>,
    textures: HashMap<String, RhiTexture2D>,

    /// 每一帧都需要重新构建的数据
    image_map: HashMap<String, u32>,
    images: HashMap<String, Rc<RhiImage2DView>>,

    device: Rc<RhiDevice>,

    /// 当前 frame in flight 的标签，每帧更新
    frame_label: FifLabel,
}
impl BindlessManager {
    pub fn new(rhi: &Rhi, frames_in_flight: usize) -> Self {
        let bindless_layout = RhiDescriptorSetLayout::<BindlessTextureBindings>::new(
            rhi,
            vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
            "bindless-layout",
        );
        let bindless_descriptor_sets = (0..frames_in_flight)
            .map(|idx| {
                RhiDescriptorSet::<BindlessTextureBindings>::new(
                    rhi,
                    rhi.descriptor_pool(),
                    &bindless_layout,
                    format!("bindless-descriptor-set-{idx}"),
                )
            })
            .collect_vec();

        Self {
            bindless_layout,
            bindless_sets: bindless_descriptor_sets,

            texture_map: HashMap::new(),
            textures: HashMap::new(),

            image_map: HashMap::new(),
            images: HashMap::new(),

            device: rhi.device.clone(),

            frame_label: FifLabel::A,
        }
    }

    /// getter
    #[inline]
    pub fn current_descriptor_set(&self) -> &RhiDescriptorSet<BindlessTextureBindings> {
        &self.bindless_sets[*self.frame_label]
    }

    /// 在每一帧绘制之前，将纹理数据绑定到 descriptor set 中
    pub fn prepare_render_data(&mut self, frame_label: FifLabel) {
        self.frame_label = frame_label;

        let mut texture_infos = Vec::with_capacity(self.textures.iter().len());
        self.texture_map.clear();
        for (tex_idx, (tex_name, tex)) in self.textures.iter().enumerate() {
            texture_infos.push(tex.descriptor_image_info(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL));
            self.texture_map.insert(tex_name.clone(), tex_idx as u32);
        }

        let mut image_infos = Vec::with_capacity(self.images.iter().len());
        self.image_map.clear();
        for (image_idx, (image_name, image)) in self.images.iter().enumerate() {
            image_infos.push(
                vk::DescriptorImageInfo::default().image_view(image.handle()).image_layout(vk::ImageLayout::GENERAL),
            );
            self.image_map.insert(image_name.clone(), image_idx as u32);
        }

        let writes = [
            BindlessTextureBindings::textures().write_image(
                self.bindless_sets[*frame_label].handle(),
                0,
                texture_infos,
            ),
            BindlessTextureBindings::images().write_image(self.bindless_sets[*frame_label].handle(), 0, image_infos),
        ];
        self.device.write_descriptor_sets(&writes);
    }

    pub fn register_texture_by_path(&mut self, rhi: &Rhi, texture_path: String) {
        if self.texture_map.contains_key(&texture_path) {
            log::error!("Texture {} is already registered", texture_path);
            return;
        }

        let texture = ImageLoader::load_image(rhi, std::path::Path::new(&texture_path));

        self.textures.insert(texture_path, texture);
    }

    pub fn register_texture(&mut self, key: String, texture: RhiTexture2D) {
        if self.texture_map.contains_key(&key) {
            log::error!("Texture {} is already registered", key);
            return;
        }
        self.textures.insert(key, texture);
    }

    pub fn register_image(&mut self, key: String, image: Rc<RhiImage2DView>) {
        self.images.insert(key, image);
    }

    pub fn unregister_image(&mut self, key: &String) {
        self.images.remove(key);
    }

    /// 获得纹理在当前帧的 bindless 索引
    pub fn get_texture_idx(&self, texture_path: &str) -> Option<shader::TextureHandle> {
        self.texture_map.get(texture_path).copied().map(|idx| shader::TextureHandle { index: idx as _ })
    }

    /// 获得图像在当前帧的 bindless 索引
    pub fn get_image_idx(&self, image_path: &str) -> Option<shader::ImageHandle> {
        self.image_map.get(image_path).copied().map(|idx| shader::ImageHandle { index: idx as _ })
    }
}
