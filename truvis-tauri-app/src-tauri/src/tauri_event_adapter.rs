use truvis_app::platform::input_event::{ElementState, InputEvent, KeyCode, MouseButton};

/// Tauri 事件适配器
/// 将 Tauri 窗口事件转换为 InputEvent
pub struct TauriEventAdapter;

impl TauriEventAdapter {
    /// 从鼠标移动事件创建 InputEvent
    pub fn from_cursor_moved(x: f64, y: f64) -> InputEvent {
        InputEvent::MouseMoved {
            physical_position: [x, y],
        }
    }

    /// 从鼠标滚轮事件创建 InputEvent
    pub fn from_mouse_wheel(delta_y: f64) -> InputEvent {
        InputEvent::MouseWheel { delta: delta_y }
    }

    /// 从鼠标按钮事件创建 InputEvent
    pub fn from_mouse_button(button: u16, pressed: bool) -> InputEvent {
        InputEvent::MouseButtonInput {
            button: Self::button_from_code(button),
            state: if pressed { ElementState::Pressed } else { ElementState::Released },
        }
    }

    /// 从键盘事件创建 InputEvent
    pub fn from_keyboard(key: &str, pressed: bool) -> InputEvent {
        InputEvent::KeyboardInput {
            key_code: Self::key_from_string(key),
            state: if pressed { ElementState::Pressed } else { ElementState::Released },
        }
    }

    /// 从窗口大小变化事件创建 InputEvent
    pub fn from_resized(width: f64, height: f64) -> InputEvent {
        InputEvent::Resized {
            physical_width: width,
            physical_height: height,
        }
    }

    /// 将按钮代码转换为 MouseButton
    fn button_from_code(button: u16) -> MouseButton {
        match button {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            3 => MouseButton::Back,
            4 => MouseButton::Forward,
            other => MouseButton::Other(other),
        }
    }

    /// 将键盘字符串转换为 KeyCode
    fn key_from_string(key: &str) -> KeyCode {
        match key.to_lowercase().as_str() {
            "w" | "keyw" => KeyCode::KeyW,
            "a" | "keya" => KeyCode::KeyA,
            "s" | "keys" => KeyCode::KeyS,
            "d" | "keyd" => KeyCode::KeyD,
            "e" | "keye" => KeyCode::KeyE,
            "q" | "keyq" => KeyCode::KeyQ,
            _ => KeyCode::Other,
        }
    }
}
