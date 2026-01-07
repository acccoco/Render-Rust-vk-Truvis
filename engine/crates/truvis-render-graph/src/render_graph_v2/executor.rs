//! RenderGraph 构建器和执行器
//!
//! 提供 `RenderGraphBuilder` 用于构建渲染图，
//! `CompiledGraph` 用于缓存编译结果并执行渲染。

use std::collections::HashMap;

use super::barrier::{BufferBarrierDesc, PassBarriers, RgImageBarrierDesc};
use super::graph::DependencyGraph;
use super::pass::{RgLambdaPassWrapper, RgPass, RgPassBuilder, RgPassContext, RgPassNode, RgPassWrapper};
use super::resource_handle::{RgBufferHandle, RgImageHandle};
use super::resource_manager::RgResourceManager;
use super::resource_state::{RgBufferState, RgImageState};
use crate::render_graph_v2::export_info::RgExportInfo;
use crate::render_graph_v2::semaphore_info::RgSemaphoreInfo;
use crate::render_graph_v2::{RgBufferDesc, RgBufferResource, RgImageDesc, RgImageResource};
use ash::vk;
use itertools::Itertools;
use slotmap::SecondaryMap;
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_gfx::commands::submit_info::GfxSubmitInfo;
use truvis_render_interface::gfx_resource_manager::GfxResourceManager;
use truvis_render_interface::handles::{GfxBufferHandle, GfxImageHandle, GfxImageViewHandle};

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
///
/// # 生命周期
///
/// `'a` 是 Pass 可以借用的外部资源的生命周期。
/// 这允许 Pass 直接引用外部的 pipeline、geometry 等资源，
pub struct RenderGraphBuilder<'a> {
    /// 资源注册表
    resources: RgResourceManager,

    /// Pass 节点列表（按添加顺序）
    passes: Vec<RgPassNode<'a>>,

    /// 导出资源信息：指定资源的最终状态和可选的 signal semaphore
    export_images: HashMap<RgImageHandle, RgExportInfo>,
}

