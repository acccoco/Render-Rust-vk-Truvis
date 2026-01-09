//! Win32 子窗口创建模块
//!
//! 在 Tauri 主窗口内创建一个 Win32 子窗口，用于 Vulkan 渲染。

#[cfg(windows)]
use raw_window_handle::{RawWindowHandle, Win32WindowHandle};
#[cfg(windows)]
use std::num::NonZeroIsize;
#[cfg(windows)]
use std::sync::Mutex;
#[cfg(windows)]
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::HBRUSH;
#[cfg(windows)]
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CreateWindowExW, DefWindowProcW, GetClientRect, RegisterClassExW,
    SetWindowPos, CS_HREDRAW, CS_OWNDC, CS_VREDRAW, HWND_TOP, SWP_SHOWWINDOW, WINDOW_EX_STYLE,
    WM_DESTROY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_KEYDOWN, WM_KEYUP,
    WNDCLASSEXW, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_VISIBLE,
};
#[cfg(windows)]
use windows::core::PCWSTR;

use truvis_app::platform::input_event::InputEvent;

/// 全局鼠标事件回调
#[cfg(windows)]
static MOUSE_EVENT_CALLBACK: Mutex<Option<Box<dyn Fn(InputEvent) + Send + Sync>>> = Mutex::new(None);

/// 设置鼠标事件回调
#[cfg(windows)]
pub fn set_mouse_event_callback<F>(callback: F)
where
    F: Fn(InputEvent) + Send + Sync + 'static,
{
    *MOUSE_EVENT_CALLBACK.lock().unwrap() = Some(Box::new(callback));
}

/// 包装 HWND 使其可以跨线程发送
#[cfg(windows)]
#[derive(Clone, Copy)]
pub struct SendableHwnd(isize);

#[cfg(windows)]
unsafe impl Send for SendableHwnd {}
#[cfg(windows)]
unsafe impl Sync for SendableHwnd {}

#[cfg(windows)]
impl SendableHwnd {
    pub fn new(hwnd: HWND) -> Self {
        Self(hwnd.0 as isize)
    }

    pub fn hwnd(&self) -> HWND {
        HWND(self.0 as *mut _)
    }
}

/// 子窗口信息（可跨线程发送）
#[cfg(windows)]
pub struct ChildWindow {
    /// 子窗口句柄
    pub hwnd: SendableHwnd,
    /// 父窗口句柄
    pub parent_hwnd: SendableHwnd,
    /// 模块实例句柄
    hinstance_ptr: isize,
}

#[cfg(windows)]
unsafe impl Send for ChildWindow {}
#[cfg(windows)]
unsafe impl Sync for ChildWindow {}

