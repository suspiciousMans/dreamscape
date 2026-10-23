//! LULLABY: slow, big, armoured in front.
//!
//! It turns slowly toward you and walks at [`SPEED_MULT`] of your pace. A
//! strike from the front only staggers it and does half damage; from behind
//! (more than [`ARMOUR_ANGLE_DEG`] away from its facing) it does full damage.
//! Every [`SLAM_EVERY`] seconds it tells for [`TELEGRAPH`] (a ring marker on
//! the floor) and then slams: a ring races outward like the boss's, and you
//! jump it or leave.
//!
//! Pure rules: positions and `dt` in, decisions out.

use crate::boss;
use crate::combat::Kind;
use crate::dream::DreamTheme;
use crate::gameplay::CELL;
use engine::glam::Vec3;

pub const HP: f32 = 6.0;
pub const SPEED_MULT: f32 = 0.35;
pub const SLAM_EVERY: f32 = 5.0;
pub const TELEGRAPH: f32 = 0.8;
/// Beyond this angle from its facing a strike counts as from behind.
pub const ARMOUR_ANGLE_DEG: f32 = 120.0;
/// Damage multiplier for a strike that lands on the armoured front.
pub const FRONT_MULT: f32 = 0.5;
pub const RING_SPEED: f32 = boss::RING_SPEED;
/// How far the slam ring spreads before it fades.
pub const RING_MAX: f32 = 3.0 * CELL;
/// It notices you inside this many world units, and slams within the ring's reach.
pub const SENSE: f32 = 6.0 * CELL;
pub const SLAM_RANGE: f32 = RING_MAX - 1.0;
/// Radians a second it can turn (slow: that is how you get behind it).
pub const TURN_RATE: f32 = 1.6;
/// It stops walking this close.
pub const STOP_DIST: f32 = 2.2;
pub const LEASH: f32 = 5.0 * CELL;
pub const SCALE_PULSE: f32 = 0.12;

fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Idle,
    /// Seconds of warning left.
    Telegraph(f32),
    /// The ring's current radius.
    Slam(f32),
}

/// Is `attacker` behind a Lullaby at `pos` facing `facing`?
pub fn from_behind(pos: Vec3, facing: Vec3, attacker: Vec3) -> bool {
    let to = flat(attacker - pos).normalize_or_zero();
    let f = flat(facing).normalize_or_zero();
    if to == Vec3::ZERO || f == Vec3::ZERO {
        return false;
    }
    to.dot(f) < ARMOUR_ANGLE_DEG.to_radians().cos()
}

#[derive(Clone, Copy, Debug)]
pub struct Lullaby {
    home: Vec3,
    player_speed: f32,
    phase: Phase,
    /// Seconds until the next telegraph may start.
    gap: f32,
    facing: Vec3,
}

