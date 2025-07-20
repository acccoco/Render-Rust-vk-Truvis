use crate::outer_app::OuterApp;
use crate::platform::camera::DrsCamera;
use crate::platform::camera_controller::CameraController;
use crate::platform::input_manager::InputManager;
use crate::renderer::renderer::Renderer;
use crate::window_system::main_window::MainWindow;
use ash::vk;
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

pub struct UserEvent;

pub struct TruvisApp<T: OuterApp> {
    renderer: Renderer,

    /// 需要等待窗口事件初始化，因此 OnceCell
    window_system: OnceCell<MainWindow>,
    last_render_area: vk::Extent2D,

    input_manager: Rc<RefCell<InputManager>>,

    camera_controller: CameraController,

    outer_app: OnceCell<T>,
}

// 总的 main 函数
impl<T: OuterApp> TruvisApp<T> {
    /// 整个程序的入口
    pub fn run() {
        std::panic::set_hook(Box::new(panic_handler));

        init_log();

        // 创建输入管理器和计时器
        let input_manager = Rc::new(RefCell::new(InputManager::new()));

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
            last_render_area: Default::default(),
            input_manager,
            camera_controller,
            outer_app: OnceCell::new(),
        };
        event_loop.run_app(&mut app).unwrap();

        log::info!("end run.");

        app.destroy();
    }
}

impl<T: OuterApp> TruvisApp<T> {
    /// 在 window 创建之后调用，初始化 Renderer 和 GUI
    fn init_after_window(&mut self, event_loop: &ActiveEventLoop) {
        let window_system = MainWindow::new(
            event_loop,
            self.renderer.rhi.clone(),
            self.renderer.frame_settings().fif_num,
            "Truvis".to_string(),
            vk::Extent2D {
                width: 1200,
                height: 800,
            },
            self.renderer.bindless_mgr.clone(),
        );

        let outer_app = T::init(&mut self.renderer, self.camera_controller.camera_mut());

        self.window_system.set(window_system).map_err(|_| ()).unwrap();
        self.outer_app.set(outer_app).map_err(|_| ()).unwrap();
    }

    fn update(&mut self) {
        // Begin Frame ============================
        if !self.renderer.time_to_render() {
            return;
        }

        self.renderer.begin_frame();
        let frame_label = self.renderer.frame_controller().frame_label();
        let elapsed = self.renderer.deltatime();

        self.window_system.get_mut().unwrap().acquire_image(frame_label);

        // Update Gui ==================================
        {
            self.window_system.get_mut().unwrap().update_gui(elapsed, |ui| {
                self.outer_app.get_mut().unwrap().draw_ui(ui);
            });
        }

        // Rendere Update ==================================
        {
            // Renderer: Resize Framebuffer
            {
                let extent = self.window_system.get().unwrap().get_render_extent();
                if self.last_render_area != extent {
                    log::info!("resize frame buffer to: {}x{}", extent.width, extent.height);
                    self.renderer.resize_frame_buffer(extent);
                    self.last_render_area = extent;
                }
            }

            // Renderer: Update Input and Camera
            {
                // TODO 这个 input manager, 以及 camera controller
                //  应该是只服务于 renderer 的，因此更新频率也应该和 renderer 一致
                //  将其移动到 Renderer 中
                self.input_manager.borrow_mut().update();
                self.camera_controller.update(self.renderer.deltatime());
            }

            // Outer App: Update
            {
                self.outer_app.get_mut().unwrap().update(&mut self.renderer);
            }
        }

        // Renderer Render ==================================
        {
            // Renderer: Before Render
            {
                self.renderer.before_render(&self.input_manager.borrow().state, self.camera_controller.camera());
            }

            // >>> Renderer: Render
            {
                // 构建出 PipelineContext
                let pipeline_ctx = self.renderer.collect_render_ctx();
                self.outer_app.get_mut().unwrap().draw(pipeline_ctx);
            }

            // >>> Renderer: After Render
            {
                self.renderer.after_render();
            }
        }

        // Window: Draw Gui ===============================
        {
            let present_data = self.renderer.get_renderer_data();
            self.window_system.get_mut().unwrap().draw_gui(present_data);
        }

        // End Frame ===================================
        {
            // >>> Window: Present Image
            {
                self.window_system.get_mut().unwrap().present_image();
            }

            // >>> Renderer: End Frame
            {
                self.renderer.end_frame();
            }
        }
    }

    fn on_window_resized(&mut self, width: u32, height: u32) {
        self.window_system.get_mut().unwrap().rebuild_after_resized();

        log::info!("try to rebuild render context");

        // TODO 这里使用 swapchian 的长宽？
        // 更新相机的宽高比
        self.camera_controller.camera_mut().asp = width as f32 / height as f32;

        self.outer_app.get_mut().unwrap().rebuild(&mut self.renderer);
    }
}

// 手动 drop
impl<T: OuterApp> TruvisApp<T> {
    fn destroy(mut self) {
        self.renderer.wait_idle();

        self.window_system.take().unwrap().destroy();
        self.renderer.destroy();
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
        self.input_manager.borrow_mut().handle_window_event(&event);

        match event {
            WindowEvent::CloseRequested => {
                self.renderer.wait_idle();
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                log::info!("window was resized, new size is : {}x{}", new_size.width, new_size.height);
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
