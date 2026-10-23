//! LEECH: crawls up to you, latches on, and drains your nerve until shaken.
//!
//! Pure rules, no GL and no clock. A touch latches it (the game checks WARD
//! first: a ward blocks the latch and is spent). Latched, it rides you and
//! asks for 1 nerve every [`DRAIN_EVERY`] seconds; the game applies that
//! through the normal hit path, so invulnerability still counts. You shake
//! it with [`SHAKE_JUMPS`] jumps inside [`SHAKE_WINDOW`], or by striking it.
//! Being shaken staggers it for [`STAGGER_SECS`], after which it resumes.

use crate::dream::DreamTheme;
use crate::gameplay::CELL;
use engine::glam::Vec3;

pub const MIN_DEPTH: u32 = 8;
pub const HP: f32 = 2.0;
/// Crawl speed, as a multiple of the player's.
pub const SPEED_MULT: f32 = 0.6;
/// One nerve is drained this often while latched (first drain after one period).
pub const DRAIN_EVERY: f32 = 2.0;
/// Jumps (ground or air, each counts once) within this many seconds shake it.
pub const SHAKE_JUMPS: usize = 3;
pub const SHAKE_WINDOW: f32 = 2.0;
/// Seconds it is stunned after being shaken or struck off.
pub const STAGGER_SECS: f32 = 1.5;
/// It touches you (and can latch) within this.
pub const TOUCH: f32 = 0.85;
/// It crawls toward you from this many cells away, and idles beyond.
pub const SENSE_CELLS: f32 = 5.0;
/// When shaken it lands this far from you.
pub const FLING: f32 = 1.5;
/// Where it clings, above the dreamer's centre.
pub const RIDE_HEIGHT: f32 = 0.6;

pub const STREAM: u64 = 0x1EEC_4;

/// One placement seed for the dream's leech, or none: below [`MIN_DEPTH`], in
/// nightmares; never more than one.
pub fn spawn_plan(depth: u32, theme: DreamTheme, seed: u64, nightmare: bool) -> Vec<u64> {
    let _ = theme;
    if nightmare || depth < MIN_DEPTH {
        return Vec::new();
    }
    vec![seed ^ STREAM]
}

/// The latch: when the next drain falls due, and the jumps so far.
#[derive(Clone, Debug, PartialEq)]
pub struct Latch {
    drain_in: f32,
    clock: f32,
    jumps: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Leech {
    pub pos: Vec3,
    pub latch: Option<Latch>,
    drain_due: bool,
}

/// What a tick found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tick {
    /// Crawling and touching you: the game decides if it latches.
    pub touching: bool,
}

impl Leech {
    pub fn new(pos: Vec3) -> Self {
        Self { pos, latch: None, drain_due: false }
    }

    pub fn latched(&self) -> bool {
        self.latch.is_some()
    }

    /// A drain is due this frame (true for exactly one `update`).
    pub fn drain_due(&self) -> bool {
        self.drain_due
    }

    /// Hooks onto the dreamer. Its drain clock starts from a full period.
    pub fn latch_on(&mut self) {
        self.latch = Some(Latch { drain_in: DRAIN_EVERY, clock: 0.0, jumps: Vec::new() });
    }

    /// Lets go and lands `FLING` away from `player` (away from where it clung).
    pub fn release(&mut self, player: Vec3, away: Vec3) {
        self.latch = None;
        self.drain_due = false;
        let away = Vec3::new(away.x, 0.0, away.z).normalize_or_zero();
        let away = if away == Vec3::ZERO { Vec3::X } else { away };
        self.pos = Vec3::new(player.x, self.pos.y.min(player.y), player.z) + away * FLING;
    }

    /// The dreamer jumped (ground or air; call once per jump). True when that
    /// was the jump that shook it off; the latch is cleared (call `release`
    /// for the landing).
    pub fn on_jump(&mut self) -> bool {
        let Some(l) = &mut self.latch else {
            return false;
        };
        l.jumps.push(l.clock);
        let now = l.clock;
        l.jumps.retain(|&t| now - t <= SHAKE_WINDOW);
        l.jumps.len() >= SHAKE_JUMPS
    }

