use std::{collections::HashMap, rc::Rc};

use crate::{pipeline_settings::FrameLabel, render_resource::ImageLoader, renderer::frame_controller::FrameController};
use ash::vk;
use itertools::Itertools;
use truvis_rhi::descriptors::descriptor_pool::DescriptorPoolCreateInfo;
use truvis_rhi::{
    descriptors::{
        descriptor::{DescriptorSet, DescriptorSetLayout},
        descriptor_pool::DescriptorPool,
    },
    render_context::RenderContext,
    resources::{
        image_view::{Image2DView, Image2DViewContainer, Image2DViewUUID},
        texture::{Texture2D, Texture2DContainer},
    },
    utilities::shader_cursor::ShaderCursor,
};
use truvis_shader_binding::shader;
use truvis_shader_layout_macro::ShaderLayout;

#[derive(ShaderLayout)]
pub struct BindlessDescriptorBinding {
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

// TODO: 将 RefCell 移动到 BindlessManager 内部，粒度更细
pub struct BindlessManager {
    descriptor_pool: DescriptorPool,

    pub bindless_descriptor_layout: DescriptorSetLayout<BindlessDescriptorBinding>,

    /// 每一个 frame in flights 都有一个 descriptor set
    pub bindless_descriptor_sets: Vec<DescriptorSet<BindlessDescriptorBinding>>,

    /// 每一帧都需要重新构建的映射
    ///
    /// key: texture path
    ///
    /// value: bindless idx
    bindless_textures: HashMap<String, u32>,
    textures: HashMap<String, Texture2DContainer>,

    /// 每一帧都需要重新构建的数据
    bindless_images: HashMap<Image2DViewUUID, u32>,
    images: HashMap<Image2DViewUUID, Image2DViewContainer>,

    /// 当前 frame in flight 的标签，每帧更新
    frame_label: FrameLabel,
}
// init & destroy
impl BindlessManager {
    pub fn new(frame_ctrl: Rc<FrameController>) -> Self {
        let descriptor_pool = Self::init_descriptor_pool();
        let bindless_layout = DescriptorSetLayout::<BindlessDescriptorBinding>::new(
            vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
            "bindless-layout",
        );
        let bindless_descriptor_sets = (0..frame_ctrl.fif_count())
            .map(|idx| {
                DescriptorSet::<BindlessDescriptorBinding>::new(
                    &descriptor_pool,
                    &bindless_layout,
                    format!("bindless-descriptor-set-{idx}"),
                )
            })
            .collect_vec();

        Self {
            descriptor_pool,

            bindless_descriptor_layout: bindless_layout,
            bindless_descriptor_sets,

            bindless_textures: HashMap::new(),
            textures: HashMap::new(),

            bindless_images: HashMap::new(),
            images: HashMap::new(),

            frame_label: FrameLabel::A,
        }
    }

    /// getter
    #[inline]
    pub fn current_descriptor_set(&self) -> &DescriptorSet<BindlessDescriptorBinding> {
        &self.bindless_descriptor_sets[*self.frame_label]
    }

    /// # Phase: Before Render
    ///
    /// 在每一帧绘制之前，将纹理数据绑定到 descriptor set 中
    pub fn prepare_render_data(&mut self, frame_label: FrameLabel) {
        self.frame_label = frame_label;

        let mut texture_infos = Vec::with_capacity(self.textures.iter().len());
        self.bindless_textures.clear();
        for (tex_idx, (tex_name, tex)) in self.textures.iter().enumerate() {
            texture_infos.push(tex.texture().descriptor_image_info(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL));
            self.bindless_textures.insert(tex_name.clone(), tex_idx as u32);
        }

        let mut image_infos = Vec::with_capacity(self.images.iter().len());
        self.bindless_images.clear();
        for (image_idx, (image_key, image_view)) in self.images.iter().enumerate() {
            image_infos.push(
                vk::DescriptorImageInfo::default()
                    .image_view(image_view.vk_image_view())
                    .image_layout(vk::ImageLayout::GENERAL),
            );
            self.bindless_images.insert(*image_key, image_idx as u32);
        }

        let writes = [
            BindlessDescriptorBinding::textures().write_image(
                self.bindless_descriptor_sets[*frame_label].handle(),
                0,
                texture_infos,
            ),
            BindlessDescriptorBinding::images().write_image(
                self.bindless_descriptor_sets[*frame_label].handle(),
                0,
                image_infos,
            ),
        ];
        RenderContext::get().device_functions().write_descriptor_sets(&writes);
    }

