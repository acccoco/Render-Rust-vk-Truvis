#[derive(Debug)]
pub struct Timer {
    pub start_time: std::time::SystemTime,
    pub current_time: std::time::SystemTime,

    pub elapse: std::time::Duration,
    pub total_time: std::time::Duration,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            start_time: std::time::SystemTime::now(),
            current_time: std::time::SystemTime::now(),
            elapse: std::time::Duration::new(0, 0),
            total_time: std::time::Duration::new(0, 0),
        }
    }
}

impl Timer {
    pub fn tic(&mut self) {
        self.elapse = self.toc();
        self.current_time = std::time::SystemTime::now();
    }

    pub fn toc(&self) -> std::time::Duration {
        let now = std::time::SystemTime::now();
        let duration = now.duration_since(self.current_time).unwrap();
        duration
    }

    pub fn elapse_ms(&self) -> f32 {
        self.elapse.as_micros() as f32 / 1000.0
    }

    pub fn elapse_s(&self) -> f32 {
        self.elapse.as_secs_f32()
    }
}
