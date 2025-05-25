use crate::frame_context::FrameContext;
use crate::platform::camera::TruCamera;
use crate::platform::camera_controller::CameraController;
use crate::platform::input_manager::{InputManager, InputState};
use crate::platform::timer::Timer;
use crate::platform::ui::{Gui, UiCreateInfo};
use crate::render::Renderer;
use crate::renderer::acc_manager::AccManager;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::gpu_scene::GpuScene;
use crate::renderer::scene_manager::TheWorld;
use shader_binding::shader;
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;
use std::sync::OnceLock;
use truvis_crate_tools::init_log::init_log;
use truvis_rhi::core::buffer::RhiStructuredBuffer;
use truvis_rhi::core::window_system::{MainWindow, WindowCreateInfo};
use truvis_rhi::rhi::Rhi;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

/// 传递给 App 的上下文，用于 App 和 Renderer 之间的交互
pub struct AppCtx<'a> {
    pub rhi: &'a Rhi,
    pub render_context: &'a mut FrameContext,
    pub timer: &'a Timer,
    pub input_state: InputState,
    pub camera: &'a TruCamera,
}

pub fn panic_handler(info: &std::panic::PanicHookInfo) {
    log::error!("{}", info);
    std::thread::sleep(std::time::Duration::from_secs(30));
}

pub trait OuterApp {
    fn init(
        rhi: &Rhi,
        render_context: &mut FrameContext,
        scene_mgr: Rc<RefCell<TheWorld>>,
        bindless_mgr: Rc<RefCell<BindlessManager>>,
        acc_mgr: Rc<RefCell<AccManager>>,
    ) -> Self;

    fn draw_ui(&mut self, ui: &mut imgui::Ui);

    fn update(&mut self, _app_ctx: &mut AppCtx) {}

    /// 发生于 acquire_frame 之后，submit_frame 之前
    fn draw(
        &self,
        app_ctx: &mut AppCtx,
        per_frame_data_buffer: &RhiStructuredBuffer<shader::PerFrameData>,
        gpu_scene: &GpuScene,
    );

    /// window 发生改变后，重建
    fn rebuild(&mut self, _rhi: &Rhi, _render_context: &mut FrameContext) {}
}

pub struct TruvisApp<T: OuterApp> {
    renderer: OnceCell<Renderer>,
    window_system: OnceCell<Rc<MainWindow>>,
    input_manager: Rc<RefCell<InputManager>>,
    gui: OnceCell<Gui>,
    timer: Timer,
    camera_controller: CameraController,

    outer_app: OnceCell<T>,
}

impl<T: OuterApp> Drop for TruvisApp<T> {
    fn drop(&mut self) {
        log::info!("Dropping TruvisApp");
        // 在 TruvisApp 被销毁时，等待 Renderer 设备空闲
        if let Some(renderer) = self.renderer.get() {
            renderer.wait_idle();
        }
    }
}

pub struct UserEvent;

impl<T: OuterApp> TruvisApp<T> {
    pub fn run() {
        std::panic::set_hook(Box::new(panic_handler));

        init_log();

        // 创建输入管理器和计时器
        let input_manager = Rc::new(RefCell::new(InputManager::new()));
        let timer = Timer::default();

        // 创建相机控制器
        let camera_controller = CameraController::new(TruCamera::default(), input_manager.clone());

        let event_loop = winit::event_loop::EventLoop::<UserEvent>::with_user_event().build().unwrap();

        let mut app = Self {
            renderer: OnceCell::new(),
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
    pub fn init(&mut self, event_loop: &ActiveEventLoop) {
        // TODO 抽离出参数来
        let window_init_info = WindowCreateInfo {
            height: 800,
            width: 800,
            title: "Truvis".to_string(),
        };

        let window_system = Rc::new(MainWindow::new(event_loop, window_init_info));
        let mut renderer = Renderer::new(window_system.clone());
        let gui = Gui::new(
            &renderer.rhi,
            &renderer.render_context,
            window_system.window(),
            &UiCreateInfo {
                // FIXME 统一一下出现的位置。RenderContext 里面也有这个配置
                frames_in_flight: 3,
            },
        );
        let outer_app = T::init(
            &renderer.rhi,
            &mut renderer.render_context,
            renderer.scene_mgr.clone(),
            renderer.bindless_mgr.clone(),
            renderer.acc_mgr.clone(),
        );

        self.window_system.set(window_system).map_err(|_| ()).unwrap();
        self.renderer.set(renderer).map_err(|_| ()).unwrap();
        self.gui.set(gui).map_err(|_| ()).unwrap();
        self.outer_app.set(outer_app).map_err(|_| ()).unwrap();

        self.timer.reset();
    }
    pub fn update(&mut self) {
        self.timer.update();
        let duration = std::time::Duration::from_secs_f32(self.timer.delta_time_s);
        self.gui.get_mut().unwrap().context.get_mut().io_mut().update_delta_time(duration);

        // 更新输入状态
        self.input_manager.borrow_mut().update();

        self.camera_controller.update(self.timer.delta_time_s);

        let renderer = self.renderer.get_mut().unwrap();
        renderer.before_frame();
        {
            let crt_frame_label = renderer.render_context.current_frame_label();
            let mut app_ctx = AppCtx {
                rhi: &renderer.rhi,
                render_context: &mut renderer.render_context,
                timer: &self.timer,
                input_state: self.input_manager.borrow().state.clone(),
                camera: self.camera_controller.camera(),
            };
            self.outer_app.get_mut().unwrap().update(&mut app_ctx);

            renderer.update_gpu_scene(&self.input_manager.borrow().state, &self.timer, self.camera_controller.camera());
            renderer.before_render();
            {
                let mut app_ctx = AppCtx {
                    rhi: &renderer.rhi,
                    render_context: &mut renderer.render_context,
                    timer: &self.timer,
                    input_state: self.input_manager.borrow().state.clone(),
                    camera: self.camera_controller.camera(),
                };
                self.outer_app.get_mut().unwrap().draw(
                    &mut app_ctx,
                    &renderer.per_frame_data_buffers[crt_frame_label],
                    &renderer.gpu_scene,
                );
            }
            renderer.after_render();

            self.gui.get_mut().unwrap().draw(
                &renderer.rhi,
                &mut renderer.render_context,
                self.window_system.get().unwrap().window(),
                |imgui| {
                    self.outer_app.get_mut().unwrap().draw_ui(imgui);
                },
            );
        }
        renderer.after_frame();
    }
    pub fn rebuild(&mut self, width: u32, height: u32) {
        let renderer = self.renderer.get_mut().unwrap();
        let outer_app = self.outer_app.get_mut().unwrap();

        renderer.wait_idle();

        log::info!("try to rebuild render context");
        renderer.rebuild_render_context();

        // 更新相机的宽高比
        self.camera_controller.camera_mut().asp = width as f32 / height as f32;

        outer_app.rebuild(&renderer.rhi, &mut renderer.render_context);
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

        self.init(event_loop);
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
                self.renderer.get().unwrap().wait_idle();
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