    /// 获得纹理在当前帧的 bindless 索引
    pub fn get_texture_handle(&self, texture_path: &str) -> Option<shader::TextureHandle> {
        self.bindless_textures.get(texture_path).copied().map(|idx| shader::TextureHandle { index: idx as _ })
    }

    /// 获得图像在当前帧的 bindless 索引
    pub fn get_image_handle(&self, image_uuid: &Image2DViewUUID) -> Option<shader::ImageHandle> {
        self.bindless_images.get(image_uuid).copied().map(|idx| shader::ImageHandle { index: idx as _ })
    }

    const DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT: u32 = 256;
    const DESCRIPTOR_POOL_MAX_MATERIAL_CNT: u32 = 256;
    const DESCRIPTOR_POOL_MAX_BINDLESS_TEXTURE_CNT: u32 = 128;

    fn init_descriptor_pool() -> DescriptorPool {
        let pool_size = vec![
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
                descriptor_count: 128,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: Self::DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: Self::DESCRIPTOR_POOL_MAX_MATERIAL_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: Self::DESCRIPTOR_POOL_MAX_MATERIAL_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::INPUT_ATTACHMENT,
                descriptor_count: 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
                descriptor_count: 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: Self::DESCRIPTOR_POOL_MAX_BINDLESS_TEXTURE_CNT + 32,
            },
        ];

        let pool_ci = Rc::new(DescriptorPoolCreateInfo::new(
            vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET | vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
            Self::DESCRIPTOR_POOL_MAX_MATERIAL_CNT + Self::DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT + 32,
            pool_size,
        ));

        DescriptorPool::new(pool_ci, "renderer")
    }
}
// register & unregister
impl BindlessManager {
    pub fn register_texture_by_path(&mut self, texture_path: String) {
        let texture = ImageLoader::load_image(std::path::Path::new(&texture_path));
        self.register_texture(texture_path, Texture2DContainer::Owned(Box::new(texture)));
    }

    pub fn register_texture_owned(&mut self, key: String, texture: Texture2D) {
        self.register_texture(key, Texture2DContainer::Owned(Box::new(texture)));
    }

    pub fn register_texture_shared(&mut self, key: String, texture: Rc<Texture2D>) {
        self.register_texture(key, Texture2DContainer::Shared(texture));
    }

    #[inline]
    fn register_texture(&mut self, key: String, texture: Texture2DContainer) {
        if self.textures.contains_key(&key) {
            log::error!("Texture {} is already registered", key);
            return;
        }
        self.textures.insert(key, texture);
    }

    pub fn unregister_texture(&mut self, key: &str) {
        self.textures.remove(key);
    }

    pub fn register_image_shared(&mut self, image: Rc<Image2DView>) {
        self.register_image(image.uuid(), Image2DViewContainer::Shared(image));
    }

    pub fn register_image_raw(&mut self, image: &Image2DView) {
        self.register_image(image.uuid(), Image2DViewContainer::Raw(image.handle()));
    }

    #[inline]
    fn register_image(&mut self, key: Image2DViewUUID, image: Image2DViewContainer) {
        if self.images.contains_key(&key) {
            log::error!("Image with UUID {} is already registered", key);
            return;
        }
        self.images.insert(key, image);
    }

    pub fn unregister_image(&mut self, key: &Image2DViewUUID) {
        self.images.remove(key);
    }
}
impl Drop for BindlessManager {
    fn drop(&mut self) {
        log::info!("Dropping BindlessManager");
    }
}
