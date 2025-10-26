use crate::renderer::frame_controller::FrameController;
use itertools::Itertools;
use std::rc::Rc;
use truvis_rhi::render_context::RenderContext;
use truvis_rhi::resources::buffer::Buffer;
use truvis_rhi::resources::special_buffers::stage_buffer::StageBuffer;
use truvis_rhi::resources_new::managed_buffer::Buffer2;
use truvis_rhi::resources_new::resource_handles::BufferHandle;

pub struct StageBufferManager {
    buffers: Vec<Vec<Buffer>>,
    stage_buffers: Vec<Vec<BufferHandle>>,

    frame_ctrl: Rc<FrameController>,
}

// init & destroy
impl StageBufferManager {
    pub fn new(frame_ctrl: Rc<FrameController>) -> Self {
        let buffers = (0..frame_ctrl.fif_count()).map(|_| Vec::new()).collect_vec();
        let stage_buffers = (0..frame_ctrl.fif_count()).map(|_| Vec::new()).collect_vec();
        Self {
            buffers,
            stage_buffers,
            frame_ctrl,
        }
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

    pub fn register_stage_buffer2(&mut self, stage_buffer: Buffer2) {
        let buffer_handle = RenderContext::get().resource_mgr_mut().register_buffer(stage_buffer);
        let frame_idx = *self.frame_ctrl.frame_label();
        self.stage_buffers[frame_idx].push(buffer_handle);
    }

    pub fn clear_frame_buffers(&mut self) {
        let frame_idx = *self.frame_ctrl.frame_label();

        self.buffers[frame_idx].clear();

        let mut resource_mgr = RenderContext::get().resource_mgr_mut();
        std::mem::take(&mut self.stage_buffers[frame_idx])
            .into_iter()
            .for_each(|buffer_handle| resource_mgr.unregister_buffer(buffer_handle));
    }
}

impl Drop for StageBufferManager {
    fn drop(&mut self) {
        log::info!("UploadBufferManager dropped.");
    }
}
