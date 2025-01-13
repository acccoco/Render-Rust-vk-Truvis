//! 窗口，输入输出管理

use std::cell::RefCell;

use derive_getters::Getters;
use winit::{
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    platform::run_on_demand::EventLoopExtRunOnDemand,
    window::{Window, WindowAttributes},
};

use crate::framework::{platform::ui::UI, rendering::render_context::RenderContext, rhi::Rhi};

pub struct WindowCreateInfo
{
    pub width: i32,
    pub height: i32,
    pub title: String,
}

type OnResetFnc = fn() -> ();
type OnKeyFunc = fn(i32, i32, i32, i32) -> ();
type OnCharFunc = fn(u32) -> ();
type OnCharModsFunc = fn(i32, u32) -> ();
type OnMouseButtonFunc = fn(i32, i32, i32) -> ();
type OnCursorPosFunc = fn(f64, f64) -> ();
type OnCursorEnterFunc = fn(i32) -> ();
type OnScrollFunc = fn(f64, f64) -> ();
type OnDropFunc = fn(i32, String) -> ();
type OnWindowSizeFunc = fn(i32, i32) -> ();
type OnWindowCloseFunc = fn() -> ();


#[derive(Getters)]
pub struct WindowSystem
{
    window: Window,

    #[getter(skip)]
    pub event_loop: RefCell<EventLoop<()>>,

    width: i32,
    height: i32,

    is_focus_mode: bool,

    #[getter(skip)]
    events: WindowSystemEvents,
}

/// 在 window system 中注册的各种事件
#[derive(Default)]
struct WindowSystemEvents
{
    on_reset_funcs: Vec<OnResetFnc>,
    on_key_funcs: Vec<OnKeyFunc>,
    on_char_funcs: Vec<OnCharFunc>,
    on_charmods_funcs: Vec<OnCharModsFunc>,
    on_mousebutton_funcs: Vec<OnMouseButtonFunc>,
    on_cursorpos_funcs: Vec<OnCursorPosFunc>,
    on_cursorenter_funcs: Vec<OnCursorEnterFunc>,
    on_scroll_funcs: Vec<OnScrollFunc>,
    on_drop_funcs: Vec<OnDropFunc>,
    on_windowsize_funcs: Vec<OnWindowSizeFunc>,
    on_windowclose_funcs: Vec<OnWindowCloseFunc>,
}

impl WindowSystem
{
    pub fn new(create_info: WindowCreateInfo) -> Self
    {
        let event_loop = EventLoop::new().unwrap();

        let window_attr = WindowAttributes::new()
            .with_title(create_info.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(f64::from(create_info.width), f64::from(create_info.height)));

        // TODO 需要参考 winit 的 example 去做。似乎 window 是在 event loop 中创建的，无法主动创建
        let window = event_loop.create_window(window_attr).unwrap();

        Self {
            window,
            event_loop: RefCell::new(event_loop),
            width: create_info.width,
            height: create_info.height,
            is_focus_mode: false,
            events: Default::default(),
        }
    }

    // TODO 改用最新的回调模式
    // FIXME 不应该传入 ui 参数，而是应该对事件进行包装，让外部在对应的事件中进行处理。window 不应该关心什么时候绘制 ui
    pub fn render_loop<F>(&self, ui: &mut UI, mut f: F)
    where
        F: FnMut(&mut UI),
    {
        self.event_loop
            .borrow_mut()
            .run_on_demand({
                let mut last_frame = std::time::Instant::now();

                move |event, _active_event_loop| {
                    ui.platform.handle_event(ui.imgui.get_mut().io_mut(), &self.window, &event);

                    match event {
                        winit::event::Event::NewEvents(_) => {
                            let now = std::time::Instant::now();
                            ui.imgui.get_mut().io_mut().update_delta_time(now - last_frame);
                            last_frame = now;
                        }
                        winit::event::Event::AboutToWait => {
                            f(ui);
                        }

                        // 这个只是打算退出
                        winit::event::Event::WindowEvent {
                            event: winit::event::WindowEvent::CloseRequested,
                            ..
                        } => {
                            // TODO exit
                            _active_event_loop.exit();
                        }
                        winit::event::Event::WindowEvent {
                            event: winit::event::WindowEvent::Resized(new_size),
                            ..
                        } => {
                            log::info!("window was resized, new size is : {}x{}", new_size.width, new_size.height);
                        }
                        winit::event::Event::WindowEvent {
                            event: winit::event::WindowEvent::RedrawRequested,
                            ..
                        } => (),
                        winit::event::Event::WindowEvent { .. } => (),

                        // 这个应该是真正的退出
                        winit::event::Event::LoopExiting => {
                            log::info!("loop exiting");
                            // TODO cleanup
                        }
                        _ => {}
                    }
                }
            })
            .expect("TODO: panic message");
    }
}
