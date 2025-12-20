use crate::pipeline_settings::FrameLabel;
use crate::render_descriptor_sets::{BindlessDescriptorBinding, RenderDescriptorSets};
use ash::vk;
use slotmap::{Key, SecondaryMap};
use truvis_gfx::sampler::{GfxSampler, GfxSamplerDesc};
use truvis_gfx::{gfx::Gfx, utilities::descriptor_cursor::GfxDescriptorCursor};
use truvis_resource::gfx_resource_manager::GfxResourceManager;
use truvis_resource::handles::{GfxImageViewHandle, GfxTextureHandle};
use truvis_shader_binding::truvisl;

#[derive(Copy, Clone)]
pub struct BindlessTextureHandle(pub truvisl::TextureHandle);
impl BindlessTextureHandle {
    #[inline]
    pub fn new(index: usize) -> Self {
        Self(truvisl::TextureHandle { index: index as i32 })
    }
    #[inline]
    pub fn null() -> Self {
        Self(truvisl::TextureHandle {
            index: truvisl::INVALID_TEX_ID,
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
pub struct BindlessUavHandle(pub truvisl::UavHandle);
impl BindlessUavHandle {
    #[inline]
    pub fn new(index: usize) -> Self {
        Self(truvisl::UavHandle { index: index as i32 })
    }
    #[inline]
    pub fn null() -> Self {
        Self(truvisl::UavHandle {
            index: truvisl::INVALID_TEX_ID,
        })
    }
    #[inline]
    pub fn index(&self) -> usize {
        self.0.index as usize
    }
}
impl Default for BindlessUavHandle {
    fn default() -> Self {
        Self::null()
    }
}

#[derive(Copy, Clone)]
pub struct BindlessSrvHandle(pub truvisl::SrvHandle);
impl BindlessSrvHandle {
    #[inline]
    pub fn new(index: usize) -> Self {
        Self(truvisl::SrvHandle { index: index as i32 })
    }
    #[inline]
    pub fn null() -> Self {
        Self(truvisl::SrvHandle {
            index: truvisl::INVALID_TEX_ID,
        })
    }
    #[inline]
    pub fn index(&self) -> usize {
        self.0.index as usize
    }
}
impl Default for BindlessSrvHandle {
    fn default() -> Self {
        Self::null()
    }
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
    // combined image sampler
    combined_sampler_srvs: SecondaryMap<GfxTextureHandle, BindlessTextureHandle>,

    // storage image
    uavs: SecondaryMap<GfxImageViewHandle, BindlessUavHandle>,
    texture_uavs: SecondaryMap<GfxTextureHandle, BindlessUavHandle>,

    // sampled image
    srvs: SecondaryMap<GfxImageViewHandle, BindlessSrvHandle>,
    texture_srvs: SecondaryMap<GfxTextureHandle, BindlessSrvHandle>,
}
// new & init
impl BindlessManager {
    pub fn new() -> Self {
        Self {
            combined_sampler_srvs: SecondaryMap::new(),
            uavs: SecondaryMap::new(),
            texture_uavs: SecondaryMap::new(),
            srvs: SecondaryMap::new(),
            texture_srvs: SecondaryMap::new(),
        }
    }
}
impl Default for BindlessManager {
    fn default() -> Self {
        Self::new()
    }
}
// destroy
impl BindlessManager {
    pub fn destroy(self) {}
}
impl Drop for BindlessManager {
    fn drop(&mut self) {
        log::info!("Dropping BindlessManager");
    }
}
// update
impl BindlessManager {
    /// # Phase: Before Render
    ///
    /// 在每一帧绘制之前，将纹理数据绑定到 descriptor set 中
    pub fn prepare_render_data(
        &mut self,
        gfx_resource_manager: &GfxResourceManager,
        render_descriptor_sets: &RenderDescriptorSets,
        frame_label: FrameLabel,
    ) {
        let _span = tracy_client::span!("BindlessManager::prepare_render_data");

        // combined image sampler 信息
        let mut combined_sampler_srvs_inofs = Vec::with_capacity(self.combined_sampler_srvs.len());
        for (tex_handle, shader_tex_handle) in self.combined_sampler_srvs.iter_mut() {
            let texture = gfx_resource_manager.get_texture(tex_handle).unwrap();
            combined_sampler_srvs_inofs.push(
                vk::DescriptorImageInfo::default()
                    .sampler(texture.sampler())
                    .image_view(texture.image_view().handle())
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
            );
            shader_tex_handle.0.index = combined_sampler_srvs_inofs.len() as i32 - 1;
        }

        // UAV 信息
        let mut uav_infos = Vec::with_capacity(self.uavs.len() + self.texture_uavs.len());
        for (image_view_handle, shader_uav_handle) in self.uavs.iter_mut() {
            let image_view = gfx_resource_manager.get_image_view(image_view_handle).unwrap();
            uav_infos.push(
                vk::DescriptorImageInfo::default()
                    .image_view(image_view.handle())
                    .image_layout(vk::ImageLayout::GENERAL),
            );
            shader_uav_handle.0.index = uav_infos.len() as i32 - 1;
        }
        for (texture_handle, shader_uav_handle) in self.texture_uavs.iter_mut() {
            let texture = gfx_resource_manager.get_texture(texture_handle).unwrap();
            uav_infos.push(
                vk::DescriptorImageInfo::default()
                    .image_view(texture.image_view().handle())
                    .image_layout(vk::ImageLayout::GENERAL),
            );
            shader_uav_handle.0.index = uav_infos.len() as i32 - 1;
        }

        // SRV 信息
        let mut srv_infos = Vec::with_capacity(self.srvs.len() + self.texture_srvs.len());
        for (image_view_handle, shader_src_handle) in self.srvs.iter_mut() {
            let image_view = gfx_resource_manager.get_image_view(image_view_handle).unwrap();
            srv_infos.push(
                vk::DescriptorImageInfo::default()
                    .image_view(image_view.handle())
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
            );
            shader_src_handle.0.index = srv_infos.len() as i32 - 1;
        }
        for (texture_handle, shader_src_handle) in self.texture_srvs.iter_mut() {
            let texture = gfx_resource_manager.get_texture(texture_handle).unwrap();
            srv_infos.push(
                vk::DescriptorImageInfo::default()
                    .image_view(texture.image_view().handle())
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
            );
            shader_src_handle.0.index = srv_infos.len() as i32 - 1;
        }

        // 将 images 和 textures 信息写入 descriptor set
        let mut writes = Vec::new();
        if !combined_sampler_srvs_inofs.is_empty() {
            writes.push(BindlessDescriptorBinding::textures().write_image(
                render_descriptor_sets.set_1_bindless[*frame_label].handle(),
                0,
                combined_sampler_srvs_inofs,
            ))
        }
        if !uav_infos.is_empty() {
            writes.push(BindlessDescriptorBinding::uavs().write_image(
                render_descriptor_sets.set_1_bindless[*frame_label].handle(),
                0,
                uav_infos,
            ))
        }
        if !srv_infos.is_empty() {
            writes.push(BindlessDescriptorBinding::srvs().write_image(
                render_descriptor_sets.set_1_bindless[*frame_label].handle(),
                0,
                srv_infos,
            ))
        }
        Gfx::get().gfx_device().write_descriptor_sets(&writes);
    }
}
// combined sampler SRV
impl BindlessManager {
    #[inline]
    pub fn register_texture(&mut self, handle: GfxTextureHandle) {
        debug_assert!(!handle.is_null());

        if self.combined_sampler_srvs.contains_key(handle) {
            log::error!("Texture handle {:?} is already registered", handle);
            return;
        }
        self.combined_sampler_srvs.insert(handle, BindlessTextureHandle::null());
        panic!("Not supported yet");
    }

    #[inline]
    pub fn unregister_texture(&mut self, handle: GfxTextureHandle) {
        debug_assert!(!handle.is_null());

        self.combined_sampler_srvs.remove(handle);
    }

    #[inline]
    pub fn get_shader_texture_handle(&self, texture_handle: GfxTextureHandle) -> BindlessTextureHandle {
        debug_assert!(!texture_handle.is_null());

        self.combined_sampler_srvs.get(texture_handle).copied().unwrap()
    }
}
// UAV
impl BindlessManager {
    #[inline]
    pub fn register_uav(&mut self, image_view_handle: GfxImageViewHandle) {
        debug_assert!(!image_view_handle.is_null());

        if self.uavs.contains_key(image_view_handle) {
            log::error!("Image view handle {:?} is already registered", image_view_handle);
            return;
        }
        self.uavs.insert(image_view_handle, BindlessUavHandle::null());
    }

    #[inline]
    pub fn register_uav_with_texture(&mut self, texture_handle: GfxTextureHandle) {
        debug_assert!(!texture_handle.is_null());

        if self.texture_uavs.contains_key(texture_handle) {
            log::error!("Texture handle {:?} is already registered for image", texture_handle);
            return;
        }
        self.texture_uavs.insert(texture_handle, BindlessUavHandle::null());
    }

    #[inline]
    pub fn unregister_uav(&mut self, image_view_handle: GfxImageViewHandle) {
        debug_assert!(!image_view_handle.is_null());

        self.uavs.remove(image_view_handle).unwrap();
    }

    #[inline]
    pub fn unregister_uav_with_texture(&mut self, texture_handle: GfxTextureHandle) {
        debug_assert!(!texture_handle.is_null());

        self.texture_uavs.remove(texture_handle).unwrap();
    }

    #[inline]
    pub fn get_shader_uav_handle(&self, image_view_handle: GfxImageViewHandle) -> BindlessUavHandle {
        debug_assert!(!image_view_handle.is_null());

        self.uavs.get(image_view_handle).copied().unwrap()
    }

    #[inline]
    pub fn get_shader_uav_handle_with_texture(&self, texture_handle: GfxTextureHandle) -> BindlessUavHandle {
        debug_assert!(!texture_handle.is_null());

        self.texture_uavs.get(texture_handle).copied().unwrap()
    }
}
// SRV
impl BindlessManager {
    #[inline]
    pub fn register_srv(&mut self, image_view_handle: GfxImageViewHandle) {
        debug_assert!(!image_view_handle.is_null());

        if self.srvs.contains_key(image_view_handle) {
            log::error!("Image view handle {:?} is already registered", image_view_handle);
            return;
        }
        self.srvs.insert(image_view_handle, BindlessSrvHandle::null());
    }

    #[inline]
    pub fn register_srv_with_texture(&mut self, texture_handle: GfxTextureHandle) {
        debug_assert!(!texture_handle.is_null());

        if self.texture_srvs.contains_key(texture_handle) {
            log::error!("Texture handle {:?} is already registered for image", texture_handle);
            return;
        }
        self.texture_srvs.insert(texture_handle, BindlessSrvHandle::null());
    }

    #[inline]
    pub fn unregister_srv(&mut self, image_view_handle: GfxImageViewHandle) {
        debug_assert!(!image_view_handle.is_null());

        self.srvs.remove(image_view_handle).unwrap();
    }

    #[inline]
    pub fn unregister_srv_with_texture(&mut self, texture_handle: GfxTextureHandle) {
        debug_assert!(!texture_handle.is_null());

        self.texture_srvs.remove(texture_handle).unwrap();
    }

    #[inline]
    pub fn get_shader_srv_handle(&self, image_view_handle: GfxImageViewHandle) -> BindlessSrvHandle {
        debug_assert!(!image_view_handle.is_null());

        self.srvs.get(image_view_handle).copied().unwrap()
    }

    #[inline]
    pub fn get_shader_srv_handle_with_texture(&self, texture_handle: GfxTextureHandle) -> BindlessSrvHandle {
        debug_assert!(!texture_handle.is_null());

        self.texture_srvs.get(texture_handle).copied().unwrap()
    }
}
