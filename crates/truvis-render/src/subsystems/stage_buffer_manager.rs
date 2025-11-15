use itertools::Itertools;

use crate::core::frame_context::FrameContext;
use crate::subsystems::subsystem::Subsystem;
use truvis_gfx::resources::buffer::Buffer;

pub struct StageBufferManager {
    buffers: Vec<Vec<Buffer>>,
}

// init & destroy
impl StageBufferManager {
    pub fn new(fif_count: usize) -> Self {
        let buffers = (0..fif_count).map(|_| Vec::new()).collect_vec();
        Self { buffers }
    }
}
impl Subsystem for StageBufferManager {
    fn before_render(&mut self) {}
}

// tools
impl StageBufferManager {
    pub fn alloc_buffer(&mut self, size: u64, debug_name: &str) -> &mut Buffer {
        let buffer = Buffer::new_stage_buffer(size, debug_name);
        let frame_idx = *FrameContext::get().frame_label();
        self.buffers[frame_idx].push(buffer);
        self.buffers[frame_idx].last_mut().unwrap()
    }

    pub fn register_stage_buffer(&mut self, stage_buffer: Buffer) {
        let frame_idx = *FrameContext::get().frame_label();
        self.buffers[frame_idx].push(stage_buffer);
    }

    pub fn clear_fif_buffers(&mut self) {
        let frame_idx = *FrameContext::get().frame_label();

        self.buffers[frame_idx].clear();
    }
}

impl Drop for StageBufferManager {
    fn drop(&mut self) {
        log::info!("UploadBufferManager dropped.");
    }
}
