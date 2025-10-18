use crate::renderer::frame_controller::FrameController;
use itertools::Itertools;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use truvis_rhi::resources::buffer::Buffer;

pub struct UploadBufferManager {
    buffers: Vec<Vec<Buffer>>,

    frame_ctrl: Rc<FrameController>,

    valid: bool,
}

impl UploadBufferManager {
    pub fn new(frame_ctrl: Rc<FrameController>) -> Self {
        let buffers = (0..frame_ctrl.fif_count()).map(|_| Vec::new()).collect_vec();
        Self {
            buffers,
            frame_ctrl,
            valid: true,
        }
    }

    pub fn destroy(mut self) {
        self.valid = false;
    }
}

impl UploadBufferManager {
    pub fn alloc_buffer(&mut self, size: u64, debug_name: &str) -> Buffer {
        unimplemented!()
    }
}

impl Drop for UploadBufferManager {
    fn drop(&mut self) {
        assert!(!self.valid, "UploadBufferManager must be destroyed manually.");
    }
}
