use crate::gui::ui::Gui;
use crate::platform::camera::DrsCamera;
use crate::platform::camera_controller::CameraController;
use crate::platform::input_manager::InputManager;
use crate::platform::timer::Timer;
use crate::render::Renderer;
use crate::renderer::window_system::{MainWindow, WindowCreateInfo};
use raw_window_handle::HasDisplayHandle;
use std::cell::{OnceCell, RefCell};
use std::ffi::CStr;
use std::rc::Rc;
use std::sync::OnceLock;
use truvis_crate_tools::init_log::init_log;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

pub fn panic_handler(info: &std::panic::PanicHookInfo) {
    log::error!("{}", info);
    // std::thread::sleep(std::time::Duration::from_secs(30));
}

pub trait OuterApp {
    fn init(renderer: &mut Renderer, camera: &mut DrsCamera) -> Self;

    fn draw_ui(&mut self, ui: &mut imgui::Ui);

    fn update(&mut self, _renderer: &mut Renderer) {}

    /// 发生于 acquire_frame 之后，submit_frame 之前
    fn draw(&self, _renderer: &mut Renderer, _timer: &Timer) {
        // 默认不做任何事情
    }

    /// window 发生改变后，重建
    fn rebuild(&mut self, _renderer: &mut Renderer) {}
}

pub struct TruvisApp<T: OuterApp> {
    renderer: Renderer,

    /// 需要等待窗口事件初始化，因此 OnceCell
    window_system: OnceCell<MainWindow>,

    input_manager: Rc<RefCell<InputManager>>,

    /// 需要在 window 之后初始化，因此 OnceCell
    gui: OnceCell<Gui>,

    timer: Timer,

    camera_controller: CameraController,

    outer_app: OnceCell<T>,
}

impl<T: OuterApp> Drop for TruvisApp<T> {
    fn drop(&mut self) {
        // 在 TruvisApp 被销毁时，等待 Renderer 设备空闲
        self.renderer.wait_idle();
    }
}

pub struct UserEvent;

impl<T: OuterApp> TruvisApp<T> {
    /// 整个程序的入口
    pub fn run() {
        std::panic::set_hook(Box::new(panic_handler));

        init_log();

        // 创建输入管理器和计时器
        let input_manager = Rc::new(RefCell::new(InputManager::new()));
        let timer = Timer::default();

        // 创建相机控制器
        let camera_controller = CameraController::new(DrsCamera::default(), input_manager.clone());

        let event_loop = winit::event_loop::EventLoop::<UserEvent>::with_user_event().build().unwrap();

        // 追加 window system 需要的 extension，在 windows 下也就是 khr::Surface
        let extra_instance_ext =
            ash_window::enumerate_required_extensions(event_loop.display_handle().unwrap().as_raw())
                .unwrap()
                .iter()
                .map(|ext| unsafe { CStr::from_ptr(*ext) })
                .collect();

        let mut app = Self {
            renderer: Renderer::new(extra_instance_ext),
            window_system: OnceCell::new(),
            input_manager,
            gui: OnceCell::new(),
            timer,
            camera_controller,
            outer_app: OnceCell::new(),
        };
        event_loop.run_app(&mut app).unwrap();

        log::info!("end run.");
    }

    /// 在 window 创建之后调用，初始化 Renderer 和 GUI
    pub fn init_after_window(&mut self, event_loop: &ActiveEventLoop) {
        let window_init_info = WindowCreateInfo {
            height: 800,
            width: 800,
            title: "Truvis".to_string(),
        };

        let window_system = MainWindow::new(event_loop, window_init_info);
        self.renderer.init_after_window(&window_system);
        let gui = Gui::new(
            &self.renderer.rhi,
            window_system.window(),
            &self.renderer.pipeline_settings(),
            self.renderer.bindless_mgr.clone(),
        );

        let outer_app = T::init(&mut self.renderer, self.camera_controller.camera_mut());

        self.window_system.set(window_system).map_err(|_| ()).unwrap();
        self.gui.set(gui).map_err(|_| ()).unwrap();
        self.outer_app.set(outer_app).map_err(|_| ()).unwrap();

        self.timer.reset();
    }

    pub fn update(&mut self) {
        // ===================== Phase: Begin Frame =====================
        self.renderer.begin_frame();

        // ===================== Phase: IO =====================
        // 更新计时器
        self.timer.update();
        let duration = std::time::Duration::from_secs_f32(self.timer.delta_time_s);
        self.gui.get_mut().unwrap().prepare_frame(self.window_system.get().unwrap().window(), duration);

        // 更新输入状态
        self.input_manager.borrow_mut().update();

        // 更新相机控制器
        self.camera_controller.update(self.timer.delta_time_s);

        // ===================== Phase: Update =====================
        self.gui.get_mut().unwrap().update(self.window_system.get().unwrap().window(), |ui| {
            self.outer_app.get_mut().unwrap().draw_ui(ui);
        });
        self.outer_app.get_mut().unwrap().update(&mut self.renderer);

        // ===================== Phase: Before Render =====================
        self.renderer.before_render(&self.input_manager.borrow().state, &self.timer, self.camera_controller.camera());

        // ===================== Phase: Render =====================
        self.outer_app.get_mut().unwrap().draw(&mut self.renderer, &self.timer);

        // ===================== Phase: After Render =====================
        self.renderer.after_render();
        let pipeline_settings = self.renderer.pipeline_settings();
        self.gui.get_mut().unwrap().render(
            &self.renderer.rhi,
            self.renderer.render_context.as_mut().unwrap(),
            self.renderer.render_swapchain.as_mut().unwrap(),
            &pipeline_settings.frame_settings,
        );

        // ===================== Phase: End Frame =====================
        self.renderer.end_frame();
    }

    pub fn rebuild(&mut self, width: u32, height: u32) {
        self.renderer.wait_idle();

        log::info!("try to rebuild render context");
        self.renderer.rebuild_after_resized(self.window_system.get().unwrap());

        // 更新相机的宽高比
        self.camera_controller.camera_mut().asp = width as f32 / height as f32;

        self.outer_app.get_mut().unwrap().rebuild(&mut self.renderer);
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
        self.gui.get_mut().unwrap().handle_event::<UserEvent>(
            self.window_system.get().unwrap().window(),
            &winit::event::Event::WindowEvent {
                window_id,
                event: event.clone(),
            },
        );

        // 使用InputManager处理窗口事件
        self.input_manager.borrow_mut().handle_window_event(&event);

        match event {
            WindowEvent::CloseRequested => {
                self.renderer.wait_idle();
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                log::info!("window was resized, new size is : {}x{}", new_size.width, new_size.height);
                self.rebuild(new_size.width, new_size.height);
            }
            WindowEvent::RedrawRequested => {
                self.update();
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) {
        // 使用InputManager处理设备事件
        self.input_manager.borrow_mut().handle_device_event(&event);
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
