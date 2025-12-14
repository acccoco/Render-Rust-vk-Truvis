use std::cell::Cell;

use crate::pipeline_settings::{DefaultRendererSettings, FrameLabel, FrameSettings, PipelineSettings};
use ash::vk;
use truvis_gfx::gfx::Gfx;

pub struct FrameContext {
    /// 当前的帧序号，一直累加，初始序号是 1
    frame_id: Cell<usize>,
    /// 当前处在 in-flight 的第几帧：A, B, C
    frame_label: Cell<FrameLabel>,
    frame_limit: Cell<f32>,

    frame_settings: Cell<FrameSettings>,

    pipeline_settings: Cell<PipelineSettings>,
}

/// 内部的对象生命周期是一致的，因此非常适合使用单例
///
/// - 可以极大降低传递参数的复杂度
/// - 可以不被 Rust 的借用检查器束缚
static mut FRAME_CONTEXT: Option<FrameContext> = None;

// new & init
impl FrameContext {
    fn new() -> Self {
        // 初始值应该是 1，因为 timeline semaphore 初始值是 0
        let init_frame_id = 1;

        let frame_settings = FrameSettings {
            color_format: vk::Format::R32G32B32A32_SFLOAT,
            depth_format: Self::get_depth_format(),
            frame_extent: vk::Extent2D {
                width: 400,
                height: 400,
            },
        };

        Self {
            frame_id: Cell::new(init_frame_id),
            frame_label: Cell::new(FrameLabel::from_usize(init_frame_id)),
            frame_limit: Cell::new(60.0),

            frame_settings: Cell::new(frame_settings),
            pipeline_settings: Cell::new(PipelineSettings::default()),
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
// destroy
impl FrameContext {
    pub fn destroy() {
        unsafe {
            // 使用 addr_of_mut! 避免直接对 static mut 创建可变引用
            let ptr = std::ptr::addr_of_mut!(FRAME_CONTEXT);
            (*ptr).take().expect("FrameContext not initialized");
        }
    }
}
// getter & setter
impl FrameContext {
    #[inline]
    pub fn frame_label(&self) -> FrameLabel {
        self.frame_label.get()
    }
    #[inline]
    pub fn frame_label2() -> FrameLabel {
        FrameLabel::from_usize(Self::frame_id() % Self::fif_count())
    }
    #[inline]
    pub fn fif_count() -> usize {
        3
    }
    #[inline]
    pub fn frame_id() -> usize {
        Self::get().frame_id.get()
    }
    #[inline]
    pub fn set_frame_id(new_frame_id: usize) {
        Self::get().frame_id.set(new_frame_id);
    }
    #[inline]
    pub fn frame_limit() -> f32 {
        Self::get().frame_limit.get()
    }

    #[inline]
    pub fn frame_name(&self) -> String {
        format!("[F{}{}]", self.frame_id.get(), self.frame_label.get())
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
    pub fn set_frame_extent(&self, extent: vk::Extent2D) {
        let mut settings = self.frame_settings.get();
        settings.frame_extent = extent;
        self.frame_settings.set(settings);
    }
}
