use std::sync::Arc;

use crate::framework::{
    core::{queue::RhiSubmitBatch, swapchain::RenderSwapchainInitInfo},
    platform::{
        ui::{UiOptions, UI},
        window_system::{WindowCreateInfo, WindowSystem},
    },
    rendering::render_context::{RenderContext, RenderContextInitInfo},
    rhi::{vk_debug_callback, Rhi, RhiInitInfo, RHI},
};

pub struct Timer
{
    pub start_time: std::time::SystemTime,
    pub last_time: std::time::SystemTime,
    pub delta_time: f32,
    pub total_time: f32,
    pub total_frame: i32,
}

impl Default for Timer
{
    fn default() -> Self
    {
        Self {
            start_time: std::time::SystemTime::now(),
            last_time: std::time::SystemTime::now(),
            total_frame: 0,
            delta_time: 0.0,
            total_time: 0.0,
        }
    }
}


impl Timer
{
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
    pub window: Arc<WindowSystem>,
    pub render_context: RenderContext,
    pub ui: UI,
}


pub struct AppInitInfo
{
    pub window_width: u32,
    pub window_height: u32,
    pub app_name: String,
    pub enable_validation: bool,
}


impl Renderer
{
    pub fn init_logger()
    {
        use simplelog::*;

        TermLogger::init(LevelFilter::Info, ConfigBuilder::new().build(), TerminalMode::Mixed, ColorChoice::Auto)
            .unwrap();
    }

    pub fn new(init_info: &AppInitInfo) -> Self
    {
        Self::init_logger();

        let window = WindowSystem::new(WindowCreateInfo {
            height: init_info.window_height as i32,
            width: init_info.window_width as i32,
            title: init_info.app_name.clone(),
        });
        let window = Arc::new(window);

        let mut rhi_init_info =
            RhiInitInfo::init_basic(init_info.app_name.clone(), window.clone(), init_info.enable_validation);
        rhi_init_info.set_debug_callback(Some(vk_debug_callback));
        let frames_in_flight = rhi_init_info.frames_in_flight;
        RHI.get_or_init(|| Rhi::new(rhi_init_info));
        let rhi = RHI.get().unwrap();

        let render_swapchain_init_info = RenderSwapchainInitInfo {
            window: Some(window.clone()),
            ..Default::default()
        };

        let render_context_init_info = RenderContextInitInfo::default();
        let render_context = RenderContext::new(rhi, &render_context_init_info, render_swapchain_init_info);

        let ui = UI::new(
            rhi,
            &render_context,
            &window.window(),
            &UiOptions {
                frames_in_flight: frames_in_flight as usize,
            },
        );

        Self {
            window,
            render_context,
            timer: Timer::default(),
            ui,
        }
    }

    pub fn get_rhi() -> &'static Rhi
    {
        RHI.get().unwrap()
    }

    pub fn render_loop<F, G>(&mut self, mut render_func: F, mut ui_builder: G)
    where
        F: FnMut(&'static Rhi, &mut RenderContext, &Timer) -> Vec<RhiSubmitBatch>,
        G: FnMut(&mut imgui::Ui) -> RhiSubmitBatch,
    {
        let rhi = Self::get_rhi();

        self.window.render_loop(&mut self.ui, |ui| {
            ui.platform.prepare_frame(ui.imgui.io_mut(), self.window.window()).unwrap();

            // draw under the UI
            // ..

            let frame = ui.imgui.new_frame();
            let ui_batch = ui_builder(frame);
            ui.platform.prepare_render(frame, self.window.window());
            let draw_data = ui.imgui.render();
            // renderer.render(.., draw_data);

            // draw over the UI

            // TODO render_func 应该返回多个 command buffer，然后统一提交


            self.timer.update();

            self.render_context.acquire_frame();

            let mut app_render_batches = render_func(rhi, &mut self.render_context, &self.timer);

            self.render_context.submit_frame();

            let mut submit_batches = Vec::new();
            submit_batches.append(&mut app_render_batches);
            submit_batches.push(ui_batch);
            rhi.graphics_queue().submit(rhi, submit_batches, None);
        });
    }
}
