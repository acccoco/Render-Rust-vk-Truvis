use std::{collections::HashMap, rc::Rc};

use ash::vk;
use itertools::Itertools;

use truvis_gfx::descriptors::descriptor_pool::DescriptorPoolCreateInfo;
use truvis_gfx::{
    descriptors::{
        descriptor::{DescriptorSet, DescriptorSetLayout},
        descriptor_pool::DescriptorPool,
    },
    gfx::Gfx,
    resources::{
        image_view::Image2DView,
        texture::{Texture2D, Texture2DContainer},
    },
    utilities::shader_cursor::ShaderCursor,
};
use truvis_shader_binding::shader;
use truvis_shader_layout_macro::ShaderLayout;

use crate::{pipeline_settings::FrameLabel, render_resource::ImageLoader};

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
    _descriptor_pool: DescriptorPool,

    pub bindless_descriptor_layout: DescriptorSetLayout<BindlessDescriptorBinding>,

    /// 每一个 frame in flights 都有一个 descriptor set
    pub bindless_descriptor_sets: Vec<DescriptorSet<BindlessDescriptorBinding>>,

    // TODO 这里不要使用 String 作为 key，这里不应该关心 Name
    /// 每一帧都需要重新构建的映射
    ///
    /// key: texture path
    ///
    /// value: bindless idx
    bindless_textures: HashMap<String, u32>,
    textures: HashMap<String, Texture2DContainer>,

    /// 每一帧都需要重新构建的数据
    images: HashMap<vk::ImageView, shader::ImageHandle>,

    /// 当前 frame in flight 的标签，每帧更新
    frame_label: FrameLabel,
}

// init & destroy
impl BindlessManager {
    pub fn new(fif_count: usize) -> Self {
        let descriptor_pool = Self::init_descriptor_pool();
        let bindless_layout = DescriptorSetLayout::<BindlessDescriptorBinding>::new(
            vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
            "bindless-layout",
        );
        let bindless_descriptor_sets = (0..fif_count)
            .map(|idx| {
                DescriptorSet::<BindlessDescriptorBinding>::new(
                    &descriptor_pool,
                    &bindless_layout,
                    format!("bindless-descriptor-set-{idx}"),
                )
            })
            .collect_vec();

        Self {
            _descriptor_pool: descriptor_pool,

            bindless_descriptor_layout: bindless_layout,
            bindless_descriptor_sets,

            bindless_textures: HashMap::new(),
            textures: HashMap::new(),

            images: HashMap::new(),

            frame_label: FrameLabel::A,
        }
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

// getters
impl BindlessManager {
    #[inline]
    pub fn current_descriptor_set(&self) -> &DescriptorSet<BindlessDescriptorBinding> {
        &self.bindless_descriptor_sets[*self.frame_label]
    }
}

// tools
impl BindlessManager {
    /// # Phase: Before Render
    ///
    /// 在每一帧绘制之前，将纹理数据绑定到 descriptor set 中
    pub fn prepare_render_data(&mut self, frame_label: FrameLabel) {
        self.frame_label = frame_label;

        let mut texture_infos = Vec::with_capacity(self.textures.len());
        self.bindless_textures.clear();
        for (tex_idx, (tex_name, tex)) in self.textures.iter().enumerate() {
            texture_infos.push(tex.texture().descriptor_image_info(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL));
            self.bindless_textures.insert(tex_name.clone(), tex_idx as u32);
        }

        // 生成 descriptor 信息，更新 ImageHandle
        let mut image_infos = Vec::with_capacity(self.images.len());
        for (image_idx, (image_view, handle)) in self.images.iter_mut().enumerate() {
            image_infos.push(
                vk::DescriptorImageInfo::default() //
                    .image_view(*image_view)
                    .image_layout(vk::ImageLayout::GENERAL),
            );
            handle.index = image_idx as i32;
        }

        // 将 images 和 textures 信息写入 descriptor set
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
        Gfx::get().gfx_device().write_descriptor_sets(&writes);
    }

    /// 获得纹理在当前帧的 bindless 索引
    pub fn get_texture_handle(&self, texture_path: &str) -> Option<shader::TextureHandle> {
        self.bindless_textures.get(texture_path).copied().map(|idx| shader::TextureHandle { index: idx as _ })
    }

    /// 获得图像在当前帧的 bindless 索引
    pub fn get_image_handle(&self, image_view: &Image2DView) -> Option<shader::ImageHandle> {
        self.images.get(&image_view.handle()).copied()
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

    pub fn register_image(&mut self, image: &Image2DView) {
        let key = image.handle();
        if self.images.contains_key(&key) {
            log::error!("Image {} has already been registered", image);
            return;
        }
        self.images.insert(key, shader::ImageHandle { index: -1 });
    }

    pub fn unregister_image2(&mut self, image: &Image2DView) {
        self.images.remove(&image.handle()).unwrap();
    }
}

impl Drop for BindlessManager {
    fn drop(&mut self) {
        log::info!("Dropping BindlessManager");
    }
}
