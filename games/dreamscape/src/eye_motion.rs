//! Eye entrance animation and REM (rapid eye movement) when the eye is closed.
//! The eye starts fully above the screen, lids shut and dreaming, and sinks
//! slowly to its place. Once it settles, the lid lifts open.
//! REM activates only when openness < 0.35 (lid threshold where pupil cannot move).

/// Entrance state machine: tracks the descent, then the lid opening.
#[derive(Clone, Copy, Debug)]
pub struct Entrance {
    pub t: f32,
    pub done: bool,
}

/// The eye reaches its place at this time (seconds).
pub const DROP_END: f32 = 3.0;
/// The lid finishes opening; the entrance is over.
pub const OPEN_END: f32 = 4.4;
/// Lid while descending: shut enough that REM is active.
pub const LID_SHUT: f32 = 0.25;
/// Seconds before the eye starts speaking (it talks as it settles).
pub const SPEECH_DELAY: f32 = 2.2;

/// Perlin's smootherstep: zero velocity *and* acceleration at both ends,
/// so the motion starts and stops without a visible kink.
fn smoother(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
}

/// How fast the lid settles toward a new target (per second): it opens a bit
/// quicker than it closes, so closing reads as a slow, heavy sink.
const LID_OPEN_RATE: f32 = 4.0;
const LID_CLOSE_RATE: f32 = 2.0;

/// Moves the lid toward `target` by exponential easing: fast at first, then
/// gentler and gentler, never overshooting, whatever the frame time.
pub fn ease_lid(current: f32, target: f32, dt: f32) -> f32 {
    let rate = if target < current { LID_CLOSE_RATE } else { LID_OPEN_RATE };
    current + (target - current) * (1.0 - (-rate * dt.max(0.0)).exp())
}

impl Entrance {
    pub fn new() -> Self {
        Self { t: 0.0, done: false }
    }

    pub fn tick(&mut self, dt: f32) {
        if !self.done {
            self.t += dt;
            if self.t >= OPEN_END {
                self.done = true;
            }
        }
    }

    pub fn reset(&mut self) {
        self.t = 0.0;
        self.done = false;
    }

    /// How far the eye still has to fall: 1.0 = fully above the screen,
    /// 0.0 = in place. The caller scales this by its own screen height.
    pub fn height(&self) -> f32 {
        1.0 - smoother(self.t / DROP_END)
    }

    /// Lid openness: shut while falling, then smoothly lifting once landed.
    pub fn lid(&self) -> f32 {
        let open = smoother((self.t - DROP_END) / (OPEN_END - DROP_END));
        LID_SHUT + (1.0 - LID_SHUT) * open
    }
}

impl Default for Entrance {
    fn default() -> Self {
        Self::new()
    }
}

/// Every this many seconds of play the eye glances at the objective.
pub const GLANCE_PERIOD: f32 = 20.0;
/// How long each glance lasts, ease-in and ease-out included.
pub const GLANCE_LEN: f32 = 3.5;
const GLANCE_EASE: f32 = 0.6;
/// Lid opening during a glance (enough for the pupil to travel and be seen).
pub const GLANCE_LID: f32 = 0.6;
/// Lid opening at the peak of a REM burst: a crack, just wide enough to look.
pub const CRACK_LID: f32 = 0.45;
/// REM stirs only while the lid is below this.
pub const REM_LID: f32 = 0.35;

/// 0..1 envelope of the objective glance at `clock` seconds of play: 0 for the
/// first `GLANCE_PERIOD`, then a smooth rise, hold and fall at the start of
/// every period after.
pub fn glance(clock: f32) -> f32 {
    if clock < GLANCE_PERIOD {
        return 0.0;
    }
    let p = clock % GLANCE_PERIOD;
    if p >= GLANCE_LEN {
        return 0.0;
    }
    smoother(p / GLANCE_EASE).min(smoother((GLANCE_LEN - p) / GLANCE_EASE))
}

/// What a shut, dreaming eye does this frame.
#[derive(Clone, Copy, Debug)]
pub struct Rem {
    /// Where the pupil looks (unit-ish screen direction).
    pub gaze: [f32; 2],
    /// 0..1: how far the lid cracks open for this saccade burst.
    pub crack: f32,
}

