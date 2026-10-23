//! TOOTH: a fast, short lunge out of the dark.
//!
//! It idles in a pocket and creeps (always slower than you) toward a dreamer
//! who comes near. Close enough, it stops and *tells* for [`TELEGRAPH`]
//! (scale pulse and an eye tint), then makes one fast lunge of about
//! [`LUNGE_DIST`] and spends [`RECOVER`] seconds recovering, during which a
//! strike does double damage. Only the lunge hurts: the danger is the lunge,
//! and dashing through it or striking its recovery are the answers.
//!
//! Pure rules: positions and `dt` in, decisions out.

use crate::gameplay::CELL;
use engine::glam::Vec3;

pub const HP: f32 = 2.0;
/// Seconds of warning before the lunge.
pub const TELEGRAPH: f32 = 0.5;
/// How far one lunge carries it: 2.5 cells.
pub const LUNGE_DIST: f32 = 2.5 * CELL;
pub const LUNGE_SECS: f32 = 0.5;
/// Lunge speed in world units a second (over twice your pace; it is short).
pub const LUNGE_SPEED: f32 = LUNGE_DIST / LUNGE_SECS;
/// Seconds spent staggered after a lunge.
pub const RECOVER: f32 = 1.5;
/// Damage multiplier for a strike that lands during the recovery.
pub const RECOVER_MULT: f32 = 2.0;
/// Creeping speed as a fraction of yours.
pub const CREEP: f32 = 0.6;
/// It notices you inside this many world units.
pub const SENSE: f32 = 6.0 * CELL;
/// It starts a telegraph when you are this close (a lunge can just reach).
pub const TRIGGER: f32 = LUNGE_DIST + 1.0;
/// How far from its pocket it will creep.
pub const LEASH: f32 = 4.0 * CELL;
/// How much the telegraph pulse swells the body.
pub const SCALE_PULSE: f32 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Idle,
    /// Seconds of warning left.
    Telegraph(f32),
    /// Distance covered so far.
    Lunge(f32),
    /// Seconds of recovery left.
    Recover(f32),
}

fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

#[derive(Clone, Copy, Debug)]
pub struct Tooth {
    home: Vec3,
    player_speed: f32,
    phase: Phase,
    dir: Vec3,
}

