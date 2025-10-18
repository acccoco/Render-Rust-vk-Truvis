use crate::renderer::frame_controller::FrameController;
use itertools::Itertools;
use std::rc::Rc;
use truvis_rhi::resources::buffer::Buffer;
use truvis_rhi::resources::special_buffers::stage_buffer::StageBuffer;

pub struct UploadBufferManager {
    buffers: Vec<Vec<Buffer>>,

    frame_ctrl: Rc<FrameController>,
}

/// init & destroy
impl UploadBufferManager {
    pub fn new(frame_ctrl: Rc<FrameController>) -> Self {
        let buffers = (0..frame_ctrl.fif_count()).map(|_| Vec::new()).collect_vec();
        Self { buffers, frame_ctrl }
    }
}

/// tools
impl UploadBufferManager {
    pub fn alloc_buffer(&mut self, size: u64, debug_name: &str) -> &mut Buffer {
        let buffer = Buffer::new_stage_buffer(size, debug_name);
        self.buffers[*self.frame_ctrl.frame_label()].push(buffer);
        self.buffers[*self.frame_ctrl.frame_label()].last_mut().unwrap()
    }

    pub fn clear_frame_buffers(&mut self) {
        self.buffers[*self.frame_ctrl.frame_label()].clear();
    }
}

impl Drop for UploadBufferManager {
    fn drop(&mut self) {
        log::info!("UploadBufferManager dropped.");
    }
}
