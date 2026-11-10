//! Death dissolve: the dreamer comes apart into embers, then the Summary
//! fades in. Pure timing and seeded particles; main.rs wires it to the
//! shader (`uDissolve`), the ember entities and the flash overlay.

use crate::ui_motion::{smoothstep, UiJuice};
use crate::worth::mix;

/// Run-ending death.
pub const FULL_SECS: f32 = 1.8;
/// Mid-run catch.
pub const SHORT_SECS: f32 = 0.4;
/// Reduced motion: a plain fade instead of the dissolve.
pub const CALM_SECS: f32 = 0.5;
/// Ember overlay cap with `reduce_flashing`.
pub const GENTLE_OVERLAY: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Full,
    Short,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ember {
    /// Offset from the dreamer's feet, metres.
    pub offset: [f32; 3],
    /// Edge length, metres.
    pub size: f32,
    /// 0..1, already faded in and out.
    pub alpha: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Death {
    kind: Kind,
    age: f32,
    first_person: bool,
    color: [f32; 3],
    seed: u64,
    calm: bool,
    embers: bool,
}

impl Death {
    pub fn new(kind: Kind, first_person: bool, color: [f32; 3]) -> Self {
        Self {
            kind,
            age: 0.0,
            first_person,
            color,
            seed: 0,
            calm: false,
            embers: true,
        }
    }

    /// Seed for the ember field.
    pub fn seeded(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Applies the accessibility settings: reduced motion = no particles and
    /// a plain 0.5 s fade for the full version; juice Off = no particles.
    pub fn accessible(mut self, reduced_motion: bool, juice: UiJuice) -> Self {
        self.calm = reduced_motion;
        self.embers = !reduced_motion && juice != UiJuice::Off;
        self
    }

    pub fn color(&self) -> [f32; 3] {
        self.color
    }

    pub fn first_person(&self) -> bool {
        self.first_person
    }

    pub fn duration(&self) -> f32 {
        match self.kind {
            Kind::Short => SHORT_SECS,
            Kind::Full if self.calm => CALM_SECS,
            Kind::Full => FULL_SECS,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.age = (self.age + dt.max(0.0)).min(self.duration());
    }

    /// 0..1, smooth; monotone.
    pub fn progress(&self) -> f32 {
        smoothstep(self.age / self.duration())
    }

    pub fn finished(&self) -> bool {
        self.age >= self.duration()
    }

    pub fn skip(&mut self) {
        self.age = self.duration();
    }

    /// `uDissolve` for the dreamer (and later the held weapon): 0 solid, 1 gone.
    pub fn body_dissolve(&self) -> f32 {
        self.progress()
    }

    /// First person: ember / desaturate overlay, 0..1. Third person: 0.
    pub fn view_fade(&self) -> f32 {
        if self.first_person {
            self.progress()
        } else {
            0.0
        }
    }

    /// Overlay strength for the flash path, capped when flashing is reduced.
    pub fn overlay(&self, reduce_flashing: bool) -> f32 {
        let v = self.view_fade() * 0.85;
        if reduce_flashing {
            v.min(GENTLE_OVERLAY)
        } else {
            v
        }
    }

    /// `n` embers drifting up from the dissolving body; none when motion is
    /// reduced / juice is off. Deterministic for a seed and age.
    pub fn particles(&self, n: usize) -> Vec<Ember> {
        if !self.embers {
            return Vec::new();
        }
        let p = self.progress();
        let unit = |i: usize, salt: u64| -> f32 {
            (mix(self.seed, (i as u64) << 8 | salt) % 10_000) as f32 / 10_000.0
        };
        (0..n)
            .map(|i| {
                // Each ember is born while the body is burning away, then rises.
                let born = 0.05 + 0.6 * unit(i, 1);
                let life = (p - born).max(0.0) / (1.0 - born);
                let ang = unit(i, 2) * std::f32::consts::TAU;
                let r = 0.35 * unit(i, 3).sqrt();
                let y0 = 1.2 * unit(i, 4);
                let sway = 0.15 * (life * 5.0 + unit(i, 5) * 6.28).sin();
                Ember {
                    offset: [
                        r * ang.cos() + sway,
                        y0 + 1.4 * life,
                        r * ang.sin() - sway,
                    ],
                    size: 0.05 + 0.07 * unit(i, 6),
                    alpha: if life > 0.0 {
                        smoothstep(life * 6.0) * (1.0 - smoothstep(life))
                    } else {
                        0.0
                    },
                }
            })
            .collect()
    }
}

/// Juice Off gets the short version of the run-ending death.
pub fn short_for(reduced_motion: bool, juice: UiJuice) -> bool {
    let _ = reduced_motion; // reduced motion is a plain 0.5 s fade, see `accessible`
    juice == UiJuice::Off
}

/// The run-ending death holds the previous mode until it has finished.
pub fn should_enter_summary(d: &Death) -> bool {
    d.finished()
}

#[cfg(test)]
mod tests {
    use super::*;

    const C: [f32; 3] = [1.0, 0.5, 0.2];

    fn run(mut d: Death) -> Vec<f32> {
        let mut v = vec![d.progress()];
        while !d.finished() {
            d.update(1.0 / 60.0);
            v.push(d.progress());
        }
        v
    }

    #[test]
    fn progress_is_monotone_from_zero_to_one() {
        let v = run(Death::new(Kind::Full, false, C));
        assert_eq!(v[0], 0.0);
        assert_eq!(*v.last().unwrap(), 1.0);
        assert!(v.windows(2).all(|w| w[1] >= w[0]));
    }

    #[test]
    fn finished_only_at_the_end() {
        let mut d = Death::new(Kind::Full, false, C);
        d.update(FULL_SECS - 0.01);
        assert!(!d.finished());
        d.update(0.02);
        assert!(d.finished());
        assert_eq!(d.progress(), 1.0);
    }

    #[test]
    fn skip_jumps_to_finished() {
        let mut d = Death::new(Kind::Full, true, C);
        d.update(0.2);
        d.skip();
        assert!(d.finished() && d.body_dissolve() == 1.0);
    }

    #[test]
    fn short_is_shorter_than_full_and_calm_is_half_a_second() {
        let s = Death::new(Kind::Short, false, C);
        let f = Death::new(Kind::Full, false, C);
        assert!(s.duration() < f.duration());
        let calm = f.accessible(true, UiJuice::Soft);
        assert_eq!(calm.duration(), CALM_SECS);
    }

    #[test]
    fn reduced_or_off_has_no_particles() {
        let mut d = Death::new(Kind::Full, false, C).seeded(3);
        d.update(1.0);
        assert!(!d.particles(16).is_empty());
        assert!(d.accessible(true, UiJuice::Full).particles(16).is_empty());
        assert!(d.accessible(false, UiJuice::Off).particles(16).is_empty());
    }

    #[test]
    fn particles_are_deterministic_per_seed() {
        let mk = |s| {
            let mut d = Death::new(Kind::Full, false, C).seeded(s);
            d.update(1.1);
            d.particles(12)
        };
        assert_eq!(mk(7), mk(7));
        assert_ne!(mk(7), mk(8));
        assert!(mk(7).iter().all(|e| (0.0..=1.0).contains(&e.alpha)));
    }

    #[test]
    fn first_person_fades_the_view_only_there() {
        let mut fp = Death::new(Kind::Full, true, C);
        let mut tp = Death::new(Kind::Full, false, C);
        fp.update(1.0);
        tp.update(1.0);
        assert!(fp.view_fade() > 0.0 && tp.view_fade() == 0.0);
        fp.skip();
        assert!(fp.overlay(true) <= GENTLE_OVERLAY);
        assert!(fp.overlay(false) > GENTLE_OVERLAY);
    }

    #[test]
    fn juice_off_picks_the_short_version() {
        assert!(short_for(false, UiJuice::Off));
        assert!(!short_for(false, UiJuice::Soft));
        assert!(!short_for(true, UiJuice::Full));
    }

    #[test]
    fn a_dying_run_enters_summary_only_when_finished() {
        let mut d = Death::new(Kind::Full, false, C);
        assert!(!should_enter_summary(&d));
        d.update(FULL_SECS * 0.5);
        assert!(!should_enter_summary(&d));
        d.skip();
        assert!(should_enter_summary(&d));
    }
}