impl Lullaby {
    pub fn new(home: Vec3, player_speed: f32) -> Self {
        Self { home, player_speed, phase: Phase::Idle, gap: SLAM_EVERY, facing: Vec3::Z }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn facing(&self) -> Vec3 {
        self.facing
    }

    pub fn speed(&self) -> f32 {
        self.player_speed * SPEED_MULT
    }

    /// Armoured toward `attacker`?
    pub fn guarded_against(&self, pos: Vec3, attacker: Vec3) -> bool {
        !from_behind(pos, self.facing, attacker)
    }

    pub fn unaware(&self, pos: Vec3, player: Vec3) -> bool {
        flat(player - pos).length() > SENSE
    }

    /// Where the ring is, when slamming: (radius).
    pub fn ring(&self) -> Option<f32> {
        match self.phase {
            Phase::Slam(r) => Some(r),
            _ => None,
        }
    }

    /// The floor marker's radius while telling, else nothing.
    pub fn marker(&self) -> Option<f32> {
        match self.phase {
            Phase::Telegraph(left) => Some(RING_MAX * (1.0 - 0.5 * left / TELEGRAPH).max(0.5)),
            _ => None,
        }
    }

    pub fn scale_mult(&self, reduced: bool) -> f32 {
        match self.phase {
            Phase::Telegraph(left) if !reduced => {
                1.0 + SCALE_PULSE * (1.0 - left / TELEGRAPH)
            }
            _ => 1.0,
        }
    }

    /// Only the ring hurts.
    pub fn hits(&self, pos: Vec3, player: Vec3) -> bool {
        self.ring().is_some_and(|r| boss::ring_hits(pos, r, player))
    }

    /// Poise broken by a strike: a telegraph is cancelled; a ring in flight is not.
    pub fn stagger(&mut self) {
        if matches!(self.phase, Phase::Telegraph(_)) {
            self.phase = Phase::Idle;
            self.gap = SLAM_EVERY * 0.6;
        }
    }

    pub fn update(&mut self, pos: &mut Vec3, player: Vec3, dt: f32) {
        let to = flat(player - *pos);
        let dist = to.length();
        let aware = dist <= SENSE;
        // Turn slowly toward you (but not mid-slam).
        if aware && !matches!(self.phase, Phase::Slam(_)) {
            self.turn_toward(to / dist.max(1e-3), dt);
        }
        self.gap = (self.gap - dt).max(0.0);
        match self.phase {
            Phase::Idle => {
                if aware && dist > STOP_DIST && flat(*pos - self.home).length() < LEASH {
                    *pos += self.facing * self.speed() * dt;
                } else if !aware && flat(self.home - *pos).length() > 0.05 {
                    let back = flat(self.home - *pos);
                    *pos += back.normalize_or_zero() * (self.speed() * dt).min(back.length());
                }
                if self.gap <= 0.0 && dist <= SLAM_RANGE {
                    self.phase = Phase::Telegraph(TELEGRAPH);
                    self.gap = SLAM_EVERY;
                }
            }
            Phase::Telegraph(left) => {
                let left = left - dt;
                self.phase = if left <= 0.0 { Phase::Slam(0.0) } else { Phase::Telegraph(left) };
            }
            Phase::Slam(r) => {
                let r = r + RING_SPEED * dt;
                self.phase = if r >= RING_MAX { Phase::Idle } else { Phase::Slam(r) };
            }
        }
    }

    fn turn_toward(&mut self, want: Vec3, dt: f32) {
        let a = self.facing.z.atan2(self.facing.x);
        let b = want.z.atan2(want.x);
        let mut d = (b - a).rem_euclid(std::f32::consts::TAU);
        if d > std::f32::consts::PI {
            d -= std::f32::consts::TAU;
        }
        let step = d.clamp(-TURN_RATE * dt, TURN_RATE * dt);
        let n = a + step;
        self.facing = Vec3::new(n.cos(), 0.0, n.sin());
    }
}

/// What a dream gets: `[]` before depth 5 and in nightmares; else at most
/// one Lullaby (its own seeded stream).
pub fn spawn_plan(depth: u32, theme: DreamTheme, seed: u64, nightmare: bool) -> Vec<(Kind, usize)> {
    if nightmare || depth < 5 || crate::newfoes::no_wandering_foes(theme) {
        return Vec::new();
    }
    if crate::newfoes::stream(seed, 0x1011_ab9) % 100 >= 35 {
        return Vec::new();
    }
    vec![(Kind::Lullaby, 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    const PS: f32 = 6.0;
    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn the_numbers_match_the_brief() {
        assert_eq!(HP, 6.0);
        assert_eq!(SPEED_MULT, 0.35);
        assert_eq!(SLAM_EVERY, 5.0);
        assert_eq!(TELEGRAPH, 0.8);
        assert_eq!(ARMOUR_ANGLE_DEG, 120.0);
        assert_eq!(FRONT_MULT, 0.5);
        assert_eq!(RING_SPEED, boss::RING_SPEED);
    }

    #[test]
    fn armour_is_the_front_two_thirds_of_the_circle() {
        let pos = Vec3::ZERO;
        let f = Vec3::Z;
        let at = |deg: f32| {
            let a = deg.to_radians();
            Vec3::new(a.sin(), 0.0, a.cos()) * 2.0
        };
        assert!(!from_behind(pos, f, at(0.0)), "dead ahead");
        assert!(!from_behind(pos, f, at(90.0)), "its flank");
        assert!(!from_behind(pos, f, at(119.0)));
        assert!(from_behind(pos, f, at(121.0)));
        assert!(from_behind(pos, f, at(180.0)));
        assert!(from_behind(pos, f, at(-150.0)));
        let l = Lullaby::new(Vec3::ZERO, PS);
        assert!(l.guarded_against(Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0)));
        assert!(!l.guarded_against(Vec3::ZERO, Vec3::new(0.0, 0.0, -2.0)));
    }

    #[test]
    fn it_is_slow_and_turns_slowly() {
        let mut l = Lullaby::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        l.facing = Vec3::Z;
        let me = Vec3::new(0.0, 0.0, 12.0);
        for _ in 0..60 {
            l.update(&mut pos, me, DT);
        }
        assert!((pos.z - PS * SPEED_MULT).abs() < 0.2, "{}", pos.z);
        // A player directly behind: after half a second it has not yet turned round.
        let mut l = Lullaby::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        for _ in 0..30 {
            l.update(&mut pos, Vec3::new(0.0, 0.0, -6.0), DT);
        }
        assert!(l.facing().z > -0.5, "turned too fast: {:?}", l.facing());
        assert!(!l.guarded_against(pos, Vec3::new(0.0, 0.0, -6.0)), "still exposed behind");
    }

    #[test]
    fn it_tells_for_point_eight_seconds_before_the_ring_and_nothing_hurts_before() {
        let mut l = Lullaby::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        let me = Vec3::new(0.0, 0.0, 4.0);
        let mut t = 0.0;
        let (mut tell, mut slam) = (None, None);
        while t < 8.0 && slam.is_none() {
            l.update(&mut pos, me, DT);
            t += DT;
            if tell.is_none() && matches!(l.phase(), Phase::Telegraph(_)) {
                tell = Some(t);
                assert!(l.marker().is_some(), "a ring marker on the floor");
            }
            if slam.is_none() && l.ring().is_some() {
                slam = Some(t);
            }
            if tell.is_none() {
                assert!(!l.hits(pos, me));
            }
        }
        let (tell, slam) = (tell.expect("told"), slam.expect("slammed"));
        assert!((slam - tell - TELEGRAPH).abs() < 2.0 * DT);
        assert!((tell - SLAM_EVERY).abs() < 0.1, "first telegraph at {tell}");
    }

    #[test]
    fn the_ring_hurts_on_the_floor_and_not_if_you_jump() {
        let mut l = Lullaby::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        l.phase = Phase::Slam(0.0);
        // A dreamer standing on the floor has her centre one radius up.
        let ground = Vec3::new(0.0, 0.5, 5.0);
        let air = Vec3::new(0.0, 0.5 + 1.5, 5.0);
        let (mut hit_ground, mut hit_air) = (false, false);
        let fixed = pos;
        while l.ring().is_some() {
            l.update(&mut pos, Vec3::new(0.0, 0.5, 400.0), DT);
            hit_ground |= l.hits(fixed, ground);
            hit_air |= l.hits(fixed, air);
        }
        assert!(hit_ground);
        assert!(!hit_air, "jumping clears it");
    }

    #[test]
    fn staggering_spoils_a_telegraph() {
        let mut l = Lullaby::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        let me = Vec3::new(0.0, 0.0, 4.0);
        while !matches!(l.phase(), Phase::Telegraph(_)) {
            l.update(&mut pos, me, DT);
        }
        l.stagger();
        assert_eq!(l.phase(), Phase::Idle);
        for _ in 0..90 {
            l.update(&mut pos, me, DT);
        }
        assert!(l.ring().is_none());
    }

    #[test]
    fn it_does_not_slam_a_player_out_of_the_rings_reach() {
        let mut l = Lullaby::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        for _ in 0..(12.0 / DT) as usize {
            l.update(&mut pos, Vec3::new(0.0, 0.0, SENSE - 0.5), DT);
            assert!(l.ring().is_none() || flat(pos - l.home).length() > 0.0);
        }
        // It can walk toward you; the leash stops it before it ever slams from afar.
        assert!(flat(pos - l.home).length() <= LEASH + 0.2);
    }

    #[test]
    fn only_one_lullaby_ever() {
        for seed in 0..300 {
            let p = spawn_plan(9, DreamTheme::Garden, seed, false);
            assert!(p.iter().map(|x| x.1).sum::<usize>() <= 1);
        }
    }
}
