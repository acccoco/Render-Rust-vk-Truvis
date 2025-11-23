use std::cell::{Cell, Ref, RefCell, RefMut};

use ash::vk;
use truvis_asset::asset_hub::AssetHub;
use truvis_gfx::commands::semaphore::GfxSemaphore;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::resources::special_buffers::structured_buffer::GfxStructuredBuffer;
use truvis_shader_binding::shader;

use crate::pipeline_settings::{AccumData, DefaultRendererSettings, FrameLabel, FrameSettings, PipelineSettings};
use crate::platform::timer::Timer;
use crate::resources::fif_buffer::FifBuffers;
use crate::subsystems::bindless_manager::BindlessManager;
use crate::subsystems::cmd_allocator::CmdAllocator;
use crate::subsystems::gpu_scene::GpuScene;
use crate::subsystems::scene_manager::SceneManager;
use crate::subsystems::stage_buffer_manager::StageBufferManager;

/// 帧上下文单例
///
/// 管理渲染所需的所有帧级资源和子系统，包括命令分配器、Bindless 管理器、GPU 场景等。
/// 采用单例模式简化参数传递，通过 `RefCell` 实现内部可变性。
///
/// # Frames in Flight
/// - 固定 3 帧并行 (A/B/C)
/// - Timeline Semaphore 同步 GPU 进度
/// - 每帧独立的命令缓冲池、描述符池、Render Target
///
/// # 使用示例
/// ```ignore
/// let cmd = FrameContext::cmd_allocator_mut().alloc_command_buffer("my-pass");
/// let frame_label = FrameContext::get().frame_label();
/// ```
///
/// # 注意
/// 避免同时持有多个 `RefCell` 引用，否则会 panic：
/// ```ignore
/// // ❌ 错误
/// let cmd = FrameContext::cmd_allocator_mut();
/// let bindless = FrameContext::bindless_mgr_mut(); // panic!
///
/// // ✅ 正确
/// { let cmd = FrameContext::cmd_allocator_mut(); /* ... */ }
/// { let bindless = FrameContext::bindless_mgr_mut(); /* ... */ }
/// ```
pub struct FrameContext {
    pub upload_buffer_manager: RefCell<StageBufferManager>,
    pub bindless_manager: RefCell<BindlessManager>,
    pub cmd_allocator: RefCell<CmdAllocator>,
    pub gpu_scene: RefCell<GpuScene>,
    pub scene_manager: RefCell<SceneManager>,
    pub asset_hub: RefCell<AssetHub>,

    pub per_frame_data_buffers: Vec<GfxStructuredBuffer<shader::PerFrameData>>,

    pub fif_buffers: RefCell<FifBuffers>,

    pub timer: RefCell<Timer>,
    pub accum_data: Cell<AccumData>,

    /// 当前的帧序号，一直累加，初始序号是 1
    frame_id: Cell<usize>,
    /// 当前处在 in-flight 的第几帧：A, B, C
    frame_label: Cell<FrameLabel>,
    fif_count: usize,
    pub frame_limit: f32,

    frame_settings: Cell<FrameSettings>,

    pipeline_settings: Cell<PipelineSettings>,

    /// fif 相关的 timeline semaphore，value 就等于 frame_id
    pub fif_timeline_semaphore: GfxSemaphore,
}

/// 内部的对象生命周期是一致的，因此非常适合使用单例
///
/// - 可以极大降低传递参数的复杂度
/// - 可以不被 Rust 的借用检查器束缚
static mut FRAME_CONTEXT: Option<FrameContext> = None;

// init & destroy
impl FrameContext {
    fn new() -> Self {
        // 初始值应该是 1，因为 timeline semaphore 初始值是 0
        let init_frame_id = 1;
        let fif_count = 3;

        let fif_timeline_semaphore = GfxSemaphore::new_timeline(0, "render-timeline");

        let upload_buffer_manager = RefCell::new(StageBufferManager::new(fif_count));
        let bindless_manager = RefCell::new(BindlessManager::new(fif_count));
        let cmd_allocator = RefCell::new(CmdAllocator::new(fif_count));
        let gpu_scene = RefCell::new(GpuScene::new(fif_count));
        let scene_manager = RefCell::new(SceneManager::new());
        let asset_hub = RefCell::new(AssetHub::new());

        let frame_settings = FrameSettings {
            color_format: vk::Format::R32G32B32A32_SFLOAT,
            depth_format: Self::get_depth_format(),
            frame_extent: vk::Extent2D {
                width: 400,
                height: 400,
            },
        };

        let fif_buffers = FifBuffers::new(&frame_settings, &mut bindless_manager.borrow_mut(), fif_count);
        let per_frame_data_buffers = (0..fif_count)
            .map(|idx| GfxStructuredBuffer::<shader::PerFrameData>::new_ubo(1, format!("per-frame-data-buffer-{idx}")))
            .collect();

        Self {
            frame_id: Cell::new(init_frame_id),
            frame_label: Cell::new(FrameLabel::from_usize(init_frame_id)),
            fif_count,
            frame_limit: 59.9,
            fif_timeline_semaphore,

            timer: RefCell::new(Timer::default()),
            accum_data: Cell::new(AccumData::default()),

            fif_buffers: RefCell::new(fif_buffers),
            per_frame_data_buffers,

            frame_settings: Cell::new(frame_settings),
            pipeline_settings: Cell::new(PipelineSettings::default()),

            upload_buffer_manager,
            bindless_manager,
            cmd_allocator,
            gpu_scene,
            scene_manager,
            asset_hub,
        }
    }