impl Tooth {
    pub fn new(home: Vec3, player_speed: f32) -> Self {
        Self { home, player_speed, phase: Phase::Idle, dir: Vec3::Z }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn creep_speed(&self) -> f32 {
        self.player_speed * CREEP
    }

    /// Only the lunge hurts.
    pub fn armed(&self) -> bool {
        matches!(self.phase, Phase::Lunge(_))
    }

    /// Reeling after its lunge: a strike does [`RECOVER_MULT`] damage.
    pub fn vulnerable(&self) -> bool {
        matches!(self.phase, Phase::Recover(_))
    }

    /// Has not noticed you (a sneak attack counts double).
    pub fn unaware(&self, pos: Vec3, player: Vec3) -> bool {
        self.phase == Phase::Idle && flat(player - pos).length() > SENSE
    }

    /// The way it is looking (unit, flat).
    pub fn facing(&self) -> Vec3 {
        self.dir
    }

    /// Telegraphing: eye tint on.
    pub fn eye_lit(&self) -> bool {
        matches!(self.phase, Phase::Telegraph(_))
    }

    /// Body scale multiplier: a pulse that quickens through the telegraph,
    /// still `1.0` when `reduced` (the tint carries the warning instead).
    pub fn scale_mult(&self, reduced: bool) -> f32 {
        match self.phase {
            Phase::Telegraph(left) if !reduced => {
                let t = 1.0 - left / TELEGRAPH;
                1.0 + SCALE_PULSE * (t * t * 28.0).sin().abs()
            }
            Phase::Recover(_) => 0.85,
            _ => 1.0,
        }
    }

    /// A strike lands: it drops its plan. Stays staggered if already
    /// recovering, otherwise goes back to idle (a hit breaks a lunge).
    pub fn hurt(&mut self) {
        if !self.vulnerable() {
            self.phase = Phase::Idle;
        }
    }

    pub fn update(&mut self, pos: &mut Vec3, player: Vec3, dt: f32) {
        let to = flat(player - *pos);
        let dist = to.length();
        match self.phase {
            Phase::Idle => {
                let home_d = flat(self.home - *pos).length();
                let step = self.creep_speed() * dt;
                if dist <= SENSE && dist > TRIGGER && home_d < LEASH {
                    self.face(to);
                    *pos += to.normalize_or_zero() * step.min(dist);
                } else if dist <= TRIGGER {
                    self.face(to);
                    self.phase = Phase::Telegraph(TELEGRAPH);
                } else if home_d > 0.05 {
                    let back = flat(self.home - *pos);
                    *pos += back.normalize_or_zero() * step.min(home_d);
                }
            }
            Phase::Telegraph(left) => {
                self.face(to); // it tracks you right up to the lunge
                let left = left - dt;
                if left <= 0.0 {
                    self.phase = Phase::Lunge(0.0);
                } else {
                    self.phase = Phase::Telegraph(left);
                }
            }
            Phase::Lunge(done) => {
                let step = (LUNGE_SPEED * dt).min(LUNGE_DIST - done);
                *pos += self.dir * step;
                let done = done + step;
                self.phase = if done >= LUNGE_DIST - 1e-4 {
                    Phase::Recover(RECOVER)
                } else {
                    Phase::Lunge(done)
                };
            }
            Phase::Recover(left) => {
                let left = left - dt;
                self.phase = if left <= 0.0 { Phase::Idle } else { Phase::Recover(left) };
            }
        }
    }

    fn face(&mut self, to: Vec3) {
        let d = to.normalize_or_zero();
        if d != Vec3::ZERO {
            self.dir = d;
        }
    }
}

use crate::combat::Kind;
use crate::dream::DreamTheme;

/// What a dream gets: `[]` before depth 3, in nightmares and in the opening
/// rooms; else one to three teeth (its own seeded stream).
pub fn spawn_plan(depth: u32, theme: DreamTheme, seed: u64, nightmare: bool) -> Vec<(Kind, usize)> {
    if nightmare || depth < 3 || crate::newfoes::no_wandering_foes(theme) {
        return Vec::new();
    }
    let r = crate::newfoes::stream(seed, 0x700_7);
    if r % 100 >= 65 {
        return Vec::new();
    }
    let n = 1 + (r / 100 % 2) as usize + usize::from(depth >= 8);
    vec![(Kind::Tooth, n.min(3))]
}

#[cfg(test)]
mod tests {
    use super::*;

    const PS: f32 = 6.0;
    const DT: f32 = 1.0 / 60.0;

    fn run(t: &mut Tooth, pos: &mut Vec3, player: Vec3, secs: f32) {
        let mut x = 0.0;
        while x < secs {
            t.update(pos, player, DT);
            x += DT;
        }
    }

    #[test]
    fn the_numbers_match_the_brief() {
        assert_eq!(HP, 2.0);
        assert_eq!(TELEGRAPH, 0.5);
        assert_eq!(RECOVER, 1.5);
        assert_eq!(RECOVER_MULT, 2.0);
        assert_eq!(LUNGE_DIST, 7.5);
        assert!(CREEP < 1.0);
        assert!(LUNGE_SPEED > PS, "the lunge is the danger");
    }

    #[test]
    fn it_idles_until_you_come_near() {
        let mut t = Tooth::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        run(&mut t, &mut pos, Vec3::new(0.0, 0.0, SENSE + 5.0), 3.0);
        assert_eq!(t.phase(), Phase::Idle);
        assert_eq!(pos, Vec3::ZERO);
        assert!(t.unaware(pos, Vec3::new(0.0, 0.0, SENSE + 5.0)));
    }

    #[test]
    fn creeping_is_slower_than_the_player() {
        let mut t = Tooth::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        let player = Vec3::new(0.0, 0.0, SENSE - 0.5);
        run(&mut t, &mut pos, player, 1.0);
        assert!(pos.z > 0.5 && pos.z < PS * 0.99, "moved {} in a second", pos.z);
        assert!((pos.z - PS * CREEP).abs() < 0.2);
    }

