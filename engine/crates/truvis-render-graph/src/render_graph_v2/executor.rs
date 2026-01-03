//! RenderGraph 构建器和执行器
//!
//! 提供 `RenderGraphBuilder` 用于构建渲染图，
//! `CompiledGraph` 用于缓存编译结果并执行渲染。

use ash::vk;
use slotmap::SecondaryMap;
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_render_interface::gfx_resource_manager::GfxResourceManager;
use truvis_render_interface::handles::{GfxBufferHandle, GfxImageHandle, GfxImageViewHandle};

use super::barrier::{BufferBarrierDesc, ImageBarrierDesc, PassBarriers};
use super::graph::{DependencyAnalyzer, DependencyGraph};
use super::handle::{RgBufferHandle, RgImageHandle};
use super::pass::{PassBuilder, PassContext, PassNode, RgPass, RgPassExecutor};
use super::resource::ResourceRegistry;
use super::state::{BufferState, ImageState};

/// RenderGraph 构建器
///
/// 用于声明式构建渲染图。
///
/// # 使用流程
///
/// 1. 创建 builder: `RenderGraphBuilder::new()`
/// 2. 导入外部资源: `builder.import_image(...)`
/// 3. 添加 Pass: `builder.add_pass("name", pass)`
/// 4. 编译: `builder.compile()`
/// 5. 执行: `compiled.execute(...)`
pub struct RenderGraphBuilder {
    /// 资源注册表
    resources: ResourceRegistry,

    /// Pass 节点列表（按添加顺序）
    passes: Vec<PassNode>,
}

impl Default for RenderGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderGraphBuilder {
    /// 创建新的 RenderGraph 构建器
    pub fn new() -> Self {
        Self { resources: ResourceRegistry::new(), passes: Vec::new() }
    }

    /// 导入外部图像资源
    ///
    /// # 参数
    /// - `name`: 资源调试名称
    /// - `image_handle`: 物理图像句柄（来自 GfxResourceManager）
    /// - `view_handle`: 可选的图像视图句柄
    /// - `initial_state`: 图像的初始状态
    ///
    /// # 返回
    /// RenderGraph 内部的图像句柄
    pub fn import_image(
        &mut self,
        name: impl Into<String>,
        image_handle: GfxImageHandle,
        view_handle: Option<GfxImageViewHandle>,
        initial_state: ImageState,
    ) -> RgImageHandle {
        self.resources.register_imported_image(name, image_handle, view_handle, initial_state)
    }

    /// 导入外部缓冲区资源
    pub fn import_buffer(
        &mut self,
        name: impl Into<String>,
        buffer_handle: GfxBufferHandle,
        initial_state: BufferState,
    ) -> RgBufferHandle {
        self.resources.register_imported_buffer(name, buffer_handle, initial_state)
    }

    /// 添加 Pass
    ///
    /// # 参数
    /// - `name`: Pass 名称（用于调试和性能分析）
    /// - `pass`: 实现了 `RgPass` trait 的 Pass 对象
    ///
    /// # 返回
    /// 返回 `&mut Self` 以支持链式调用
    pub fn add_pass<P: RgPass>(&mut self, name: impl Into<String>, mut pass: P) -> &mut Self {
        let name = name.into();

        // 创建 PassBuilder 供 Pass 声明依赖
        let mut builder = PassBuilder {
            name: name.clone(),
            image_reads: Vec::new(),
            image_writes: Vec::new(),
            buffer_reads: Vec::new(),
            buffer_writes: Vec::new(),
            resources: &mut self.resources,
        };

        // 调用 Pass 的 setup 方法
        pass.setup(&mut builder);

        // 创建 PassNode
        let node = PassNode {
            name,
            image_reads: builder.image_reads,
            image_writes: builder.image_writes,
            buffer_reads: builder.buffer_reads,
            buffer_writes: builder.buffer_writes,
            executor: Box::new(RgPassExecutor { pass }),
        };

        self.passes.push(node);
        self
    }

