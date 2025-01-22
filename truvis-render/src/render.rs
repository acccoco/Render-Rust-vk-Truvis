use std::sync::Arc;

use ash::vk;

use crate::framework::{
    basic::color::{GREEN, RED},
    core::{queue::RhiSubmitInfo, swapchain::RenderSwapchainInitInfo},
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

    pub fn render_loop<F>(&mut self, mut render_func: F)
    where
        F: FnMut(&'static Rhi, &mut RenderContext, &Timer, &mut imgui::Ui),
    {
        let rhi = Self::get_rhi();

        self.window.render_loop(&mut self.ui, |ui| {
            self.timer.update();

            rhi.graphics_queue_begin_label("[acquire-frame]", GREEN);
            self.render_context.acquire_frame();
            rhi.graphics_queue_end_label();

            // TODO 优化一下这个执行顺序
            ui.platform.prepare_frame(ui.imgui.get_mut().io_mut(), self.window.window()).unwrap();

            let frame = ui.imgui.get_mut().new_frame();

            // FIXME 调整一下调用顺序
            // main pass
            render_func(rhi, &mut self.render_context, &self.timer, frame);

            // ui pass
            rhi.graphics_queue_begin_label("[ui-pass]draw", GREEN);
            {
                // FIXME ui cmd 需要释放
                ui.platform.prepare_render(frame, self.window.window());
                let ui_cmd = ui.draw(rhi, &mut self.render_context);

                if let Some(ui_cmd) = ui_cmd {
                    // FIXME barrier cmd 也需要释放
                    let mut barrier_cmd = self.render_context.alloc_command_buffer("ui pipeline barrier");
                    barrier_cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                    barrier_cmd.begin_label("[uipass]barrier", RED);
                    {
                        barrier_cmd.image_memory_barrier(
                            vk::DependencyFlags::empty(),
                            &[vk::ImageMemoryBarrier2::default()
                                .image(self.render_context.current_present_image())
                                .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                                .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                                .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                                .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                                .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                                .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                                .subresource_range(
                                    vk::ImageSubresourceRange::default()
                                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                                        .layer_count(1)
                                        .level_count(1),
                                )],
                        );
                    }
                    barrier_cmd.end_label();
                    barrier_cmd.end();

                    rhi.graphics_queue_submit(
                        vec![RhiSubmitInfo {
                            command_buffers: vec![ui_cmd, barrier_cmd],
                            wait_info: Vec::new(),
                            signal_info: Vec::new(),
                        }],
                        None,
                    );
                }
            }
            rhi.graphics_queue_end_label();

            rhi.graphics_queue_begin_label("[submit-frame]", GREEN);
            self.render_context.submit_frame();
            rhi.graphics_queue_end_label();
        });
    }
}
