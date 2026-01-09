// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod child_window;
mod render_thread;
mod tauri_event_adapter;

use crate::child_window::{calculate_vulkan_region, calculate_vulkan_region_with_margins, ChildWindow, set_mouse_event_callback};
use crate::render_thread::RenderThread;
use crate::tauri_event_adapter::TauriEventAdapter;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use serde::Deserialize;
use std::sync::Mutex;
use tauri::{Listener, Manager, RunEvent, WebviewWindow, WindowEvent};
use truvis_app::outer_app::cornell_app::CornellApp;
use truvis_app::outer_app::sponza_app::SponzaApp;
use truvis_app::outer_app::triangle::triangle_app::HelloTriangleApp;

/// 全局渲染线程句柄
static RENDER_THREAD: Mutex<Option<RenderThread>> = Mutex::new(None);

/// 全局子窗口句柄（用于同步大小）
#[cfg(windows)]
static CHILD_WINDOW: Mutex<Option<ChildWindow>> = Mutex::new(None);

/// 全局 AppHandle（用于在窗口事件中访问 WebviewWindow）
static APP_HANDLE: Mutex<Option<tauri::AppHandle>> = Mutex::new(None);

/// 当前布局边距（用于窗口 resize 时保持布局）
static CURRENT_MARGINS: Mutex<(i32, i32, i32, i32)> = Mutex::new((40, 200, 200, 24)); // (top, left, right, bottom);


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

