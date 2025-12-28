use crate::handles::{GfxBufferHandle, GfxImageHandle, GfxImageViewHandle, GfxTextureHandle};
use crate::texture::GfxTexture;
use ash::vk;
use slotmap::{SecondaryMap, SlotMap};
use truvis_gfx::resources::buffer::GfxBuffer;
use truvis_gfx::resources::image::{GfxImage, GfxImageCreateInfo};
use truvis_gfx::resources::image_view::GfxImageView;
use truvis_gfx::resources::image_view::GfxImageViewDesc;

/// 资源管理器
///
/// 负责管理所有的 GPU 资源，包括 Buffer、Image 和 ImageView。
/// 使用 SlotMap 存储资源，对外提供轻量级的 Handle。
/// 支持资源的延迟销毁（Frames in Flight）。
pub struct GfxResourceManager {
    /// 存储所有的 Buffer 资源
    buffer_pool: SlotMap<GfxBufferHandle, GfxBuffer>,
    /// 存储所有的 Image 资源
    image_pool: SlotMap<GfxImageHandle, GfxImage>,
    /// 存储所有的 ImageView 资源
    image_view_pool: SlotMap<GfxImageViewHandle, GfxImageView>,

    image_view_map: SecondaryMap<GfxImageHandle, Vec<(GfxImageViewDesc, GfxImageViewHandle)>>,

    textures: SlotMap<GfxTextureHandle, GfxTexture>,

    // 待销毁队列 (用于延迟销毁，例如在帧结束时)
    // (handle, frame_index)
    pending_destroy_buffers: Vec<(GfxBufferHandle, u64)>,
    pending_destroy_images: Vec<(GfxImageHandle, u64)>,
    pending_destroy_textures: Vec<(GfxTextureHandle, u64)>,

    /// 当前帧索引，用于判断资源是否可以安全销毁
    current_frame_index: u64,

    #[cfg(debug_assertions)]
    destroyed: bool,
}
impl Default for GfxResourceManager {
    fn default() -> Self {
        Self::new()
    }
}
// new & init
impl GfxResourceManager {
    /// 创建一个新的资源管理器
    pub fn new() -> Self {
        Self {
            buffer_pool: SlotMap::with_key(),
            image_pool: SlotMap::with_key(),
            image_view_pool: SlotMap::with_key(),
            image_view_map: SecondaryMap::new(),
            textures: SlotMap::with_key(),

            pending_destroy_buffers: Vec::new(),
            pending_destroy_images: Vec::new(),
            pending_destroy_textures: Vec::new(),

            current_frame_index: 0,

            #[cfg(debug_assertions)]
            destroyed: false,
        }
    }
}
// destroy
impl GfxResourceManager {
    pub fn destroy(mut self) {
        self.destroy_mut();
    }
    pub fn destroy_mut(&mut self) {
        let _span = tracy_client::span!("ResourceManager::destroy_all");

        // destroy 所有的 textures
        for (_, texture) in self.textures.drain() {
            texture.destroy()
        }

        // destroy 所有的 image views
        for (_, image_view) in self.image_view_pool.drain() {
            image_view.destroy()
        }
        self.image_view_map.clear();

        // Destroy 所有的 images
        for (_, image) in self.image_pool.drain() {
            image.destroy()
        }

        // Destroy 所有的 buffers
        for (_, buffer) in self.buffer_pool.drain() {
            buffer.destroy()
        }

        // Clear pending queues
        self.pending_destroy_buffers.clear();
        self.pending_destroy_images.clear();
        self.pending_destroy_textures.clear();

        #[cfg(debug_assertions)]
        {
            self.destroyed = true;
        }
    }
}
impl Drop for GfxResourceManager {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        {
            assert!(self.destroyed);
        }
    }
}
// Subsystem API
impl GfxResourceManager {
    /// 设置当前帧索引
    ///
    /// 在每一帧开始时调用，用于更新资源管理器的内部时间戳。
    pub fn set_current_frame_index(&mut self, frame_index: u64) {
        self.current_frame_index = frame_index;
    }

    /// 清理已过期的资源
    ///
    /// 检查待销毁队列，销毁那些已经不再被 GPU 使用的资源（即提交销毁时的帧索引 <= completed_frame_index）。
    pub fn cleanup(&mut self, completed_frame_index: u64) {
        let _span = tracy_client::span!("ResourceManager::cleanup");

        // 清理 textures
        let mut textures_to_destroy = Vec::new();
        self.pending_destroy_textures.retain(|(texture_handle, frame_index)| {
            if *frame_index <= completed_frame_index {
                textures_to_destroy.push(*texture_handle);
                false
            } else {
                true
            }
        });
        for texture_handle in textures_to_destroy {
            if let Some(texture) = self.textures.remove(texture_handle) {
                texture.destroy()
            }
        }

        // 清理 buffers
        let mut buffers_to_destroy = Vec::new();
        self.pending_destroy_buffers.retain(|(buffer_handle, frame_index)| {
            if *frame_index <= completed_frame_index {
                buffers_to_destroy.push(*buffer_handle);
                false
            } else {
                true
            }
        });
        for buffer_handle in buffers_to_destroy {
            if let Some(buffer) = self.buffer_pool.remove(buffer_handle) {
                buffer.destroy()
            }
        }

        // 清理 images
        let mut images_to_destroy = Vec::new();
        self.pending_destroy_images.retain(|(image_handle, frame_index)| {
            if *frame_index <= completed_frame_index {
                images_to_destroy.push(*image_handle);
                false
            } else {
                true
            }
        });
        for image_handle in &images_to_destroy {
            // 先清理基于 image 创建的 image views
            if let Some(views) = self.image_view_map.remove(*image_handle) {
                for (_, image_view_handle) in views {
                    if let Some(image_view) = self.image_view_pool.remove(image_view_handle) {
                        image_view.destroy()
                    }
                }
            }
            // 再销毁 image 本身
            if let Some(image) = self.image_pool.remove(*image_handle) {
                image.destroy()
            }
        }
    }
}
// Buffer API
impl GfxResourceManager {
    pub fn register_buffer(&mut self, buffer: GfxBuffer) -> GfxBufferHandle {
        self.buffer_pool.insert(buffer)
    }