    /// 编译渲染图
    ///
    /// 执行依赖分析、拓扑排序、barrier 计算。
    ///
    /// # 返回
    /// 编译后的 `CompiledGraph`，可以多次执行
    ///
    /// # Panics
    /// 如果检测到循环依赖
    pub fn compile(self) -> CompiledGraph {
        let pass_count = self.passes.len();

        // 收集每个 Pass 的读写资源句柄
        let image_reads: Vec<Vec<RgImageHandle>> =
            self.passes.iter().map(|p| p.read_image_handles().collect()).collect();
        let image_writes: Vec<Vec<RgImageHandle>> =
            self.passes.iter().map(|p| p.write_image_handles().collect()).collect();
        let buffer_reads: Vec<Vec<RgBufferHandle>> =
            self.passes.iter().map(|p| p.read_buffer_handles().collect()).collect();
        let buffer_writes: Vec<Vec<RgBufferHandle>> =
            self.passes.iter().map(|p| p.write_buffer_handles().collect()).collect();

        // 依赖分析
        let dep_graph =
            DependencyAnalyzer::analyze(pass_count, &image_reads, &image_writes, &buffer_reads, &buffer_writes);

        // 拓扑排序
        let execution_order = dep_graph.topological_sort().unwrap_or_else(|cycle| {
            let cycle_names: Vec<_> = cycle.iter().map(|&i| &self.passes[i].name).collect();
            panic!("RenderGraph: Cycle detected involving passes: {:?}", cycle_names);
        });

        // 计算每个 Pass 的 barriers
        let barriers = self.compute_barriers(&execution_order);

        CompiledGraph { resources: self.resources, passes: self.passes, execution_order, barriers, dep_graph }
    }

    /// 计算每个 Pass 需要的 barriers
    fn compute_barriers(&self, execution_order: &[usize]) -> Vec<PassBarriers> {
        let mut barriers = vec![PassBarriers::new(); self.passes.len()];

        // 跟踪每个资源的当前状态 (使用 SecondaryMap)
        let mut image_states: SecondaryMap<RgImageHandle, ImageState> = SecondaryMap::new();
        let mut buffer_states: SecondaryMap<RgBufferHandle, BufferState> = SecondaryMap::new();

        // 初始化状态
        for (handle, res) in self.resources.iter_images() {
            image_states.insert(handle, res.current_state);
        }
        for (handle, res) in self.resources.iter_buffers() {
            buffer_states.insert(handle, res.current_state);
        }

        for &pass_idx in execution_order {
            let pass = &self.passes[pass_idx];
            let pass_barriers = &mut barriers[pass_idx];

            // 处理图像读取
            for read in &pass.image_reads {
                if let Some(&current) = image_states.get(read.handle) {
                    let required = read.state;
                    pass_barriers.add_image_barrier(ImageBarrierDesc::new(read.handle, current, required));
                    // 读取可能需要 layout 转换
                    if current.layout != required.layout {
                        image_states.insert(read.handle, required);
                    }
                }
            }

            // 处理图像写入
            for write in &pass.image_writes {
                if let Some(&current) = image_states.get(write.handle) {
                    let required = write.state;
                    pass_barriers.add_image_barrier(ImageBarrierDesc::new(write.handle, current, required));
                    // 写入更新状态
                    image_states.insert(write.handle, required);
                }
            }

            // 处理缓冲区读取
            for read in &pass.buffer_reads {
                if let Some(&current) = buffer_states.get(read.handle) {
                    let required = read.state;
                    pass_barriers.add_buffer_barrier(BufferBarrierDesc::new(read.handle, current, required));
                }
            }

            // 处理缓冲区写入
            for write in &pass.buffer_writes {
                if let Some(&current) = buffer_states.get(write.handle) {
                    let required = write.state;
                    pass_barriers.add_buffer_barrier(BufferBarrierDesc::new(write.handle, current, required));
                    buffer_states.insert(write.handle, required);
                }
            }
        }

        barriers
    }
}

