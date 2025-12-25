use crate::outer_app::OuterApp;
use crate::platform::camera_controller::CameraController;
use ash::vk;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::ffi::CStr;
use truvis_gfx::gfx::Gfx;
use truvis_render_core::core::renderer::Renderer;
use truvis_render_core::platform::input_state::InputState;

pub struct RenderApp<T: OuterApp> {
    pub renderer: Renderer,
    pub camera_controller: CameraController,

    pub outer_app: T,
}
// new & init
impl<T: OuterApp> RenderApp<T> {
    pub fn new(extra_instance_ext: Vec<&'static CStr>) -> Self {
        let mut renderer = Renderer::new(extra_instance_ext);
        let mut camera_controller = CameraController::new();

        let outer_app = {
            let _span = tracy_client::span!("OuterApp::init");
            T::init(&mut renderer, camera_controller.camera_mut())
        };

        Self {
            renderer,
            outer_app,
            camera_controller,
        }
    }
    pub fn init_after_window(&mut self, raw_display_handle: RawDisplayHandle, raw_window_handle: RawWindowHandle) {
        self.renderer.init_after_window(raw_display_handle, raw_window_handle);
    }
}
// destroy
impl<T: OuterApp> RenderApp<T> {
    pub fn destroy(self) {
        Gfx::get().wait_idel();

        self.renderer.destroy();
    }
}
// getter
impl<T: OuterApp> RenderApp<T> {
    pub fn get_delta_time(&self) -> std::time::Duration {
        self.renderer.timer.delta_time
    }
}
// update
impl<T: OuterApp> RenderApp<T> {
    pub fn time_to_render(&mut self) -> bool {
        self.renderer.time_to_render()
    }

    pub fn begin_frame(&mut self) {
        self.renderer.begin_frame();
    }

    pub fn build_ui(&mut self, ui: &imgui::Ui) {
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

        self.outer_app.draw_ui(ui);
    }

    pub fn update_scene(&mut self, input_state: &InputState, extent: vk::Extent2D) {
        // Renderer: Resize Framebuffer
        if self.renderer.render_context.frame_settings.frame_extent != extent {
            self.renderer.resize_frame_buffer(extent);
        }

        // Renderer: Update Input and Camera
        self.camera_controller.update(
            input_state,
            glam::vec2(extent.width as f32, extent.height as f32),
            self.renderer.timer.delta_time,
        );

        // Outer App: Update
        {
            self.outer_app.update(&mut self.renderer);
        }
    }

    pub fn render(&mut self, input_state: &InputState) {
        self.renderer.before_render(input_state, self.camera_controller.camera());
        {
            let _span = tracy_client::span!("OuterApp::draw");
            // 构建出 PipelineContext
            self.outer_app.draw(&self.renderer.render_context);
        }
    }

    pub fn draw_to_window(&mut self, ui_draw_data: Option<&imgui::DrawData>) {
        self.renderer.draw_to_window(ui_draw_data);
    }

    pub fn end_frame(&mut self) {
        self.renderer.end_frame();
    }

    pub fn on_window_resized(&mut self, raw_display_handle: RawDisplayHandle, raw_window_handle: RawWindowHandle) {
        self.renderer.render_present.as_mut().unwrap().rebuild_after_resized(raw_display_handle, raw_window_handle);

        self.outer_app.rebuild(&mut self.renderer);
    }
}