#[cfg(windows)]
impl ChildWindow {
    /// 从 Tauri 窗口的 RawWindowHandle 创建子窗口
    ///
    /// # Arguments
    /// * `parent_handle` - 父窗口的 RawWindowHandle
    /// * `x` - 子窗口 X 坐标（相对于父窗口客户区）
    /// * `y` - 子窗口 Y 坐标
    /// * `width` - 子窗口宽度
    /// * `height` - 子窗口高度
    ///
    /// # Returns
    /// 返回子窗口信息和对应的 RawWindowHandle
    pub fn create(
        parent_handle: RawWindowHandle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<(Self, RawWindowHandle), String> {
        // 从 RawWindowHandle 提取父窗口 HWND
        let parent_hwnd = match parent_handle {
            RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as *mut _),
            _ => return Err("Not a Win32 window handle".to_string()),
        };

        unsafe {
            // 获取模块句柄
            let hmodule = GetModuleHandleW(None).map_err(|e| e.to_string())?;
            let hinstance = HINSTANCE(hmodule.0);

            // 注册窗口类（只需注册一次）
            let class_name = windows::core::w!("TruvisVulkanChildWindow");

            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
                lpfnWndProc: Some(child_window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: Default::default(),
                hCursor: Default::default(),
                hbrBackground: HBRUSH::default(), // 使用默认背景（Vulkan 会覆盖）
                lpszMenuName: PCWSTR::null(),
                lpszClassName: class_name,
                hIconSm: Default::default(),
            };

            // 尝试注册窗口类（如果已存在则忽略错误）
            let _ = RegisterClassExW(&wc);

            // 创建子窗口
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class_name,
                windows::core::w!("VulkanRenderChild"),
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
                x,
                y,
                width,
                height,
                parent_hwnd,
                None,
                hinstance,
                None,
            )
            .map_err(|e| format!("Failed to create child window: {}", e))?;

            // 将子窗口置于 z-order 顶部（在 WebView2 之上）
            SetWindowPos(hwnd, HWND_TOP, x, y, width, height, SWP_SHOWWINDOW)
                .map_err(|e| format!("Failed to set window z-order: {}", e))?;
            
            // 额外调用 BringWindowToTop 确保在最前面
            let _ = BringWindowToTop(hwnd);

            // 构造 RawWindowHandle
            let raw_handle = create_raw_window_handle(hwnd, hinstance);

            let child_window = ChildWindow {
                hwnd: SendableHwnd::new(hwnd),
                parent_hwnd: SendableHwnd::new(parent_hwnd),
                hinstance_ptr: hinstance.0 as isize,
            };

            Ok((child_window, raw_handle))
        }
    }

    /// 调整子窗口大小和位置
    pub fn set_position(&self, x: i32, y: i32, width: i32, height: i32) -> Result<(), String> {
        unsafe {
            // 使用 HWND_TOP 确保子窗口在 WebView2 之上
            SetWindowPos(self.hwnd.hwnd(), HWND_TOP, x, y, width, height, SWP_SHOWWINDOW)
                .map_err(|e| format!("Failed to set window position: {}", e))?;
            
            // 额外调用 BringWindowToTop 确保在最前面
            let _ = BringWindowToTop(self.hwnd.hwnd());
            Ok(())
        }
    }

    /// 获取父窗口客户区大小
    pub fn get_parent_client_size(&self) -> Result<(i32, i32), String> {
        unsafe {
            let mut rect = std::mem::zeroed();
            GetClientRect(self.parent_hwnd.hwnd(), &mut rect)
                .map_err(|e| format!("Failed to get client rect: {}", e))?;
            Ok((rect.right - rect.left, rect.bottom - rect.top))
        }
    }

    /// 获取子窗口的 RawWindowHandle
    pub fn raw_window_handle(&self) -> RawWindowHandle {
        let hinstance = HINSTANCE(self.hinstance_ptr as *mut _);
        create_raw_window_handle(self.hwnd.hwnd(), hinstance)
    }
}

/// 构造 RawWindowHandle
#[cfg(windows)]
fn create_raw_window_handle(hwnd: HWND, hinstance: HINSTANCE) -> RawWindowHandle {
    let mut handle = Win32WindowHandle::new(NonZeroIsize::new(hwnd.0 as isize).unwrap());
    handle.hinstance = NonZeroIsize::new(hinstance.0 as isize);
    RawWindowHandle::Win32(handle)
}

