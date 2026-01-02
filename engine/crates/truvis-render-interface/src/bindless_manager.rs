use crate::gfx_resource_manager::GfxResourceManager;
use crate::global_descriptor_sets::{BindlessDescriptorBinding, GlobalDescriptorSets};
use crate::handles::GfxImageViewHandle;
use crate::pipeline_settings::FrameLabel;
use ash::vk;
use slotmap::{Key, SecondaryMap};
use truvis_gfx::{gfx::Gfx, utilities::descriptor_cursor::GfxDescriptorCursor};
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
    // storage image
    uavs: SecondaryMap<GfxImageViewHandle, BindlessUavHandle>,

    // sampled image
    srvs: SecondaryMap<GfxImageViewHandle, BindlessSrvHandle>,
}

// new & init
impl BindlessManager {
    pub fn new() -> Self {
        Self {
            uavs: SecondaryMap::new(),
            srvs: SecondaryMap::new(),
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
        render_descriptor_sets: &GlobalDescriptorSets,
        frame_label: FrameLabel,
    ) {
        let _span = tracy_client::span!("BindlessManager::prepare_render_data");

        // combined image sampler 信息
        let combined_sampler_srvs_inofs = Vec::new();

        // UAV 信息
        let mut uav_infos = Vec::with_capacity(self.uavs.len());
        for (image_view_handle, shader_uav_handle) in self.uavs.iter_mut() {
            let image_view = gfx_resource_manager.get_image_view(image_view_handle).unwrap();
            uav_infos.push(
                vk::DescriptorImageInfo::default()
                    .image_view(image_view.handle())
                    .image_layout(vk::ImageLayout::GENERAL),
            );
            shader_uav_handle.0.index = uav_infos.len() as i32 - 1;
        }

        // SRV 信息
        let mut srv_infos = Vec::with_capacity(self.srvs.len());
        for (image_view_handle, shader_src_handle) in self.srvs.iter_mut() {
            let image_view = gfx_resource_manager.get_image_view(image_view_handle).unwrap();
            srv_infos.push(
                vk::DescriptorImageInfo::default()
                    .image_view(image_view.handle())
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
            );
            shader_src_handle.0.index = srv_infos.len() as i32 - 1;
        }

        // 将 images 和 textures 信息写入 descriptor set
        let mut writes = Vec::new();
        if !combined_sampler_srvs_inofs.is_empty() {
            writes.push(BindlessDescriptorBinding::textures().write_image(
                render_descriptor_sets.current_bindless_set(frame_label).handle(),
                0,
                combined_sampler_srvs_inofs,
            ))
        }
        if !uav_infos.is_empty() {
            writes.push(BindlessDescriptorBinding::uavs().write_image(
                render_descriptor_sets.current_bindless_set(frame_label).handle(),
                0,
                uav_infos,
            ))
        }
        if !srv_infos.is_empty() {
            writes.push(BindlessDescriptorBinding::srvs().write_image(
                render_descriptor_sets.current_bindless_set(frame_label).handle(),
                0,
                srv_infos,
            ))
        }
        Gfx::get().gfx_device().write_descriptor_sets(&writes);
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
    pub fn unregister_uav(&mut self, image_view_handle: GfxImageViewHandle) {
        debug_assert!(!image_view_handle.is_null());

        self.uavs.remove(image_view_handle).unwrap();
    }

    #[inline]
    pub fn get_shader_uav_handle(&self, image_view_handle: GfxImageViewHandle) -> BindlessUavHandle {
        debug_assert!(!image_view_handle.is_null());

        self.uavs.get(image_view_handle).copied().unwrap()
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
    pub fn unregister_srv(&mut self, image_view_handle: GfxImageViewHandle) {
        debug_assert!(!image_view_handle.is_null());

        self.srvs.remove(image_view_handle).unwrap();
    }

    #[inline]
    pub fn get_shader_srv_handle(&self, image_view_handle: GfxImageViewHandle) -> BindlessSrvHandle {
        debug_assert!(!image_view_handle.is_null());

        self.srvs.get(image_view_handle).copied().unwrap()
    }
}
