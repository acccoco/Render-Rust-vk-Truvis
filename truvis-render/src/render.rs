use std::{sync::Arc, todo};

use ash::vk;
use winit::{
    event::{StartCause, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::framework::{
    basic::color::LabelColor,
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
pub struct Renderer<A: App>
{
    pub timer: Timer,
    pub window: Option<Arc<WindowSystem>>,
    pub render_context: Option<RenderContext>,
    pub ui: Option<UI>,
    pub inner_app: Option<Box<A>>,
}


pub struct AppInitInfo
{
    pub window_width: u32,
    pub window_height: u32,
    pub app_name: String,
    pub enable_validation: bool,
}


pub struct UserEvent {}


pub trait App
{
    fn update_ui(&mut self, ui: &mut imgui::Ui);

    fn update(&mut self, rhi: &'static Rhi, render_context: &mut RenderContext, timer: &Timer);

    /// 发生于 acquire_frame 之后，submit_frame 之前
    fn draw(&self, rhi: &'static Rhi, render_context: &mut RenderContext, timer: &Timer);


    fn init(rhi: &'static Rhi, render_context: &mut RenderContext) -> Self;

    /// 由 App 提供的，用于初始化 Rhi
    fn get_render_init_info() -> AppInitInfo;


    // FIXME
    fn get_depth_attachment(depth_image_view: vk::ImageView) -> vk::RenderingAttachmentInfo<'static>
    {
        vk::RenderingAttachmentInfo::default()
            .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .image_view(depth_image_view)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .clear_value(vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1_f32,
                    stencil: 0,
                },
            })
    }

    fn get_color_attachment(image_view: vk::ImageView) -> vk::RenderingAttachmentInfo<'static>
    {
        vk::RenderingAttachmentInfo::default()
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image_view(image_view)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0_f32, 0_f32, 0_f32, 1_f32],
                },
            })
    }

    // FIXME
    fn get_render_info<'a, 'b, 'c>(
        area: vk::Rect2D,
        color_attachs: &'a [vk::RenderingAttachmentInfo],
        depth_attach: &'b vk::RenderingAttachmentInfo,
    ) -> vk::RenderingInfo<'c>
    where
        'b: 'c,
        'a: 'c,
    {
        vk::RenderingInfo::default()
            .layer_count(1)
            .render_area(area)
            .color_attachments(color_attachs)
            .depth_attachment(depth_attach)
    }
}


pub fn panic_handler(info: &std::panic::PanicInfo)
{
    log::error!("{}", info);
    // std::thread::sleep(std::time::Duration::from_secs(3));
}

impl<A: App> Renderer<A>
{
    pub fn run()
    {
        std::panic::set_hook(Box::new(panic_handler));

        // init logger
        {
            use simplelog::*;
            TermLogger::init(LevelFilter::Info, ConfigBuilder::new().build(), TerminalMode::Mixed, ColorChoice::Auto)
                .unwrap();
        }

        let event_loop = winit::event_loop::EventLoop::<UserEvent>::with_user_event().build().unwrap();

        let mut renderer = Self::new();
        event_loop.run_app(&mut renderer).expect("TODO: panic message");
    }

    fn new() -> Self
    {
        Self {
            timer: Timer::default(),
            window: None,
            render_context: None,
            ui: None,
            inner_app: None,
        }
    }

