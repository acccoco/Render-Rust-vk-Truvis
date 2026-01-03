//! 资源注册表
//!
//! 管理 RenderGraph 中的虚拟资源，维护虚拟句柄到物理句柄的映射。

use ash::vk;
use truvis_render_interface::handles::{GfxBufferHandle, GfxImageHandle, GfxImageViewHandle};

use super::handle::{RgBufferHandle, RgImageHandle};
use super::state::{BufferState, ImageState};

/// 图像资源描述（用于创建临时资源）
#[derive(Clone, Debug)]
pub struct RgImageDesc {
    /// 图像宽度
    pub width: u32,
    /// 图像高度
    pub height: u32,
    /// 图像深度（3D 纹理）
    pub depth: u32,
    /// Mip 级别数
    pub mip_levels: u32,
    /// 数组层数
    pub array_layers: u32,
    /// 图像格式
    pub format: vk::Format,
    /// 图像用途
    pub usage: vk::ImageUsageFlags,
    /// 采样数
    pub samples: vk::SampleCountFlags,
    /// 图像类型
    pub image_type: vk::ImageType,
}

impl Default for RgImageDesc {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            depth: 1,
            mip_levels: 1,
            array_layers: 1,
            format: vk::Format::R8G8B8A8_UNORM,
            usage: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::STORAGE,
            samples: vk::SampleCountFlags::TYPE_1,
            image_type: vk::ImageType::TYPE_2D,
        }
    }
}

impl RgImageDesc {
    /// 创建 2D 图像描述
    #[inline]
    pub fn new_2d(width: u32, height: u32, format: vk::Format, usage: vk::ImageUsageFlags) -> Self {
        Self {
            width,
            height,
            format,
            usage,
            ..Default::default()
        }
    }

    /// 设置尺寸
    #[inline]
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// 设置格式
    #[inline]
    pub fn with_format(mut self, format: vk::Format) -> Self {
        self.format = format;
        self
    }

    /// 设置用途
    #[inline]
    pub fn with_usage(mut self, usage: vk::ImageUsageFlags) -> Self {
        self.usage = usage;
        self
    }
}

/// 缓冲区资源描述（用于创建临时资源）
#[derive(Clone, Debug)]
pub struct RgBufferDesc {
    /// 缓冲区大小（字节）
    pub size: vk::DeviceSize,
    /// 缓冲区用途
    pub usage: vk::BufferUsageFlags,
}

impl Default for RgBufferDesc {
    fn default() -> Self {
        Self {
            size: 0,
            usage: vk::BufferUsageFlags::STORAGE_BUFFER,
        }
    }
}

impl RgBufferDesc {
    /// 创建新描述
    #[inline]
    pub fn new(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> Self {
        Self { size, usage }
    }
}

/// 图像资源的来源
#[derive(Clone, Debug)]
pub enum ImageSource {
    /// 从外部导入的图像（已存在于 GfxResourceManager）
    Imported {
        image_handle: GfxImageHandle,
        view_handle: Option<GfxImageViewHandle>,
    },
    /// 由 RenderGraph 创建的临时图像
    Transient { desc: RgImageDesc },
}

/// 缓冲区资源的来源
#[derive(Clone, Debug)]
pub enum BufferSource {
    /// 从外部导入的缓冲区
    Imported { buffer_handle: GfxBufferHandle },
    /// 由 RenderGraph 创建的临时缓冲区
    Transient { desc: RgBufferDesc },
}

/// 图像资源条目
#[derive(Clone, Debug)]
pub struct ImageResource {
    /// 资源来源
    pub source: ImageSource,
    /// 当前状态
    pub current_state: ImageState,
    /// 调试名称
    pub name: String,
    /// 当前版本（被写入的次数）
    pub version: u32,
}

impl ImageResource {
    /// 创建导入的图像资源
    pub fn imported(
        name: impl Into<String>,
        image_handle: GfxImageHandle,
        view_handle: Option<GfxImageViewHandle>,
        initial_state: ImageState,
    ) -> Self {
        Self {
            source: ImageSource::Imported {
                image_handle,
                view_handle,
            },
            current_state: initial_state,
            name: name.into(),
            version: 0,
        }
    }

    /// 创建临时图像资源
    pub fn transient(name: impl Into<String>, desc: RgImageDesc) -> Self {
        Self {
            source: ImageSource::Transient { desc },
            current_state: ImageState::UNDEFINED,
            name: name.into(),
            version: 0,
        }
    }

    /// 获取物理 image handle（仅对导入资源有效）
    pub fn physical_handle(&self) -> Option<GfxImageHandle> {
        match &self.source {
            ImageSource::Imported { image_handle, .. } => Some(*image_handle),
            ImageSource::Transient { .. } => None,
        }
    }

    /// 获取物理 image view handle（仅对导入资源有效）
    pub fn physical_view_handle(&self) -> Option<GfxImageViewHandle> {
        match &self.source {
            ImageSource::Imported { view_handle, .. } => *view_handle,
            ImageSource::Transient { .. } => None,
        }
    }

