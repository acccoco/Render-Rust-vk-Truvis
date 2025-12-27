use crate::input_event::KeyCode;
use std::collections::HashMap;

/// 记录输入信息
#[derive(Default, Clone)]
pub struct InputState {
    /// 当前帧的鼠标位置 pixel
    pub crt_mouse_pos: [f64; 2],
    /// 上一帧的鼠标位置 pixel
    pub last_mouse_pos: [f64; 2],
    pub right_button_pressed: bool,
    pub key_pressed: HashMap<KeyCode, bool>,
}

impl InputState {
    /// 检查键盘按键是否被按下
    pub fn is_key_pressed(&self, key_code: KeyCode) -> bool {
        self.key_pressed.get(&key_code).copied().unwrap_or(false)
    }

    /// 获取鼠标位置
    pub fn get_mouse_position(&self) -> [f64; 2] {
        self.crt_mouse_pos
    }

    /// 获取鼠标位置变化
    pub fn get_mouse_delta(&self) -> [f64; 2] {
        [
            self.crt_mouse_pos[0] - self.last_mouse_pos[0],
            self.crt_mouse_pos[1] - self.last_mouse_pos[1],
        ]
    }

    /// 检查鼠标右键是否被按下
    pub fn is_right_button_pressed(&self) -> bool {
        self.right_button_pressed
    }
}
