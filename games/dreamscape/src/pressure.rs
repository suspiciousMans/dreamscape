//! PRESSURE (hard mode): the dream does not wait for you. After a grace
//! period the eye stirs (enemies get bolder and quicker), then the dream
//! collapses and you wake early. Pure logic; wiring lives in `main.rs`
//! inside the `// === pressure ===` blocks.
#![allow(dead_code)]

/// First depth where pressure applies.
pub const MIN_DEPTH: u32 = 2;
/// Grace at depth 0, minus `GRACE_PER_DEPTH` per depth, floored at `MIN_GRACE`.
pub const GRACE_START: f32 = 75.0;
pub const GRACE_PER_DEPTH: f32 = 5.0;
pub const MIN_GRACE: f32 = 35.0;
/// Seconds the stir rises from 0 to 1.
pub const STIR_SECS: f32 = 30.0;
/// Seconds of warning between full stir and the collapse.
pub const WARN_SECS: f32 = 20.0;
/// Seconds a shard pushes the clock back.
pub const SHARD_REFUND: f32 = 12.0;
/// At full stir: +40% alert radius, +10% speed.
pub const CHASE_BONUS: f32 = 0.40;
pub const SPEED_BONUS: f32 = 0.10;
/// Share of the dream's run dust kept when it collapses.
pub const COLLAPSE_DUST_KEPT: f32 = 0.5;
/// Eye-voice lines.
pub const LINE_STIRRING: &str = "i am waking. do not linger.";
pub const LINE_WARNING: &str = "the dream is coming apart.";
pub const SUMMARY_LINE: &str = "the dream collapsed. half its dust was lost.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Calm,
    Stirring,
    /// Fully stirred; the collapse is counting down.
    Warning,
    Collapsed,
}