/// REM: during each burst the lid cracks open (smoothly) and the pupil darts,
/// leaning toward the objective when there is one. Between bursts it is still.
pub fn rem(age: f32, seed: u64, motion_scale: f32, toward: Option<[f32; 2]>) -> Rem {
    let (g, darting) = rem_gaze(age, seed, motion_scale);
    if !darting {
        return Rem { gaze: g, crack: 0.0 };
    }
    let phase = age.rem_euclid(BURST_CYCLE);
    let crack = (((phase - BURST_START) / (BURST_CYCLE - BURST_START)) * std::f32::consts::PI)
        .sin()
        .max(0.0);
    let gaze = match toward {
        Some(d) => [
            (d[0] * 0.7 + g[0] * 0.5).clamp(-1.0, 1.0),
            (d[1] * 0.7 + g[1] * 0.5).clamp(-1.0, 1.0),
        ],
        None => g,
    };
    Rem { gaze, crack }
}

/// One REM burst every 15s: 14.5s still, then a 0.5s flurry.
const BURST_CYCLE: f32 = 15.0;
const BURST_START: f32 = 14.5;

/// Bursty REM saccade pattern: returns (gaze direction, is_darting_right_now).
/// Caller must ensure openness < 0.35 before calling.
/// Seeded by dream_age and seed for deterministic variation.
pub fn rem_gaze(age: f32, seed: u64, motion_scale: f32) -> ([f32; 2], bool) {
    // If player has reduced_motion on, suppress bursts but keep subtle drift.
    let suppress = motion_scale < 0.3;

    let phase = age.rem_euclid(BURST_CYCLE); // 2s quiet, 0.5s burst

    if !suppress && phase > BURST_START {
        // Bursting: ~4 darts over 0.5s, ~50ms each, horizontal-dominant
        let dart_ix = ((age * 20.0) as u32) % 4;  // 4 darts per burst
        let seed_this = seed ^ (dart_ix as u64);
        let h = hash01(seed_this, 1);
        let v = hash01(seed_this, 2);

        // Horizontal: ±6 cells max, vertical: ±1 cell (favour horizontal)
        let gaze = [
            -1.0 + 2.0 * h,          // -1..1 horizontal (saccade range)
            (v - 0.5) * 0.33,        // -0.165..0.165 vertical (small)
        ];
        (gaze, true)  // is_darting
    } else {
        // Quiet phase: subtle slow drift
        let t = phase / BURST_START;  // 0..1 over the quiet stretch
        let gaze = [
            (t * 4.0 - 2.0).sin() * 0.3,      // slow sine, small amplitude
            ((t * 2.0) * std::f32::consts::PI).sin() * 0.15,
        ];
        (gaze, false)
    }
}

