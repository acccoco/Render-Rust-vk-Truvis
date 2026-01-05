// 参考 winit::MouseButton
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

// 参考 winit::ElementState
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum ElementState {
    Pressed,
    Released,
}

// 参考 winit::KeyCode
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyCode {
    KeyW,
    KeyA,
    KeyS,
    KeyD,
    KeyE,
    KeyQ,

    Other,
}

/// 输入事件类型
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// 键盘按键事件
    KeyboardInput {
        key_code: KeyCode,
        state: ElementState,
    },
    /// 鼠标按键事件
    MouseButtonInput {
        button: MouseButton,
        state: ElementState,
    },
    /// 鼠标移动事件
    MouseMoved {
        physical_position: [f64; 2],
    },
    /// 鼠标滚轮事件
    MouseWheel {
        delta: f64,
    },
    /// 窗口大小改变事件
    Resized {
        physical_width: u32,
        physical_height: u32,
    },

    Other,
}
