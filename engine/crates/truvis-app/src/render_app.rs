use crate::gui_front::GuiHost;
use crate::outer_app::OuterApp;
use crate::platform::camera_controller::CameraController;
use ash::vk;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::ffi::CStr;
use truvis_crate_tools::init_log::init_log;
use truvis_gfx::gfx::Gfx;
use truvis_platform::input_event::InputEvent;
use truvis_platform::input_manager::InputManager;
use truvis_platform::input_state::InputState;
use truvis_render_core::core::renderer::Renderer;

pub fn panic_handler(info: &std::panic::PanicHookInfo) {
    log::error!("{}", info);
    // std::thread::sleep(std::time::Duration::from_secs(30));
}

pub struct RenderApp {
    pub renderer: Renderer,
    pub camera_controller: CameraController,
    pub input_manager: InputManager,
    pub gui_host: GuiHost,

    pub last_render_area: vk::Extent2D,

    pub outer_app: Option<Box<dyn OuterApp>>,
}
// new & init
impl RenderApp {
    pub fn new(raw_display_handle: RawDisplayHandle, mut outer_app: Box<dyn OuterApp>) -> Self {
        // 追加 window system 需要的 extension，在 windows 下也就是 khr::Surface
        let extra_instance_ext = ash_window::enumerate_required_extensions(raw_display_handle)
            .unwrap()
            .iter()
            .map(|ext| unsafe { CStr::from_ptr(*ext) })
            .collect();

        let mut renderer = Renderer::new(extra_instance_ext);
        let mut camera_controller = CameraController::new();

        {
            let _span = tracy_client::span!("OuterApp::init");
            outer_app.init(&mut renderer, camera_controller.camera_mut());
        };

        Self {
            renderer,
            outer_app: Some(outer_app),
            camera_controller,
            input_manager: InputManager::new(),
            gui_host: GuiHost::new(),
            last_render_area: vk::Extent2D::default(),
        }
    }
    pub fn init_after_window(
        &mut self,
        raw_display_handle: RawDisplayHandle,
        raw_window_handle: RawWindowHandle,
        window_scale_factor: f64,
    ) {
        self.gui_host.hidpi_factor = window_scale_factor;

        self.renderer.init_after_window(raw_display_handle, raw_window_handle);

        let (fonts_atlas, font_tex_id) = self.gui_host.init_font();
        self.renderer.render_present.as_mut().unwrap().gui_backend.register_font(
            &mut self.renderer.render_context.bindless_manager,
            &mut self.renderer.render_context.gfx_resource_manager,
            fonts_atlas,
            font_tex_id,
        );
    }

    pub fn init_env() {
        std::panic::set_hook(Box::new(panic_handler));

        init_log();

        tracy_client::Client::start();
        tracy_client::set_thread_name!("RenderThread");
    }
}
// destroy
impl RenderApp {
    pub fn destroy(mut self) {
        Gfx::get().wait_idel();

        self.outer_app = None;
        self.renderer.destroy();

        Gfx::destroy();
    }
}
// update
impl RenderApp {
    pub fn time_to_render(&mut self) -> bool {
        self.renderer.time_to_render()
    }

    pub fn begin_frame(&mut self) {
        self.renderer.begin_frame();
    }

    pub fn handle_event(&mut self, event: &InputEvent) {
        // TODO 判断是否改下方处理
        self.gui_host.handle_event(event);

        // 使用InputManager处理窗口事件
        self.input_manager.handle_window_event(event.clone());
    }

    pub fn build_ui(&mut self) {
        let elapsed = self.renderer.timer.delta_time();

        self.gui_host.new_frame(
            elapsed,
            |ui, content_size| {
                let min_pos = ui.window_content_region_min();
                ui.set_cursor_pos([min_pos[0] + 5.0, min_pos[1] + 5.0]);
                ui.text(format!("FPS: {:.2}", 1.0 / elapsed.as_secs_f32()));
                ui.text(format!("size: {:.0}x{:.0}", content_size[0], content_size[1]));
            },
            |ui| {
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
                        let pipeline_settings = &mut self.renderer.render_context.pipeline_settings;
                        ui.slider("channel", 0, 3, &mut pipeline_settings.channel);
                    }
                    ui.text(format!("Accum Frames: {}", self.renderer.render_context.accum_data.accum_frames_num()));
                    ui.new_line();
                }

                self.outer_app.as_mut().unwrap().draw_ui(ui);
            },
        );
    }

    pub fn big_update(&mut self) {
        // Begin Frame ============================
        if !self.time_to_render() {
            return;
        }

        self.begin_frame();

        // build Gui ==================================
        {
            let _span = tracy_client::span!("Update Gui");

            self.build_ui();
        }

        // Rendere Update ==================================
        {
            let _span = tracy_client::span!("Renderer Update");
            self.update();
        }

        // Renderer: Render ================================
        self.render();

        // Window: Draw Gui ===============================
        {
            self.draw_to_window();
        }

        // End Frame ===================================
        self.end_frame();

        tracy_client::frame_mark();
    }

    pub fn update(&mut self) {
        self.last_render_area = self.gui_host.get_render_region().extent;
        self.input_manager.update();
        self.update_scene(&self.input_manager.state().clone(), self.last_render_area);
    }

    fn update_scene(&mut self, input_state: &InputState, extent: vk::Extent2D) {
        // Renderer: Resize Framebuffer
        if self.renderer.render_context.frame_settings.frame_extent != extent {
            self.renderer.resize_frame_buffer(extent);
        }

        // Renderer: Update Input and Camera
        self.camera_controller.update(
            input_state,
            glam::vec2(extent.width as f32, extent.height as f32),
            self.renderer.timer.delta_time(),
        );

        // Outer App: Update
        {
            self.outer_app.as_mut().unwrap().update(&mut self.renderer);
        }
    }

    pub fn render(&mut self) {
        let input_state = self.input_manager.state();

        self.renderer.before_render(input_state, self.camera_controller.camera());
        {
            let _span = tracy_client::span!("OuterApp::draw");
            // 构建出 PipelineContext
            self.outer_app.as_mut().unwrap().draw(&self.renderer.render_context);
        }
    }

    pub fn draw_to_window(&mut self) {
        let ui_draw_data = self.gui_host.compile_ui();
        self.renderer.draw_to_window(ui_draw_data);
    }

    pub fn end_frame(&mut self) {
        self.renderer.end_frame();
    }

    pub fn on_window_resized(&mut self, raw_display_handle: RawDisplayHandle, raw_window_handle: RawWindowHandle) {
        self.renderer.render_present.as_mut().unwrap().rebuild_after_resized(raw_display_handle, raw_window_handle);

        self.outer_app.as_mut().unwrap().rebuild(&mut self.renderer);
    }
}