/// Fast hash: map u64 seed to f32 in [0, 1). Used by rem_gaze for reproducible randomness.
pub fn hash01(k: u64, salt: u32) -> f32 {
    let mut x = k ^ (salt as u64);
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 33;
    x = x.wrapping_mul(0xC2B2_AE35);
    x ^= x >> 33;
    (x as f32) / (u64::MAX as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entrance_starts_above_the_screen_with_lid_shut() {
        let e = Entrance::new();
        assert_eq!(e.height(), 1.0, "starts fully above the screen");
        assert_eq!(e.lid(), LID_SHUT);
        assert!(e.lid() < 0.35, "REM is active while falling");
        assert!(!e.done);
    }

    #[test]
    fn eye_lands_before_the_lid_opens() {
        let mut e = Entrance::new();
        e.t = DROP_END;
        assert!(e.height().abs() < 1e-6, "in place at DROP_END");
        assert_eq!(e.lid(), LID_SHUT, "still shut on landing");
    }

    #[test]
    fn descent_only_ever_moves_down() {
        let mut e = Entrance::new();
        let mut last = e.height();
        for i in 1..=100 {
            e.t = DROP_END * i as f32 / 100.0;
            let h = e.height();
            assert!(h <= last + 1e-6, "rose at step {i}");
            last = h;
        }
    }

    #[test]
    fn descent_eases_in_and_out() {
        let mut e = Entrance::new();
        e.t = DROP_END * 0.05;
        assert!(e.height() > 0.99, "barely moves at first");
        e.t = DROP_END * 0.5;
        assert!((e.height() - 0.5).abs() < 1e-6, "halfway at the midpoint");
        e.t = DROP_END * 0.95;
        assert!(e.height() < 0.01, "settles gently");
    }

    #[test]
    fn lid_opens_smoothly_and_monotonically() {
        let mut e = Entrance::new();
        let mut last = 0.0;
        for i in 0..=100 {
            e.t = DROP_END + (OPEN_END - DROP_END) * i as f32 / 100.0;
            let l = e.lid();
            assert!(l >= last - 1e-6, "lid closed again at step {i}");
            assert!(l <= 1.0 + 1e-6);
            last = l;
        }
        assert!((last - 1.0).abs() < 1e-6, "fully open at OPEN_END");
    }

    #[test]
    fn glance_is_quiet_for_the_first_period() {
        for i in 0..200 {
            assert_eq!(glance(i as f32 * 0.1), 0.0);
        }
    }

    #[test]
    fn glance_fires_every_period_and_lets_go() {
        for k in 1..=5 {
            let t0 = GLANCE_PERIOD * k as f32;
            assert_eq!(glance(t0), 0.0, "starts from nothing");
            assert!((glance(t0 + GLANCE_LEN * 0.5) - 1.0).abs() < 1e-6, "full hold");
            assert_eq!(glance(t0 + GLANCE_LEN), 0.0, "fully released");
            assert_eq!(glance(t0 + GLANCE_PERIOD - 0.1), 0.0, "quiet until the next");
        }
    }

    #[test]
    fn glance_ramps_without_jumps() {
        let mut last = glance(0.0);
        let mut t = 0.0;
        while t < GLANCE_PERIOD * 3.0 {
            t += 0.01;
            let g = glance(t);
            assert!((0.0..=1.0).contains(&g));
            assert!((g - last).abs() < 0.05, "jump at t={t}");
            last = g;
        }
    }

    #[test]
    fn rem_cracks_the_lid_only_during_bursts() {
        assert_eq!(rem(1.0, 7, 1.0, None).crack, 0.0, "still between bursts");
        assert!(rem(14.75, 7, 1.0, None).crack > 0.99, "widest mid-burst");
        assert!(rem(14.51, 7, 1.0, None).crack < 0.1, "eases in");
        assert_eq!(rem(14.75, 7, 0.1, None).crack, 0.0, "reduced motion: no cracking");
    }

    #[test]
    fn a_shut_eye_cannot_look_but_a_cracked_one_can() {
        use crate::pixels::gaze_centre;
        let side = Some([1.0, 0.0]);
        assert_eq!(gaze_centre(side, LID_SHUT, false), (0, 0), "shut: pupil is stuck");
        for lucid in [false, true] {
            let at_crack = gaze_centre(side, CRACK_LID, lucid).0;
            let at_glance = gaze_centre(side, GLANCE_LID, lucid).0;
            assert!(at_glance >= at_crack, "wider lid never looks less far");
            assert!(at_glance >= 2, "glance visibly looks aside (lucid={lucid}): {at_glance}");
        }
        assert!(gaze_centre(side, CRACK_LID, false).0 >= 1, "crack lets the pupil move");
    }

    #[test]
    fn lid_closes_gradually_instead_of_snapping() {
        let mut lid = 1.0;
        let mut last = lid;
        for _ in 0..30 {
            lid = ease_lid(lid, 0.08, 1.0 / 60.0);
            assert!(lid < last, "keeps closing");
            assert!(last - lid < 0.05, "no big step in one frame");
            last = lid;
        }
        assert!(lid > 0.08, "not there yet after half a second");
        for _ in 0..600 {
            lid = ease_lid(lid, 0.08, 1.0 / 60.0);
        }
        assert!((lid - 0.08).abs() < 1e-3, "arrives eventually");
    }

    #[test]
    fn lid_easing_never_overshoots_or_depends_on_a_long_frame() {
        assert!(ease_lid(1.0, 0.08, 5.0) >= 0.08);
        assert!(ease_lid(0.08, 1.0, 5.0) <= 1.0);
        assert_eq!(ease_lid(0.5, 0.5, 0.016), 0.5);
        assert_eq!(ease_lid(0.9, 0.1, 0.0), 0.9);
    }

    #[test]
    fn lid_closes_slower_than_it_opens() {
        let down = 1.0 - ease_lid(1.0, 0.0, 0.1);
        let up = ease_lid(0.0, 1.0, 0.1);
        assert!(up > down);
    }

    #[test]
    fn rem_leans_toward_the_objective() {
        let toward = Some([1.0, 0.0]);
        let mut sum = 0.0;
        let mut n = 0.0;
        for i in 0..40 {
            let age = BURST_START + 0.5 * i as f32 / 40.0 + BURST_CYCLE * i as f32;
            sum += rem(age, 99, 1.0, toward).gaze[0];
            n += 1.0;
        }
        assert!(sum / n > 0.2, "mean gaze should sit toward +x, got {}", sum / n);
    }

    #[test]
    fn rem_bursts_are_horizontal_and_fast() {
        // phase > 14.5 = burst phase
        let (g, is_dart) = rem_gaze(14.7, 12345, 1.0);
        assert!(is_dart, "bursting");
        assert!(g[0].abs() > 0.1, "horizontal component exists");
        assert!(g[1].abs() < 0.3, "vertical is small");
    }

    #[test]
    fn rem_quiet_phase_is_slow_and_subtle() {
        // phase < 14.5 = quiet phase
        let (g, is_dart) = rem_gaze(1.0, 12345, 1.0);
        assert!(!is_dart, "not darting in quiet");
        assert!(g[0].abs() < 0.5, "horizontal stays small");
        assert!(g[1].abs() < 0.3, "vertical stays small");
    }

    #[test]
    fn rem_suppressed_when_reduced_motion() {
        let (_, is_dart_full) = rem_gaze(14.7, 12345, 1.0);
        let (_, is_dart_reduced) = rem_gaze(14.7, 12345, 0.25);

        assert!(is_dart_full, "bursts normally");
        assert!(!is_dart_reduced, "no bursts with reduced_motion");
    }

    #[test]
    fn rem_cycles_every_15_seconds() {
        let (_, d1) = rem_gaze(14.6, 12345, 1.0);  // burst
        let (_, d2) = rem_gaze(14.6 + 15.0, 12345, 1.0);  // next cycle, same phase
        assert_eq!(d1, d2, "cycle repeats every 15s");
    }

    #[test]
    fn rem_gaze_is_seeded_and_deterministic() {
        let (g1a, _) = rem_gaze(14.7, 100, 1.0);
        let (g1b, _) = rem_gaze(14.7, 100, 1.0);
        assert_eq!(g1a, g1b, "same seed, same gaze");

        let (g2, _) = rem_gaze(14.7, 101, 1.0);
        assert_ne!(g1a[0], g2[0], "different seed, different gaze");
    }

    #[test]
    fn hash01_spreads_inputs_and_stays_in_range() {
        for seed in &[0, 1, 12345, u64::MAX / 2, u64::MAX] {
            for salt in 0..10u32 {
                let h = hash01(*seed, salt);
                assert!(h >= 0.0 && h < 1.0, "hash01 out of range: {h}");
            }
        }
    }

    #[test]
    fn entrance_tick_increments_time() {
        let mut e = Entrance::new();
        assert_eq!(e.t, 0.0);
        e.tick(0.5);
        assert_eq!(e.t, 0.5);
        e.tick(0.3);
        assert_eq!(e.t, 0.8);
    }

    #[test]
    fn entrance_reset_clears_state() {
        let mut e = Entrance::new();
        e.t = 5.0;
        e.done = true;
        e.reset();
        assert_eq!(e.t, 0.0);
        assert!(!e.done);
    }

    #[test]
    fn entrance_marks_done_after_open_end() {
        let mut e = Entrance::new();
        e.tick(OPEN_END - 0.01);
        assert!(!e.done, "not done yet");
        e.tick(0.02);
        assert!(e.done, "marked done after OPEN_END");
    }
}
