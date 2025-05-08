use std::collections::HashMap;

/// 记录输入信息
#[derive(Default)]
pub struct InputState {
    /// 当前帧的鼠标位置 pixel
    pub crt_mouse_pos: glam::DVec2,
    /// 上一帧的鼠标位置 pixel
    pub last_mouse_pos: glam::DVec2,
    pub right_button_pressed: bool,
    pub key_pressed: HashMap<winit::keyboard::KeyCode, bool>,
}
