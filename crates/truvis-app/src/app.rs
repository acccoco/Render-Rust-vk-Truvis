use std::{cell::OnceCell, ffi::CStr, sync::OnceLock};

use ash::vk;
use raw_window_handle::HasDisplayHandle;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, StartCause, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::outer_app::OuterApp;
use crate::platform::camera_controller::CameraController;
use crate::window_system::main_window::{MainWindow, PresentData};
use truvis_crate_tools::init_log::init_log;
use truvis_gfx::commands::barrier::GfxBarrierMask;
use truvis_gfx::gfx::Gfx;
use truvis_render::core::frame_context::FrameContext;
use truvis_render::core::renderer::Renderer;
use truvis_render::platform::input_manager::InputManager;

pub fn panic_handler(info: &std::panic::PanicHookInfo) {
    log::error!("{}", info);
    // std::thread::sleep(std::time::Duration::from_secs(30));
}

pub struct UserEvent;

/// Truvis 应用主结构
///
/// 封装了完整的渲染应用框架，包括窗口系统、渲染器、输入管理、相机控制等。
/// 通过泛型参数 `T: OuterApp` 集成用户自定义应用逻辑。
///
/// # Frames in Flight
/// 采用 3 帧并行渲染（A/B/C），通过 timeline semaphore 同步 GPU 进度。
pub struct TruvisApp<T: OuterApp> {
    renderer: Renderer,

    /// 需要等待窗口事件初始化，因此 OnceCell
    window_system: OnceCell<MainWindow>,
    last_render_area: vk::Extent2D,

    input_manager: InputManager,
    camera_controller: CameraController,

    outer_app: OnceCell<T>,
}

// 总的 main 函数
impl<T: OuterApp> TruvisApp<T> {
    /// 整个程序的入口
    pub fn run() {
        std::panic::set_hook(Box::new(panic_handler));

        init_log();
        tracy_client::Client::start();
        tracy_client::set_thread_name!("MiaowThread");

        // 创建输入管理器和计时器
        let input_manager = InputManager::new();

        // 创建相机控制器
        let camera_controller = CameraController::new();

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

        Gfx::destroy();
    }
}

impl<T: OuterApp> TruvisApp<T> {
    /// 在 window 创建之后调用，初始化 Renderer 和 GUI
    fn init_after_window(&mut self, event_loop: &ActiveEventLoop) {
        let window_system = MainWindow::new(
            &mut self.renderer,
            event_loop,
            "Truvis".to_string(),
            vk::Extent2D {
                width: 1200,
                height: 800,
            },
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
        let frame_label = FrameContext::get().frame_label();
        let elapsed = self.renderer.render_context.timer.delta_time;

        {
            let _span = tracy_client::span!("Acquire Image");
            self.window_system.get_mut().unwrap().acquire_image(frame_label);
        }

        // Update Gui ==================================
        {
            let _span = tracy_client::span!("Update Gui");
            self.window_system.get_mut().unwrap().update_gui(elapsed, |ui| {
                // camera info
                {
                    let camera = self.camera_controller.camera();
                    ui.text(format!(
                        "CameraPos: ({:.2}, {:.2}, {:.2})",
                        camera.position.x, camera.position.y, camera.position.z
                    ));
                    ui.text(format!(
                        "CameraEuler: ({:.2}, {:.2}, {:.2})",
                        camera.euler_yaw_deg, camera.euler_pitch_deg, camera.euler_roll_deg
                    ));
                    ui.text(format!(
                        "CameraForward: ({:.2}, {:.2}, {:.2})",
                        camera.camera_forward().x,
                        camera.camera_forward().y,
                        camera.camera_forward().z
                    ));
                    ui.text(format!("CameraAspect: {:.2}", camera.asp));
                    ui.text(format!("CameraFov(Vertical): {:.2}°", camera.fov_deg_vertical));
                    {
                        let mut pipeline_settings = FrameContext::get().pipeline_settings();
                        ui.slider("channel", 0, 3, &mut pipeline_settings.channel);
                        FrameContext::get().set_pipeline_settings(pipeline_settings);
                    }
                    ui.text(format!("Accum Frames: {}", self.renderer.render_context.accum_data.accum_frames_num));
                    ui.new_line();
                }

                self.outer_app.get_mut().unwrap().draw_ui(ui);
            });
        }

        // Rendere Update ==================================
        {
            let _span = tracy_client::span!("Renderer Update");
            let extent = self.window_system.get().unwrap().get_render_extent();

            // Renderer: Resize Framebuffer
            {
                if self.last_render_area != extent {
                    // log::info!("resize frame buffer to: {}x{}", extent.width, extent.height);
                    self.renderer.resize_frame_buffer(extent);
                    self.last_render_area = extent;
                }
            }

            // Renderer: Update Input and Camera
            {
                self.input_manager.update();
                self.camera_controller.update(
                    &self.input_manager,
                    glam::vec2(extent.width as f32, extent.height as f32),
                    self.renderer.render_context.timer.delta_time,
                );
            }

            // Outer App: Update
            {
                self.outer_app.get_mut().unwrap().update(&mut self.renderer);
            }
        }

        // Renderer: Render ================================
        self.renderer.before_render(self.input_manager.state(), self.camera_controller.camera());
        {
            let _span = tracy_client::span!("OuterApp::draw");
            // 构建出 PipelineContext
            self.outer_app
                .get_mut()
                .unwrap()
                .draw(&self.renderer.render_context, &mut self.renderer.render_context_mut);
        }

        // Window: Draw Gui ===============================
        {
            let _span = tracy_client::span!("Present GUI");
            let present_data = {
                let render_target_texture =
                    self.renderer.render_context.fif_buffers.render_target_texture_handle(frame_label);

                PresentData {
                    render_target: render_target_texture,
                    render_target_barrier: GfxBarrierMask {
                        src_stage: vk::PipelineStageFlags2::COMPUTE_SHADER,
                        src_access: vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                        dst_stage: vk::PipelineStageFlags2::NONE,
                        dst_access: vk::AccessFlags2::NONE,
                    },
                }
            };
            self.window_system.get_mut().unwrap().draw_gui(
                &self.renderer.render_context,
                &mut self.renderer.render_context_mut,
                present_data,
            );
        }

        // End Frame ===================================
        {
            self.window_system.get_mut().unwrap().present_image();
        }
        self.renderer.end_frame();

        tracy_client::frame_mark();
    }

    fn on_window_resized(&mut self, _width: u32, _height: u32) {
        self.window_system.get_mut().unwrap().rebuild_after_resized();

        // log::info!("try to rebuild render context");
        self.outer_app.get_mut().unwrap().rebuild(&mut self.renderer);
    }
}

// 手动 drop
impl<T: OuterApp> TruvisApp<T> {
    fn destroy(mut self) {
        Gfx::get().wait_idel();

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
        self.input_manager.handle_window_event(&event);

        match event {
            WindowEvent::CloseRequested => {
                Gfx::get().wait_idel();
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                // log::info!("window was resized, new size is : {}x{}", new_size.width,
                // new_size.height);
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
        self.input_manager.handle_device_event(&event);
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