impl Default for RenderGraphBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> RenderGraphBuilder<'a> {
    /// 创建新的 RenderGraph 构建器
    pub fn new() -> Self {
        Self {
            resources: RgResourceManager::new(),
            passes: Vec::new(),
            export_images: HashMap::new(),
        }
    }

    /// 导入外部图像资源
    ///
    /// # 参数
    /// - `name`: 资源调试名称
    /// - `image_handle`: 物理图像句柄（来自 GfxResourceManager）
    /// - `view_handle`: 可选的图像视图句柄
    /// - `format`: 图像格式（用于推断 barrier aspect）
    /// - `initial_state`: 图像的初始状态
    /// - `wait_semaphore`: 可选的外部 semaphore 等待（在首次使用此资源前等待）
    ///
    /// # 返回
    /// RenderGraph 内部的图像句柄
    pub fn import_image(
        &mut self,
        name: impl Into<String>,
        image_handle: GfxImageHandle,
        view_handle: Option<GfxImageViewHandle>,
        format: vk::Format,
        initial_state: RgImageState,
        wait_semaphore: Option<RgSemaphoreInfo>,
    ) -> RgImageHandle {
        self.resources.register_image(RgImageResource::imported(
            name,
            image_handle,
            view_handle,
            format,
            initial_state,
            wait_semaphore,
        ))
    }

    /// 导出图像资源
    ///
    /// 声明资源在渲染图执行完成后的最终状态，并可选地发出 semaphore 信号。
    /// 这会在图的末尾插入必要的 barrier 将资源转换到指定的 final_state。
    ///
    /// # 参数
    /// - `handle`: 要导出的图像句柄
    /// - `final_state`: 资源的最终状态（layout, access, stage）
    /// - `signal_semaphore`: 可选的 semaphore 信号
    ///
    /// # 返回
    /// 返回 `&mut Self` 以支持链式调用
    pub fn export_image(
        &mut self,
        handle: RgImageHandle,
        final_state: RgImageState,
        signal_semaphore: Option<RgSemaphoreInfo>,
    ) -> &mut Self {
        self.export_images.insert(
            handle,
            RgExportInfo {
                final_state,
                signal_semaphore,
            },
        );
        self
    }

    /// 导入外部缓冲区资源
    pub fn import_buffer(
        &mut self,
        name: impl Into<String>,
        buffer_handle: GfxBufferHandle,
        initial_state: RgBufferState,
    ) -> RgBufferHandle {
        self.resources.register_buffer(RgBufferResource::imported(name, buffer_handle, initial_state))
    }

    pub fn create_image(&mut self, name: impl Into<String>, desc: RgImageDesc) -> RgImageHandle {
        self.resources.register_image(RgImageResource::transient(name, desc))
    }

    pub fn create_buffer(&mut self, name: impl Into<String>, desc: RgBufferDesc) -> RgBufferHandle {
        self.resources.register_buffer(RgBufferResource::transient(name, desc))
    }

    /// 添加 Pass
    ///
    /// # 参数
    /// - `name`: Pass 名称（用于调试和性能分析）
    /// - `pass`: 实现了 `RgPass` trait 的 Pass 对象
    ///
    /// # 返回
    /// 返回 `&mut Self` 以支持链式调用
    pub fn add_pass<P: RgPass + 'a>(&mut self, name: impl Into<String>, mut pass: P) -> &mut Self {
        let name = name.into();

        // 创建 PassBuilder 供 Pass 声明依赖
        let mut builder = RgPassBuilder {
            name: name.clone(),
            image_reads: Vec::new(),
            image_writes: Vec::new(),
            buffer_reads: Vec::new(),
            buffer_writes: Vec::new(),
        };

        // 调用 Pass 的 setup 方法
        pass.setup(&mut builder);

        // 创建 PassNode
        let node = RgPassNode {
            name,
            image_reads: builder.image_reads,
            image_writes: builder.image_writes,
            buffer_reads: builder.buffer_reads,
            buffer_writes: builder.buffer_writes,
            executor: Box::new(RgPassWrapper { pass }),
        };

        self.passes.push(node);
        self
    }

    /// 通过闭包添加 Pass
    ///
    /// 这是一个便捷方法，允许使用闭包快速定义 Pass，无需创建额外的结构体。
    ///
    /// # 参数
    /// - `name`: Pass 名称（用于调试和性能分析）
    /// - `setup_fn`: setup 闭包，用于声明资源依赖
    /// - `execute_fn`: execute 闘包，用于执行渲染逻辑
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let input_image = builder.import_image("input", ...);
    /// let output_image = builder.import_image("output", ...);
    ///
    /// builder.add_pass_lambda(
    ///     "my-compute-pass",
    ///     |b| {
    ///         b.read_image(input_image, RgImageState::SHADER_READ_COMPUTE);
    ///         b.write_image(output_image, RgImageState::STORAGE_WRITE_COMPUTE);
    ///     },
    ///     move |ctx| {
    ///         let cmd = ctx.cmd;
    ///         // 绑定 pipeline, 设置描述符, dispatch...
    ///     },
    /// );
    /// ```
    ///
    /// # 返回
    /// 返回 `&mut Self` 以支持链式调用
    pub fn add_pass_lambda<S, E>(&mut self, name: impl Into<String>, setup_fn: S, execute_fn: E) -> &mut Self
    where
        S: FnMut(&mut RgPassBuilder) + 'a,
        E: Fn(&RgPassContext<'_>) + 'a,
    {
        let pass = RgLambdaPassWrapper::new(setup_fn, execute_fn);
        self.add_pass(name, pass)
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
    pub fn compile(self) -> CompiledGraph<'a> {
        let _span = tracy_client::span!("RenderGraphBuilder::compile");

        let pass_count = self.passes.len();

        // 收集每个 Pass 的读写资源句柄
        let image_reads = self.passes.iter().map(|p| p.image_reads.iter().map(|s| s.0).collect_vec()).collect_vec();
        let image_writes = self.passes.iter().map(|p| p.image_writes.iter().map(|s| s.0).collect_vec()).collect_vec();
        let buffer_reads = self.passes.iter().map(|p| p.buffer_reads.iter().map(|s| s.0).collect_vec()).collect_vec();
        let buffer_writes = self.passes.iter().map(|p| p.buffer_writes.iter().map(|s| s.0).collect_vec()).collect_vec();

        // 依赖分析
        let dep_graph =
            DependencyGraph::analyze(pass_count, &image_reads, &image_writes, &buffer_reads, &buffer_writes);

        // 拓扑排序
        let execution_order = dep_graph.topological_sort().unwrap_or_else(|cycle| {
            let cycle_names: Vec<_> = cycle.iter().map(|&i| &self.passes[i].name).collect();
            panic!("RenderGraph: Cycle detected involving passes: {:?}", cycle_names);
        });

        // 计算每个 Pass 的 barriers（同时返回最终的资源状态用于计算 epilogue barriers）
        let (barriers, final_image_states) = self.compute_barriers(&execution_order);

        // 收集外部 wait semaphores（来自导入资源）
        let wait_semaphores = self.resources.iter_images().filter_map(|(_, res)| res.wait_semaphore()).collect_vec();

        // 收集外部 signal semaphores（来自导出资源）
        let signal_semaphores = self.export_images.values().filter_map(|info| info.signal_semaphore).collect_vec();

        // 计算 epilogue barriers：将导出资源从最后使用状态转换到 final_state
        let epilogue_barriers = self.compute_epilogue_barriers(&final_image_states);

        CompiledGraph {
            resources: self.resources,
            passes: self.passes,
            execution_order,
            barriers,
            epilogue_barriers,
            dep_graph,
            wait_semaphores,
            signal_semaphores,
        }
    }

    /// 计算 epilogue barriers
    ///
    /// 将导出资源从最后使用状态转换到声明的 final_state
    fn compute_epilogue_barriers(
        &self,
        final_image_states: &SecondaryMap<RgImageHandle, RgImageState>,
    ) -> PassBarriers {
        let mut epilogue = PassBarriers::new();

        for (&handle, export_info) in &self.export_images {
            if let Some(&current_state) = final_image_states.get(handle) {
                let final_state = export_info.final_state;

                // 只有状态不同时才需要 barrier
                if current_state != final_state
                    && let Some(res) = self.resources.get_image(handle)
                {
                    let aspect = res.infer_aspect();
                    epilogue.add_image_barrier(
                        RgImageBarrierDesc::new(handle, current_state, final_state).with_aspect(aspect),
                    );
                }
            }
        }

        epilogue
    }

    /// 计算每个 Pass 需要的 barriers
    ///
    /// 模拟 pass 的执行顺序，跟踪资源的状态变化，生成必要的 barriers
    ///
    /// # 返回
    /// - barriers: 每个 Pass 的 barriers
    /// - final_image_states: 所有图像资源的最终状态（用于计算 epilogue barriers）
    fn compute_barriers(
        &self,
        execution_order: &[usize],
    ) -> (Vec<PassBarriers>, SecondaryMap<RgImageHandle, RgImageState>) {
        let mut barriers = vec![PassBarriers::new(); self.passes.len()];

        // 跟踪每个资源的当前状态 (使用 SecondaryMap)
        let mut image_states: SecondaryMap<RgImageHandle, RgImageState> = SecondaryMap::new();
        let mut buffer_states: SecondaryMap<RgBufferHandle, RgBufferState> = SecondaryMap::new();

        // 初始化状态
        for (handle, res) in self.resources.iter_images() {
            image_states.insert(handle, res.current_state);
        }
        for (handle, res) in self.resources.iter_buffers() {
            buffer_states.insert(handle, res.current_state);
        }

        let get_image_aspect = |handle: RgImageHandle| {
            let image_resource = self.resources.get_image(handle).unwrap();
            image_resource.infer_aspect()
        };

        for &pass_idx in execution_order {
            let pass = &self.passes[pass_idx];
            let pass_barriers = &mut barriers[pass_idx];

            // 收集此 Pass 中每个图像的所有使用
            // Key: handle, Value: (is_write, required_state)
            let mut image_usage: HashMap<RgImageHandle, (bool, RgImageState)> = HashMap::new();

            // 处理读取声明
            for (handle, state) in &pass.image_reads {
                image_usage.entry(*handle).or_insert((false, *state));
            }

            // 处理写入声明（写入会覆盖读取的目标状态）
            for (handle, state) in &pass.image_writes {
                image_usage.insert(*handle, (true, *state));
            }

            // 为每个使用的图像生成 barrier
            for (handle, (is_write, required_state)) in image_usage {
                if let Some(&crt_state) = image_states.get(handle) {
                    let aspect = get_image_aspect(handle);

                    pass_barriers.add_image_barrier(
                        RgImageBarrierDesc::new(handle, crt_state, required_state).with_aspect(aspect),
                    );

                    // 如果是写入或 layout 改变，更新状态
                    if is_write || crt_state.layout != required_state.layout {
                        image_states.insert(handle, required_state);
                    }
                }
            }

            // 缓冲区使用类似逻辑
            let mut buffer_usage: HashMap<RgBufferHandle, (bool, RgBufferState)> = HashMap::new();

            for (handle, state) in &pass.buffer_reads {
                buffer_usage.entry(*handle).or_insert((false, *state));
            }

            for (handle, state) in &pass.buffer_writes {
                buffer_usage.insert(*handle, (true, *state));
            }

            for (handle, (is_write, required)) in buffer_usage {
                if let Some(&current) = buffer_states.get(handle) {
                    pass_barriers.add_buffer_barrier(BufferBarrierDesc::new(handle, current, required));

                    if is_write {
                        buffer_states.insert(handle, required);
                    }
                }
            }
        }

        (barriers, image_states)
    }
}

