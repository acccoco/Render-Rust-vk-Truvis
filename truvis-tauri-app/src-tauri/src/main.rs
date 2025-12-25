// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tauri::Manager;
use tauri::window::WindowBuilder;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 获取主窗口 (Tauri 2 默认会创建 "main" 窗口)
            let main_window = app.get_webview_window("main")
                .expect("Failed to get main window");
            
            // 创建一个新的纯窗口（没有 WebView），用于 Vulkan 渲染
            let render_window = WindowBuilder::new(app, "render")
                .title("Render Window")
                .inner_size(1280.0, 720.0)
                .always_on_top(true)           // 始终浮在顶层
                .decorations(true)              // 有窗口装饰
                .resizable(true)                // 可调整大小
                .visible(true)                  // 可见
                .build()
                .expect("Failed to create render window");
            
            // 获取 render 窗口的 raw-window-handle
            let window_handle = render_window.window_handle()
                .expect("Failed to get window handle");
            let display_handle = render_window.display_handle()
                .expect("Failed to get display handle");
            
            println!("Render Window Handle: {:?}", window_handle.as_raw());
            println!("Render Display Handle: {:?}", display_handle.as_raw());
            
            // 初始化 Vulkan 渲染器
            init_vulkan_renderer(&render_window);
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_vulkan_renderer(window: &tauri::Window) {
    // 获取窗口句柄用于 Vulkan
    let window_handle = window.window_handle()
        .expect("Failed to get window handle");
    let display_handle = window.display_handle()
        .expect("Failed to get display handle");
    
    // 获取窗口尺寸
    let size = window.inner_size().unwrap_or(tauri::PhysicalSize { width: 800, height: 600 });
    
    println!("初始化 Vulkan 渲染器...");
    println!("窗口尺寸: {}x{}", size.width, size.height);
    println!("RawWindowHandle: {:?}", window_handle.as_raw());
    println!("RawDisplayHandle: {:?}", display_handle.as_raw());
    
    // TODO: 在这里创建 Vulkan instance, surface, device 等
    // 使用 ash 或 vulkano 时，可以这样创建 surface:
    //
    // use ash::vk;
    // use raw_window_handle::{RawWindowHandle, RawDisplayHandle};
    //
    // match (window_handle.as_raw(), display_handle.as_raw()) {
    //     (RawWindowHandle::Win32(handle), _) => {
    //         // Windows: 使用 vkCreateWin32SurfaceKHR
    //         let hwnd = handle.hwnd.get();
    //         let hinstance = handle.hinstance.unwrap().get();
    //         // ...
    //     }
    //     _ => panic!("Unsupported platform"),
    // }
}
