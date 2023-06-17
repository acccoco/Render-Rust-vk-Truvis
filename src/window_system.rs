//! 窗口，输入输出管理

use std::cell::RefCell;

use derive_getters::Getters;
use winit::{
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::{Window, WindowBuilder},
};


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


static mut WINDOW_SYSTEM: Option<WindowSystem> = None;


#[derive(Getters)]
pub struct WindowSystem
{
    window: Window,

    #[getter(skip)]
    event_loop: RefCell<EventLoop<()>>,

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
    pub fn init(create_info: WindowCreateInfo)
    {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title(create_info.title)
            .with_inner_size(winit::dpi::LogicalSize::new(f64::from(create_info.width), f64::from(create_info.height)))
            .build(&event_loop)
            .unwrap();

        let window_system = Self {
            window,
            event_loop: RefCell::new(event_loop),
            width: create_info.width,
            height: create_info.height,
            is_focus_mode: false,
            events: Default::default(),
        };

        unsafe {
            WINDOW_SYSTEM = Some(window_system);
        }
    }

    #[inline]
    pub fn instance() -> &'static Self { unsafe { WINDOW_SYSTEM.as_ref().unwrap() } }

    pub fn render_loop(&self, f: impl Fn())
    {
        self.event_loop.borrow_mut().run_return(|event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            match event {
                // TODO 这里需要做的，应该是去记录事件。应该使用回调模式，还是使用查询模式？
                winit::event::Event::MainEventsCleared => f(),
                _ => (),
            }
        });
    }
}
