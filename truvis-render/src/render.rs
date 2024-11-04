use std::rc::Rc;

use crate::{
    framework::{
        core::swapchain::{RenderSwapchain, RenderSwapchainInitInfo},
        platform::window_system::{WindowCreateInfo, WindowSystem},
        rendering::render_context::{RenderContext, RenderContextInitInfo},
        rhi::{vk_debug_callback, Rhi, RhiInitInfo},
    },
    render_init::ENGINE_NAME,
};


pub struct Timer
{
    pub start_time: std::time::SystemTime,
    pub last_time: std::time::SystemTime,
    pub delta_time: f32,
    pub total_time: f32,
    pub total_frame: i32,
}

impl Timer
{
    pub fn new() -> Self
    {
        Self {
            start_time: std::time::SystemTime::now(),
            last_time: std::time::SystemTime::now(),
            total_frame: 0,
            delta_time: 0.0,
            total_time: 0.0,
        }
    }

    pub fn reset(&mut self)
    {
        self.start_time = std::time::SystemTime::now();
        self.last_time = std::time::SystemTime::now();
        self.total_frame = 0;
        self.delta_time = 0.0;
        self.total_time = 0.0;
    }

    pub fn update(&mut self)
    {
        let now = std::time::SystemTime::now();
        let total_time = now.duration_since(self.start_time).unwrap().as_secs_f32();
        let delta_time = now.duration_since(self.last_time).unwrap().as_secs_f32();
        self.last_time = now;
        self.total_frame += 1;
        self.total_time = total_time;
        self.delta_time = delta_time;
    }
}


/// 表示整个渲染器进程，需要考虑 platform, render, rhi, log 之类的各种模块
pub struct Renderer
{
    pub timer: Timer,
    pub window: WindowSystem,
    render_swapchain: Rc<RenderSwapchain>,
    pub render_context: RenderContext,
}


pub struct RenderInitInfo
{
    pub window_width: u32,
    pub window_height: u32,
    pub app_name: String,
}

static mut RHI: Option<Rhi> = None;

impl Renderer
{
    pub fn init_logger()
    {
        use simplelog::*;

        TermLogger::init(LevelFilter::Info, ConfigBuilder::new().build(), TerminalMode::Mixed, ColorChoice::Auto)
            .unwrap();
    }

    pub fn new(init_info: &RenderInitInfo) -> Self
    {
        Self::init_logger();

        let window = WindowSystem::new(WindowCreateInfo {
            height: init_info.window_height as i32,
            width: init_info.window_width as i32,
            title: init_info.app_name.clone(),
        });

        let mut rhi_init_info = RhiInitInfo::init_basic(Some(vk_debug_callback), &window);
        rhi_init_info.app_name = Some(init_info.app_name.clone());
        rhi_init_info.engine_name = Some(ENGINE_NAME.to_string());
        rhi_init_info.is_complete().unwrap();
        unsafe {
            RHI = Some(Rhi::new(rhi_init_info).unwrap());
        }
        let rhi = unsafe { RHI.as_ref().unwrap() };

        let mut render_swapchain_init_info = RenderSwapchainInitInfo::default();
        render_swapchain_init_info.window = Some(&window);
        let render_swapchain = Rc::new(RenderSwapchain::new(&rhi, &render_swapchain_init_info));

        let render_context_init_info = RenderContextInitInfo::default();
        let render_context = RenderContext::new(&rhi, &render_context_init_info, render_swapchain.clone());


        Self {
            window,
            render_swapchain,
            render_context,
            timer: Timer::new(),
        }
    }

    pub fn get_rhi() -> &'static Rhi
    {
        unsafe { RHI.as_ref().unwrap() }
    }
}