    #[inline]
    pub fn get() -> &'static FrameContext {
        unsafe {
            // 使用 addr_of! 避免直接对 static mut 创建引用，编译器不允许这种行为
            let ptr = std::ptr::addr_of!(FRAME_CONTEXT);
            (*ptr).as_ref().expect("FrameContext not initialized. Call FrameContext::init() first.")
        }
    }

    pub fn init() {
        unsafe {
            // 使用 addr_of! 避免直接对 static mut 创建引用，编译器不允许这种行为
            let ptr = std::ptr::addr_of_mut!(FRAME_CONTEXT);
            assert!((*ptr).is_none(), "FrameContext already initialized");
            *ptr = Some(Self::new());
        }
    }

    pub fn destroy() {
        unsafe {
            // 使用 addr_of_mut! 避免直接对 static mut 创建可变引用
            let ptr = std::ptr::addr_of_mut!(FRAME_CONTEXT);
            let context = (*ptr).take().expect("FrameContext not initialized");

            context.fif_timeline_semaphore.destroy();

            drop(context.upload_buffer_manager);
            drop(context.cmd_allocator);
            drop(context.bindless_manager);
            drop(context.gpu_scene);
            drop(context.scene_manager);
            drop(context.asset_hub);
        }
    }

    /// 根据 vulkan 实例和显卡，获取合适的深度格式
    fn get_depth_format() -> vk::Format {
        Gfx::get()
            .find_supported_format(
                DefaultRendererSettings::DEPTH_FORMAT_CANDIDATES,
                vk::ImageTiling::OPTIMAL,
                vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
            )
            .first()
            .copied()
            .unwrap_or(vk::Format::UNDEFINED)
    }
}

// getter
impl FrameContext {
    #[inline]
    pub fn frame_label(&self) -> FrameLabel {
        self.frame_label.get()
    }

    #[inline]
    pub fn frame_id(&self) -> usize {
        self.frame_id.get()
    }

    #[inline]
    pub fn frame_name(&self) -> String {
        format!("[F{}{}]", self.frame_id.get(), self.frame_label.get())
    }

    #[inline]
    pub fn fif_count(&self) -> usize {
        self.fif_count
    }

    #[inline]
    pub fn bindless_manager_mut() -> RefMut<'static, BindlessManager> {
        let context = Self::get();
        context.bindless_manager.borrow_mut()
    }
    #[inline]
    pub fn bindless_manager() -> Ref<'static, BindlessManager> {
        let context = Self::get();
        context.bindless_manager.borrow()
    }

    #[inline]
    pub fn cmd_allocator_mut() -> RefMut<'static, CmdAllocator> {
        let context = Self::get();
        context.cmd_allocator.borrow_mut()
    }

    #[inline]
    pub fn gpu_scene_mut() -> RefMut<'static, GpuScene> {
        let context = Self::get();
        context.gpu_scene.borrow_mut()
    }
    #[inline]
    pub fn gpu_scene() -> Ref<'static, GpuScene> {
        let context = Self::get();
        context.gpu_scene.borrow()
    }

    #[inline]
    pub fn scene_manager_mut() -> RefMut<'static, SceneManager> {
        let context = Self::get();
        context.scene_manager.borrow_mut()
    }
    #[inline]
    pub fn scene_manager() -> Ref<'static, SceneManager> {
        let context = Self::get();
        context.scene_manager.borrow()
    }

    #[inline]
    pub fn asset_hub_mut() -> RefMut<'static, AssetHub> {
        let context = Self::get();
        context.asset_hub.borrow_mut()
    }
    #[inline]
    pub fn asset_hub() -> Ref<'static, AssetHub> {
        let context = Self::get();
        context.asset_hub.borrow()
    }

    #[inline]
    pub fn stage_buffer_manager() -> RefMut<'static, StageBufferManager> {
        let context = Self::get();
        context.upload_buffer_manager.borrow_mut()
    }

    #[inline]
    pub fn frame_settings(&self) -> FrameSettings {
        self.frame_settings.get()
    }

    #[inline]
    pub fn set_pipeline_settings(&self, settings: PipelineSettings) {
        self.pipeline_settings.set(settings);
    }

    #[inline]
    pub fn pipeline_settings(&self) -> PipelineSettings {
        self.pipeline_settings.get()
    }
}

// tools
impl FrameContext {
    pub fn end_frame(&self) {
        let new_frame_id = self.frame_id.get() + 1;
        let new_frame_label = FrameLabel::from_usize(new_frame_id % self.fif_count);
        self.frame_id.set(new_frame_id);
        self.frame_label.set(new_frame_label);
    }

    pub fn set_frame_extent(&self, extent: vk::Extent2D) {
        let mut settings = self.frame_settings.get();
        settings.frame_extent = extent;
        self.frame_settings.set(settings);
    }
}
