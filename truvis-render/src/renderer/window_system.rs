use derive_getters::Getters;
use winit::{event_loop::ActiveEventLoop, platform::windows::WindowAttributesExtWindows, window::Window};

fn load_icon(bytes: &[u8]) -> winit::window::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(bytes).unwrap().into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    winit::window::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}

pub struct WindowCreateInfo {
    pub width: i32,
    pub height: i32,
    pub title: String,
}

#[derive(Getters)]
pub struct MainWindow {
    window: Window,

    width: i32,
    height: i32,
}

impl MainWindow {
    pub fn new(event_loop: &ActiveEventLoop, create_info: WindowCreateInfo) -> Self {
        let icon = load_icon(include_bytes!("../../resources/DruvisIII.png"));
        let window_attr = Window::default_attributes()
            .with_title(create_info.title.clone())
            .with_window_icon(Some(icon.clone()))
            .with_taskbar_icon(Some(icon.clone()))
            .with_transparent(true)
            .with_inner_size(winit::dpi::LogicalSize::new(f64::from(create_info.width), f64::from(create_info.height)));

        let window = event_loop.create_window(window_attr).unwrap();

        Self {
            window,
            width: create_info.width,
            height: create_info.height,
        }
    }
}
