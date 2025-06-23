use crate::gui::gui::Gui;
use crate::pipeline_settings::DefaultRendererSettings;
use crate::renderer::swapchain::RenderSwapchain;
use ash::vk;
use derive_getters::Getters;
use std::rc::Rc;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::command_pool::RhiCommandPool;
use truvis_rhi::core::command_queue::RhiQueue;
use truvis_rhi::rhi::Rhi;
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

    swapchain: RenderSwapchain,
    gui: Gui,

    cmd_buffer: RhiCommandBuffer,
    command_pool: RhiCommandPool,
    present_queue: Rc<RhiQueue>,

    width: i32,
    height: i32,
}

impl MainWindow {
    pub fn new(event_loop: &ActiveEventLoop, rhi: &Rhi, create_info: WindowCreateInfo) -> Self {
        let icon = load_icon(include_bytes!("../../resources/DruvisIII.png"));
        let window_attr = Window::default_attributes()
            .with_title(create_info.title.clone())
            .with_window_icon(Some(icon.clone()))
            .with_taskbar_icon(Some(icon.clone()))
            .with_transparent(true)
            .with_inner_size(winit::dpi::LogicalSize::new(f64::from(create_info.width), f64::from(create_info.height)));

        let window = event_loop.create_window(window_attr).unwrap();

        let swapchain = RenderSwapchain::new(
            rhi,
            &window,
            DefaultRendererSettings::DEFAULT_PRESENT_MODE,
            DefaultRendererSettings::DEFAULT_SURFACE_FORMAT,
        );

        let present_queue = rhi.present_queue.clone();

        let present_command_pool = RhiCommandPool::new(
            rhi.device.clone(),
            present_queue.queue_family().clone(),
            vk::CommandPoolCreateFlags::empty(),
            "window-present",
        );
        
        let cmd_buffer = 

        Self {
            window,
            swapchain,
            width: create_info.width,
            height: create_info.height,
        }
    }

    pub fn on_window_resize(&mut self, width: u32, height: u32) {
        self.width = width as i32;
        self.height = height as i32;
    }
}