    #[test]
    fn telegraph_comes_before_the_lunge_and_the_lunge_before_any_damage() {
        let mut t = Tooth::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        let player = Vec3::new(0.0, 0.0, TRIGGER - 1.0);
        let mut age = 0.0;
        let mut first_armed = None;
        let mut first_tell = None;
        while age < 3.0 {
            t.update(&mut pos, player, DT);
            age += DT;
            if first_tell.is_none() && t.eye_lit() {
                first_tell = Some(age);
            }
            if first_armed.is_none() && t.armed() {
                first_armed = Some(age);
            }
        }
        let (tell, armed) = (first_tell.expect("told"), first_armed.expect("lunged"));
        assert!(armed - tell >= TELEGRAPH - 2.0 * DT, "tell {tell}, armed {armed}");
    }

    #[test]
    fn a_lunge_covers_two_and_a_half_cells_then_it_recovers_for_one_and_a_half_seconds() {
        let mut t = Tooth::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        let player = Vec3::new(0.0, 0.0, 3.0);
        while !t.armed() {
            t.update(&mut pos, player, DT);
        }
        let start = pos;
        while t.armed() {
            t.update(&mut pos, player, DT);
        }
        assert!((flat(pos - start).length() - LUNGE_DIST).abs() < 0.05, "{}", flat(pos - start).length());
        assert!(t.vulnerable());
        let mut recovering = 0.0;
        while t.vulnerable() {
            t.update(&mut pos, player, DT);
            recovering += DT;
        }
        assert!((recovering - RECOVER).abs() < 2.0 * DT, "{recovering}");
        assert!(!t.armed(), "recovery never bites");
    }

    #[test]
    fn it_never_hurts_outside_the_lunge() {
        let mut t = Tooth::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        let player = Vec3::new(0.0, 0.0, 4.0);
        let mut armed_secs = 0.0;
        let mut total = 0.0;
        while total < 10.0 {
            t.update(&mut pos, player, DT);
            total += DT;
            if t.armed() {
                armed_secs += DT;
            }
        }
        assert!(armed_secs < 10.0 * LUNGE_SECS / (TELEGRAPH + LUNGE_SECS + RECOVER) + 1.2);
    }

    #[test]
    fn the_dash_moves_you_out_of_the_locked_line() {
        // The lunge direction locks when it starts: a player who steps aside
        // during the lunge is not on the line any more.
        let mut t = Tooth::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        while !t.armed() {
            t.update(&mut pos, Vec3::new(0.0, 0.0, 4.0), DT);
        }
        let aside = Vec3::new(4.0, 0.0, 4.0);
        for _ in 0..10 {
            t.update(&mut pos, aside, DT);
        }
        assert!(pos.x.abs() < 1e-3, "kept its line: {pos}");
    }

    #[test]
    fn a_hit_breaks_the_plan_but_not_the_recovery() {
        let mut t = Tooth::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        while t.phase() == Phase::Idle {
            t.update(&mut pos, Vec3::new(0.0, 0.0, 3.0), DT);
        }
        assert!(matches!(t.phase(), Phase::Telegraph(_)));
        t.hurt();
        assert_eq!(t.phase(), Phase::Idle);
        while !t.vulnerable() {
            t.update(&mut pos, Vec3::new(0.0, 0.0, 3.0), DT);
        }
        t.hurt();
        assert!(t.vulnerable());
    }

    #[test]
    fn the_leash_keeps_it_near_its_pocket() {
        let mut t = Tooth::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        let player = Vec3::new(0.0, 0.0, SENSE - 0.1);
        run(&mut t, &mut pos, player, 20.0);
        assert!(pos.z < LEASH + LUNGE_DIST + 0.5, "{}", pos.z);
    }

    #[test]
    fn reduced_motion_swaps_the_pulse_for_the_tint() {
        let mut t = Tooth::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        while !t.eye_lit() {
            t.update(&mut pos, Vec3::new(0.0, 0.0, 3.0), DT);
        }
        for _ in 0..12 {
            t.update(&mut pos, Vec3::new(0.0, 0.0, 3.0), DT);
        }
        assert_eq!(t.scale_mult(true), 1.0);
        assert!(t.eye_lit());
        assert!(t.scale_mult(false) > 1.0);
    }
}
