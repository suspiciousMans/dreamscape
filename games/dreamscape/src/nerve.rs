//! NERVE: how many hits the dreamer can take before the dream catches them.
//!
//! Pure state, no GL and no clock: the game feeds it `dt` and asks what to do.
//! A hit costs nerve and grants a short invulnerability (so a crowd can never
//! chain-hit you, and flashes stay under about one per 1.2s). At zero the game
//! runs its old *caught* flow (shard dropped, respawn), and nerve refills to
//! [`REFILL`]. Taking a portal restores one.

/// Nerve at the start of a run, and the most a run can reach.
pub const START: u32 = 3;
pub const MAX: u32 = 5;
/// Nerve left after being caught (the respawn).
pub const REFILL: u32 = 2;
/// Seconds of invulnerability after a hit.
pub const IFRAMES: f32 = 1.2;
/// Extra seconds per stack of LONG BREATH.
pub const IFRAMES_PER_BREATH: f32 = 0.4;
/// How long a shove from a hit lasts, and how fast it starts (units / s).
pub const KNOCK_SECS: f32 = 0.25;
pub const KNOCK_SPEED: f32 = 8.0;
/// Flicker rate while invulnerable (toggles per second).
pub const FLICKER_HZ: f32 = 12.0;
/// A lost heart plays its break for this long (seconds since the hit).
pub const BREAK_SECS: f32 = 0.5;

