use crate::input_event::{ElementState, InputEvent, MouseButton};
use crate::input_state::InputState;
use std::collections::VecDeque;

/// 输入管理器
pub struct InputManager {
    /// 输入状态
    state: InputState,
    /// 事件队列
    events: VecDeque<InputEvent>,
}
impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}
// getter
impl InputManager {
    #[inline]
    pub fn state(&self) -> &InputState {
        &self.state
    }
}
impl InputManager {
    /// 创建新的输入管理器
    pub fn new() -> Self {
        Self {
            state: InputState::default(),
            events: VecDeque::new(),
        }
    }

    /// 处理窗口事件
    pub fn push_event(&mut self, event: InputEvent) {
        self.events.push_back(event);
    }

    pub fn get_events(&self) -> &VecDeque<InputEvent> {
        &self.events
    }

    /// 更新输入状态
    /// 处理所有队列中的事件，更新输入状态
    pub fn process_events(&mut self) {
        // 保存上一帧的鼠标位置
        self.state.last_mouse_pos = self.state.crt_mouse_pos;

        // 处理事件队列中的所有事件
        while let Some(event) = self.events.pop_front() {
            match event {
                InputEvent::KeyboardInput { key_code, state } => {
                    self.state.key_pressed.insert(key_code, state == ElementState::Pressed);
                }
                InputEvent::MouseButtonInput { button, state } => {
                    if button == MouseButton::Right {
                        self.state.right_button_pressed = state == ElementState::Pressed;
                    }
                }
                InputEvent::MouseMoved {
                    physical_position: position,
                } => {
                    self.state.crt_mouse_pos = position;
                }
                InputEvent::MouseWheel { delta: _ } => {
                    // 可以在这里处理鼠标滚轮事件
                }
                InputEvent::Resized { .. } => {}
                InputEvent::Other => {}
            }
        }
    }
}
