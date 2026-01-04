//! Pass 定义和构建器
//!
//! 提供 `RgPass` trait 用于声明式定义渲染 Pass，
//! 以及 `PassBuilder` 用于在 setup 阶段声明资源依赖。

use super::resource_handle::{RgBufferHandle, RgImageHandle};
use super::resource_registry::RgResourceRegistry;
use crate::render_graph_v2::buffer_resource::RgBufferDesc;
use crate::render_graph_v2::executor::RgPassExecutor;
use crate::render_graph_v2::image_resource::RgImageDesc;
use crate::render_graph_v2::{RgBufferResource, RgBufferState, RgImageResource, RgImageState};
use slotmap::SecondaryMap;
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_render_interface::handles::{GfxBufferHandle, GfxImageHandle, GfxImageViewHandle};

/// Pass 执行时的上下文
///
/// 提供 Pass 执行所需的资源访问和命令缓冲区。
pub struct RgPassContext<'a> {
    /// 命令缓冲区
    pub cmd: &'a GfxCommandBuffer,

    /// 资源管理器引用（用于获取物理资源）
    pub resource_manager: &'a truvis_render_interface::gfx_resource_manager::GfxResourceManager,

    /// 物理资源查询表（编译后填充）
    pub(crate) image_handles: &'a SecondaryMap<RgImageHandle, (GfxImageHandle, GfxImageViewHandle)>,
    pub(crate) buffer_handles: &'a SecondaryMap<RgBufferHandle, GfxBufferHandle>,
}

impl<'a> RgPassContext<'a> {
    /// 获取图像的物理句柄
    #[inline]
    pub fn get_image(&self, handle: RgImageHandle) -> Option<(GfxImageHandle, GfxImageViewHandle)> {
        self.image_handles.get(handle).copied()
    }

    /// 获取图像的 handle
    #[inline]
    pub fn get_image_handle(&self, handle: RgImageHandle) -> Option<GfxImageHandle> {
        self.get_image(handle).map(|(h, _)| h)
    }

    /// 获取图像的 view handle
    #[inline]
    pub fn get_image_view_handle(&self, handle: RgImageHandle) -> Option<GfxImageViewHandle> {
        self.get_image(handle).map(|(_, v)| v)
    }

    /// 获取缓冲区的物理句柄
    #[inline]
    pub fn get_buffer(&self, handle: RgBufferHandle) -> Option<GfxBufferHandle> {
        self.buffer_handles.get(handle).copied()
    }
}

/// Pass 构建器
///
/// 在 `RgPass::setup()` 中使用，声明 Pass 的资源依赖。
pub struct RgPassBuilder<'a> {
    /// Pass 名称
    #[allow(dead_code)]
    pub(crate) name: String,

    /// 图像读取列表
    pub(crate) image_reads: Vec<(RgImageHandle, RgImageState)>,
    /// 图像写入列表
    pub(crate) image_writes: Vec<(RgImageHandle, RgImageState)>,
    /// 缓冲区读取列表
    pub(crate) buffer_reads: Vec<(RgBufferHandle, RgBufferState)>,
    /// 缓冲区写入列表
    pub(crate) buffer_writes: Vec<(RgBufferHandle, RgBufferState)>,

    /// 资源注册表引用（用于创建临时资源）
    pub(crate) resources: &'a mut RgResourceRegistry,
}

impl<'a> RgPassBuilder<'a> {
    /// 声明读取图像
    ///
    /// # 参数
    /// - `handle`: 要读取的图像句柄
    /// - `state`: 期望的图像状态（用于自动生成 barrier）
    ///
    /// # 返回
    /// 返回相同的句柄（语义上表示读取后的引用）
    #[inline]
    pub fn read_image(&mut self, handle: RgImageHandle, state: RgImageState) -> RgImageHandle {
        self.image_reads.push((handle, state));
        handle
    }

