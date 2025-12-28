// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod render_thread;
mod tauri_event_adapter;

use crate::render_thread::RenderThread;
use crate::tauri_event_adapter::TauriEventAdapter;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use serde::Deserialize;
use std::sync::Mutex;
use tauri::window::WindowBuilder;
use tauri::{Listener, Manager, RunEvent, WindowEvent};
use truvis_app::outer_app::OuterApp;
use truvis_app::outer_app::triangle::triangle_app;
use truvis_app::outer_app::triangle::triangle_app::HelloTriangleApp;

/// 全局渲染线程句柄
static RENDER_THREAD: Mutex<Option<RenderThread>> = Mutex::new(None);

/// 从前端接收的鼠标移动事件
#[derive(Debug, Clone, Deserialize)]
struct MouseMovePayload {
    x: f64,
    y: f64,
}

/// 从前端接收的鼠标按钮事件
#[derive(Debug, Clone, Deserialize)]
struct MouseButtonPayload {
    button: u16,
    pressed: bool,
}

/// 从前端接收的键盘事件
#[derive(Debug, Clone, Deserialize)]
struct KeyboardPayload {
    key: String,
    pressed: bool,
}

/// 从前端接收的鼠标滚轮事件
#[derive(Debug, Clone, Deserialize)]
struct MouseWheelPayload {
    delta: f64,
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 获取主窗口 (Tauri 2 默认会创建 "main" 窗口)
            let _main_window = app.get_webview_window("main").expect("Failed to get main window");

            // 创建一个新的纯窗口（没有 WebView），用于 Vulkan 渲染
            let render_window = WindowBuilder::new(app, "render")
                .title("Vulkan Render Window")
                .inner_size(1280.0, 720.0)
                .always_on_top(false)
                .decorations(true)
                .resizable(true)
                .visible(true)
                .build()
                .expect("Failed to create render window");

            // 获取 render 窗口的 raw-window-handle
            let display_handle = render_window.display_handle().expect("Failed to get display handle");
            let raw_display_handle = display_handle.as_raw();

            // 在独立线程中启动渲染器
            let render_thread = RenderThread::spawn(raw_display_handle, || Box::new(HelloTriangleApp::default()));

            // 发送窗口初始化消息
            let window_handle = render_window.window_handle().expect("Failed to get window handle");
            let scale_factor = render_window.scale_factor().unwrap_or(1.0);

            render_thread.init_window(raw_display_handle, window_handle.as_raw(), scale_factor);

            // 保存渲染线程句柄
            *RENDER_THREAD.lock().unwrap() = Some(render_thread);

            // 设置窗口事件监听
            setup_window_events(app, &render_window);

            // 设置前端事件监听（用于接收前端转发的输入事件）
            setup_frontend_events(app);

            println!("Tauri setup complete. Render thread started.");

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // 运行应用程序并处理事件
    app.run(|_app_handle, event| {
        match &event {
            RunEvent::Exit => {
                // 应用退出时，关闭渲染线程
                println!("Application exiting...");
                if let Some(render_thread) = RENDER_THREAD.lock().unwrap().take() {
                    render_thread.join();
                }
            }
            RunEvent::WindowEvent { label, event, .. } => {
                if label == "render" {
                    handle_render_window_event(event);
                }
            }
            _ => {}
        }
    });
}

/// 设置渲染窗口的事件监听
fn setup_window_events(app: &tauri::App, render_window: &tauri::Window) {
    // 监听窗口关闭
    let app_handle = app.handle().clone();
    render_window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            println!("Render window close requested");
            // 关闭渲染线程
            if let Some(render_thread) = RENDER_THREAD.lock().unwrap().take() {
                render_thread.join();
            }
            // 退出应用
            app_handle.exit(0);
        }
    });
}

/// 设置从前端接收输入事件的监听器
fn setup_frontend_events(app: &tauri::App) {
    // 监听鼠标移动事件
    app.listen("render:mouse_move", |event| {
        if let Ok(payload) = serde_json::from_str::<MouseMovePayload>(event.payload()) {
            send_event_to_render_thread(TauriEventAdapter::from_cursor_moved(payload.x, payload.y));
        }
    });

    // 监听鼠标按钮事件
    app.listen("render:mouse_button", |event| {
        if let Ok(payload) = serde_json::from_str::<MouseButtonPayload>(event.payload()) {
            send_event_to_render_thread(TauriEventAdapter::from_mouse_button(payload.button, payload.pressed));
        }
    });

    // 监听键盘事件
    app.listen("render:keyboard", |event| {
        if let Ok(payload) = serde_json::from_str::<KeyboardPayload>(event.payload()) {
            send_event_to_render_thread(TauriEventAdapter::from_keyboard(&payload.key, payload.pressed));
        }
    });

    // 监听鼠标滚轮事件
    app.listen("render:mouse_wheel", |event| {
        if let Ok(payload) = serde_json::from_str::<MouseWheelPayload>(event.payload()) {
            send_event_to_render_thread(TauriEventAdapter::from_mouse_wheel(payload.delta));
        }
    });
}

/// 发送事件到渲染线程
fn send_event_to_render_thread(event: truvis_app::platform::input_event::InputEvent) {
    let render_thread = RENDER_THREAD.lock().unwrap();
    if let Some(ref thread) = *render_thread {
        thread.send_event(event);
    }
}

/// 处理渲染窗口事件
fn handle_render_window_event(event: &WindowEvent) {
    let render_thread = RENDER_THREAD.lock().unwrap();
    if let Some(ref thread) = *render_thread {
        match event {
            WindowEvent::Resized(size) => {
                let input_event = TauriEventAdapter::from_resized(size.width as f64, size.height as f64);
                thread.send_event(input_event);
            }
            WindowEvent::Focused(focused) => {
                // 可以在这里处理焦点变化
                println!("Render window focused: {}", focused);
            }
            WindowEvent::Moved(_) => {
                // 窗口移动事件
            }
            _ => {}
        }
    }
}
