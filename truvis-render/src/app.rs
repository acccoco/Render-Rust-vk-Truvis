use crate::frame_context::FrameContext;
use crate::platform::input_manager::InputState;
use crate::platform::timer::Timer;
use crate::platform::ui::{Gui, UiCreateInfo};
use crate::render::Renderer;
use ash::vk;
use raw_window_handle::HasRawDisplayHandle;
use std::cell::{OnceCell, RefCell, RefMut};
use std::convert::identity;
use std::io::Write;
use std::rc::Rc;
use std::sync::OnceLock;
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
    pub input_state: &'a InputState,
}

pub fn panic_handler(info: &std::panic::PanicHookInfo) {
    log::error!("{}", info);
    // std::thread::sleep(std::time::Duration::from_secs(3));
}

pub trait OuterApp {
    fn init(rhi: &Rhi, render_context: &mut FrameContext) -> Self;

    fn draw_ui(&mut self, ui: &mut imgui::Ui);

    fn update(&mut self, app_ctx: &mut AppCtx);

    /// 发生于 acquire_frame 之后，submit_frame 之前
    fn draw(&self, app_ctx: &mut AppCtx);

    /// window 发生改变后，重建
    fn rebuild(&mut self, rhi: &Rhi, render_context: &mut FrameContext) {}
}

pub struct TruvisApp<T: OuterApp> {
    renderer: OnceCell<Renderer>,
    window_system: OnceCell<Rc<MainWindow>>,
    input_state: InputState,
    gui: OnceCell<Gui>,
    timer: Timer,

    outer_app: OnceCell<T>,
}

pub struct UserEvent;

impl<T: OuterApp> TruvisApp<T> {
    pub fn run() {
        std::panic::set_hook(Box::new(panic_handler));

        // init logger
        env_logger::Builder::new()
            .format(|buf, record| {
                let mut info_style = buf
                    .default_level_style(log::Level::Info)
                    .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)));
                let mut warn_style = buf
                    .default_level_style(log::Level::Warn)
                    .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
                let mut error_style = buf
                    .default_level_style(log::Level::Error)
                    .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red)));

                let style = match record.level() {
                    log::Level::Info => info_style,
                    log::Level::Warn => warn_style,
                    log::Level::Error => error_style,
                    _ => buf.default_level_style(record.level()),
                };

                writeln!(
                    buf,
                    "{style}{}:{} {} [{}]{style:#}\n{}",
                    record.file().unwrap(),
                    record.line().unwrap(),
                    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                    record.level(),
                    record.args()
                )
            })
            .filter(None, log::LevelFilter::Info)
            .init();

        let event_loop = winit::event_loop::EventLoop::<UserEvent>::with_user_event().build().unwrap();

        let mut app = Self {
            renderer: OnceCell::new(),
            window_system: OnceCell::new(),
            input_state: InputState::default(),
            gui: OnceCell::new(),
            timer: Timer::default(),
            outer_app: OnceCell::new(),
        };
        event_loop.run_app(&mut app).unwrap();
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
        let outer_app = T::init(&renderer.rhi, &mut renderer.render_context);

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

        let renderer = self.renderer.get_mut().unwrap();
        renderer.before_frame();
        {
            renderer.before_render();
            {
                let mut app_ctx = AppCtx {
                    rhi: &renderer.rhi,
                    render_context: &mut renderer.render_context,
                    timer: &self.timer,
                    input_state: &self.input_state,
                };
                self.outer_app.get_mut().unwrap().update(&mut app_ctx);
                self.outer_app.get_mut().unwrap().draw(&mut app_ctx);
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

        self.input_state.last_mouse_pos = self.input_state.crt_mouse_pos;
    }

    pub fn rebuild(&mut self, width: u32, height: u32) {
        let mut renderer = self.renderer.get_mut().unwrap();
        let mut outer_app = self.outer_app.get_mut().unwrap();

        renderer.wait_idle();

        renderer.rebuild_swapchain();

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
            WindowEvent::CursorMoved { position, .. } => {
                self.input_state.crt_mouse_pos = glam::dvec2(position.x, position.y);
            }
            WindowEvent::MouseWheel { .. } => {}
            WindowEvent::MouseInput { state, button, .. } => {
                if button == winit::event::MouseButton::Right {
                    self.input_state.right_button_pressed = state == winit::event::ElementState::Pressed;
                } else if button == winit::event::MouseButton::Middle {
                    self.rebuild(0, 0);
                }
            }
            WindowEvent::Focused(_) => {}
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                        state,
                        ..
                    },
                ..
            } => {
                self.input_state.key_pressed.insert(key_code, state == winit::event::ElementState::Pressed);
            }

            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, _event: DeviceEvent) {
        // todo!()
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
