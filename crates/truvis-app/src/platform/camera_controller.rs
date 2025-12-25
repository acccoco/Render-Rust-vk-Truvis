use truvis_render_core::platform::camera::Camera;
use truvis_render_core::platform::event::KeyCode;
use truvis_render_core::platform::input_state::InputState;

pub struct CameraController {
    camera: Camera,
}

impl Default for CameraController {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraController {
    /// 创建新的相机控制器
    pub fn new() -> Self {
        Self {
            camera: Camera::default(),
        }
    }

    /// 获取相机引用
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// 获取相机可变引用
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    /// 根据输入更新相机状态
    pub fn update(&mut self, input_state: &InputState, viewport_size: glam::Vec2, deltatime: std::time::Duration) {
        let delta_time_s = deltatime.as_secs_f32();

        self.camera.set_aspect_ratio(viewport_size.x / viewport_size.y);

        if input_state.is_right_button_pressed() {
            let mouse_delta = input_state.get_mouse_delta();

            self.camera.rotate_yaw(-mouse_delta[0] as f32 / 7.0);
            self.camera.rotate_pitch(-mouse_delta[1] as f32 / 7.0);
        }

        let move_speed = 320_f32;
        if input_state.is_key_pressed(KeyCode::KeyW) {
            self.camera.move_forward(delta_time_s * move_speed);
        }
        if input_state.is_key_pressed(KeyCode::KeyS) {
            self.camera.move_forward(-delta_time_s * move_speed);
        }
        if input_state.is_key_pressed(KeyCode::KeyA) {
            self.camera.move_right(-delta_time_s * move_speed);
        }
        if input_state.is_key_pressed(KeyCode::KeyD) {
            self.camera.move_right(delta_time_s * move_speed);
        }
        if input_state.is_key_pressed(KeyCode::KeyE) {
            self.camera.move_up(delta_time_s * move_speed);
        }
        if input_state.is_key_pressed(KeyCode::KeyQ) {
            self.camera.move_up(-delta_time_s * move_speed);
        }
    }
}
