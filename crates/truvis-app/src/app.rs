use std::{cell::OnceCell, ffi::CStr, sync::OnceLock};

use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, StartCause, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::outer_app::OuterApp;
use crate::platform::camera_controller::CameraController;
use crate::render_app::RenderApp;
use crate::window_system::main_window::MainWindow;
use truvis_crate_tools::init_log::init_log;
use truvis_gfx::commands::barrier::GfxBarrierMask;
use truvis_gfx::gfx::Gfx;
use truvis_render_core::core::renderer::Renderer;
use truvis_render_core::platform::input_manager::InputManager;
use truvis_render_core::present::render_present::PresentData;

pub fn panic_handler(info: &std::panic::PanicHookInfo) {
    log::error!("{}", info);
    // std::thread::sleep(std::time::Duration::from_secs(30));
}

pub struct UserEvent;

/// Truvis 应用主结构
///
/// 封装了完整的渲染应用框架，包括窗口系统、渲染器、输入管理、相机控制等。
/// 通过泛型参数 `T: OuterApp` 集成用户自定义应用逻辑。
///
/// # Frames in Flight
/// 采用 3 帧并行渲染（A/B/C），通过 timeline semaphore 同步 GPU 进度。
pub struct TruvisApp<T: OuterApp> {
    render_app: RenderApp<T>,

    /// 需要等待窗口事件初始化，因此 OnceCell
    window_system: OnceCell<MainWindow>,
    last_render_area: vk::Extent2D,

    input_manager: InputManager,
}

// 总的 main 函数
impl<T: OuterApp> TruvisApp<T> {
    /// 整个程序的入口
    pub fn run() {
        std::panic::set_hook(Box::new(panic_handler));

        init_log();
        tracy_client::Client::start();
        tracy_client::set_thread_name!("MiaowThread");

        // 创建输入管理器和计时器
        let input_manager = InputManager::new();

        // 创建相机控制器
        let camera_controller = CameraController::new();

        let event_loop = winit::event_loop::EventLoop::<UserEvent>::with_user_event().build().unwrap();

        // 追加 window system 需要的 extension，在 windows 下也就是 khr::Surface
        let extra_instance_ext =
            ash_window::enumerate_required_extensions(event_loop.display_handle().unwrap().as_raw())
                .unwrap()
                .iter()
                .map(|ext| unsafe { CStr::from_ptr(*ext) })
                .collect();

        let mut app = Self {
            render_app: RenderApp::new(extra_instance_ext),
            window_system: OnceCell::new(),
            last_render_area: Default::default(),
            input_manager,
        };
        event_loop.run_app(&mut app).unwrap();

        log::info!("end run.");

        app.destroy();

        Gfx::destroy();
    }
}

impl<T: OuterApp> TruvisApp<T> {
    /// 在 window 创建之后调用，初始化 Renderer 和 GUI
    fn init_after_window(&mut self, event_loop: &ActiveEventLoop) {
        let mut window_system = MainWindow::new(
            event_loop,
            "Truvis".to_string(),
            vk::Extent2D {
                width: 1200,
                height: 800,
            },
        );
        self.render_app.init_after_window(
            window_system.window().display_handle().unwrap().as_raw(),
            window_system.window().window_handle().unwrap().as_raw(),
        );

        window_system.init_with_gui_backend(
            &mut self.render_app.renderer.render_present.as_mut().unwrap().gui_backend,
            &mut self.render_app.renderer.render_context.bindless_manager,
            &mut self.render_app.renderer.render_context.gfx_resource_manager,
        );

        self.window_system.set(window_system).map_err(|_| ()).unwrap();
    }

    fn update(&mut self) {
        // Begin Frame ============================
        if !self.render_app.time_to_render() {
            return;
        }
        let elapsed = self.render_app.get_delta_time();

        self.render_app.begin_frame();

        // Update Gui ==================================
        {
            let _span = tracy_client::span!("Update Gui");
            self.window_system.get_mut().unwrap().update_gui(elapsed, |ui| {
                self.render_app.build_ui(ui);
            });
        }

        // Rendere Update ==================================
        {
            let _span = tracy_client::span!("Renderer Update");
            self.last_render_area = self.window_system.get().unwrap().get_render_extent();
            self.input_manager.update();
            self.render_app.update_scene(self.input_manager.state(), self.last_render_area);
        }

        // Renderer: Render ================================
        self.render_app.render(self.input_manager.state());

        // Window: Draw Gui ===============================
        self.render_app.draw_to_window(self.window_system.get_mut().unwrap().gui_host.compile_ui());

        // End Frame ===================================
        self.render_app.end_frame();

        tracy_client::frame_mark();
    }

    fn on_window_resized(&mut self, _width: u32, _height: u32) {
        let window = self.window_system.get().unwrap().window();
        self.render_app
            .on_window_resized(window.display_handle().unwrap().as_raw(), window.window_handle().unwrap().as_raw());
    }
}

// 手动 drop
impl<T: OuterApp> TruvisApp<T> {
    fn destroy(mut self) {
        self.render_app.destroy();
        self.window_system.take().unwrap().destroy();
    }
}

impl<T: OuterApp> ApplicationHandler<UserEvent> for TruvisApp<T> {
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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        self.window_system.get_mut().unwrap().handle_event::<UserEvent>(&winit::event::Event::WindowEvent {
            window_id,
            event: event.clone(),
        });

        // FIXME 这一部分应该接收 imgui 的事件
        // 使用InputManager处理窗口事件
        self.input_manager.handle_window_event(&event);

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

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) {
        // 使用InputManager处理设备事件
        self.input_manager.handle_device_event(&event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.window_system.get().unwrap().window().request_redraw();
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
