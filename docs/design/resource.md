# 资源系统重构设计 (SlotMap Based)

## 目标
使用基于 `slotmap` 的资源系统替代现在的基于 `Rc` 的资源系统。
- **所有权集中**: 资源由 `ResourceManager` 统一管理。
- **弱引用访问**: 外部持有轻量级的 `Handle` (Copy)，不直接持有资源所有权。
- **类型安全**: 提供强类型的 Handle (如 `StructuredBufferHandle<T>`)。
- **元数据管理**: 资源内部包含 Vulkan 对象、内存分配信息以及元数据 (如顶点数量、格式等)。

## 核心数据结构

### 1. Handles (对外接口)

使用 `slotmap` 的 `new_key_type!` 宏定义内部 Key，外部封装为具体的 Handle 类型。

```rust
use slotmap::new_key_type;
use std::marker::PhantomData;

// 内部 Key (不直接暴露给普通用户，或者作为底层 API)
new_key_type! {
    pub struct InnerImageHandle;
    pub struct InnerImageViewHandle;
    pub struct InnerBufferHandle;
}

// --- Buffer Handles ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferHandle {
    pub(crate) inner: InnerBufferHandle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VertexBufferHandle<T> {
    pub(crate) inner: InnerBufferHandle,
    pub(crate) _marker: PhantomData<T>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IndexBufferHandle {
    pub(crate) inner: InnerBufferHandle,
    // 可以包含 index type 信息，或者在 resource meta 中存储
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StructuredBufferHandle<T> {
    pub(crate) inner: InnerBufferHandle,
    pub(crate) _marker: PhantomData<T>,
}

// --- Image Handles ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageHandle {
    pub(crate) inner: InnerImageHandle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageViewHandle {
    pub(crate) inner: InnerImageViewHandle,
}

// 可以扩展 TextureHandle 等
```

### 2. Resources (内部存储)

资源结构体存储实际的 Vulkan 对象和元数据。

```rust
use ash::vk;
use vk_mem::Allocation;

// --- Buffer Resource ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferType {
    Vertex,
    Index,
    Uniform,
    Storage,
    Stage,
    Raw,
}

pub struct BufferResource {
    // Vulkan 资源
    pub buffer: vk::Buffer,
    pub allocation: Allocation,
    
    pub buffer_type: BufferType,
    
    // 基础信息
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    
    // 映射指针 (如果是 host visible)
    pub mapped_ptr: Option<*mut u8>,
    
    // Device Address (如果支持)
    pub device_addr: Option<vk::DeviceAddress>,
    
    // 元数据 (可选，或者使用单独的 Meta 结构)
    pub element_count: u32, // 例如顶点数、索引数、结构体数组长度
    pub stride: u32,        // 元素大小
    
    #[cfg(debug_assertions)]
    pub debug_name: String,
}

// --- Image Resource ---

pub enum ImageSource {
    Allocated(Allocation),
    External, // 例如 Swapchain Image，不管理生命周期
}

pub struct ImageResource {
    // Vulkan 资源
    pub image: vk::Image,
    pub source: ImageSource,
    
    // 基础信息
    pub extent: vk::Extent3D,
    pub format: vk::Format,
    pub usage: vk::ImageUsageFlags,
    pub aspect_flags: vk::ImageAspectFlags,
    
    // 视图 (通常 Image 伴随一个默认 View)
    pub default_view: ImageViewHandle,
    
    #[cfg(debug_assertions)]
    pub debug_name: String,
}

// --- Image View Resource ---

pub struct ImageViewResource {
    pub handle: vk::ImageView,
    pub image: ImageHandle, // 关联的 Image
    
    // Meta
    pub subresource_range: vk::ImageSubresourceRange,
    pub view_type: vk::ImageViewType,
    pub format: vk::Format,
}
```

### 3. Resource Manager (管理器)

```rust
use slotmap::SlotMap;

pub struct ResourceManager {
    buffers: SlotMap<InnerBufferHandle, BufferResource>,
    images: SlotMap<InnerImageHandle, ImageResource>,
    image_views: SlotMap<InnerImageViewHandle, ImageViewResource>,
    
    // 待销毁队列 (用于延迟销毁，例如在帧结束时)
    pending_destroy_buffers: Vec<(InnerBufferHandle, u64)>, // (handle, frame_index)
    pending_destroy_images: Vec<(InnerImageHandle, u64)>,
    pending_destroy_image_views: Vec<(InnerImageViewHandle, u64)>,
}

impl ResourceManager {
    // --- Buffer API ---
    
    pub fn create_buffer(
        &mut self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        mapped: bool,
        buffer_type: BufferType,
        name: impl AsRef<str>,
    ) -> BufferHandle {
        // ...
    }
    
    pub fn get_buffer(&self, handle: BufferHandle) -> Option<&BufferResource> {
        self.buffers.get(handle.inner)
    }
    
    pub fn get_buffer_mut(&mut self, handle: BufferHandle) -> Option<&mut BufferResource> {
        self.buffers.get_mut(handle.inner)
    }
    
    pub fn destroy_buffer(&mut self, handle: BufferHandle) {
        // 添加到待销毁队列，或者立即销毁
    }

    // --- Image API ---
    
    pub fn create_image(
        &mut self,
        create_info: &vk::ImageCreateInfo,
        alloc_info: &vk_mem::AllocationCreateInfo,
        name: impl AsRef<str>,
    ) -> ImageHandle {
        // ...
    }

    pub fn create_image_with_data(
        &mut self,
        create_info: &vk::ImageCreateInfo,
        alloc_info: &vk_mem::AllocationCreateInfo,
        data: &[u8],
        name: impl AsRef<str>,
    ) -> ImageHandle {
        // ...
    }

    pub fn upload_image_data(&mut self, image_handle: ImageHandle, data: &[u8]) {
        // ...
    }

    pub fn create_external_image(
        &mut self,
        image: vk::Image,
        create_info: &vk::ImageCreateInfo,
        name: impl AsRef<str>,
    ) -> ImageHandle {
        // 注册外部 Image (如 Swapchain Image)
    }
    
    pub fn get_image(&self, handle: ImageHandle) -> Option<&ImageResource> {
        self.images.get(handle.inner)
    }

    // --- Image View API ---

    pub fn create_image_view(&mut self, info: &ImageViewCreateInfo) -> ImageViewHandle {
        // ...
    }

    pub fn get_image_view(&self, handle: ImageViewHandle) -> Option<&ImageViewResource> {
        self.image_views.get(handle.inner)
    }
    
    pub fn destroy_image_view(&mut self, handle: ImageViewHandle) {
        // ...
    }
    
    // ...
}
```