    pub fn create_buffer(
        &mut self,
        buffer_size: vk::DeviceSize,
        buffer_usage: vk::BufferUsageFlags,
        align: Option<vk::DeviceSize>,
        mem_map: bool,
        name: impl AsRef<str>,
    ) -> GfxBufferHandle {
        let buffer = GfxBuffer::new(buffer_size, buffer_usage, align, mem_map, name.as_ref());
        self.register_buffer(buffer)
    }

    /// 获取 Buffer 资源引用
    pub fn get_buffer(&self, handle: GfxBufferHandle) -> Option<&GfxBuffer> {
        self.buffer_pool.get(handle)
    }

    /// 获取 Buffer 资源可变引用
    pub fn get_buffer_mut(&mut self, handle: GfxBufferHandle) -> Option<&mut GfxBuffer> {
        self.buffer_pool.get_mut(handle)
    }

    /// 销毁 Buffer（指定帧索引）
    ///
    /// 将 Buffer 加入待销毁队列，在 `current_frame_index` 对应的帧完成后销毁。
    pub fn destroy_buffer(&mut self, handle: GfxBufferHandle, current_frame_index: u64) {
        self.pending_destroy_buffers.push((handle, current_frame_index));
    }

    /// 自动销毁 Buffer
    ///
    /// 使用当前管理器的 `current_frame_index` 作为销毁时间点。
    #[inline]
    pub fn destroy_buffer_auto(&mut self, handle: GfxBufferHandle) {
        self.destroy_buffer(handle, self.current_frame_index)
    }
}
// Image API
impl GfxResourceManager {
    pub fn register_image(&mut self, image: GfxImage) -> GfxImageHandle {
        self.image_pool.insert(image)
    }

    pub fn create_image(
        &mut self,
        image_info: &GfxImageCreateInfo,
        alloc_info: &vk_mem::AllocationCreateInfo,
        debug_name: &str,
    ) -> GfxImageHandle {
        let image = GfxImage::new(image_info, alloc_info, debug_name);
        self.register_image(image)
    }

    /// 获取 Image 资源引用
    pub fn get_image(&self, handle: GfxImageHandle) -> Option<&GfxImage> {
        self.image_pool.get(handle)
    }

    /// 销毁 Image（指定帧索引）
    ///
    /// 同时会销毁默认的 ImageView。
    pub fn destroy_image(&mut self, handle: GfxImageHandle, current_frame_index: u64) {
        self.pending_destroy_images.push((handle, current_frame_index));
    }

    /// 自动销毁 Image
    #[inline]
    pub fn destroy_image_auto(&mut self, handle: GfxImageHandle) {
        self.destroy_image(handle, self.current_frame_index)
    }
}
// ImageView API
impl GfxResourceManager {
    /// 创建一个 ImageView
    pub fn try_create_image_view(
        &mut self,
        image_handle: GfxImageHandle,
        view_desc: GfxImageViewDesc,
        name: impl AsRef<str>,
    ) -> GfxImageViewHandle {
        let _span = tracy_client::span!("ResourceManager::create_image_view");

        let views = self.image_view_map.entry(image_handle).unwrap().or_default();

        // 如果已经存在相同描述的 ImageView，则直接返回
        for (desc, view) in views.iter() {
            if *desc == view_desc {
                return *view;
            }
        }

        let image = self.image_pool.get(image_handle).expect("Invalid image handle");
        let image_view = GfxImageView::new(image.handle(), view_desc, name);
        let image_view_handle = self.image_view_pool.insert(image_view);
        views.push((view_desc, image_view_handle));

        image_view_handle
    }

    /// 获取 ImageView 资源引用
    pub fn get_image_view(&self, handle: GfxImageViewHandle) -> Option<&GfxImageView> {
        self.image_view_pool.get(handle)
    }

    /// 销毁所有资源
    ///
    /// 通常在程序退出时调用。
    pub fn destroy_all(&mut self) {}
}
// Texture API
impl GfxResourceManager {
    pub fn register_texture(&mut self, texture: GfxTexture) -> GfxTextureHandle {
        self.textures.insert(texture)
    }

    pub fn get_texture(&self, handle: GfxTextureHandle) -> Option<&GfxTexture> {
        self.textures.get(handle)
    }

    pub fn destroy_texture(&mut self, handle: GfxTextureHandle, current_frame_index: u64) {
        self.pending_destroy_textures.push((handle, current_frame_index));
    }

    pub fn destroy_texture_auto(&mut self, handle: GfxTextureHandle) {
        self.destroy_texture(handle, self.current_frame_index)
    }
}
