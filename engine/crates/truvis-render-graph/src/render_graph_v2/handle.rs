//! RenderGraph 资源句柄定义
//!
//! 这些句柄是 graph 内部的虚拟引用，与 `GfxResourceManager` 的物理句柄分离。
//! 每个句柄包含版本号，用于跟踪资源在 Pass 之间的状态变化。

use std::fmt;

/// Graph 内部的 Image 句柄
///
/// 用于在 RenderGraph 构建阶段引用图像资源。
/// `version` 字段跟踪资源经过的 Pass 次数，用于依赖分析。
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RgImageHandle {
    /// 资源在 ResourceRegistry 中的索引
    pub(crate) id: u32,
    /// 版本号，每次写操作后递增
    pub(crate) version: u32,
}

impl RgImageHandle {
    /// 创建新句柄
    #[inline]
    pub(crate) fn new(id: u32) -> Self {
        Self { id, version: 0 }
    }

    /// 创建指定版本的句柄
    #[inline]
    pub(crate) fn with_version(id: u32, version: u32) -> Self {
        Self { id, version }
    }

    /// 获取资源 ID
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// 获取版本号
    #[inline]
    pub fn version(&self) -> u32 {
        self.version
    }

    /// 创建下一个版本的句柄（写操作后使用）
    #[inline]
    pub(crate) fn next_version(&self) -> Self {
        Self {
            id: self.id,
            version: self.version + 1,
        }
    }
}

impl fmt::Debug for RgImageHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RgImage({}.v{})", self.id, self.version)
    }
}

/// Graph 内部的 Buffer 句柄
///
/// 用于在 RenderGraph 构建阶段引用缓冲区资源。
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RgBufferHandle {
    /// 资源在 ResourceRegistry 中的索引
    pub(crate) id: u32,
    /// 版本号，每次写操作后递增
    pub(crate) version: u32,
}

impl RgBufferHandle {
    /// 创建新句柄
    #[inline]
    pub(crate) fn new(id: u32) -> Self {
        Self { id, version: 0 }
    }

    /// 创建指定版本的句柄
    #[inline]
    pub(crate) fn with_version(id: u32, version: u32) -> Self {
        Self { id, version }
    }

    /// 获取资源 ID
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// 获取版本号
    #[inline]
    pub fn version(&self) -> u32 {
        self.version
    }

    /// 创建下一个版本的句柄（写操作后使用）
    #[inline]
    pub(crate) fn next_version(&self) -> Self {
        Self {
            id: self.id,
            version: self.version + 1,
        }
    }
}

impl fmt::Debug for RgBufferHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RgBuffer({}.v{})", self.id, self.version)
    }
}
