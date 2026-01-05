use crate::gui_front::GuiHost;
use crate::outer_app::OuterApp;
use crate::platform::camera_controller::CameraController;
use crate::platform::input_event::InputEvent;
use crate::platform::input_manager::InputManager;
use crate::platform::input_state::InputState;
use ash::vk;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::ffi::CStr;
use truvis_crate_tools::init_log::init_log;
use truvis_gfx::gfx::Gfx;
use truvis_renderer::renderer::Renderer;

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
        window_physical_size: [u32; 2],
    ) {
        self.gui_host.hidpi_factor = window_scale_factor;

        self.renderer.init_after_window(raw_display_handle, raw_window_handle, window_physical_size);

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
    fn time_to_render(&mut self) -> bool {
        self.renderer.time_to_render()
    }

    pub fn handle_event(&mut self, event: &InputEvent) {
        // 使用InputManager处理窗口事件
        self.input_manager.push_event(event.clone());
    }

    fn build_ui(&mut self) {
        let elapsed = self.renderer.timer.delta_time();
        let swapchain_image_size = self.renderer.render_present.as_ref().unwrap().swapchain.as_ref().unwrap().extent();

        self.gui_host.new_frame(elapsed, |ui| {
            // 创建一个全屏的、固定位置的、无边框的透明窗口作为 UI 容器
            // 这样可以直接相对于 framebuffer 左上角绘制，而不会有可拖动的窗口
            ui.window("##overlay")
                .position([0.0, 0.0], imgui::Condition::Always)
                .size([swapchain_image_size.width as f32, swapchain_image_size.height as f32], imgui::Condition::Always)
                .flags(
                    imgui::WindowFlags::NO_TITLE_BAR
                        | imgui::WindowFlags::NO_RESIZE
                        | imgui::WindowFlags::NO_MOVE
                        | imgui::WindowFlags::NO_SCROLLBAR
                        | imgui::WindowFlags::NO_SCROLL_WITH_MOUSE
                        | imgui::WindowFlags::NO_COLLAPSE
                        | imgui::WindowFlags::NO_BACKGROUND
                        | imgui::WindowFlags::NO_SAVED_SETTINGS
                        | imgui::WindowFlags::NO_MOUSE_INPUTS
                        | imgui::WindowFlags::NO_FOCUS_ON_APPEARING
                        | imgui::WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS
                        | imgui::WindowFlags::NO_NAV_INPUTS
                        | imgui::WindowFlags::NO_NAV_FOCUS,
                )
                .build(|| {
                    // fps
                    {
                        ui.set_cursor_pos([5.0, 5.0]);
                        ui.text(format!("FPS: {:.2}", 1.0 / elapsed.as_secs_f32()));
                        ui.text(format!(
                            "swapchain: {:.0}x{:.0}",
                            swapchain_image_size.width, swapchain_image_size.height
                        ));
                    }

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
                        ui.text(format!(
                            "Accum Frames: {}",
                            self.renderer.render_context.accum_data.accum_frames_num()
                        ));
                        ui.new_line();
                    }
                });

            // 可交互的控制面板窗口
            ui.window("Controls")
                .position([10.0, 200.0], imgui::Condition::FirstUseEver)
                .size([250.0, 100.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    let pipeline_settings = &mut self.renderer.render_context.pipeline_settings;
                    ui.slider("channel", 0, 3, &mut pipeline_settings.channel);
                });

            self.outer_app.as_mut().unwrap().draw_ui(ui);
        });
    }

    pub fn big_update(&mut self) {
        if !self.time_to_render() {
            return;
        }

        // Begin Frame
        {
            let _span = tracy_client::span!("Begin Frame");
            self.renderer.begin_frame();
        }

        // 处理事件
        {
            let _span = tracy_client::span!("Process Input Events");

            for event in self.input_manager.get_events() {
                // imgui 处理事件
                // TODO imgui 是否吞掉事件
                self.gui_host.handle_event(event);

                // resize 相关事件
                if let InputEvent::Resized {
                    physical_width,
                    physical_height,
                } = event
                {
                    if *physical_width < 1 || *physical_height < 1 {
                        log::error!("Invalid window size: {}x{}", physical_width, physical_height);
                        continue;
                    } else {
                        self.renderer
                            .render_present
                            .as_mut()
                            .unwrap()
                            .update_window_size([*physical_width, *physical_height]);
                    }
                }
            }

            // input manager 处理事件
            self.input_manager.process_events();
        }

        // resize
        if self.renderer.need_resize() {
            self.renderer.recreate_swapchain();
            self.outer_app.as_mut().unwrap().on_window_resized(&mut self.renderer);
        }
        self.renderer.update_frame_settings();

        // GPU 帧的开始
        // acquire image 应该等到 CPU world 更新完毕再执行，但是放到这里可以简化 resize 的处理
        {
            self.renderer.acquire_image();
        }

        // GUI 绘制
        {
            let _span = tracy_client::span!("Build Gui");

            self.build_ui();
            self.gui_host.compile_ui();
        }

        // 更新 CPU world
        {
            let _span = tracy_client::span!("Renderer Update");

            self.update_scene(&self.input_manager.state().clone());
        }

        // 将数据上传到 GPU
        {
            let _span = tracy_client::span!("Renderer Before Render");
            self.renderer.before_render(self.camera_controller.camera());
        }

        // Renderer: Render ================================
        {
            let _span = tracy_client::span!("Renderer Render");

            self.outer_app.as_mut().unwrap().draw(&self.renderer.render_context);
            self.renderer.draw_to_window(self.gui_host.get_render_data());
        }

        // GPU 帧的结束
        {
            self.renderer.present_image();
        }

        // End Frame ===================================
        {
            let _span = tracy_client::span!("End  Frame");
            self.renderer.end_frame();
        }

        tracy_client::frame_mark();
    }

    fn update_scene(&mut self, input_state: &InputState) {
        let frame_extent = self.renderer.render_context.frame_settings.frame_extent;

        // Renderer: Update Input and Camera
        self.camera_controller.update(
            input_state,
            glam::vec2(frame_extent.width as f32, frame_extent.height as f32),
            self.renderer.timer.delta_time(),
        );

        // Outer App: Update
        {
            self.outer_app.as_mut().unwrap().update(&mut self.renderer);
        }
    }
}
