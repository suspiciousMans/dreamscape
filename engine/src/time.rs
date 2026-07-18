use std::time::Instant;

pub struct Time {
    start: Instant,
    last_frame: Instant,
    pub delta: f32,
    pub elapsed: f32,
}

impl Time {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last_frame: now,
            delta: 0.0,
            elapsed: 0.0,
        }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        self.delta = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.elapsed = (now - self.start).as_secs_f32();
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}
