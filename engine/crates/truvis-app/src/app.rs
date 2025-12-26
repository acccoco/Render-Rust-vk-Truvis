use std::sync::OnceLock;

use crate::outer_app::OuterApp;
use crate::render_app::RenderApp;
use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use truvis_crate_tools::init_log::init_log;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::gfx::Gfx;
use truvis_render_core::platform::winit_event_adapter::WinitEventAdapter;
use winit::platform::windows::WindowAttributesExtWindows;
use winit::window::Window;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, StartCause, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
};

pub fn panic_handler(info: &std::panic::PanicHookInfo) {
    log::error!("{}", info);
    // std::thread::sleep(std::time::Duration::from_secs(30));
}

pub struct UserEvent;

pub struct WinitApp {
    render_app: RenderApp,

    window: Option<Window>,
}
// 总的 main 函数
impl WinitApp {
    /// 整个程序的入口
    pub fn run(outer_app: Box<dyn OuterApp>) {
        std::panic::set_hook(Box::new(panic_handler));

        init_log();
        tracy_client::Client::start();
        tracy_client::set_thread_name!("MiaowThread");

        let event_loop = winit::event_loop::EventLoop::<UserEvent>::with_user_event().build().unwrap();

        let mut app = Self {
            render_app: RenderApp::new(event_loop.display_handle().unwrap().as_raw(), outer_app),
            window: None,
        };
        event_loop.run_app(&mut app).unwrap();

        log::info!("end run.");

        app.destroy();
    }
}
// new & init
impl WinitApp {
    /// 在 window 创建之后调用，初始化 Renderer 和 GUI
    fn init_after_window(&mut self, event_loop: &ActiveEventLoop) {
        let window = Self::create_window(
            event_loop,
            "Truvis".to_string(),
            vk::Extent2D {
                width: 1200,
                height: 800,
            },
        );

        self.render_app.init_after_window(
            window.display_handle().unwrap().as_raw(),
            window.window_handle().unwrap().as_raw(),
            window.scale_factor(),
        );

        self.window = Some(window);
    }

    fn create_window(event_loop: &ActiveEventLoop, window_title: String, window_extent: vk::Extent2D) -> Window {
        fn load_icon(bytes: &[u8]) -> winit::window::Icon {
            let (icon_rgba, icon_width, icon_height) = {
                let image = image::load_from_memory(bytes).unwrap().into_rgba8();
                let (width, height) = image.dimensions();
                let rgba = image.into_raw();
                (rgba, width, height)
            };
            winit::window::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
        }

        let icon_data =
            std::fs::read(TruvisPath::resources_path_str("DruvisIII.png")).expect("Failed to read icon file");
        let icon = load_icon(icon_data.as_ref());
        let window_attr = Window::default_attributes()
            .with_title(window_title)
            .with_window_icon(Some(icon.clone()))
            .with_taskbar_icon(Some(icon.clone()))
            .with_transparent(true)
            .with_inner_size(winit::dpi::LogicalSize::new(window_extent.width as f64, window_extent.height as f64));

        event_loop.create_window(window_attr).unwrap()
    }
}
// update
impl WinitApp {
    fn update(&mut self) {
        self.render_app.big_update();
    }

    fn on_window_resized(&mut self, _width: u32, _height: u32) {
        let window = self.window.as_ref().unwrap();
        self.render_app
            .on_window_resized(window.display_handle().unwrap().as_raw(), window.window_handle().unwrap().as_raw());
    }
}
// destroy
impl WinitApp {
    fn destroy(mut self) {
        self.render_app.destroy();
        self.window = None;
    }
}
// 各种 winit 的事件处理
impl ApplicationHandler<UserEvent> for WinitApp {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        // TODO 确认一下发送时机
        // TODO 可以在此处更新 timer
    }

    // 建议在这里创建 window 和 Renderer
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        static INIT_FLAG: OnceLock<bool> = OnceLock::new();

        log::info!("winit event: resumed");

        self.init_after_window(event_loop);
        INIT_FLAG.set(true).unwrap();
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: UserEvent) {
        todo!()
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let input_event = WinitEventAdapter::from_winit_event(&event);
        self.render_app.handle_event(&input_event);

        // TODO 可以放到 render app 里面去处理，加入队列中
        match event {
            WindowEvent::CloseRequested => {
                Gfx::get().wait_idel();
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                // log::info!("window was resized, new size is : {}x{}", new_size.width,
                // new_size.height);
                self.on_window_resized(new_size.width, new_size.height);
            }
            WindowEvent::RedrawRequested => {
                self.update();
                // TODO 是否应该手动调用 redraw，实现死循环？
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, _event: DeviceEvent) {
        // 使用InputManager处理设备事件
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.window.as_ref().unwrap().request_redraw();
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        log::warn!("winit event: suspended");
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        log::info!("loop exiting");
    }

    fn memory_warning(&mut self, _event_loop: &ActiveEventLoop) {
        log::warn!("memory warning");
    }
}
