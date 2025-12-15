use crate::pipeline_settings::FrameLabel;

pub struct FrameCounter {
    /// 当前的帧序号，一直累加
    pub frame_id: usize,
    pub frame_limit: f32,
}
impl FrameCounter {
    #[inline]
    pub const fn fif_count() -> usize {
        3
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