/// 编译后的渲染图
///
/// 包含执行顺序、预计算的 barriers，可以多次执行。
///
/// # 生命周期
///
/// `'a` 是 Pass 借用的外部资源的生命周期。
/// CompiledGraph 的生命周期不能超过这些外部资源。
pub struct CompiledGraph<'a> {
    /// 资源注册表
    resources: RgResourceManager,
    /// Pass 节点列表
    passes: Vec<RgPassNode<'a>>,
    /// 执行顺序（拓扑排序后）
    execution_order: Vec<usize>,
    /// 每个 Pass 的 barriers（按 pass 索引）
    barriers: Vec<PassBarriers>,
    /// 尾声 barriers：将导出资源转换到最终状态
    epilogue_barriers: PassBarriers,
    /// 依赖图（用于调试）
    #[allow(dead_code)]
    dep_graph: DependencyGraph,
    /// 收集的外部 wait semaphores（来自导入资源）
    wait_semaphores: Vec<RgSemaphoreInfo>,
    /// 收集的外部 signal semaphores（来自导出资源）
    signal_semaphores: Vec<RgSemaphoreInfo>,
}

impl CompiledGraph<'_> {
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
        let _span = tracy_client::span!("CompiledGraph::execute");

        // 构建物理资源查询表（使用 SecondaryMap）
        let mut image_handles: SecondaryMap<RgImageHandle, (GfxImageHandle, GfxImageViewHandle)> = SecondaryMap::new();
        let mut buffer_handles: SecondaryMap<RgBufferHandle, GfxBufferHandle> = SecondaryMap::new();

        for (image_handle, image_resource) in self.resources.iter_images() {
            if let Some(img) = image_resource.physical_handle() {
                let view = image_resource.physical_view_handle().unwrap_or_default();
                image_handles.insert(image_handle, (img, view));
            }
        }

        for (buffer_handle, buffer_resource) in self.resources.iter_buffers() {
            if let Some(buf) = buffer_resource.physical_handle() {
                buffer_handles.insert(buffer_handle, buf);
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
            let ctx = RgPassContext {
                cmd,
                resource_manager,
                image_handles: &image_handles,
                buffer_handles: &buffer_handles,
            };
            pass.executor.execute(&ctx);

            // 结束 Pass debug label
            cmd.end_label();
        }

        // 录制 epilogue barriers（将导出资源转换到最终状态）
        if self.epilogue_barriers.has_barriers() {
            cmd.begin_label("rg-epilogue", truvis_gfx::basic::color::LabelColor::COLOR_PASS);
            self.record_barriers(cmd, &self.epilogue_barriers, resource_manager);
            cmd.end_label();
        }
    }

    /// 构建包含外部同步信息的 SubmitInfo
    ///
    /// 返回的 `GfxSubmitInfo` 包含了从导入资源收集的 wait semaphores
    /// 和从导出资源收集的 signal semaphores。
    ///
    /// # 参数
    /// - `commands`: 要提交的命令缓冲区列表
    ///
    /// # 示例
    ///
    /// ```ignore
    /// cmd.begin(...);
    /// compiled_graph.execute(&cmd, resource_manager);
    /// cmd.end();
    ///
    /// let submit_info = compiled_graph.build_submit_info(&[cmd]);
    /// queue.submit(vec![submit_info], fence);
    /// ```
    pub fn build_submit_info(&self, commands: &[GfxCommandBuffer]) -> GfxSubmitInfo {
        let mut submit_info = GfxSubmitInfo::new(commands);

        // 添加 wait semaphores
        for wait in &self.wait_semaphores {
            submit_info = submit_info.wait_raw(wait.semaphore, wait.stage, wait.value);
        }

        // 添加 signal semaphores
        for signal in &self.signal_semaphores {
            submit_info = submit_info.signal_raw(signal.semaphore, signal.stage, signal.value);
        }

        submit_info
    }

    /// 获取 wait semaphores 列表（用于调试或手动构建 submit info）
    pub fn wait_semaphores(&self) -> &[RgSemaphoreInfo] {
        &self.wait_semaphores
    }

    /// 获取 signal semaphores 列表（用于调试或手动构建 submit info）
    pub fn signal_semaphores(&self) -> &[RgSemaphoreInfo] {
        &self.signal_semaphores
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
                // 跳过不需要的 barrier
                if !desc.needs_barrier() {
                    return None;
                }

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
}

