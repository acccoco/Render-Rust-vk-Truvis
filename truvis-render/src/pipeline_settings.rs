use ash::vk;

#[derive(Copy, Clone, Default)]
pub struct FrameSettings {
    /// 会随着 swapchain 的重建而刷新
    pub viewport_extent: vk::Extent2D,
    pub rt_extent: vk::Extent2D,
    pub rt_offset: vk::Offset2D,
}

#[derive(Copy, Clone)]
pub struct PipelineSettings {
    pub frames_in_flight: usize,
    pub color_format: vk::Format,
    pub depth_format: vk::Format,

    pub frame_settings: FrameSettings,
}

/// 用于逐帧累积的数据
#[derive(Copy, Clone, Default)]
pub struct AccumData {
    pub last_camera_pos: glam::Vec3,
    pub last_camera_dir: glam::Vec3,

    pub accum_frames_num: usize,
}
impl AccumData {
    /// call phase: BeforeRender-CollectData
    pub fn update_accum_frames(&mut self, camera_pos: glam::Vec3, camera_dir: glam::Vec3) {
        if self.last_camera_dir != camera_dir || self.last_camera_pos != camera_pos {
            self.accum_frames_num = 0;
        } else {
            self.accum_frames_num += 1;
        }

        self.last_camera_pos = camera_pos;
        self.last_camera_dir = camera_dir;
    }

    pub fn reset(&mut self) {
        self.last_camera_pos = glam::Vec3::ZERO;
        self.last_camera_dir = glam::Vec3::ZERO;
        self.accum_frames_num = 0;
    }
}

/// frames in flight name
pub const FRAME_ID_MAP: [char; 4] = ['A', 'B', 'C', 'D'];
