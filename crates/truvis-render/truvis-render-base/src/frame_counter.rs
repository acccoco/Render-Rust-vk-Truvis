use crate::pipeline_settings::FrameLabel;

pub struct FrameCounter {
    /// 当前的帧序号，一直累加
    pub frame_id: usize,
    pub frame_limit: f32,
}
impl FrameCounter {
    const FIF_COUNT: usize = 3;
    #[inline]
    pub const fn fif_count() -> usize {
        Self::FIF_COUNT
    }
    #[inline]
    pub const fn frame_labes() -> [FrameLabel; Self::FIF_COUNT] {
        [FrameLabel::A, FrameLabel::B, FrameLabel::C]
    }
    #[inline]
    pub fn frame_label(&self) -> FrameLabel {
        FrameLabel::from_usize(self.frame_id % Self::fif_count())
    }
    #[inline]
    pub fn frame_name(&self) -> String {
        format!("[F{}{}]", self.frame_id, self.frame_label())
    }
}
