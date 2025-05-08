#[derive(Debug)]
pub struct Timer {
    pub start_time: std::time::SystemTime,
    pub last_time: std::time::SystemTime,
    // FIXME 改成 Duration
    pub delta_time_s: f32,
    pub total_time_s: f32,
    pub total_frame: i32,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            start_time: std::time::SystemTime::now(),
            last_time: std::time::SystemTime::now(),
            total_frame: 0,
            delta_time_s: 0.0,
            total_time_s: 0.0,
        }
    }
}

impl Timer {
    pub fn reset(&mut self) {
        self.start_time = std::time::SystemTime::now();
        self.last_time = std::time::SystemTime::now();
        self.total_frame = 0;
        self.delta_time_s = 0.0;
        self.total_time_s = 0.0;
    }

    pub fn update(&mut self) {
        let now = std::time::SystemTime::now();
        let total_time = now.duration_since(self.start_time).unwrap().as_secs_f32();
        let delta_time = now.duration_since(self.last_time).unwrap().as_secs_f32();
        self.last_time = now;
        self.total_frame += 1;
        self.total_time_s = total_time;
        self.delta_time_s = delta_time;
    }
}
