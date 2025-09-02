use std::collections::HashMap;

use crate::resources_new::{
    managed_buffer::ManagedBuffer,
    managed_image::ManagedImage,
    managed_image_view::ManagedImageView,
    resource_handles::{BufferHandle, ImageHandle, ImageViewHandle},
};

/// RHI 资源管理器
///
/// 统一管理 Vulkan 图形资源的生命周期，通过 Handle 系统提供类型安全的资源访问。
/// 支持的资源类型：Buffer、Image、ImageView
///
/// 仅处理 Vulkan 的资源，不涉及具体的 Buffer 的使用，比如 VertexBuffer, IndexBuffer 等
pub struct ResourceManager
{
    /// 按 Handle 索引的图像资源存储
    images: HashMap<ImageHandle, ManagedImage>,
    /// 按 Handle 索引的缓冲区资源存储
    buffers: HashMap<BufferHandle, ManagedBuffer>,
    /// 按 Handle 索引的图像视图资源存储
    image_views: HashMap<ImageViewHandle, ManagedImageView>,

    /// 每个 ImageHandle 对应的 ImageViewHandle 列表，用于跟踪依赖关系
    image_to_views: HashMap<ImageHandle, Vec<ImageViewHandle>>,

    /// Handle 分配器：下一个可用的图像 ID
    next_image_id: u64,
    /// Handle 分配器：下一个可用的缓冲区 ID
    next_buffer_id: u64,
    /// Handle 分配器：下一个可用的视图 ID
    next_view_id: u64,
}
impl Default for ResourceManager
{
    fn default() -> Self
    {
        Self::new()
    }
}
// 构造函数
impl ResourceManager
{
    /// 创建新的资源管理器实例
    pub fn new() -> Self
    {
        Self {
            images: HashMap::new(),
            buffers: HashMap::new(),
            image_views: HashMap::new(),
            image_to_views: HashMap::new(),
            next_image_id: 0,
            next_buffer_id: 0,
            next_view_id: 0,
        }
    }

    pub fn desotry(&mut self)
    {
        todo!()
    }
}
// 资源注册
impl ResourceManager
{
    /// 注册图像资源，返回唯一的 Handle
    ///
    /// # 参数
    /// * `image` - 要注册的 ManagedImage2D 实例
    ///
    /// # 返回
    /// 返回新分配的 ImageHandle，用于后续访问该资源
    pub fn register_image(&mut self, image: ManagedImage) -> ImageHandle
    {
        let handle = ImageHandle(self.next_image_id);
        self.images.insert(handle, image);
        self.next_image_id += 1;
        handle
    }

    /// 注册缓冲区资源，返回唯一的 Handle
    pub fn register_buffer(&mut self, buffer: ManagedBuffer) -> BufferHandle
    {
        let handle = BufferHandle(self.next_buffer_id);
        self.buffers.insert(handle, buffer);
        self.next_buffer_id += 1;
        handle
    }

    /// 注册图像视图资源，自动维护与父图像的关联关系
    pub fn register_image_view(&mut self, view: ManagedImageView) -> ImageViewHandle
    {
        let image_handle = view.image_handle();
        let handle = ImageViewHandle(self.next_view_id);
        self.image_views.insert(handle, view);
        self.next_view_id += 1;

        // 维护 image_to_views 映射，用于资源依赖管理
        if let Some(views) = self.image_to_views.get_mut(&image_handle) {
            views.push(handle);
        } else {
            self.image_to_views.insert(image_handle, vec![handle]);
        }

        handle
    }
}
// 资源访问
impl ResourceManager
{
    /// 获取图像资源的不可变引用
    pub fn get_image(&self, handle: ImageHandle) -> Option<&ManagedImage>
    {
        self.images.get(&handle)
    }

    /// 获取图像资源的可变引用
    pub fn get_image_mut(&mut self, handle: ImageHandle) -> Option<&mut ManagedImage>
    {
        self.images.get_mut(&handle)
    }

    /// 获取缓冲区资源的不可变引用
    pub fn get_buffer(&self, handle: BufferHandle) -> Option<&ManagedBuffer>
    {
        self.buffers.get(&handle)
    }

    /// 获取缓冲区资源的可变引用
    pub fn get_buffer_mut(&mut self, handle: BufferHandle) -> Option<&mut ManagedBuffer>
    {
        self.buffers.get_mut(&handle)
    }

    /// 获取图像视图资源的不可变引用
    pub fn get_image_view(&self, handle: ImageViewHandle) -> Option<&ManagedImageView>
    {
        self.image_views.get(&handle)
    }
}
