//! Shared UI motion: easing curves, a damped spring, number tick-ups and the
//! UI-juice setting every animated screen reads.
//!
//! Pure: no GL, no clock. Callers feed `dt` / elapsed seconds. Kept soft on
//! purpose: overshoot is small and nothing here loops on its own.

use serde::{Deserialize, Serialize};

/// How much the UI moves. `Off` is stillness (final state at t = 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiJuice {
    Off,
    #[default]
    Soft,
    Full,
}

impl UiJuice {
    /// Strength multiplier for amplitudes and durations.
    pub fn scale(self) -> f32 {
        match self {
            UiJuice::Off => 0.0,
            UiJuice::Soft => 1.0,
            UiJuice::Full => 1.4,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            UiJuice::Off => "off",
            UiJuice::Soft => "soft",
            UiJuice::Full => "full",
        }
    }

    /// Left/right on the settings row: Off <-> Soft <-> Full, no wrap.
    pub fn step(self, dir: i32) -> Self {
        const ORDER: [UiJuice; 3] = [UiJuice::Off, UiJuice::Soft, UiJuice::Full];
        let i = ORDER.iter().position(|&j| j == self).unwrap_or(1) as i32;
        ORDER[(i + dir.signum()).clamp(0, 2) as usize]
    }
}

/// Effective motion strength: zero when the player asked for reduced motion
/// or turned UI juice off.
pub fn effective(juice: UiJuice, reduced_motion: bool) -> f32 {
    if reduced_motion {
        0.0
    } else {
        juice.scale()
    }
}

pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = 1.0 - t.clamp(0.0, 1.0);
    1.0 - t * t * t
}

/// Ease-out with a gentle overshoot (peaks near 1.04), settling at 1.
pub fn ease_out_back(t: f32) -> f32 {
    const C1: f32 = 0.9;
    const C3: f32 = C1 + 1.0;
    let t = t.clamp(0.0, 1.0) - 1.0;
    1.0 + C3 * t * t * t + C1 * t * t
}

/// A value chasing a target with a damped spring.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub pos: f32,
    pub vel: f32,
}

impl Spring {
    pub const fn at(pos: f32) -> Self {
        Self { pos, vel: 0.0 }
    }

    /// Advance by `dt`. Sub-stepped so large frames (up to 0.1 s) stay stable.
    pub fn step(&mut self, target: f32, dt: f32, stiffness: f32, damping: f32) {
        let dt = dt.clamp(0.0, 0.1);
        let n = ((dt / 0.004).ceil() as usize).max(1);
        let h = dt / n as f32;
        for _ in 0..n {
            let acc = stiffness * (target - self.pos) - damping * self.vel;
            self.vel += acc * h;
            self.pos += self.vel * h;
        }
    }

    /// Settled: close to the target and nearly still.
    #[cfg(test)]
    pub fn settled(&self, target: f32) -> bool {
        (self.pos - target).abs() < 1e-3 && self.vel.abs() < 1e-2
    }
}

/// Stiffness / damping that feel soft and settle in about a third of a second.
pub const SOFT_STIFFNESS: f32 = 220.0;
pub const SOFT_DAMPING: f32 = 26.0;

/// A counter easing from `from` to `to` over `t` in 0..=1; exact at both ends.
pub fn tick_up(from: i64, to: i64, t: f32) -> i64 {
    if t <= 0.0 {
        return from;
    }
    if t >= 1.0 {
        return to;
    }
    let k = ease_out_cubic(t) as f64;
    from + ((to - from) as f64 * k).round() as i64
}

/// Progress 0..=1 of item `i` in a staggered entrance that started at `t`
/// seconds: each item starts `per_item` after the last and lasts `dur`.
pub fn stagger(i: usize, per_item: f32, dur: f32, t: f32) -> f32 {
    if dur <= 0.0 {
        return 1.0;
    }
    ((t - i as f32 * per_item) / dur).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easings_hit_their_endpoints() {
        for f in [smoothstep, ease_out_cubic, ease_out_back] {
            assert!(f(0.0).abs() < 1e-6);
            assert!((f(1.0) - 1.0).abs() < 1e-6);
            assert!(f(-3.0).abs() < 1e-6 && (f(9.0) - 1.0).abs() < 1e-6, "clamped");
        }
    }

    #[test]
    fn smoothstep_and_cubic_are_monotone() {
        for f in [smoothstep, ease_out_cubic] {
            let mut last = -1.0;
            for i in 0..=100 {
                let v = f(i as f32 / 100.0);
                assert!(v >= last);
                last = v;
            }
        }
    }

    #[test]
    fn back_overshoot_stays_soft() {
        let peak = (0..=1000)
            .map(|i| ease_out_back(i as f32 / 1000.0))
            .fold(0.0, f32::max);
        assert!(peak > 1.0 && peak <= 1.1, "peak {peak}");
    }

    #[test]
    fn spring_converges_and_stays_finite() {
        for dt in [0.004_f32, 0.016, 0.05, 0.1, 5.0] {
            let mut s = Spring::at(0.0);
            for _ in 0..((6.0 / dt.min(0.1)) as usize) {
                s.step(1.0, dt, SOFT_STIFFNESS, SOFT_DAMPING);
                assert!(s.pos.is_finite() && s.pos.abs() < 3.0, "dt {dt}: {s:?}");
            }
            assert!(s.settled(1.0), "dt {dt}: {s:?}");
        }
    }

    #[test]
    fn spring_overshoot_is_small() {
        let mut s = Spring::at(0.0);
        let mut peak: f32 = 0.0;
        for _ in 0..600 {
            s.step(1.0, 0.016, SOFT_STIFFNESS, SOFT_DAMPING);
            peak = peak.max(s.pos);
        }
        assert!(peak < 1.12, "peak {peak}");
    }

    #[test]
    fn tick_up_is_monotone_and_exact_at_the_ends() {
        assert_eq!(tick_up(10, 250, 0.0), 10);
        assert_eq!(tick_up(10, 250, 1.0), 250);
        let mut last = 10;
        for i in 0..=200 {
            let v = tick_up(10, 250, i as f32 / 200.0);
            assert!(v >= last && (10..=250).contains(&v));
            last = v;
        }
        assert_eq!(tick_up(50, 5, 1.0), 5, "counting down works");
    }

    #[test]
    fn stagger_orders_items_and_is_bounded() {
        assert_eq!(stagger(0, 0.03, 0.2, 0.0), 0.0);
        assert!(stagger(0, 0.03, 0.2, 0.1) > stagger(3, 0.03, 0.2, 0.1));
        assert_eq!(stagger(5, 0.03, 0.2, 9.0), 1.0);
        assert_eq!(stagger(5, 0.03, 0.0, 0.0), 1.0, "zero duration = done");
    }

    #[test]
    fn off_and_reduced_motion_mean_no_motion() {
        assert_eq!(effective(UiJuice::Off, false), 0.0);
        assert_eq!(effective(UiJuice::Full, true), 0.0);
        assert_eq!(effective(UiJuice::Soft, false), 1.0);
        assert!(effective(UiJuice::Full, false) > 1.0);
    }

    #[test]
    fn juice_steps_without_wrapping() {
        assert_eq!(UiJuice::default(), UiJuice::Soft);
        assert_eq!(UiJuice::Off.step(-1), UiJuice::Off);
        assert_eq!(UiJuice::Off.step(1), UiJuice::Soft);
        assert_eq!(UiJuice::Soft.step(1), UiJuice::Full);
        assert_eq!(UiJuice::Full.step(1), UiJuice::Full);
    }
}
