use crate::platform::camera::DrsCamera;
use crate::platform::input_manager::InputManager;
use winit::keyboard::KeyCode;

pub struct CameraController {
    camera: DrsCamera,
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
            camera: DrsCamera::default(),
        }
    }

    /// 获取相机引用
    pub fn camera(&self) -> &DrsCamera {
        &self.camera
    }

    /// 获取相机可变引用
    pub fn camera_mut(&mut self) -> &mut DrsCamera {
        &mut self.camera
    }

    /// 根据输入更新相机状态
    pub fn update(&mut self, input_mgr: &InputManager, viewport_size: glam::Vec2, deltatime: std::time::Duration) {
        let delta_time_s = deltatime.as_secs_f32();

        self.camera.set_aspect_ratio(viewport_size.x / viewport_size.y);

        if input_mgr.is_right_button_pressed() {
            let mouse_delta = input_mgr.get_mouse_delta() / 7.0;

            self.camera.rotate_yaw(mouse_delta.x as f32);
            self.camera.rotate_pitch(mouse_delta.y as f32);
        }

        let move_speed = 320_f32;
        if input_mgr.is_key_pressed(KeyCode::KeyW) {
            self.camera.move_forward(delta_time_s * move_speed);
        }
        if input_mgr.is_key_pressed(KeyCode::KeyS) {
            self.camera.move_forward(-delta_time_s * move_speed);
        }
        if input_mgr.is_key_pressed(KeyCode::KeyA) {
            self.camera.move_right(-delta_time_s * move_speed);
        }
        if input_mgr.is_key_pressed(KeyCode::KeyD) {
            self.camera.move_right(delta_time_s * move_speed);
        }
        if input_mgr.is_key_pressed(KeyCode::KeyE) {
            self.camera.move_up(-delta_time_s * move_speed);
        }
        if input_mgr.is_key_pressed(KeyCode::KeyQ) {
            self.camera.move_up(delta_time_s * move_speed);
        }
    }
}
