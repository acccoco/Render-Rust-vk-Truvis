//! Pass 定义和构建器
//!
//! 提供 `RgPass` trait 用于声明式定义渲染 Pass，
//! 以及 `PassBuilder` 用于在 setup 阶段声明资源依赖。

use std::any::Any;

use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_render_interface::handles::{GfxBufferHandle, GfxImageHandle, GfxImageViewHandle};

use super::handle::{RgBufferHandle, RgImageHandle};
use super::resource::{RgBufferDesc, RgImageDesc};
use super::state::{BufferState, ImageState};

/// Pass 执行时的上下文
///
/// 提供 Pass 执行所需的资源访问和命令缓冲区。
pub struct PassContext<'a> {
    /// 命令缓冲区
    pub cmd: &'a GfxCommandBuffer,

    /// 物理资源查询表（编译后填充）
    pub(crate) image_handles: &'a [(GfxImageHandle, GfxImageViewHandle)],
    pub(crate) buffer_handles: &'a [GfxBufferHandle],

    /// 资源索引映射（RgHandle.id -> 物理资源索引）
    pub(crate) image_index_map: &'a [usize],
    pub(crate) buffer_index_map: &'a [usize],
}

impl<'a> PassContext<'a> {
    /// 获取图像的物理句柄
    #[inline]
    pub fn get_image(&self, handle: RgImageHandle) -> (GfxImageHandle, GfxImageViewHandle) {
        let idx = self.image_index_map[handle.id as usize];
        self.image_handles[idx]
    }

    /// 获取图像的 handle
    #[inline]
    pub fn get_image_handle(&self, handle: RgImageHandle) -> GfxImageHandle {
        self.get_image(handle).0
    }

    /// 获取图像的 view handle
    #[inline]
    pub fn get_image_view_handle(&self, handle: RgImageHandle) -> GfxImageViewHandle {
        self.get_image(handle).1
    }

    /// 获取缓冲区的物理句柄
    #[inline]
    pub fn get_buffer(&self, handle: RgBufferHandle) -> GfxBufferHandle {
        let idx = self.buffer_index_map[handle.id as usize];
        self.buffer_handles[idx]
    }
}

/// 资源读取声明
#[derive(Clone, Debug)]
pub struct ImageRead {
    /// 资源句柄
    pub handle: RgImageHandle,
    /// 期望的状态
    pub state: ImageState,
}

/// 资源写入声明
#[derive(Clone, Debug)]
pub struct ImageWrite {
    /// 资源句柄（写入前的版本）
    pub handle: RgImageHandle,
    /// 期望的状态
    pub state: ImageState,
    /// 写入后的新句柄（版本递增）
    pub output_handle: RgImageHandle,
}

/// 缓冲区读取声明
#[derive(Clone, Debug)]
pub struct BufferRead {
    /// 资源句柄
    pub handle: RgBufferHandle,
    /// 期望的状态
    pub state: BufferState,
}

/// 缓冲区写入声明
#[derive(Clone, Debug)]
pub struct BufferWrite {
    /// 资源句柄
    pub handle: RgBufferHandle,
    /// 期望的状态
    pub state: BufferState,
    /// 写入后的新句柄
    pub output_handle: RgBufferHandle,
}

/// Pass 构建器
///
/// 在 `RgPass::setup()` 中使用，声明 Pass 的资源依赖。
pub struct PassBuilder<'a> {
    /// Pass 名称
    pub(crate) name: String,

    /// 图像读取列表
    pub(crate) image_reads: Vec<ImageRead>,
    /// 图像写入列表
    pub(crate) image_writes: Vec<ImageWrite>,
    /// 缓冲区读取列表
    pub(crate) buffer_reads: Vec<BufferRead>,
    /// 缓冲区写入列表
    pub(crate) buffer_writes: Vec<BufferWrite>,

    /// 临时图像创建请求
    pub(crate) transient_images: Vec<(String, RgImageDesc)>,
    /// 临时缓冲区创建请求
    pub(crate) transient_buffers: Vec<(String, RgBufferDesc)>,

    /// 下一个可用的临时资源 ID（由外部 graph 提供）
    pub(crate) next_image_id: &'a mut u32,
    pub(crate) next_buffer_id: &'a mut u32,

    /// 当前资源版本表（用于跟踪写入）
    pub(crate) image_versions: &'a mut Vec<u32>,
    pub(crate) buffer_versions: &'a mut Vec<u32>,
}

impl<'a> PassBuilder<'a> {
    /// 声明读取图像
    ///
    /// # 参数
    /// - `handle`: 要读取的图像句柄
    /// - `state`: 期望的图像状态（用于自动生成 barrier）
    ///
    /// # 返回
    /// 返回相同的句柄（语义上表示读取后的引用）
    #[inline]
    pub fn read_image(&mut self, handle: RgImageHandle, state: ImageState) -> RgImageHandle {
        self.image_reads.push(ImageRead { handle, state });
        handle
    }