    /// Advances by `dt`. Crawling: toward the dreamer, flat. Latched: rides
    /// the dreamer and counts down to the next drain.
    pub fn update(&mut self, dt: f32, player: Vec3, player_speed: f32) -> Tick {
        self.drain_due = false;
        if let Some(l) = &mut self.latch {
            l.clock += dt;
            l.drain_in -= dt;
            if l.drain_in <= 0.0 {
                l.drain_in += DRAIN_EVERY;
                self.drain_due = true;
            }
            self.pos = player + Vec3::Y * RIDE_HEIGHT;
            return Tick::default();
        }
        let flat = Vec3::new(player.x - self.pos.x, 0.0, player.z - self.pos.z);
        let d = flat.length();
        if d < SENSE_CELLS * CELL && d > 0.05 {
            let step = (player_speed * SPEED_MULT * dt).min(d);
            self.pos += flat / d * step;
        }
        Tick { touching: self.pos.distance(player) < TOUCH + 0.4 && (self.pos.y - player.y).abs() < 1.5 }
    }

    /// 0..1: how close the next drain is (for the pulse on the dreamer).
    pub fn drain_progress(&self) -> f32 {
        self.latch.as_ref().map_or(0.0, |l| 1.0 - (l.drain_in / DRAIN_EVERY).clamp(0.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::ALL_THEMES;
    use crate::gameplay::MOVE_SPEED;

    const DT: f32 = 1.0 / 60.0;
    const ME: Vec3 = Vec3::new(0.0, 0.5, 0.0);

    fn latched() -> Leech {
        let mut l = Leech::new(ME);
        l.latch_on();
        l
    }

    #[test]
    fn the_constants_are_the_briefs() {
        assert_eq!(MIN_DEPTH, 8);
        assert_eq!(HP, 2.0);
        assert_eq!(SPEED_MULT, 0.6);
        assert_eq!(DRAIN_EVERY, 2.0);
        assert_eq!(SHAKE_JUMPS, 3);
        assert_eq!(SHAKE_WINDOW, 2.0);
        assert_eq!(STAGGER_SECS, 1.5);
        assert_eq!(crate::combat::base_hp(crate::combat::Kind::Leech), 2.0);
    }

    #[test]
    fn it_crawls_at_six_tenths_of_the_players_speed() {
        let mut l = Leech::new(Vec3::new(0.0, 0.5, 10.0));
        let me = Vec3::new(0.0, 0.5, 0.0);
        for _ in 0..60 {
            l.update(DT, me, MOVE_SPEED);
        }
        assert!((10.0 - l.pos.z - MOVE_SPEED * 0.6).abs() < 0.05, "{:?}", l.pos);
        assert!(MOVE_SPEED * SPEED_MULT < MOVE_SPEED);
    }

    #[test]
    fn it_idles_when_you_are_far() {
        let mut l = Leech::new(Vec3::new(0.0, 0.5, 30.0));
        for _ in 0..60 {
            l.update(DT, ME, MOVE_SPEED);
        }
        assert_eq!(l.pos.z, 30.0);
    }

    #[test]
    fn touching_is_reported_only_when_it_reaches_you() {
        let mut l = Leech::new(Vec3::new(0.0, 0.5, 6.0));
        assert!(!l.update(DT, ME, MOVE_SPEED).touching);
        let mut touched_at = None;
        for k in 0..(60 * 5) {
            if l.update(DT, ME, MOVE_SPEED).touching {
                touched_at = Some(k as f32 * DT);
                break;
            }
        }
        let t = touched_at.expect("it reaches you");
        assert!(t > 1.0 && t < 3.0, "{t}");
    }

    #[test]
    fn a_latched_leech_drains_one_every_two_seconds() {
        let mut l = latched();
        let mut drains = Vec::new();
        let mut t = 0.0;
        for _ in 0..(60 * 7) {
            t += DT;
            l.update(DT, ME, MOVE_SPEED);
            if l.drain_due() {
                drains.push(t);
            }
        }
        assert_eq!(drains.len(), 3, "{drains:?}");
        for (k, d) in drains.iter().enumerate() {
            assert!((d - 2.0 * (k + 1) as f32).abs() < 0.05, "drain {k} at {d}");
        }
    }

    #[test]
    fn a_drain_is_due_for_one_update_only() {
        let mut l = latched();
        let mut run = 0;
        let mut longest = 0;
        for _ in 0..300 {
            l.update(DT, ME, MOVE_SPEED);
            if l.drain_due() {
                run += 1;
                longest = longest.max(run);
            } else {
                run = 0;
            }
        }
        assert_eq!(longest, 1);
    }

    #[test]
    fn it_rides_you_while_you_move() {
        let mut l = latched();
        let mut p = ME;
        for _ in 0..120 {
            p.x += MOVE_SPEED * DT;
            let t = l.update(DT, p, MOVE_SPEED);
            assert!(!t.touching, "already on you");
            assert!(l.pos.distance(p + Vec3::Y * RIDE_HEIGHT) < 1e-4);
        }
        assert!(l.latched());
    }

    #[test]
    fn three_jumps_within_two_seconds_shake_it() {
        let mut l = latched();
        assert!(!l.on_jump());
        for _ in 0..30 {
            l.update(DT, ME, MOVE_SPEED);
        }
        assert!(!l.on_jump());
        for _ in 0..30 {
            l.update(DT, ME, MOVE_SPEED);
        }
        assert!(l.on_jump(), "the third jump, 1s after the first");
    }

    #[test]
    fn jumps_spread_over_more_than_two_seconds_do_not() {
        let mut l = latched();
        // One jump every 1.1s: any three span 2.2s.
        for _ in 0..8 {
            assert!(!l.on_jump());
            for _ in 0..66 {
                l.update(DT, ME, MOVE_SPEED);
            }
        }
        assert!(l.latched());
    }

    #[test]
    fn the_window_edge_counts_and_a_jump_counts_once() {
        let mut l = latched();
        assert!(!l.on_jump());
        for _ in 0..60 {
            l.update(DT, ME, MOVE_SPEED);
        }
        assert!(!l.on_jump());
        for _ in 0..59 {
            l.update(DT, ME, MOVE_SPEED);
        }
        assert!(l.on_jump(), "first jump 1.98s ago: still inside the window");
        let mut slow = latched();
        slow.on_jump();
        for _ in 0..70 {
            slow.update(DT, ME, MOVE_SPEED);
        }
        slow.on_jump();
        for _ in 0..70 {
            slow.update(DT, ME, MOVE_SPEED);
        }
        assert!(!slow.on_jump(), "the first jump has aged out");
    }

    #[test]
    fn jumps_before_the_latch_do_not_count() {
        let mut l = Leech::new(ME);
        assert!(!l.on_jump());
        assert!(!l.on_jump());
        assert!(!l.on_jump());
        l.latch_on();
        assert!(!l.on_jump(), "the jumps were made before it latched");
    }

    #[test]
    fn releasing_puts_it_a_step_away_and_clears_the_clock() {
        let mut l = latched();
        l.update(1.9, ME, MOVE_SPEED);
        l.release(ME, Vec3::new(0.0, 0.0, 1.0));
        assert!(!l.latched());
        assert!(!l.drain_due());
        assert!((l.pos.z - FLING).abs() < 1e-4, "{:?}", l.pos);
        assert_eq!(l.drain_progress(), 0.0);
        // Latching again starts a full period.
        l.latch_on();
        l.update(1.9, ME, MOVE_SPEED);
        assert!(!l.drain_due());
    }

    #[test]
    fn spawn_plan_gates_by_depth_nightmare_and_caps_at_one() {
        let theme = ALL_THEMES[0];
        assert!(spawn_plan(7, theme, 7, false).is_empty());
        assert!(spawn_plan(30, theme, 7, true).is_empty());
        for depth in 8..30 {
            assert_eq!(spawn_plan(depth, theme, 7, false).len(), 1);
        }
        assert_ne!(spawn_plan(8, theme, 1, false), crate::mask::spawn_plan(8, theme, 1, false));
    }

    #[test]
    fn all_three_stay_within_the_total_cap() {
        let theme = ALL_THEMES[0];
        for depth in 0..40 {
            for nightmare in [false, true] {
                let n = crate::moths::spawn_plan(depth, theme, 9, nightmare).len()
                    + crate::mask::spawn_plan(depth, theme, 9, nightmare).len()
                    + spawn_plan(depth, theme, 9, nightmare).len();
                assert!(n <= crate::foespawn::MAX_NEW);
                if nightmare || depth < 6 {
                    assert_eq!(n, 0, "depth {depth}");
                }
            }
        }
        let at = |d| {
            crate::moths::spawn_plan(d, theme, 9, false).len()
                + crate::mask::spawn_plan(d, theme, 9, false).len()
                + spawn_plan(d, theme, 9, false).len()
        };
        assert_eq!((at(6), at(7), at(8)), (1, 2, 3));
    }
}