/// 子窗口消息处理函数
#[cfg(windows)]
unsafe extern "system" fn child_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    use truvis_app::platform::input_event::{ElementState, KeyCode, MouseButton};
    
    // 从 lparam 提取鼠标坐标
    let get_mouse_pos = || {
        let x = (lparam.0 & 0xFFFF) as i16 as f64;
        let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f64;
        (x, y)
    };
    
    // 发送事件到回调
    let send_event = |event: InputEvent| {
        if let Ok(callback) = MOUSE_EVENT_CALLBACK.lock() {
            if let Some(ref cb) = *callback {
                cb(event);
            }
        }
    };
    
    match msg {
        WM_DESTROY => {
            // 子窗口被销毁
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let (x, y) = get_mouse_pos();
            send_event(InputEvent::MouseMoved { physical_position: [x, y] });
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            send_event(InputEvent::MouseButtonInput { 
                button: MouseButton::Left, 
                state: ElementState::Pressed 
            });
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            send_event(InputEvent::MouseButtonInput { 
                button: MouseButton::Left, 
                state: ElementState::Released 
            });
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            send_event(InputEvent::MouseButtonInput { 
                button: MouseButton::Right, 
                state: ElementState::Pressed 
            });
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            send_event(InputEvent::MouseButtonInput { 
                button: MouseButton::Right, 
                state: ElementState::Released 
            });
            LRESULT(0)
        }
        WM_MBUTTONDOWN => {
            send_event(InputEvent::MouseButtonInput { 
                button: MouseButton::Middle, 
                state: ElementState::Pressed 
            });
            LRESULT(0)
        }
        WM_MBUTTONUP => {
            send_event(InputEvent::MouseButtonInput { 
                button: MouseButton::Middle, 
                state: ElementState::Released 
            });
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            // HIWORD of wparam contains the wheel delta (in multiples of WHEEL_DELTA = 120)
            let delta = ((wparam.0 >> 16) as i16) as f64 / 120.0;
            send_event(InputEvent::MouseWheel { delta });
            LRESULT(0)
        }
        WM_KEYDOWN => {
            let key_code = wparam_to_keycode(wparam.0 as u32);
            send_event(InputEvent::KeyboardInput { 
                key_code, 
                state: ElementState::Pressed 
            });
            LRESULT(0)
        }
        WM_KEYUP => {
            let key_code = wparam_to_keycode(wparam.0 as u32);
            send_event(InputEvent::KeyboardInput { 
                key_code, 
                state: ElementState::Released 
            });
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// 将 Win32 虚拟键码转换为 KeyCode
#[cfg(windows)]
fn wparam_to_keycode(vk: u32) -> truvis_app::platform::input_event::KeyCode {
    use truvis_app::platform::input_event::KeyCode;
    match vk {
        0x57 => KeyCode::KeyW, // 'W'
        0x41 => KeyCode::KeyA, // 'A'
        0x53 => KeyCode::KeyS, // 'S'
        0x44 => KeyCode::KeyD, // 'D'
        0x45 => KeyCode::KeyE, // 'E'
        0x51 => KeyCode::KeyQ, // 'Q'
        _ => KeyCode::Other,
    }
}

/// 计算子窗口布局（Vulkan 渲染区域）
///
/// # Arguments
/// * `parent_width` - 父窗口宽度
/// * `parent_height` - 父窗口高度
/// * `margin_top` - 顶部边距（像素）
/// * `margin_left` - 左侧边距（像素）
/// * `margin_right` - 右侧边距（像素）
/// * `margin_bottom` - 底部边距（像素）
///
/// # Returns
/// 返回 (x, y, width, height) 子窗口区域
#[cfg(windows)]
pub fn calculate_vulkan_region_with_margins(
    parent_width: i32,
    parent_height: i32,
    margin_top: i32,
    margin_left: i32,
    margin_right: i32,
    margin_bottom: i32,
) -> (i32, i32, i32, i32) {
    let x = margin_left;
    let y = margin_top;
    let width = (parent_width - margin_left - margin_right).max(1);
    let height = (parent_height - margin_top - margin_bottom).max(1);
    (x, y, width, height)
}

/// 旧版兼容：计算子窗口布局（Vulkan 渲染区域占据右侧部分）
#[cfg(windows)]
pub fn calculate_vulkan_region(
    parent_width: i32,
    parent_height: i32,
    sidebar_width: i32,
) -> (i32, i32, i32, i32) {
    calculate_vulkan_region_with_margins(parent_width, parent_height, 0, sidebar_width, 0, 0)
}

// 非 Windows 平台的占位实现
#[cfg(not(windows))]
pub struct ChildWindow;

#[cfg(not(windows))]
impl ChildWindow {
    pub fn create(
        _parent_handle: raw_window_handle::RawWindowHandle,
        _x: i32,
        _y: i32,
        _width: i32,
        _height: i32,
    ) -> Result<(Self, raw_window_handle::RawWindowHandle), String> {
        Err("Child window is only supported on Windows".to_string())
    }

    pub fn set_position(&self, _x: i32, _y: i32, _width: i32, _height: i32) -> Result<(), String> {
        Err("Child window is only supported on Windows".to_string())
    }

    pub fn get_parent_client_size(&self) -> Result<(i32, i32), String> {
        Err("Child window is only supported on Windows".to_string())
    }

    pub fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        panic!("Child window is only supported on Windows")
    }
}

#[cfg(not(windows))]
pub fn calculate_vulkan_region_with_margins(
    parent_width: i32,
    parent_height: i32,
    margin_top: i32,
    margin_left: i32,
    margin_right: i32,
    margin_bottom: i32,
) -> (i32, i32, i32, i32) {
    let x = margin_left;
    let y = margin_top;
    let width = (parent_width - margin_left - margin_right).max(1);
    let height = (parent_height - margin_top - margin_bottom).max(1);
    (x, y, width, height)
}

#[cfg(not(windows))]
pub fn calculate_vulkan_region(
    parent_width: i32,
    parent_height: i32,
    sidebar_width: i32,
) -> (i32, i32, i32, i32) {
    calculate_vulkan_region_with_margins(parent_width, parent_height, 0, sidebar_width, 0, 0)
}
