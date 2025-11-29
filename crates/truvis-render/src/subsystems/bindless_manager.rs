use std::{collections::HashMap, rc::Rc};

use ash::vk;
use itertools::Itertools;

use slotmap::SecondaryMap;
use truvis_asset::handle::TextureHandle;
use truvis_gfx::descriptors::descriptor_pool::GfxDescriptorPoolCreateInfo;
use truvis_gfx::resources::handles::ImageViewHandle;
use truvis_gfx::{
    descriptors::{
        descriptor::{GfxDescriptorSet, GfxDescriptorSetLayout},
        descriptor_pool::GfxDescriptorPool,
    },
    gfx::Gfx,
    resources::texture::{GfxTexture2D, Texture2DContainer},
    utilities::shader_cursor::GfxShaderCursor,
};
use truvis_shader_binding::shader;
use truvis_shader_layout_macro::ShaderLayout;

use crate::core::frame_context::FrameContext;
use crate::subsystems::subsystem::Subsystem;
use crate::{pipeline_settings::FrameLabel, resources::ImageLoader};

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

pub enum BindlessTextureSource {
    Container(Texture2DContainer),
    Handle(ImageViewHandle, vk::Sampler),
}

/// Bindless 描述符管理器
///
/// 管理 Bindless 纹理和存储图像，通过数组索引访问资源。
/// 每帧独立的描述符集，支持 UPDATE_AFTER_BIND 和 PARTIALLY_BOUND。
///
/// # Bindless 架构
/// - Binding 0: 纹理数组（COMBINED_IMAGE_SAMPLER，最多 128 个）
/// - Binding 1: 存储图像数组（STORAGE_IMAGE，最多 128 个）
/// - 着色器通过索引访问：`textures[index]`
///
/// # 使用示例
/// ```ignore
/// let key = FrameContext::bindless_manager_mut().register_texture("albedo", texture);
/// // 在着色器中: textures[key]
/// ```
pub struct BindlessManager {
    _descriptor_pool: GfxDescriptorPool,

    pub bindless_descriptor_layout: GfxDescriptorSetLayout<BindlessDescriptorBinding>,

    /// 每一个 frame in flights 都有一个 descriptor set
    pub bindless_descriptor_sets: Vec<GfxDescriptorSet<BindlessDescriptorBinding>>,

    // TODO 这里不要使用 String 作为 key，这里不应该关心 Name
    /// 每一帧都需要重新构建的映射
    ///
    /// key: texture path
    ///
    /// value: bindless idx
    bindless_textures: HashMap<String, u32>,
    textures: HashMap<String, BindlessTextureSource>,

    /// AssetHub 纹理索引映射
    asset_texture_indices: SecondaryMap<TextureHandle, u32>,

    /// 每一帧都需要重新构建的数据
    images: HashMap<ImageViewHandle, shader::ImageHandle>,

    /// 当前 frame in flight 的标签，每帧更新
    frame_label: FrameLabel,
}

