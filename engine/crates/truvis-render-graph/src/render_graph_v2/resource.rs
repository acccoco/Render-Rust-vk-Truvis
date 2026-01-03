//! 资源注册表
//!
//! 管理 RenderGraph 中的虚拟资源，维护虚拟句柄到物理句柄的映射。

use ash::vk;
use slotmap::SlotMap;
use truvis_gfx::resources::image_view::GfxImageViewDesc;
use truvis_render_interface::handles::{GfxBufferHandle, GfxImageHandle, GfxImageViewHandle};

use super::handle::{RgBufferHandle, RgImageHandle};
use super::state::{BufferState, ImageState};

/// 图像资源描述（用于创建临时资源）
///
/// 包含创建 `vk::Image` 所需的所有信息，以及可选的默认视图描述。
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
    /// 可选的默认视图描述（用于自动创建 ImageView）
    pub default_view_desc: Option<GfxImageViewDesc>,
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
            default_view_desc: None,
        }
    }
}

impl RgImageDesc {
    /// 创建 2D 图像描述
    #[inline]
    pub fn new_2d(width: u32, height: u32, format: vk::Format, usage: vk::ImageUsageFlags) -> Self {
        Self { width, height, format, usage, ..Default::default() }
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

    /// 设置默认视图描述
    #[inline]
    pub fn with_default_view(mut self, view_desc: GfxImageViewDesc) -> Self {
        self.default_view_desc = Some(view_desc);
        self
    }

    /// 自动推断并生成默认视图描述
    ///
    /// 根据图像格式和类型推断 aspect 和 view_type
    pub fn infer_default_view(&self) -> GfxImageViewDesc {
        let aspect = Self::infer_aspect(self.format);
        let view_type = Self::infer_view_type(self.image_type, self.array_layers);

        GfxImageViewDesc::new(
            self.format,
            view_type,
            aspect,
            (0, self.mip_levels as u8),
            (0, self.array_layers as u8),
        )
    }

    /// 从格式推断 aspect
    fn infer_aspect(format: vk::Format) -> vk::ImageAspectFlags {
        match format {
            vk::Format::D16_UNORM | vk::Format::D32_SFLOAT | vk::Format::X8_D24_UNORM_PACK32 => {
                vk::ImageAspectFlags::DEPTH
            }
            vk::Format::S8_UINT => vk::ImageAspectFlags::STENCIL,
            vk::Format::D16_UNORM_S8_UINT | vk::Format::D24_UNORM_S8_UINT | vk::Format::D32_SFLOAT_S8_UINT => {
                vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
            }
            _ => vk::ImageAspectFlags::COLOR,
        }
    }

    /// 从图像类型推断视图类型
    fn infer_view_type(image_type: vk::ImageType, array_layers: u32) -> vk::ImageViewType {
        match image_type {
            vk::ImageType::TYPE_1D => {
                if array_layers > 1 {
                    vk::ImageViewType::TYPE_1D_ARRAY
                } else {
                    vk::ImageViewType::TYPE_1D
                }
            }
            vk::ImageType::TYPE_2D => {
                if array_layers > 1 {
                    vk::ImageViewType::TYPE_2D_ARRAY
                } else {
                    vk::ImageViewType::TYPE_2D
                }
            }
            vk::ImageType::TYPE_3D => vk::ImageViewType::TYPE_3D,
            _ => vk::ImageViewType::TYPE_2D,
        }
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
/// 使用 SlotMap 存储资源，提供稳定的句柄和高效的访问。
#[derive(Default)]
pub struct ResourceRegistry {
    /// 图像资源表
    images: SlotMap<RgImageHandle, ImageResource>,
    /// 缓冲区资源表
    buffers: SlotMap<RgBufferHandle, BufferResource>,
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
        self.images.insert(ImageResource::imported(name, image_handle, view_handle, initial_state))
    }

    /// 注册临时图像（将在编译阶段创建）
    pub fn register_transient_image(&mut self, name: impl Into<String>, desc: RgImageDesc) -> RgImageHandle {
        self.images.insert(ImageResource::transient(name, desc))
    }

    /// 注册导入的缓冲区
    pub fn register_imported_buffer(
        &mut self,
        name: impl Into<String>,
        buffer_handle: GfxBufferHandle,
        initial_state: BufferState,
    ) -> RgBufferHandle {
        self.buffers.insert(BufferResource::imported(name, buffer_handle, initial_state))
    }

    /// 注册临时缓冲区
    pub fn register_transient_buffer(&mut self, name: impl Into<String>, desc: RgBufferDesc) -> RgBufferHandle {
        self.buffers.insert(BufferResource::transient(name, desc))
    }

    /// 获取图像资源
    #[inline]
    pub fn get_image(&self, handle: RgImageHandle) -> Option<&ImageResource> {
        self.images.get(handle)
    }

    /// 获取可变图像资源
    #[inline]
    pub fn get_image_mut(&mut self, handle: RgImageHandle) -> Option<&mut ImageResource> {
        self.images.get_mut(handle)
    }

    /// 获取缓冲区资源
    #[inline]
    pub fn get_buffer(&self, handle: RgBufferHandle) -> Option<&BufferResource> {
        self.buffers.get(handle)
    }

    /// 获取可变缓冲区资源
    #[inline]
    pub fn get_buffer_mut(&mut self, handle: RgBufferHandle) -> Option<&mut BufferResource> {
        self.buffers.get_mut(handle)
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
        self.images.iter()
    }

    /// 迭代所有缓冲区资源
    #[inline]
    pub fn iter_buffers(&self) -> impl Iterator<Item = (RgBufferHandle, &BufferResource)> {
        self.buffers.iter()
    }
}