## 实现步骤划分

### Phase 1: 基础架构搭建 (truvis-gfx) [已完成]
1.  **引入依赖**: 在 `truvis-gfx/Cargo.toml` 中添加 `slotmap`。
2.  **定义 Handles**: 创建 `crates/truvis-gfx/src/resources/handles.rs`，定义所有 Handle 类型。
3.  **定义 Resources**: 创建 `crates/truvis-gfx/src/resources/resource_data.rs`，定义 `BufferResource`, `ImageResource`, `ImageViewResource`。
4.  **实现 ResourceManager**:
    *   创建 `crates/truvis-gfx/src/resources/manager.rs`。
    *   实现 `create_buffer`, `destroy_buffer` (对接 VMA)。
    *   实现 `create_image`, `create_image_with_data`, `upload_image_data`, `create_external_image`, `destroy_image`。
    *   实现 `create_image_view`, `destroy_image_view`。
    *   实现 `cleanup` 方法，处理延迟销毁队列。
    *   **新增**: 实现 `create_vertex_buffer`, `create_index_buffer`, `create_structured_buffer` 等辅助方法，确保不丢失 `special_buffers` 中的细节（如 Usage Flags 和 Size 计算）。
5.  **集成到 Gfx**: 在 `Gfx` 结构体中添加 `resource_manager: RefCell<ResourceManager>`，并提供访问接口。

### Phase 2: Buffer 系统迁移 [已完成]
1.  **Model Manager 适配**: 修改 `truvis-model-manager`，将 `Geometry` 中的 `GfxBuffer` 替换为 `VertexBufferHandle` / `IndexBufferHandle`。[已完成]
2.  **Render Pass 适配**: 修改 `truvis-render` 中的渲染管线。[已完成]
    *   在 `draw` 调用时，通过 `FrameContext::resource_manager()` 获取 `vk::Buffer` 进行绑定。
3.  **Uniform/Storage Buffer**: 将原本使用 `GfxBuffer` 创建的 Uniform Buffer 改为使用 `ResourceManager` 创建，并持有 Handle。[已完成]
4.  **移除 GfxBuffer**: 确认所有引用已移除后，删除 `GfxBuffer` 结构体。[已完成]
    *   注意：`GfxBuffer` 目前仍用于 Staging Buffer 和内部传输，需要迁移。[已完成]

### Phase 3: Image 系统迁移 [已完成]
1.  **Swapchain 适配**: 修改 `truvis-gfx/src/swapchain`，将获取到的 Swapchain Images 注册到 `ResourceManager`，获取 `ImageHandle`。[已完成]
2.  **Render Targets**: 修改 `FifBuffers` (Frames in Flight Buffers)，使用 `ImageHandle` 管理 Depth/Color Attachments。[已完成]
3.  **Texture Assets**: 修改纹理加载逻辑，加载后上传到 GPU 并返回 `ImageHandle`。[已完成]
4.  **Descriptor Set 更新**: 修改 `DescriptorSet` 更新逻辑，接受 Handle，内部解析为 `vk::ImageView` / `vk::Sampler`。[已完成]
5.  **移除 GfxImage2D**: 确认所有引用已移除后，删除 `GfxImage2D` 结构体。[已完成]

### Phase 4: 验证与优化 [已完成]
1.  **Bindless 集成**: 确保 Bindless 系统能正确通过 Handle 获取资源并更新全局 Descriptor Set。[已完成]
2.  **延迟销毁验证**: 验证 `pending_destroy` 队列是否在帧结束时正确清理资源，无内存泄漏或过早释放。[已完成]
3.  **多线程测试**: [已取消] 当前架构设计为单线程渲染提交，`ResourceManager` 位于 `Gfx` 单例中，由 `RefCell` 保护，暂不需要多线程支持。

## 细节考虑

-   **线程安全**: 当前设计为单线程访问。`ResourceManager` 存储在 `Gfx` 单例中，使用 `RefCell` 提供内部可变性。所有资源创建和销毁操作应在主渲染线程进行。
-   **Bindless**: Bindless 系统需要直接访问 `vk::Buffer` 和 `vk::ImageView`，可以通过 Handle 从 Manager 获取后更新 Descriptor Set。
-   **生命周期**: 使用 Handle 后，需要注意资源的生命周期管理。如果 Handle 失效（资源被销毁），`get` 方法应返回 `None` 或 panic (取决于策略)。
-   **泛型 Handle**: `StructuredBufferHandle<T>` 提供了编译期类型检查，避免将错误的 Buffer 绑定到 Shader。