/// 编译后的渲染图
///
/// 包含执行顺序、预计算的 barriers，可以多次执行。
pub struct CompiledGraph {
    /// 资源注册表
    resources: ResourceRegistry,
    /// Pass 节点列表
    passes: Vec<PassNode>,
    /// 执行顺序（拓扑排序后）
    execution_order: Vec<usize>,
    /// 每个 Pass 的 barriers（按 pass 索引）
    barriers: Vec<PassBarriers>,
    /// 依赖图（用于调试）
    #[allow(dead_code)]
    dep_graph: DependencyGraph,
}

impl CompiledGraph {
    /// 获取执行顺序
    pub fn execution_order(&self) -> &[usize] {
        &self.execution_order
    }

    /// 获取 Pass 数量
    pub fn pass_count(&self) -> usize {
        self.passes.len()
    }

    /// 获取 Pass 名称
    pub fn pass_name(&self, index: usize) -> &str {
        &self.passes[index].name
    }

    /// 执行渲染图
    ///
    /// # 参数
    /// - `cmd`: 命令缓冲区（已经 begin）
    /// - `resource_manager`: 资源管理器（用于获取物理资源）
    pub fn execute(&self, cmd: &GfxCommandBuffer, resource_manager: &GfxResourceManager) {
        // 构建物理资源查询表（使用 SecondaryMap）
        let mut image_handles: SecondaryMap<RgImageHandle, (GfxImageHandle, GfxImageViewHandle)> =
            SecondaryMap::new();
        let mut buffer_handles: SecondaryMap<RgBufferHandle, GfxBufferHandle> = SecondaryMap::new();

        for (handle, res) in self.resources.iter_images() {
            if let Some(img) = res.physical_handle() {
                let view = res.physical_view_handle().unwrap_or_default();
                image_handles.insert(handle, (img, view));
            }
        }

        for (handle, res) in self.resources.iter_buffers() {
            if let Some(buf) = res.physical_handle() {
                buffer_handles.insert(handle, buf);
            }
        }

        // 按顺序执行 Pass
        for &pass_idx in &self.execution_order {
            let pass = &self.passes[pass_idx];
            let pass_barriers = &self.barriers[pass_idx];

            // 插入 barriers
            if pass_barriers.has_barriers() {
                self.record_barriers(cmd, pass_barriers, resource_manager);
            }

            // 开始 Pass debug label
            cmd.begin_label(&pass.name, truvis_gfx::basic::color::LabelColor::COLOR_PASS);

            // 执行 Pass
            let ctx = PassContext { cmd, image_handles: &image_handles, buffer_handles: &buffer_handles };
            pass.executor.execute(&ctx);

            // 结束 Pass debug label
            cmd.end_label();
        }
    }

    /// 录制 barriers
    fn record_barriers(
        &self,
        cmd: &GfxCommandBuffer,
        pass_barriers: &PassBarriers,
        resource_manager: &GfxResourceManager,
    ) {
        use truvis_gfx::commands::barrier::GfxImageBarrier;

        let image_barriers: Vec<GfxImageBarrier> = pass_barriers
            .image_barriers
            .iter()
            .filter_map(|desc| {
                let res = self.resources.get_image(desc.handle)?;
                let phys_handle = res.physical_handle()?;
                let image = resource_manager.get_image(phys_handle)?;
                Some(desc.to_gfx_barrier(image.handle()))
            })
            .collect();

        if !image_barriers.is_empty() {
            cmd.image_memory_barrier(vk::DependencyFlags::empty(), &image_barriers);
        }

        // 缓冲区 barriers（类似处理）
        // TODO: 实现缓冲区 barrier 录制
    }

    /// 打印执行计划（用于调试）
    pub fn print_execution_plan(&self) {
        log::info!("=== RenderGraph Execution Plan ===");
        for (order, &pass_idx) in self.execution_order.iter().enumerate() {
            let pass = &self.passes[pass_idx];
            let barriers = &self.barriers[pass_idx];
            log::info!(
                "[{}] {} - {} image barriers, {} buffer barriers",
                order,
                pass.name,
                barriers.image_barrier_count(),
                barriers.buffer_barrier_count()
            );
        }
        log::info!("==================================");
    }
}
