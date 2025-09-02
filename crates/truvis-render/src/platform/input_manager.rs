use std::collections::{HashMap, VecDeque};

use winit::{
    event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

/// 输入事件类型
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// 键盘按键事件
    KeyboardInput { key_code: KeyCode, state: ElementState },
    /// 鼠标按键事件
    MouseButtonInput { button: MouseButton, state: ElementState },
    /// 鼠标移动事件
    MouseMoved { position: glam::DVec2 },
    /// 鼠标滚轮事件
    MouseWheel { delta: f32 },
}

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
    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let pos = glam::dvec2(position.x, position.y);
                self.events.push_back(InputEvent::MouseMoved { position: pos });
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // 简化处理，仅考虑垂直滚动
                let delta_value = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => *y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                };
                self.events.push_back(InputEvent::MouseWheel { delta: delta_value });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.events.push_back(InputEvent::MouseButtonInput {
                    button: *button,
                    state: *state,
                });
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state,
                        ..
                    },
                ..
            } => {
                self.events.push_back(InputEvent::KeyboardInput {
                    key_code: *key_code,
                    state: *state,
                });
            }
            _ => {}
        }
    }

    /// 处理设备事件
    pub fn handle_device_event(&mut self, _event: &DeviceEvent) {
        // match event {
        //     // 添加对其他设备事件的处理，如游戏手柄等
        //     _ => {}
        // }
    }

    /// 更新输入状态
    /// 处理所有队列中的事件，更新输入状态
    pub fn update(&mut self) {
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
                InputEvent::MouseMoved { position } => {
                    self.state.crt_mouse_pos = position;
                }
                InputEvent::MouseWheel { delta: _ } => {
                    // 可以在这里处理鼠标滚轮事件
                }
            }
        }
    }

    /// 检查键盘按键是否被按下
    pub fn is_key_pressed(&self, key_code: KeyCode) -> bool {
        self.state.key_pressed.get(&key_code).copied().unwrap_or(false)
    }

    /// 获取鼠标位置
    pub fn get_mouse_position(&self) -> glam::DVec2 {
        self.state.crt_mouse_pos
    }

    /// 获取鼠标位置变化
    pub fn get_mouse_delta(&self) -> glam::DVec2 {
        self.state.crt_mouse_pos - self.state.last_mouse_pos
    }

    /// 检查鼠标右键是否被按下
    pub fn is_right_button_pressed(&self) -> bool {
        self.state.right_button_pressed
    }
}

/// 记录输入信息
#[derive(Default, Clone)]
pub struct InputState {
    /// 当前帧的鼠标位置 pixel
    pub crt_mouse_pos: glam::DVec2,
    /// 上一帧的鼠标位置 pixel
    pub last_mouse_pos: glam::DVec2,
    pub right_button_pressed: bool,
    pub key_pressed: HashMap<KeyCode, bool>,
}