    /// 声明写入图像
    ///
    /// # 参数
    /// - `handle`: 要写入的图像句柄
    /// - `state`: 写入时的图像状态
    ///
    /// # 返回
    /// 返回相同的句柄（依赖通过 Pass 顺序确定）
    pub fn write_image(&mut self, handle: RgImageHandle, state: RgImageState) -> RgImageHandle {
        self.image_writes.push((handle, state));
        handle
    }

    /// 声明读写图像（同时读取和写入）
    ///
    /// 常用于累积操作（如 RT 累积、后处理）
    pub fn read_write_image(&mut self, handle: RgImageHandle, state: RgImageState) -> RgImageHandle {
        self.read_image(handle, state);
        self.write_image(handle, state)
    }

    /// 创建临时图像
    ///
    /// 图像将在编译阶段创建，执行完毕后自动销毁。
    pub fn create_image(&mut self, name: impl Into<String>, desc: RgImageDesc) -> RgImageHandle {
        self.resources.register_image(RgImageResource::transient(name, desc))
    }

    /// 声明读取缓冲区
    #[inline]
    pub fn read_buffer(&mut self, handle: RgBufferHandle, state: RgBufferState) -> RgBufferHandle {
        self.buffer_reads.push((handle, state));
        handle
    }

    /// 声明写入缓冲区
    pub fn write_buffer(&mut self, handle: RgBufferHandle, state: RgBufferState) -> RgBufferHandle {
        self.buffer_writes.push((handle, state));
        handle
    }

    /// 创建临时缓冲区
    pub fn create_buffer(&mut self, name: impl Into<String>, desc: RgBufferDesc) -> RgBufferHandle {
        self.resources.register_buffer(RgBufferResource::transient(name, desc))
    }
}

/// Pass 节点数据（编译后使用）
pub struct RgPassNode<'a> {
    /// Pass 名称
    pub name: String,

    /// 图像读取
    pub image_reads: Vec<(RgImageHandle, RgImageState)>,
    /// 图像写入
    pub image_writes: Vec<(RgImageHandle, RgImageState)>,
    /// 缓冲区读取
    pub buffer_reads: Vec<(RgBufferHandle, RgBufferState)>,
    /// 缓冲区写入
    pub buffer_writes: Vec<(RgBufferHandle, RgBufferState)>,

    /// 执行回调（类型擦除的 Pass 实现）
    pub(crate) executor: Box<dyn RgPassExecutor + 'a>,
}

/// RgPass trait
///
/// 定义渲染图中的一个 Pass。用户需要实现此 trait 来创建自定义 Pass。
///
/// # 示例
///
/// ```ignore
/// struct MyPass {
///     input: RgImageHandle,
///     output: RgImageHandle,
/// }
///
/// impl RgPass for MyPass {
///     fn setup(&mut self, builder: &mut PassBuilder) {
///         builder.read_image(self.input, ImageState::SHADER_READ_COMPUTE);
///         self.output = builder.write_image(self.output, ImageState::STORAGE_WRITE_COMPUTE);
///     }
///
///     fn execute(&self, ctx: &PassContext) {
///         let input_view = ctx.get_image_view_handle(self.input);
///         let output_view = ctx.get_image_view_handle(self.output);
///         // 绑定 pipeline, dispatch...
///     }
/// }
/// ```
///
/// # 线程安全
///
/// Pass 不需要是 Send + Sync，因为 RenderGraph 通常在单线程中使用。
/// Pass 可以借用外部资源，生命周期由 RenderGraphBuilder 的生命周期参数约束。
pub trait RgPass {
    /// 声明 Pass 的资源依赖
    ///
    /// 在此方法中使用 `PassBuilder` 声明读取和写入的资源。
    fn setup(&mut self, builder: &mut RgPassBuilder);

    /// 执行 Pass 的渲染逻辑
    ///
    /// 命令缓冲区已经开始录制，直接录制命令即可。
    fn execute(&self, ctx: &RgPassContext<'_>);
}
