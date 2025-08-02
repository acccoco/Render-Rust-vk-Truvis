use crate::pipeline_settings::{FrameLabel, FrameSettings};
use std::cell::Cell;

pub struct FrameController {
    /// 当前处在 in-flight 的第几帧：A, B, C
    frame_label: Cell<FrameLabel>,
    /// 当前的帧序号，一直累加，初始序号是 1
    frame_id: Cell<usize>,
    fif_count: usize,
}

// ctor
impl FrameController {
    pub fn new(frame_settings: &FrameSettings) -> Self {
        // 初始值应该是 1，因为 timeline semaphore 初始值是 0
        let init_frame_id = 1;
        Self {
            frame_id: Cell::new(init_frame_id),
            frame_label: Cell::new(FrameLabel::from_usize(init_frame_id)),
            fif_count: frame_settings.fif_num,
        }
    }
}

// getter
impl FrameController {
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
}

// phase methods
impl FrameController {
    pub fn end_frame(&self) {
        let new_frame_id = self.frame_id.get() + 1;
        let new_frame_label = FrameLabel::from_usize(new_frame_id % self.fif_count);
        self.frame_id.set(new_frame_id);
        self.frame_label.set(new_frame_label);
    }
}
