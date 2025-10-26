use std::rc::Rc;

use itertools::Itertools;

use truvis_rhi::render_context::RenderContext;
use truvis_rhi::resources::buffer::Buffer;
use truvis_rhi::resources::special_buffers::stage_buffer::StageBuffer;

use crate::renderer::frame_controller::FrameController;

pub struct StageBufferManager {
    buffers: Vec<Vec<Buffer>>,

    frame_ctrl: Rc<FrameController>,
}

// init & destroy
impl StageBufferManager {
    pub fn new(frame_ctrl: Rc<FrameController>) -> Self {
        let buffers = (0..frame_ctrl.fif_count()).map(|_| Vec::new()).collect_vec();
        Self { buffers, frame_ctrl }
    }
}

// tools
impl StageBufferManager {
    pub fn alloc_buffer(&mut self, size: u64, debug_name: &str) -> &mut Buffer {
        let buffer = Buffer::new_stage_buffer(size, debug_name);
        let frame_idx = *self.frame_ctrl.frame_label();
        self.buffers[frame_idx].push(buffer);
        self.buffers[frame_idx].last_mut().unwrap()
    }

    pub fn register_stage_buffer(&mut self, stage_buffer: Buffer) {
        let frame_idx = *self.frame_ctrl.frame_label();
        self.buffers[frame_idx].push(stage_buffer);
    }

    pub fn clear_frame_buffers(&mut self) {
        let frame_idx = *self.frame_ctrl.frame_label();

        self.buffers[frame_idx].clear();
    }
}

impl Drop for StageBufferManager {
    fn drop(&mut self) {
        log::info!("UploadBufferManager dropped.");
    }
}
