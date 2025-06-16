#[derive(Debug)]
pub struct Timer {
    pub start_time: std::time::SystemTime,
    pub current_time: std::time::SystemTime,

    pub delta_time: std::time::Duration,
    pub total_time: std::time::Duration,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            start_time: std::time::SystemTime::now(),
            current_time: std::time::SystemTime::now(),
            delta_time: std::time::Duration::new(0, 0),
            total_time: std::time::Duration::new(0, 0),
        }
    }
}

impl Timer {
    pub fn tic(&mut self) {
        self.delta_time = self.toc();
        self.current_time = std::time::SystemTime::now();
        self.total_time += self.delta_time;
    }

    pub fn toc(&self) -> std::time::Duration {
        let now = std::time::SystemTime::now();
        now.duration_since(self.current_time).unwrap()
    }

    #[inline]
    pub fn delte_time_ms(&self) -> f32 {
        self.delta_time.as_micros() as f32 / 1000.0
    }

    #[inline]
    pub fn delta_time_s(&self) -> f32 {
        self.delta_time.as_secs_f32()
    }
}