/// Damage by what hit you: nightmare attacks hurt twice as much.
pub fn damage_from(source: &str) -> u32 {
    match source {
        "hunter" | "boss attack" | "slam" => 2,
        "tooth" | "tear" => 1,
        _ => 1,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    /// Ignored: still invulnerable (or nothing left to lose).
    Ignored,
    /// Lost nerve and is still standing.
    Hurt,
    /// Nerve reached zero: the dream catches you.
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nerve {
    cur: u32,
    max: u32,
    /// Seconds of invulnerability left.
    iframes: f32,
    /// Seconds since the last hit (large when none).
    since_hit: f32,
    /// Unit direction (x, z) of the shove, and seconds of it left.
    knock: (f32, f32, f32),
}

impl Default for Nerve {
    fn default() -> Self {
        Self::new(START)
    }
}

impl Nerve {
    pub fn new(start: u32) -> Self {
        let n = start.clamp(1, MAX);
        Self {
            cur: n,
            max: n.max(START),
            iframes: 0.0,
            since_hit: f32::MAX,
            knock: (0.0, 0.0, 0.0),
        }
    }

    pub fn cur(&self) -> u32 {
        self.cur
    }

    pub fn max(&self) -> u32 {
        self.max
    }

    pub fn invulnerable(&self) -> bool {
        self.iframes > 0.0
    }

    /// 1 the instant of a hit, easing to 0 over [`BREAK_SECS`]: the eye's
    /// flinch and the lost heart's break.
    pub fn flinch(&self) -> f32 {
        (1.0 - self.since_hit / BREAK_SECS).clamp(0.0, 1.0)
    }

    pub fn tick(&mut self, dt: f32) {
        self.iframes = (self.iframes - dt).max(0.0);
        self.since_hit = (self.since_hit + dt).min(f32::MAX / 2.0);
        self.knock.2 = (self.knock.2 - dt).max(0.0);
    }

    /// Takes `damage` from something at direction `away` (unit, x/z) from you.
    /// `extra_iframes` is the invulnerability beyond the base (LONG BREATH).
    pub fn hit(&mut self, damage: u32, away: (f32, f32), extra_iframes: f32) -> Hit {
        if self.invulnerable() || damage == 0 {
            return Hit::Ignored;
        }
        self.since_hit = 0.0;
        if damage >= self.cur {
            self.cur = REFILL.min(self.max);
            self.iframes = 0.0;
            self.knock = (0.0, 0.0, 0.0);
            return Hit::Down;
        }
        self.cur -= damage;
        self.iframes = IFRAMES + extra_iframes.max(0.0);
        let len = (away.0 * away.0 + away.1 * away.1).sqrt();
        self.knock = if len > 1e-4 {
            (away.0 / len, away.1 / len, KNOCK_SECS)
        } else {
            (0.0, 0.0, 0.0)
        };
        Hit::Hurt
    }

    /// Restores up to `n` nerve (never past the max).
    pub fn heal(&mut self, n: u32) {
        self.cur = (self.cur + n).min(self.max);
    }

    /// Raises the most you can hold (and heals the same amount).
    #[allow(dead_code)] // SECOND HEARTBEAT (milestone C4)
    pub fn raise_max(&mut self, n: u32) {
        self.max = (self.max + n).min(MAX);
        self.heal(n);
    }

    /// The shove right now, as an (x, z) velocity; fades to nothing.
    pub fn knock_velocity(&self) -> (f32, f32) {
        let (x, z, left) = self.knock;
        let k = KNOCK_SPEED * (left / KNOCK_SECS).clamp(0.0, 1.0);
        (x * k, z * k)
    }

    /// Whether the dreamer is drawn faint right now: blinking while
    /// invulnerable, steady (still faint) with reduced motion.
    pub fn faint(&self, now: f32, reduced: bool) -> bool {
        self.invulnerable() && (reduced || (now * FLICKER_HZ) as i64 % 2 == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AWAY: (f32, f32) = (1.0, 0.0);

    #[test]
    fn a_run_starts_with_three_and_can_hold_five() {
        let n = Nerve::default();
        assert_eq!((n.cur(), n.max()), (3, 3));
        let mut n = n;
        n.raise_max(2);
        assert_eq!((n.cur(), n.max()), (5, 5));
        n.raise_max(1);
        assert_eq!(n.max(), MAX, "never past the cap");
    }

    #[test]
    fn a_hit_costs_one_and_grants_invulnerability() {
        let mut n = Nerve::default();
        assert_eq!(n.hit(1, AWAY, 0.0), Hit::Hurt);
        assert_eq!(n.cur(), 2);
        assert!(n.invulnerable());
        // A second touch during the window does nothing.
        assert_eq!(n.hit(1, AWAY, 0.0), Hit::Ignored);
        assert_eq!(n.cur(), 2);
        n.tick(IFRAMES + 0.01);
        assert!(!n.invulnerable());
        assert_eq!(n.hit(1, AWAY, 0.0), Hit::Hurt);
        assert_eq!(n.cur(), 1);
    }

    #[test]
    fn a_crowd_cannot_chain_hit_faster_than_the_window() {
        let mut n = Nerve::new(MAX);
        let mut hurt = 0;
        let mut t = 0.0;
        while t < 6.0 {
            if n.hit(1, AWAY, 0.0) == Hit::Hurt {
                hurt += 1;
            }
            n.tick(0.016);
            t += 0.016;
        }
        // About one hit per 1.2s, and never more than the nerve allows.
        assert!(hurt <= (6.0 / IFRAMES).ceil() as u32 + 1, "{hurt}");
    }

    #[test]
    fn zero_nerve_is_down_and_refills_to_two() {
        let mut n = Nerve::default();
        n.hit(1, AWAY, 0.0);
        n.tick(2.0);
        n.hit(1, AWAY, 0.0);
        n.tick(2.0);
        assert_eq!(n.cur(), 1);
        assert_eq!(n.hit(1, AWAY, 0.0), Hit::Down);
        assert_eq!(n.cur(), REFILL);
        assert!(!n.invulnerable(), "the respawn grace covers it, not nerve");
    }

    #[test]
    fn heavy_hits_can_take_two_or_finish_you() {
        assert_eq!(damage_from("patrol"), 1);
        assert_eq!(damage_from("hunter"), 2);
        assert_eq!(damage_from("boss attack"), 2);
        assert_eq!(damage_from("tooth"), 1);
        assert_eq!(damage_from("tear"), 1);
        assert_eq!(damage_from("slam"), 2);
        let mut n = Nerve::default();
        assert_eq!(n.hit(2, AWAY, 0.0), Hit::Hurt);
        assert_eq!(n.cur(), 1);
        n.tick(2.0);
        assert_eq!(n.hit(2, AWAY, 0.0), Hit::Down, "more damage than nerve");
    }

    #[test]
    fn portals_heal_one_and_never_past_the_max() {
        let mut n = Nerve::default();
        n.hit(1, AWAY, 0.0);
        n.heal(1);
        assert_eq!(n.cur(), 3);
        n.heal(1);
        assert_eq!(n.cur(), 3);
    }

    #[test]
    fn long_breath_stretches_the_window() {
        let mut n = Nerve::default();
        n.hit(1, AWAY, IFRAMES_PER_BREATH * 2.0);
        n.tick(IFRAMES + 0.5);
        assert!(n.invulnerable());
        n.tick(0.5);
        assert!(!n.invulnerable());
    }

    #[test]
    fn the_shove_points_away_and_fades() {
        let mut n = Nerve::default();
        n.hit(1, (3.0, 4.0), 0.0); // not unit length: normalised
        let (x, z) = n.knock_velocity();
        assert!((x - 0.6 * KNOCK_SPEED).abs() < 1e-4);
        assert!((z - 0.8 * KNOCK_SPEED).abs() < 1e-4);
        n.tick(KNOCK_SECS / 2.0);
        let (x2, _) = n.knock_velocity();
        assert!(x2 < x && x2 > 0.0);
        n.tick(KNOCK_SECS);
        assert_eq!(n.knock_velocity(), (0.0, 0.0));
    }

    #[test]
    fn a_hit_from_nowhere_does_not_shove() {
        let mut n = Nerve::default();
        n.hit(1, (0.0, 0.0), 0.0);
        assert_eq!(n.knock_velocity(), (0.0, 0.0));
    }

    #[test]
    fn faint_blinks_while_invulnerable_and_holds_with_reduced_motion() {
        let mut n = Nerve::default();
        assert!(!n.faint(0.0, false));
        n.hit(1, AWAY, 0.0);
        let on = (0..40).filter(|&i| n.faint(i as f32 * 0.02, false)).count();
        assert!(on > 5 && on < 35, "blinks, got {on}/40");
        assert!((0..40).all(|i| n.faint(i as f32 * 0.02, true)), "steady");
    }

    #[test]
    fn flinch_peaks_at_the_hit_and_is_gone_after_the_break() {
        let mut n = Nerve::default();
        assert_eq!(n.flinch(), 0.0);
        n.hit(1, AWAY, 0.0);
        assert_eq!(n.flinch(), 1.0);
        n.tick(BREAK_SECS);
        assert_eq!(n.flinch(), 0.0);
    }
}