// 调试方法
impl CompiledGraph<'_> {
    /// 打印执行计划（用于调试）
    ///
    /// 输出详细的调试信息，包括：
    /// - 每个 Pass 的执行顺序
    /// - 每个 Pass 的 image/buffer 读写信息（包含资源名称）
    /// - 每个 Pass 的 barrier 详细信息（layout 转换、目标资源名称）
    pub fn print_execution_plan(&self) {
        log::info!("╔══════════════════════════════════════════════════════════════════╗");
        log::info!("║              RenderGraph Execution Plan                          ║");
        log::info!("╠══════════════════════════════════════════════════════════════════╣");
        log::info!(
            "║ Total Passes: {}  |  Execution Order: [{}]",
            self.passes.len(),
            self.execution_order.iter().map(|i| self.passes[*i].name.as_str()).collect::<Vec<_>>().join(" → ")
        );
        log::info!("╚══════════════════════════════════════════════════════════════════╝");

        for (order, &pass_idx) in self.execution_order.iter().enumerate() {
            let pass = &self.passes[pass_idx];
            let barriers = &self.barriers[pass_idx];

            log::info!("");
            log::info!("┌─────────────────────────────────────────────────────────────────┐");
            log::info!("│ [{}/{}] Pass: \"{}\"", order + 1, self.execution_order.len(), pass.name);
            log::info!("├─────────────────────────────────────────────────────────────────┤");

            // 打印 Image 读取信息
            if !pass.image_reads.is_empty() {
                log::info!("│ Image Reads:");
                for (handle, state) in &pass.image_reads {
                    let name = self.resources.get_image(*handle).map(|r| r.name.as_str()).unwrap_or("<unknown>");
                    log::info!(
                        "│   📖 \"{}\" @ {:?} (stage: {}, access: {})",
                        name,
                        state.layout,
                        Self::format_pipeline_stage(state.stage),
                        Self::format_access_flags(state.access)
                    );
                }
            }

            // 打印 Image 写入信息
            if !pass.image_writes.is_empty() {
                log::info!("│ Image Writes:");
                for (handle, state) in &pass.image_writes {
                    let name = self.resources.get_image(*handle).map(|r| r.name.as_str()).unwrap_or("<unknown>");
                    log::info!(
                        "│   ✏️  \"{}\" @ {:?} (stage: {}, access: {})",
                        name,
                        state.layout,
                        Self::format_pipeline_stage(state.stage),
                        Self::format_access_flags(state.access)
                    );
                }
            }

            // 打印 Buffer 读取信息
            if !pass.buffer_reads.is_empty() {
                log::info!("│ Buffer Reads:");
                for (handle, state) in &pass.buffer_reads {
                    let name = self.resources.get_buffer(*handle).map(|r| r.name.as_str()).unwrap_or("<unknown>");
                    log::info!(
                        "│   📖 \"{}\" (stage: {}, access: {})",
                        name,
                        Self::format_pipeline_stage(state.stage),
                        Self::format_access_flags(state.access)
                    );
                }
            }

            // 打印 Buffer 写入信息
            if !pass.buffer_writes.is_empty() {
                log::info!("│ Buffer Writes:");
                for (handle, state) in &pass.buffer_writes {
                    let name = self.resources.get_buffer(*handle).map(|r| r.name.as_str()).unwrap_or("<unknown>");
                    log::info!(
                        "│   ✏️  \"{}\" (stage: {}, access: {})",
                        name,
                        Self::format_pipeline_stage(state.stage),
                        Self::format_access_flags(state.access)
                    );
                }
            }

            // 打印 Barrier 详细信息
            if barriers.has_barriers() {
                log::info!("├─────────────────────────────────────────────────────────────────┤");
                log::info!(
                    "│ Barriers: {} image, {} buffer",
                    barriers.image_barrier_count(),
                    barriers.buffer_barrier_count()
                );

                // Image Barriers
                for barrier in &barriers.image_barriers {
                    let name = self.resources.get_image(barrier.handle).map(|r| r.name.as_str()).unwrap_or("<unknown>");
                    let layout_change = if barrier.src_state.layout != barrier.dst_state.layout {
                        format!("{:?} → {:?}", barrier.src_state.layout, barrier.dst_state.layout)
                    } else {
                        format!("{:?} (no layout change)", barrier.src_state.layout)
                    };
                    log::info!("│   🔒 Image \"{}\":", name);
                    log::info!("│       Layout: {}", layout_change);
                    log::info!(
                        "│       Stage:  {} → {}",
                        Self::format_pipeline_stage(barrier.src_state.stage),
                        Self::format_pipeline_stage(barrier.dst_state.stage)
                    );
                    log::info!(
                        "│       Access: {} → {}",
                        Self::format_access_flags(barrier.src_state.access),
                        Self::format_access_flags(barrier.dst_state.access)
                    );
                    log::info!("│       Aspect: {:?}", barrier.aspect);
                }

                // Buffer Barriers
                for barrier in &barriers.buffer_barriers {
                    let name =
                        self.resources.get_buffer(barrier.handle).map(|r| r.name.as_str()).unwrap_or("<unknown>");
                    log::info!("│   🔒 Buffer \"{}\":", name);
                    log::info!(
                        "│       Stage:  {} → {}",
                        Self::format_pipeline_stage(barrier.src_state.stage),
                        Self::format_pipeline_stage(barrier.dst_state.stage)
                    );
                    log::info!(
                        "│       Access: {} → {}",
                        Self::format_access_flags(barrier.src_state.access),
                        Self::format_access_flags(barrier.dst_state.access)
                    );
                }
            } else {
                log::info!("│ No barriers required");
            }

            log::info!("└─────────────────────────────────────────────────────────────────┘");
        }

        log::info!("");
        log::info!("═══════════════════════ End of Execution Plan ═══════════════════════");
    }

    /// 格式化 PipelineStageFlags2 为可读字符串
    fn format_pipeline_stage(stage: vk::PipelineStageFlags2) -> String {
        macro_rules! check_stages {
            ($($flag:ident => $name:expr),* $(,)?) => {{
                let mut stages = Vec::new();
                $(if stage.contains(vk::PipelineStageFlags2::$flag) { stages.push($name); })*
                stages
            }};
        }

        let stages = check_stages![
            TOP_OF_PIPE => "TOP_OF_PIPE",
            BOTTOM_OF_PIPE => "BOTTOM_OF_PIPE",
            VERTEX_INPUT => "VERTEX_INPUT",
            VERTEX_SHADER => "VERTEX_SHADER",
            FRAGMENT_SHADER => "FRAGMENT_SHADER",
            COLOR_ATTACHMENT_OUTPUT => "COLOR_ATTACHMENT_OUTPUT",
            EARLY_FRAGMENT_TESTS => "EARLY_FRAGMENT_TESTS",
            LATE_FRAGMENT_TESTS => "LATE_FRAGMENT_TESTS",
            COMPUTE_SHADER => "COMPUTE_SHADER",
            TRANSFER => "TRANSFER",
            RAY_TRACING_SHADER_KHR => "RAY_TRACING",
            ACCELERATION_STRUCTURE_BUILD_KHR => "ACCEL_BUILD",
            ALL_GRAPHICS => "ALL_GRAPHICS",
            ALL_COMMANDS => "ALL_COMMANDS",
        ];

        if stages.is_empty() { format!("{:?}", stage) } else { stages.join(" | ") }
    }

    /// 格式化 AccessFlags2 为可读字符串
    fn format_access_flags(access: vk::AccessFlags2) -> String {
        if access == vk::AccessFlags2::NONE {
            return "NONE".to_string();
        }

        macro_rules! check_access {
            ($($flag:ident => $name:expr),* $(,)?) => {{
                let mut flags = Vec::new();
                $(if access.contains(vk::AccessFlags2::$flag) { flags.push($name); })*
                flags
            }};
        }

        let flags = check_access![
            INDIRECT_COMMAND_READ => "INDIRECT_CMD_READ",
            INDEX_READ => "INDEX_READ",
            VERTEX_ATTRIBUTE_READ => "VERTEX_ATTR_READ",
            UNIFORM_READ => "UNIFORM_READ",
            SHADER_SAMPLED_READ => "SAMPLED_READ",
            SHADER_STORAGE_READ => "STORAGE_READ",
            SHADER_STORAGE_WRITE => "STORAGE_WRITE",
            COLOR_ATTACHMENT_READ => "COLOR_READ",
            COLOR_ATTACHMENT_WRITE => "COLOR_WRITE",
            DEPTH_STENCIL_ATTACHMENT_READ => "DEPTH_READ",
            DEPTH_STENCIL_ATTACHMENT_WRITE => "DEPTH_WRITE",
            TRANSFER_READ => "TRANSFER_READ",
            TRANSFER_WRITE => "TRANSFER_WRITE",
            MEMORY_READ => "MEM_READ",
            MEMORY_WRITE => "MEM_WRITE",
            ACCELERATION_STRUCTURE_READ_KHR => "ACCEL_READ",
            ACCELERATION_STRUCTURE_WRITE_KHR => "ACCEL_WRITE",
        ];

        if flags.is_empty() { format!("{:?}", access) } else { flags.join(" | ") }
    }
}