/// Seconds of calm at this depth.
pub fn grace(depth: u32) -> f32 {
    (GRACE_START - GRACE_PER_DEPTH * depth.min(1000) as f32).max(MIN_GRACE)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pressure {
    active: bool,
    grace: f32,
    /// Effective elapsed seconds (refunds push this back).
    t: f32,
}

impl Pressure {
    /// Pressure for a dream at `depth` (inactive below `MIN_DEPTH`).
    pub fn new(depth: u32) -> Self {
        Self {
            active: depth >= MIN_DEPTH,
            grace: grace(depth),
            t: 0.0,
        }
    }

    /// Never ticks, never stirs.
    pub fn off() -> Self {
        Self {
            active: false,
            grace: f32::MAX,
            t: 0.0,
        }
    }

    /// The rules for a real dream: normal dreams only, and never on top of
    /// a HASTY timer.
    pub fn for_dream(depth: u32, normal: bool, hasty: bool) -> Self {
        if normal && !hasty {
            Self::new(depth)
        } else {
            Self::off()
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn tick(&mut self, dt: f32) {
        if self.active && self.phase() != Phase::Collapsed {
            self.t += dt.max(0.0);
        }
    }

    pub fn refund_on_shard(&mut self) {
        if self.active && self.phase() != Phase::Collapsed {
            self.t = (self.t - SHARD_REFUND).max(0.0);
        }
    }

    pub fn phase(&self) -> Phase {
        if !self.active || self.t < self.grace {
            Phase::Calm
        } else if self.t < self.grace + STIR_SECS {
            Phase::Stirring
        } else if self.t < self.grace + STIR_SECS + WARN_SECS {
            Phase::Warning
        } else {
            Phase::Collapsed
        }
    }

    /// 0 while calm, rising to 1 across the stirring phase, 1 after.
    pub fn stir(&self) -> f32 {
        if !self.active {
            return 0.0;
        }
        ((self.t - self.grace) / STIR_SECS).clamp(0.0, 1.0)
    }

    /// Multiplier on the enemy alert radius.
    pub fn chase_mult(&self) -> f32 {
        1.0 + CHASE_BONUS * self.stir()
    }

    /// Multiplier on enemy speed (callers keep the player-speed cap).
    pub fn speed_mult(&self) -> f32 {
        1.0 + SPEED_BONUS * self.stir()
    }

    /// Seconds until the collapse, once the eye has stirred.
    pub fn collapse_in(&self) -> Option<f32> {
        match self.phase() {
            Phase::Stirring | Phase::Warning => {
                Some(self.grace + STIR_SECS + WARN_SECS - self.t)
            }
            _ => None,
        }
    }

    /// How far the eyelid has closed for the HUD: 0 open .. 1 shut. Starts
    /// creeping only when stirring begins.
    pub fn lid(&self) -> f32 {
        if !self.active {
            return 0.0;
        }
        ((self.t - self.grace) / (STIR_SECS + WARN_SECS)).clamp(0.0, 1.0)
    }
}

/// Dust a collapsing dream leaves you: the part earned during the dream is
/// halved. `before` = run dust at dream start, `now` = run dust now.
pub fn collapse_dust(before: u32, now: u32) -> u32 {
    let earned = now.saturating_sub(before);
    before + (earned as f32 * COLLAPSE_DUST_KEPT).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(p: &mut Pressure, secs: f32) {
        for _ in 0..(secs * 10.0) as u32 {
            p.tick(0.1);
        }
    }

    #[test]
    fn grace_scales_with_depth_and_floors() {
        assert_eq!(grace(2), 65.0);
        assert_eq!(grace(4), 55.0);
        assert_eq!(grace(8), 35.0);
        assert_eq!(grace(50), MIN_GRACE);
    }

    #[test]
    fn inactive_below_min_depth() {
        for d in 0..MIN_DEPTH {
            let mut p = Pressure::new(d);
            run(&mut p, 500.0);
            assert_eq!(p.phase(), Phase::Calm);
            assert_eq!(p.stir(), 0.0);
            assert_eq!(p.chase_mult(), 1.0);
            assert_eq!(p.collapse_in(), None);
        }
    }

    #[test]
    fn phase_boundaries() {
        let mut p = Pressure::new(8); // grace 35
        run(&mut p, 34.0);
        assert_eq!(p.phase(), Phase::Calm);
        assert_eq!(p.stir(), 0.0);
        run(&mut p, 2.0); // 36
        assert_eq!(p.phase(), Phase::Stirring);
        assert!(p.stir() > 0.0 && p.stir() < 0.1);
        run(&mut p, 30.0); // 66
        assert_eq!(p.phase(), Phase::Warning);
        assert_eq!(p.stir(), 1.0);
        run(&mut p, 18.0); // 84 (< 85)
        assert_eq!(p.phase(), Phase::Warning);
        assert!(p.collapse_in().unwrap() < 2.0);
        run(&mut p, 2.0);
        assert_eq!(p.phase(), Phase::Collapsed);
        assert_eq!(p.collapse_in(), None);
    }

    #[test]
    fn multipliers_rise_to_caps() {
        let mut p = Pressure::new(8);
        run(&mut p, 35.0 + 15.0);
        assert!((p.stir() - 0.5).abs() < 0.01);
        assert!((p.chase_mult() - 1.2).abs() < 0.01);
        assert!((p.speed_mult() - 1.05).abs() < 0.01);
        run(&mut p, 15.0);
        assert!((p.chase_mult() - 1.4).abs() < 1e-4);
        assert!((p.speed_mult() - 1.1).abs() < 1e-4);
    }

    #[test]
    fn collapse_in_counts_down_from_total() {
        let mut p = Pressure::new(8);
        run(&mut p, 35.0 + 0.5);
        let left = p.collapse_in().unwrap();
        assert!((left - 49.5).abs() < 0.05, "{left}");
    }

    #[test]
    fn shard_refund_pushes_back_and_floors_at_zero() {
        let mut p = Pressure::new(8);
        run(&mut p, 60.0);
        let before = p.collapse_in().unwrap();
        p.refund_on_shard();
        assert!((p.collapse_in().unwrap() - before - SHARD_REFUND).abs() < 0.01);
        let mut q = Pressure::new(8);
        run(&mut q, 5.0);
        q.refund_on_shard();
        assert_eq!(q.phase(), Phase::Calm);
        run(&mut q, 34.0);
        assert_eq!(q.phase(), Phase::Calm); // a full 35s still owed
    }

    #[test]
    fn refund_can_calm_the_stir_down() {
        let mut p = Pressure::new(8);
        run(&mut p, 45.0);
        let s = p.stir();
        p.refund_on_shard();
        assert!(p.stir() < s);
    }

    #[test]
    fn collapsed_is_final() {
        let mut p = Pressure::new(8);
        run(&mut p, 200.0);
        assert_eq!(p.phase(), Phase::Collapsed);
        p.refund_on_shard();
        assert_eq!(p.phase(), Phase::Collapsed);
    }

    #[test]
    fn no_stacking_with_hasty_or_special_dreams() {
        assert!(Pressure::for_dream(6, true, false).active());
        assert!(!Pressure::for_dream(6, true, true).active());
        assert!(!Pressure::for_dream(6, false, false).active());
        let mut p = Pressure::for_dream(6, true, true);
        run(&mut p, 500.0);
        assert_eq!(p.phase(), Phase::Calm);
    }

    #[test]
    fn deeper_dreams_stir_sooner() {
        let (mut a, mut b) = (Pressure::new(2), Pressure::new(6));
        run(&mut a, 50.0);
        run(&mut b, 50.0);
        assert_eq!(a.phase(), Phase::Calm);
        assert_eq!(b.phase(), Phase::Stirring);
    }

    #[test]
    fn lid_closes_from_stir_to_collapse() {
        let mut p = Pressure::new(8);
        run(&mut p, 30.0);
        assert_eq!(p.lid(), 0.0);
        run(&mut p, 5.0 + 25.0);
        assert!((p.lid() - 0.5).abs() < 0.01);
    }

    #[test]
    fn collapse_halves_only_the_dreams_dust() {
        assert_eq!(collapse_dust(20, 20), 20);
        assert_eq!(collapse_dust(20, 30), 25);
        assert_eq!(collapse_dust(20, 31), 26);
        assert_eq!(collapse_dust(20, 10), 20);
    }
}