/// 从前端接收的布局更新事件
#[derive(Debug, Clone, Deserialize)]
struct LayoutUpdatePayload {
    sidebar_width: i32,
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![update_sidebar_width, update_vulkan_bounds])
        .setup(|app| {
            // 保存 AppHandle 供后续使用
            *APP_HANDLE.lock().unwrap() = Some(app.handle().clone());
            
            // 获取主窗口 (Tauri 2 默认会创建 "main" 窗口)
            let main_window = app
                .get_webview_window("main")
                .expect("Failed to get main window");

            // 设置主窗口大小
            main_window
                .set_size(tauri::Size::Physical(tauri::PhysicalSize {
                    width: 1280,
                    height: 720,
                }))
                .expect("Failed to set window size");

            // 初始化子窗口和渲染线程
            #[cfg(windows)]
            {
                init_vulkan_child_window(&main_window)?;
                
                // 新版布局：WebView 覆盖整个窗口（透明背景），Vulkan 区域通过前端控制
                // 不再需要手动调整 WebView bounds
                println!("WebView set to cover entire window (transparent mode)");
            }

            // 设置窗口事件监听（用于同步子窗口大小）
            setup_window_events(app, &main_window);

            // 设置前端事件监听（用于接收前端转发的输入事件）
            setup_frontend_events(app);

            println!("Tauri setup complete. Render thread started with embedded child window.");

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // 运行应用程序并处理事件
    app.run(|_app_handle, event| match &event {
        RunEvent::Exit => {
            // 应用退出时，关闭渲染线程
            println!("Application exiting...");
            if let Some(render_thread) = RENDER_THREAD.lock().unwrap().take() {
                render_thread.join();
            }
        }
        RunEvent::WindowEvent { label, event, .. } => {
            if label == "main" {
                handle_main_window_event(event);
            }
        }
        _ => {}
    });
}

/// 初始化 Vulkan 子窗口
#[cfg(windows)]
fn init_vulkan_child_window(main_window: &WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    // 获取主窗口的 raw-window-handle
    let display_handle = main_window
        .display_handle()
        .expect("Failed to get display handle");
    let raw_display_handle = display_handle.as_raw();

    let window_handle = main_window
        .window_handle()
        .expect("Failed to get window handle");
    let raw_window_handle = window_handle.as_raw();

    // 获取主窗口大小
    let window_size = main_window.inner_size().unwrap();
    let scale_factor = main_window.scale_factor().unwrap_or(1.0);

    // 计算子窗口区域（使用默认边距）
    let (top, left, right, bottom) = *CURRENT_MARGINS.lock().unwrap();
    let (x, y, width, height) = calculate_vulkan_region_with_margins(
        window_size.width as i32,
        window_size.height as i32,
        (top as f64 * scale_factor) as i32,
        (left as f64 * scale_factor) as i32,
        (right as f64 * scale_factor) as i32,
        (bottom as f64 * scale_factor) as i32,
    );

    // 创建子窗口
    let (child_window, child_raw_handle) =
        ChildWindow::create(raw_window_handle, x, y, width, height)
            .map_err(|e| format!("Failed to create child window: {}", e))?;

    println!(
        "Created Vulkan child window at ({}, {}) with size {}x{}",
        x, y, width, height
    );

    // 在独立线程中启动渲染器
    let render_thread =
        RenderThread::spawn(raw_display_handle, || Box::new(CornellApp::default()));

    // 发送窗口初始化消息（使用子窗口的句柄）
    render_thread.init_window(
        raw_display_handle,
        child_raw_handle,
        scale_factor,
        [width as u32, height as u32],
    );

    // 保存渲染线程和子窗口句柄
    *RENDER_THREAD.lock().unwrap() = Some(render_thread);
    *CHILD_WINDOW.lock().unwrap() = Some(child_window);

    // 设置鼠标事件回调，将事件转发到渲染线程
    set_mouse_event_callback(|event| {
        let render_thread = RENDER_THREAD.lock().unwrap();
        if let Some(ref thread) = *render_thread {
            thread.send_event(event);
        }
    });

    Ok(())
}

/// 设置主窗口的事件监听
fn setup_window_events(app: &tauri::App, main_window: &WebviewWindow) {
    // 监听窗口关闭
    let app_handle = app.handle().clone();
    main_window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            println!("Main window close requested");
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
            // println!("Mouse move: {:?}", payload);
            send_event_to_render_thread(TauriEventAdapter::from_cursor_moved(payload.x, payload.y));
        } else {
            eprintln!("Failed to parse mouse_move payload: {}", event.payload());
        }
    });

    // 监听鼠标按钮事件
    app.listen("render:mouse_button", |event| {
        if let Ok(payload) = serde_json::from_str::<MouseButtonPayload>(event.payload()) {
            println!("Mouse button: {:?}", payload);
            send_event_to_render_thread(TauriEventAdapter::from_mouse_button(payload.button, payload.pressed));
        } else {
            eprintln!("Failed to parse mouse_button payload: {}", event.payload());
        }
    });

    // 监听键盘事件
    app.listen("render:keyboard", |event| {
        if let Ok(payload) = serde_json::from_str::<KeyboardPayload>(event.payload()) {
            println!("Keyboard: {:?}", payload);
            send_event_to_render_thread(TauriEventAdapter::from_keyboard(&payload.key, payload.pressed));
        } else {
            eprintln!("Failed to parse keyboard payload: {}", event.payload());
        }
    });

    // 监听鼠标滚轮事件
    app.listen("render:mouse_wheel", |event| {
        if let Ok(payload) = serde_json::from_str::<MouseWheelPayload>(event.payload()) {
            println!("Mouse wheel: {:?}", payload);
            send_event_to_render_thread(TauriEventAdapter::from_mouse_wheel(payload.delta));
        } else {
            eprintln!("Failed to parse mouse_wheel payload: {}", event.payload());
        }
    });

    // 监听布局更新事件
    app.listen("render:layout_update", |event| {
        if let Ok(payload) = serde_json::from_str::<LayoutUpdatePayload>(event.payload()) {
            update_child_window_layout(payload.sidebar_width);
        }
    });
}

/// Tauri command: 更新侧边栏宽度（旧版兼容）
#[tauri::command]
fn update_sidebar_width(sidebar_width: i32) {
    update_child_window_layout(sidebar_width);
}

