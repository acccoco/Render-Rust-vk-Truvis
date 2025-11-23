use ash::vk;
use vk_mem::Allocation;

use super::handles::{ImageHandle, ImageViewHandle};

// --- Buffer Resource ---

/// Buffer 类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferType {
    /// 顶点 Buffer
    Vertex,
    /// 索引 Buffer
    Index,
    /// Uniform Buffer
    Uniform,
    /// Storage Buffer (Structured Buffer)
    Storage,
    /// Staging Buffer (用于数据传输)
    Stage,
    /// 原始 Buffer (未指定特定用途)
    Raw,
}

/// Buffer 资源结构体
///
/// 包含 Vulkan Buffer 对象、内存分配信息以及元数据。
pub struct BufferResource {
    // Vulkan 资源
    /// Vulkan Buffer 句柄
    pub buffer: vk::Buffer,
    /// VMA 内存分配信息
    pub allocation: Allocation,

    /// Buffer 类型
    pub buffer_type: BufferType,

    // 基础信息
    /// Buffer 大小（字节）
    pub size: vk::DeviceSize,
    /// Buffer 用途标志
    pub usage: vk::BufferUsageFlags,

    // 映射指针 (如果是 host visible)
    /// 映射的主机内存指针（如果 Buffer 是 Host Visible 且已映射）
    pub mapped_ptr: Option<*mut u8>,

    // Device Address (如果支持)
    /// Buffer Device Address（用于 Ray Tracing 或 Bindless）
    pub device_addr: Option<vk::DeviceAddress>,

    // 元数据 (可选，或者使用单独的 Meta 结构)
    /// 元素数量（例如顶点数、索引数、结构体数组长度）
    pub element_count: u32,
    /// 元素大小（字节）
    pub stride: u32,

    #[cfg(debug_assertions)]
    /// 调试名称
    pub debug_name: String,
}

// --- Image Resource ---

/// Image 来源枚举
pub enum ImageSource {
    /// 由 VMA 分配的 Image
    Allocated(Allocation),
    /// 外部 Image（例如 Swapchain Image），不管理其内存生命周期
    External,
}

/// Image 资源结构体
///
/// 包含 Vulkan Image 对象、内存分配信息以及元数据。
pub struct ImageResource {
    // Vulkan 资源
    /// Vulkan Image 句柄
    pub image: vk::Image,
    /// Image 来源及内存信息
    pub source: ImageSource,

    // 基础信息
    /// Image 尺寸
    pub extent: vk::Extent3D,
    /// Image 格式
    pub format: vk::Format,
    /// Image 用途标志
    pub usage: vk::ImageUsageFlags,
    /// Image Aspect 标志（Color, Depth, Stencil）
    pub aspect_flags: vk::ImageAspectFlags,

    // 视图 (通常 Image 伴随一个默认 View)
    /// 默认 ImageView Handle（覆盖整个 Image）
    pub default_view: ImageViewHandle,

    #[cfg(debug_assertions)]
    /// 调试名称
    pub debug_name: String,
}

// --- Image View Resource ---

/// ImageView 资源结构体
///
/// 包含 Vulkan ImageView 对象以及关联的 Image Handle。
pub struct ImageViewResource {
    /// Vulkan ImageView 句柄
    pub handle: vk::ImageView,
    /// 关联的 Image Handle
    pub image: ImageHandle,

    // Meta
    /// 子资源范围（Mip Levels, Array Layers）
    pub subresource_range: vk::ImageSubresourceRange,
    /// View 类型（1D, 2D, 3D, Cube 等）
    pub view_type: vk::ImageViewType,
    /// View 格式
    pub format: vk::Format,
}