    /// 声明写入图像
    ///
    /// # 参数
    /// - `handle`: 要写入的图像句柄
    /// - `state`: 写入时的图像状态
    ///
    /// # 返回
    /// 返回新版本的句柄，用于后续 Pass 的依赖声明
    pub fn write_image(&mut self, handle: RgImageHandle, state: ImageState) -> RgImageHandle {
        // 确保版本表足够大
        while self.image_versions.len() <= handle.id as usize {
            self.image_versions.push(0);
        }

        // 递增版本
        let current_version = self.image_versions[handle.id as usize];
        self.image_versions[handle.id as usize] = current_version + 1;

        let output_handle = RgImageHandle::with_version(handle.id, current_version + 1);

        self.image_writes.push(ImageWrite {
            handle,
            state,
            output_handle,
        });

        output_handle
    }

    /// 声明读写图像（同时读取和写入）
    ///
    /// 常用于累积操作（如 RT 累积、后处理）
    pub fn read_write_image(&mut self, handle: RgImageHandle, state: ImageState) -> RgImageHandle {
        self.read_image(handle, state);
        self.write_image(handle, state)
    }

    /// 创建临时图像
    ///
    /// 图像将在编译阶段创建，执行完毕后自动销毁。
    pub fn create_image(&mut self, name: impl Into<String>, desc: RgImageDesc) -> RgImageHandle {
        let id = *self.next_image_id;
        *self.next_image_id += 1;

        let name = name.into();
        self.transient_images.push((name, desc));

        // 初始化版本
        while self.image_versions.len() <= id as usize {
            self.image_versions.push(0);
        }

        RgImageHandle::new(id)
    }

    /// 声明读取缓冲区
    #[inline]
    pub fn read_buffer(&mut self, handle: RgBufferHandle, state: BufferState) -> RgBufferHandle {
        self.buffer_reads.push(BufferRead { handle, state });
        handle
    }

    /// 声明写入缓冲区
    pub fn write_buffer(&mut self, handle: RgBufferHandle, state: BufferState) -> RgBufferHandle {
        while self.buffer_versions.len() <= handle.id as usize {
            self.buffer_versions.push(0);
        }

        let current_version = self.buffer_versions[handle.id as usize];
        self.buffer_versions[handle.id as usize] = current_version + 1;

        let output_handle = RgBufferHandle::with_version(handle.id, current_version + 1);

        self.buffer_writes.push(BufferWrite {
            handle,
            state,
            output_handle,
        });

        output_handle
    }

    /// 创建临时缓冲区
    pub fn create_buffer(&mut self, name: impl Into<String>, desc: RgBufferDesc) -> RgBufferHandle {
        let id = *self.next_buffer_id;
        *self.next_buffer_id += 1;

        let name = name.into();
        self.transient_buffers.push((name, desc));

        while self.buffer_versions.len() <= id as usize {
            self.buffer_versions.push(0);
        }

        RgBufferHandle::new(id)
    }
}

/// Pass 节点数据（编译后使用）
pub struct PassNode {
    /// Pass 名称
    pub name: String,

    /// 图像读取
    pub image_reads: Vec<ImageRead>,
    /// 图像写入
    pub image_writes: Vec<ImageWrite>,
    /// 缓冲区读取
    pub buffer_reads: Vec<BufferRead>,
    /// 缓冲区写入
    pub buffer_writes: Vec<BufferWrite>,

    /// 执行回调（类型擦除的 Pass 实现）
    pub(crate) executor: Box<dyn PassExecutor>,
}

impl PassNode {
    /// 获取所有读取的图像 ID
    pub fn read_image_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.image_reads.iter().map(|r| r.handle.id)
    }

    /// 获取所有写入的图像 ID
    pub fn write_image_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.image_writes.iter().map(|w| w.handle.id)
    }

    /// 获取所有读取的缓冲区 ID
    pub fn read_buffer_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.buffer_reads.iter().map(|r| r.handle.id)
    }

    /// 获取所有写入的缓冲区 ID
    pub fn write_buffer_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.buffer_writes.iter().map(|w| w.handle.id)
    }
}

/// 类型擦除的 Pass 执行器 trait
pub(crate) trait PassExecutor: Send + Sync {
    /// 执行 Pass
    fn execute(&self, ctx: &PassContext<'_>);

    /// 获取 Any 引用（用于向下转换）
    fn as_any(&self) -> &dyn Any;
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
pub trait RgPass: Send + Sync + 'static {
    /// 声明 Pass 的资源依赖
    ///
    /// 在此方法中使用 `PassBuilder` 声明读取和写入的资源。
    fn setup(&mut self, builder: &mut PassBuilder);

    /// 执行 Pass 的渲染逻辑
    ///
    /// 命令缓冲区已经开始录制，直接录制命令即可。
    fn execute(&self, ctx: &PassContext<'_>);
}

/// 包装用户 Pass 实现的执行器
pub(crate) struct RgPassExecutor<P: RgPass> {
    pub pass: P,
}

impl<P: RgPass> PassExecutor for RgPassExecutor<P> {
    fn execute(&self, ctx: &PassContext<'_>) {
        self.pass.execute(ctx);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