// init & destroy
impl BindlessManager {
    pub fn new(fif_count: usize) -> Self {
        let descriptor_pool = Self::init_descriptor_pool();
        let bindless_layout = GfxDescriptorSetLayout::<BindlessDescriptorBinding>::new(
            vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
            "bindless-layout",
        );
        let bindless_descriptor_sets = (0..fif_count)
            .map(|idx| {
                GfxDescriptorSet::<BindlessDescriptorBinding>::new(
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
            asset_texture_indices: SecondaryMap::new(),

            images: HashMap::new(),

            frame_label: FrameLabel::A,
        }
    }

    const DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT: u32 = 256;
    const DESCRIPTOR_POOL_MAX_MATERIAL_CNT: u32 = 256;
    const DESCRIPTOR_POOL_MAX_BINDLESS_TEXTURE_CNT: u32 = 128;

    fn init_descriptor_pool() -> GfxDescriptorPool {
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

        let pool_ci = Rc::new(GfxDescriptorPoolCreateInfo::new(
            vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET | vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
            Self::DESCRIPTOR_POOL_MAX_MATERIAL_CNT + Self::DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT + 32,
            pool_size,
        ));

        GfxDescriptorPool::new(pool_ci, "renderer")
    }
}

impl Subsystem for BindlessManager {
    fn before_render(&mut self) {}
}

// getters
impl BindlessManager {
    #[inline]
    pub fn current_descriptor_set(&self) -> &GfxDescriptorSet<BindlessDescriptorBinding> {
        &self.bindless_descriptor_sets[*self.frame_label]
    }
}

// tools
impl BindlessManager {
    /// # Phase: Before Render
    ///
    /// 在每一帧绘制之前，将纹理数据绑定到 descriptor set 中
    pub fn prepare_render_data(&mut self, frame_label: FrameLabel) {
        let _span = tracy_client::span!("BindlessManager::prepare_render_data");
        self.frame_label = frame_label;

        let mut texture_infos = Vec::with_capacity(self.textures.len());
        self.bindless_textures.clear();
        for (tex_idx, (tex_name, tex_source)) in self.textures.iter().enumerate() {
            let info = match tex_source {
                BindlessTextureSource::Container(tex) => {
                    tex.texture().descriptor_image_info(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                }
                BindlessTextureSource::Handle(view_handle, sampler) => {
                    let view = Gfx::get().resource_manager().get_image_view(*view_handle).unwrap().handle;
                    vk::DescriptorImageInfo::default()
                        .sampler(*sampler)
                        .image_view(view)
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                }
            };
            texture_infos.push(info);
            self.bindless_textures.insert(tex_name.clone(), tex_idx as u32);
        }

        // Sync with AssetHub
        let asset_hub = FrameContext::asset_hub();
        self.asset_texture_indices.clear();
        for handle in asset_hub.iter_handles() {
            let resource = asset_hub.get_texture(handle);

            let rm = Gfx::get().resource_manager();
            let view_vk = rm.get_image_view(resource.view).unwrap().handle;

            let info = vk::DescriptorImageInfo::default()
                .image_view(view_vk)
                .sampler(resource.sampler)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

            let idx = texture_infos.len() as u32;
            texture_infos.push(info);
            self.asset_texture_indices.insert(handle, idx);
        }

        // 生成 descriptor 信息，更新 ImageHandle
        let mut image_infos = Vec::with_capacity(self.images.len());
        for (image_idx, (image_view_handle, handle)) in self.images.iter_mut().enumerate() {
            let view = Gfx::get().resource_manager().get_image_view(*image_view_handle).unwrap().handle;
            image_infos.push(
                vk::DescriptorImageInfo::default() //
                    .image_view(view)
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

    /// 获得 AssetHub 纹理在当前帧的 bindless 索引
    pub fn get_asset_texture_handle(&self, handle: TextureHandle) -> Option<shader::TextureHandle> {
        self.asset_texture_indices.get(handle).copied().map(|idx| shader::TextureHandle { index: idx as _ })
    }

    /// 获得图像在当前帧的 bindless 索引
    pub fn get_image_handle(&self, image_view: ImageViewHandle) -> Option<shader::ImageHandle> {
        self.images.get(&image_view).copied()
    }
}

// register & unregister
impl BindlessManager {
    pub fn register_texture_by_path(&mut self, texture_path: String) {
        let _span = tracy_client::span!("register_texture");
        let texture = ImageLoader::load_image(std::path::Path::new(&texture_path));
        self.register_texture(texture_path, Texture2DContainer::Owned(Box::new(texture)));
    }
    pub fn register_texture_owned(&mut self, key: String, texture: GfxTexture2D) {
        self.register_texture(key, Texture2DContainer::Owned(Box::new(texture)));
    }
    pub fn register_texture_shared(&mut self, key: String, texture: Rc<GfxTexture2D>) {
        self.register_texture(key, Texture2DContainer::Shared(texture));
    }

    pub fn register_texture_handle(&mut self, key: String, view: ImageViewHandle, sampler: vk::Sampler) {
        if self.textures.contains_key(&key) {
            log::error!("Texture {} is already registered", key);
            return;
        }
        self.textures.insert(key, BindlessTextureSource::Handle(view, sampler));
    }

    #[inline]
    fn register_texture(&mut self, key: String, texture: Texture2DContainer) {
        if self.textures.contains_key(&key) {
            log::error!("Texture {} is already registered", key);
            return;
        }
        self.textures.insert(key, BindlessTextureSource::Container(texture));
    }

    pub fn unregister_texture(&mut self, key: &str) {
        self.textures.remove(key);
    }

    pub fn register_image(&mut self, image: ImageViewHandle) {
        if self.images.contains_key(&image) {
            // log::error!("Image {:?} has already been registered", image);
            return;
        }
        self.images.insert(image, shader::ImageHandle { index: -1 });
    }

    pub fn unregister_image2(&mut self, image: ImageViewHandle) {
        self.images.remove(&image).unwrap();
    }
}

impl Drop for BindlessManager {
    fn drop(&mut self) {
        log::info!("Dropping BindlessManager");
    }
}
