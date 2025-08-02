use ash::vk;
use std::fmt::Display;
use std::ops::Deref;

pub struct DefaultRendererSettings;
impl DefaultRendererSettings {
    pub const DEFAULT_SURFACE_FORMAT: vk::SurfaceFormatKHR = vk::SurfaceFormatKHR {
        format: vk::Format::R8G8B8A8_UNORM,
        color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
    };
    pub const DEFAULT_PRESENT_MODE: vk::PresentModeKHR = vk::PresentModeKHR::MAILBOX;
    pub const DEPTH_FORMAT_CANDIDATES: &'static [vk::Format] = &[
        vk::Format::D32_SFLOAT_S8_UINT,
        vk::Format::D32_SFLOAT,
        vk::Format::D24_UNORM_S8_UINT,
        vk::Format::D16_UNORM_S8_UINT,
        vk::Format::D16_UNORM,
    ];
}

#[derive(Copy, Clone, Default)]
pub struct FrameSettings {
    pub color_format: vk::Format,
    pub depth_format: vk::Format,
    pub frame_extent: vk::Extent2D,
}

#[derive(Copy, Clone)]
pub struct PresentSettings {
    pub canvas_extent: vk::Extent2D,

    pub swapchain_image_cnt: usize,
    pub color_format: vk::Format,
}

#[derive(Debug, Clone, Copy)]
pub enum FrameLabel {
    A,
    B,
    C,
}
impl Deref for FrameLabel {
    type Target = usize;
    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            Self::A => &Self::INDEX[0],
            Self::B => &Self::INDEX[1],
            Self::C => &Self::INDEX[2],
        }
    }
}
impl Display for FrameLabel {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::B => write!(f, "B"),
            Self::C => write!(f, "C"),
        }
    }
}
impl FrameLabel {
    const INDEX: [usize; 3] = [0, 1, 2];

    #[inline]
    pub fn from_usize(idx: usize) -> Self {
        match idx {
            0 => Self::A,
            1 => Self::B,
            2 => Self::C,
            _ => panic!("Invalid frame index: {idx}"),
        }
    }
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
