use std::rc::Rc;

use ash::vk;
use itertools::Itertools;

use slotmap::{Key, SecondaryMap};
use truvis_gfx::descriptors::descriptor_pool::GfxDescriptorPoolCreateInfo;
use truvis_gfx::{
    descriptors::{
        descriptor::{GfxDescriptorSet, GfxDescriptorSetLayout},
        descriptor_pool::GfxDescriptorPool,
    },
    gfx::Gfx,
    utilities::shader_cursor::GfxShaderCursor,
};
use truvis_shader_binding::shader;
use truvis_shader_layout_macro::ShaderLayout;

use crate::pipeline_settings::FrameLabel;
use truvis_resource::gfx_resource_manager::GfxResourceManager;
use truvis_resource::handles::{GfxImageViewHandle, GfxTextureHandle};

#[derive(Copy, Clone)]
pub struct BindlessTextureHandle(pub shader::TextureHandle);
impl BindlessTextureHandle {
    #[inline]
    pub fn new(index: usize) -> Self {
        Self(shader::TextureHandle { index: index as i32 })
    }
    #[inline]
    pub fn null() -> Self {
        Self(shader::TextureHandle {
            index: shader::INVALID_TEX_ID,
        })
    }
    #[inline]
    pub fn index(&self) -> usize {
        self.0.index as usize
    }
}
impl Default for BindlessTextureHandle {
    fn default() -> Self {
        Self::null()
    }
}
#[derive(Copy, Clone)]
pub struct BindlessImageHandle(pub shader::ImageHandle);
impl BindlessImageHandle {
    #[inline]
    pub fn new(index: usize) -> Self {
        Self(shader::ImageHandle { index: index as i32 })
    }
    #[inline]
    pub fn null() -> Self {
        Self(shader::ImageHandle {
            index: shader::INVALID_TEX_ID,
        })
    }
    #[inline]
    pub fn index(&self) -> usize {
        self.0.index as usize
    }
}
impl Default for BindlessImageHandle {
    fn default() -> Self {
        Self::null()
    }
}

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

    /// 每一帧都需要重新构建的映射
    textures: SecondaryMap<GfxTextureHandle, BindlessTextureHandle>,

    /// 每一帧都需要重新构建的数据
    images: SecondaryMap<GfxImageViewHandle, BindlessImageHandle>,
    /// 纹理里面的图形
    texture_images: SecondaryMap<GfxTextureHandle, BindlessImageHandle>,

    /// 当前 frame in flight 的标签，每帧更新
    frame_label: FrameLabel,
}
// new & init
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

            textures: SecondaryMap::new(),
            images: SecondaryMap::new(),
            texture_images: SecondaryMap::new(),

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
// destroy
impl BindlessManager {}
impl Drop for BindlessManager {
    fn drop(&mut self) {
        log::info!("Dropping BindlessManager");
    }
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
    pub fn prepare_render_data(&mut self, gfx_resource_manager: &GfxResourceManager, frame_label: FrameLabel) {
        let _span = tracy_client::span!("BindlessManager::prepare_render_data");
        self.frame_label = frame_label;

        // 生成 texture 的 descriptor 信息
        let mut texture_infos = Vec::with_capacity(self.textures.len());
        for (tex_idx, (tex_handle, shader_tex_handle)) in self.textures.iter_mut().enumerate() {
            let texture = gfx_resource_manager.get_texture(tex_handle).unwrap();
            texture_infos.push(
                vk::DescriptorImageInfo::default()
                    .sampler(texture.sampler())
                    .image_view(texture.image_view().handle())
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
            );
            shader_tex_handle.0.index = tex_idx as i32;
        }

        // 生成 image views 的 descriptor 信息
        let mut image_infos = Vec::with_capacity(self.images.len() + self.texture_images.len());
        for (image_idx, (image_view_handle, shader_image_handle)) in self.images.iter_mut().enumerate() {
            let image_view = gfx_resource_manager.get_image_view(image_view_handle).unwrap();
            image_infos.push(
                vk::DescriptorImageInfo::default()
                    .image_view(image_view.handle())
                    .image_layout(vk::ImageLayout::GENERAL),
            );
            shader_image_handle.0.index = image_idx as i32;
        }
        // 生成 image views 的 descriptor 信息 (from textures)
        for (image_idx, (texture_handle, shader_image_handle)) in self.texture_images.iter_mut().enumerate() {
            let texture = gfx_resource_manager.get_texture(texture_handle).unwrap();
            image_infos.push(
                vk::DescriptorImageInfo::default()
                    .image_view(texture.image_view().handle())
                    .image_layout(vk::ImageLayout::GENERAL),
            );
            shader_image_handle.0.index = (image_idx + self.images.len()) as i32;
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
}
// register & unregister
impl BindlessManager {
    // TODO rename: register_texture2 -> register_texture_handle
    #[inline]
    pub fn register_texture2(&mut self, handle: GfxTextureHandle) {
        debug_assert!(!handle.is_null());

        if self.textures.contains_key(handle) {
            log::error!("Texture handle {:?} is already registered", handle);
            return;
        }
        self.textures.insert(handle, BindlessTextureHandle::null());
    }

    // TODO rename: unregister_texture2 -> unregister_texture_handle
    pub fn unregister_texture2(&mut self, handle: GfxTextureHandle) {
        debug_assert!(!handle.is_null());

        self.textures.remove(handle);
    }

    pub fn register_image2(&mut self, image_view_handle: GfxImageViewHandle) {
        debug_assert!(!image_view_handle.is_null());

        if self.images.contains_key(image_view_handle) {
            log::error!("Image view handle {:?} is already registered", image_view_handle);
            return;
        }
        self.images.insert(image_view_handle, BindlessImageHandle::null());
    }

    pub fn unregister_image(&mut self, image_view_handle: GfxImageViewHandle) {
        debug_assert!(!image_view_handle.is_null());

        self.images.remove(image_view_handle).unwrap();
    }

    pub fn register_image_in_texture(&mut self, texture_handle: GfxTextureHandle) {
        debug_assert!(!texture_handle.is_null());

        if self.texture_images.contains_key(texture_handle) {
            log::error!("Texture handle {:?} is already registered for image", texture_handle);
            return;
        }
        self.texture_images.insert(texture_handle, BindlessImageHandle::null());
    }

    pub fn unregister_image_in_texture(&mut self, texture_handle: GfxTextureHandle) {
        debug_assert!(!texture_handle.is_null());

        self.texture_images.remove(texture_handle).unwrap();
    }

    /// 获得纹理在当前帧的 bindless 索引
    pub fn get_texture_handle2(&self, texture_handle: GfxTextureHandle) -> Option<BindlessTextureHandle> {
        debug_assert!(!texture_handle.is_null());

        self.textures.get(texture_handle).copied()
    }

    /// 获得图像在当前帧的 bindless 索引
    pub fn get_image_handle2(&self, image_view_handle: GfxImageViewHandle) -> Option<BindlessImageHandle> {
        debug_assert!(!image_view_handle.is_null());

        self.images.get(image_view_handle).copied()
    }

    pub fn get_image_handle_in_texture(&self, texture_handle: GfxTextureHandle) -> Option<BindlessImageHandle> {
        debug_assert!(!texture_handle.is_null());

        self.texture_images.get(texture_handle).copied()
    }
}
