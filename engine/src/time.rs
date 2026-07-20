use std::time::Instant;

pub struct Time {
    start: Instant,
    last_frame: Instant,
    pub delta: f32,
    pub elapsed: f32,
}

impl Time {
    /// Upper bound on a single frame's `delta`. The very first `tick()`
    /// after startup measures the whole asset-loading `init()` duration
    /// (`last_frame` is captured before `init()` runs), and any mid-run
    /// stall — alt-tab, window drag, a GC/driver hitch — produces a
    /// multi-second gap too. An unclamped multi-second `dt` fed to the
    /// discrete-collision physics integrator makes bodies tunnel straight
    /// through geometry in one step, so the spike is clamped here. At
    /// steady state (~16 ms frames) this never triggers; `elapsed` stays
    /// true wall-clock and is unaffected.
    const MAX_DELTA: f32 = 0.1;

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
        self.delta = (now - self.last_frame).as_secs_f32().min(Self::MAX_DELTA);
        self.last_frame = now;
        self.elapsed = (now - self.start).as_secs_f32();
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}