/// Tauri command: 更新 Vulkan 区域布局（4-margin 模式）
#[tauri::command]
fn update_vulkan_bounds(top: i32, left: i32, right: i32, bottom: i32) {
    // 保存当前边距
    *CURRENT_MARGINS.lock().unwrap() = (top, left, right, bottom);
    
    #[cfg(windows)]
    {
        let child_window = CHILD_WINDOW.lock().unwrap();
        if let Some(ref cw) = *child_window {
            if let Ok((parent_width, parent_height)) = cw.get_parent_client_size() {
                let (x, y, width, height) =
                    calculate_vulkan_region_with_margins(parent_width, parent_height, top, left, right, bottom);

                if let Err(e) = cw.set_position(x, y, width, height) {
                    eprintln!("Failed to update child window position: {}", e);
                } else {
                    println!("Vulkan region updated: pos({}, {}) size({}x{})", x, y, width, height);
                    
                    // 通知渲染线程窗口大小变化
                    let render_thread = RENDER_THREAD.lock().unwrap();
                    if let Some(ref thread) = *render_thread {
                        thread.send_event(TauriEventAdapter::from_resized(
                            width as u32,
                            height as u32,
                        ));
                    }
                }
            }
        }
    }
}

/// 更新子窗口布局
#[allow(unused_variables)]
fn update_child_window_layout(sidebar_width: i32) {
    #[cfg(windows)]
    {
        let child_window = CHILD_WINDOW.lock().unwrap();
        if let Some(ref cw) = *child_window {
            if let Ok((parent_width, parent_height)) = cw.get_parent_client_size() {
                let (x, y, width, height) =
                    calculate_vulkan_region(parent_width, parent_height, sidebar_width);

                if let Err(e) = cw.set_position(x, y, width, height) {
                    eprintln!("Failed to update child window position: {}", e);
                } else {
                    // 通知渲染线程窗口大小变化
                    let render_thread = RENDER_THREAD.lock().unwrap();
                    if let Some(ref thread) = *render_thread {
                        thread.send_event(TauriEventAdapter::from_resized(
                            width as u32,
                            height as u32,
                        ));
                    }
                }
            }
        }
    }
}

/// 发送事件到渲染线程
fn send_event_to_render_thread(event: truvis_app::platform::input_event::InputEvent) {
    let render_thread = RENDER_THREAD.lock().unwrap();
    if let Some(ref thread) = *render_thread {
        thread.send_event(event);
    }
}

/// 处理主窗口事件
fn handle_main_window_event(event: &WindowEvent) {
    match event {
        WindowEvent::Resized(size) => {
            // 主窗口大小变化时，同步子窗口大小
            #[cfg(windows)]
            {
                let child_window = CHILD_WINDOW.lock().unwrap();
                if let Some(ref cw) = *child_window {
                    // 使用保存的边距
                    let (top, left, right, bottom) = *CURRENT_MARGINS.lock().unwrap();
                    let (x, y, width, height) = calculate_vulkan_region_with_margins(
                        size.width as i32,
                        size.height as i32,
                        top,
                        left,
                        right,
                        bottom,
                    );

                    if let Err(e) = cw.set_position(x, y, width, height) {
                        eprintln!("Failed to resize child window: {}", e);
                    } else {
                        // 通知渲染线程窗口大小变化
                        let render_thread = RENDER_THREAD.lock().unwrap();
                        if let Some(ref thread) = *render_thread {
                            thread.send_event(TauriEventAdapter::from_resized(
                                width as u32,
                                height as u32,
                            ));
                        }
                    }
                }
                
                // 注意：新布局下 WebView 覆盖整个窗口，不再需要手动调整 bounds
                // 因为我们使用 auto_resize = true 让 WebView 自动跟随窗口大小
            }
        }
        WindowEvent::Focused(focused) => {
            println!("Main window focused: {}", focused);
        }
        _ => {}
    }
}