    /// 检查是否为临时资源
    pub fn is_transient(&self) -> bool {
        matches!(&self.source, ImageSource::Transient { .. })
    }
}

/// 缓冲区资源条目
#[derive(Clone, Debug)]
pub struct BufferResource {
    /// 资源来源
    pub source: BufferSource,
    /// 当前状态
    pub current_state: BufferState,
    /// 调试名称
    pub name: String,
    /// 当前版本
    pub version: u32,
}

impl BufferResource {
    /// 创建导入的缓冲区资源
    pub fn imported(name: impl Into<String>, buffer_handle: GfxBufferHandle, initial_state: BufferState) -> Self {
        Self {
            source: BufferSource::Imported { buffer_handle },
            current_state: initial_state,
            name: name.into(),
            version: 0,
        }
    }

    /// 创建临时缓冲区资源
    pub fn transient(name: impl Into<String>, desc: RgBufferDesc) -> Self {
        Self {
            source: BufferSource::Transient { desc },
            current_state: BufferState::UNDEFINED,
            name: name.into(),
            version: 0,
        }
    }

    /// 获取物理 buffer handle（仅对导入资源有效）
    pub fn physical_handle(&self) -> Option<GfxBufferHandle> {
        match &self.source {
            BufferSource::Imported { buffer_handle } => Some(*buffer_handle),
            BufferSource::Transient { .. } => None,
        }
    }

    /// 检查是否为临时资源
    pub fn is_transient(&self) -> bool {
        matches!(&self.source, BufferSource::Transient { .. })
    }
}

/// 资源注册表
///
/// 管理 RenderGraph 中所有声明的资源，提供虚拟句柄到资源信息的映射。
#[derive(Default)]
pub struct ResourceRegistry {
    /// 图像资源列表
    images: Vec<ImageResource>,
    /// 缓冲区资源列表
    buffers: Vec<BufferResource>,
}

impl ResourceRegistry {
    /// 创建新的资源注册表
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册导入的图像
    pub fn register_imported_image(
        &mut self,
        name: impl Into<String>,
        image_handle: GfxImageHandle,
        view_handle: Option<GfxImageViewHandle>,
        initial_state: ImageState,
    ) -> RgImageHandle {
        let id = self.images.len() as u32;
        self.images.push(ImageResource::imported(name, image_handle, view_handle, initial_state));
        RgImageHandle::new(id)
    }

    /// 注册临时图像（将在编译阶段创建）
    pub fn register_transient_image(&mut self, name: impl Into<String>, desc: RgImageDesc) -> RgImageHandle {
        let id = self.images.len() as u32;
        self.images.push(ImageResource::transient(name, desc));
        RgImageHandle::new(id)
    }

    /// 注册导入的缓冲区
    pub fn register_imported_buffer(
        &mut self,
        name: impl Into<String>,
        buffer_handle: GfxBufferHandle,
        initial_state: BufferState,
    ) -> RgBufferHandle {
        let id = self.buffers.len() as u32;
        self.buffers.push(BufferResource::imported(name, buffer_handle, initial_state));
        RgBufferHandle::new(id)
    }

    /// 注册临时缓冲区
    pub fn register_transient_buffer(&mut self, name: impl Into<String>, desc: RgBufferDesc) -> RgBufferHandle {
        let id = self.buffers.len() as u32;
        self.buffers.push(BufferResource::transient(name, desc));
        RgBufferHandle::new(id)
    }

    /// 获取图像资源
    #[inline]
    pub fn get_image(&self, handle: RgImageHandle) -> Option<&ImageResource> {
        self.images.get(handle.id as usize)
    }

    /// 获取可变图像资源
    #[inline]
    pub fn get_image_mut(&mut self, handle: RgImageHandle) -> Option<&mut ImageResource> {
        self.images.get_mut(handle.id as usize)
    }

    /// 获取缓冲区资源
    #[inline]
    pub fn get_buffer(&self, handle: RgBufferHandle) -> Option<&BufferResource> {
        self.buffers.get(handle.id as usize)
    }

    /// 获取可变缓冲区资源
    #[inline]
    pub fn get_buffer_mut(&mut self, handle: RgBufferHandle) -> Option<&mut BufferResource> {
        self.buffers.get_mut(handle.id as usize)
    }

    /// 获取图像数量
    #[inline]
    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    /// 获取缓冲区数量
    #[inline]
    pub fn buffer_count(&self) -> usize {
        self.buffers.len()
    }

    /// 迭代所有图像资源
    #[inline]
    pub fn iter_images(&self) -> impl Iterator<Item = (RgImageHandle, &ImageResource)> {
        self.images.iter().enumerate().map(|(i, r)| (RgImageHandle::new(i as u32), r))
    }

    /// 迭代所有缓冲区资源
    #[inline]
    pub fn iter_buffers(&self) -> impl Iterator<Item = (RgBufferHandle, &BufferResource)> {
        self.buffers.iter().enumerate().map(|(i, r)| (RgBufferHandle::new(i as u32), r))
    }
}