    /// event loop 的 resume 中调用
    fn init(&mut self, event_loop: &ActiveEventLoop)
    {
        //
        self.timer.reset();

        let render_init_info = A::get_render_init_info();

        // window
        self.window = Some(Arc::new(WindowSystem::new(
            event_loop,
            WindowCreateInfo {
                height: render_init_info.window_height as i32,
                width: render_init_info.window_width as i32,
                title: render_init_info.app_name.clone(),
            },
        )));

        // rhi
        {
            let mut rhi_init_info = RhiInitInfo::init_basic(
                render_init_info.app_name.clone(),
                self.window.as_ref().unwrap().clone(),
                render_init_info.enable_validation,
            );
            rhi_init_info.set_debug_callback(Some(vk_debug_callback));
            RHI.get_or_init(|| Rhi::new(rhi_init_info));
        }
        let rhi = RHI.get().unwrap();

        // render context
        {
            let render_swapchain_init_info = RenderSwapchainInitInfo {
                window: Some(self.window.as_ref().unwrap().clone()),
                ..Default::default()
            };

            let render_context_init_info = RenderContextInitInfo::default();
            let render_context = RenderContext::new(rhi, &render_context_init_info, render_swapchain_init_info);
            self.render_context = Some(render_context);
        }

        // ui
        self.ui = Some(UI::new(
            rhi,
            &self.render_context.as_ref().unwrap(),
            self.window.as_ref().unwrap().window(),
            &UiOptions {
                // FIXME 统一一下出现的位置。RenderContext 里面也有这个配置
                frames_in_flight: 3,
            },
        ));


        // application
        self.inner_app = Some(Box::new(A::init(Self::get_rhi(), self.render_context.as_mut().unwrap())));
    }

    fn tick(&mut self)
    {
        self.timer.update();

        self.render_context.as_mut().unwrap().acquire_frame();

        self.ui.as_ref().unwrap().prepare_frame(self.window.as_mut().unwrap().window());

        let rhi = Self::get_rhi();

        // FIXME 调整一下调用顺序
        // main pass
        rhi.graphics_queue_begin_label("[main-pass]", LabelColor::COLOR_PASS);
        {
            let app = self.inner_app.as_mut().unwrap();
            app.update(Self::get_rhi(), self.render_context.as_mut().unwrap(), &self.timer);
            app.draw(Self::get_rhi(), self.render_context.as_mut().unwrap(), &self.timer);
        }
        rhi.graphics_queue_end_label();

        // ui pass
        rhi.graphics_queue_begin_label("[ui-pass]", LabelColor::COLOR_PASS);
        {
            let frame = self.ui.as_mut().unwrap().new_frame(self.window.as_ref().unwrap().window());
            self.inner_app.as_mut().unwrap().update_ui(frame);

            // FIXME ui cmd 需要释放
            let ui_cmd = self.ui.as_mut().unwrap().draw(rhi, self.render_context.as_mut().unwrap());

            if let Some(ui_cmd) = ui_cmd {
                // FIXME barrier cmd 也需要释放
                let mut barrier_cmd = self.render_context.as_mut().unwrap().alloc_command_buffer("ui pipeline barrier");
                barrier_cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[uipass]color-attach-barrier");
                {
                    barrier_cmd.image_memory_barrier(
                        vk::DependencyFlags::empty(),
                        &[vk::ImageMemoryBarrier2::default()
                            .image(self.render_context.as_ref().unwrap().current_present_image())
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

        self.render_context.as_mut().unwrap().submit_frame();
    }


    pub fn get_rhi() -> &'static Rhi
    {
        RHI.get().unwrap()
    }
}


impl<A: App> winit::application::ApplicationHandler<UserEvent> for Renderer<A>
{
    // TODO 测试一下这个事件的发送时机：是否会在每个键盘事件之前发送？还是每一帧发送一次
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause)
    {
        // self.ui.as_mut().unwrap().imgui.get_mut().io_mut().update_delta_time();
    }

    // FIXME 这个是什么时候调用呢？
    fn resumed(&mut self, event_loop: &ActiveEventLoop)
    {
        self.init(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent)
    {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                log::info!("window was resized, new size is : {}x{}", new_size.width, new_size.height);
            }
            WindowEvent::RedrawRequested => {
                self.tick();
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop)
    {
        // FIXME 似乎不应该使用这个事件，应该使用 redraw 来驱动绘制
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop)
    {
        log::info!("loop exiting");
    }
}
