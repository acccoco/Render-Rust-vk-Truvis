use crate::resources_new::{
    managed_buffer::Buffer2,
    managed_image::Image2,
    managed_image_view::ImageView2,
    resource_handles::{BufferHandle, ImageHandle, ImageViewHandle},
};
use ash::vk::Handle;
use std::collections::HashMap;

/// RHI 资源管理器
///
/// 统一管理 Vulkan 图形资源的生命周期，通过 Handle 系统提供类型安全的资源访问。
/// 支持的资源类型：Buffer、Image、ImageView
///
/// 仅处理 Vulkan 的资源，不涉及具体的 Buffer 的使用，比如 VertexBuffer,
/// IndexBuffer 等
pub struct ResourceManager {
    /// 按 Handle 索引的图像资源存储
    images: HashMap<ImageHandle, Image2>,
    /// 按 Handle 索引的缓冲区资源存储
    buffers: HashMap<BufferHandle, Buffer2>,
    /// 按 Handle 索引的图像视图资源存储
    image_views: HashMap<ImageViewHandle, ImageView2>,

    /// 每个 ImageHandle 对应的 ImageViewHandle 列表，用于跟踪依赖关系
    image_to_views: HashMap<ImageHandle, Vec<ImageViewHandle>>,
    // /// Handle 分配器：下一个可用的图像 ID
    // next_image_id: u64,
    // /// Handle 分配器：下一个可用的缓冲区 ID
    // next_buffer_id: u64,
    // /// Handle 分配器：下一个可用的视图 ID
    // next_view_id: u64,
}
impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}
// init & destroy
impl ResourceManager {
    /// 创建新的资源管理器实例
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
            buffers: HashMap::new(),
            image_views: HashMap::new(),
            image_to_views: HashMap::new(),
        }
    }

    pub fn desotry(&mut self) {
        assert!(self.images.is_empty(), "ResourceManager: images not empty on destroy");
        assert!(self.buffers.is_empty(), "ResourceManager: buffers not empty on destroy");
        assert!(self.image_views.is_empty(), "ResourceManager: image_views not empty on destroy");
    }
}
// 资源注册
impl ResourceManager {
    /// 注册图像资源，返回唯一的 Handle
    ///
    /// `image` - 要注册的 ManagedImage2D 实例
    ///
    /// 返回新分配的 ImageHandle，用于后续访问该资源
    pub fn register_image(&mut self, image: Image2) -> ImageHandle {
        let handle = ImageHandle(image.vk_image().as_raw());
        self.images.insert(handle, image);
        handle
    }

    /// 注册缓冲区资源，返回唯一的 Handle
    pub fn register_buffer(&mut self, buffer: Buffer2) -> BufferHandle {
        let handle = BufferHandle(buffer.vk_buffer().as_raw());
        self.buffers.insert(handle, buffer);
        handle
    }

    /// 注册图像视图资源，自动维护与父图像的关联关系
    pub fn register_image_view(&mut self, view: ImageView2) -> ImageViewHandle {
        let image_handle = view.vk_image_();
        let handle = ImageViewHandle(view.vk_image_view().as_raw());
        self.image_views.insert(handle, view);

        // 维护 image_to_views 映射，用于资源依赖管理
        if let Some(views) = self.image_to_views.get_mut(&image_handle) {
            views.push(handle);
        } else {
            self.image_to_views.insert(image_handle, vec![handle]);
        }

        handle
    }
}
// 资源注销
impl ResourceManager {
    /// 注销图像资源及其相关的图像视图
    pub fn unregister_image(&mut self, handle: ImageHandle) {
        // 首先移除相关的图像视图
        if let Some(view_handles) = self.image_to_views.remove(&handle) {
            for view_handle in view_handles {
                self.image_views.remove(&view_handle);
            }
        }
        // 然后移除图像资源本身
        self.images.remove(&handle).unwrap().destroy();
    }

    /// 注销缓冲区资源
    pub fn unregister_buffer(&mut self, handle: BufferHandle) {
        self.buffers.remove(&handle).unwrap().destroy()
    }
}
// 资源访问
impl ResourceManager {
    /// 获取图像资源的不可变引用
    pub fn get_image(&self, handle: ImageHandle) -> Option<&Image2> {
        self.images.get(&handle)
    }

    /// 获取图像资源的可变引用
    pub fn get_image_mut(&mut self, handle: ImageHandle) -> Option<&mut Image2> {
        self.images.get_mut(&handle)
    }

    /// 获取缓冲区资源的不可变引用
    pub fn get_buffer(&self, handle: BufferHandle) -> Option<&Buffer2> {
        self.buffers.get(&handle)
    }

    /// 获取缓冲区资源的可变引用
    pub fn get_buffer_mut(&mut self, handle: BufferHandle) -> Option<&mut Buffer2> {
        self.buffers.get_mut(&handle)
    }

    /// 获取图像视图资源的不可变引用
    pub fn get_image_view(&self, handle: ImageViewHandle) -> Option<&ImageView2> {
        self.image_views.get(&handle)
    }
}
