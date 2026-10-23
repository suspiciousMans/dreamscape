//! WEEPER: keeps its distance and spits slow tears.
//!
//! It holds [`KEEP_MIN`]..[`KEEP_MAX`] from you, backing away (never faster
//! than you) when you close in. Every [`SPIT_EVERY`] seconds it tells for
//! [`TELEGRAPH`] and then spits one slow tear at where you were standing.
//! Tears are slow ([`TEAR_SPEED`] x your pace), live [`TEAR_LIFE`] seconds,
//! die on walls, and at most [`MAX_TEARS`] fly at once. Blink or Dash in to
//! reach it; a Decoy draws its fire.
//!
//! Pure rules: positions and `dt` in, decisions out.

use crate::combat::Kind;
use crate::dream::DreamTheme;
use crate::gameplay::{CELL, PLAYER_RADIUS};
use engine::glam::Vec3;

pub const HP: f32 = 2.0;
pub const KEEP_MIN: f32 = 4.0 * CELL;
pub const KEEP_MAX: f32 = 6.0 * CELL;
/// Its walking pace as a fraction of yours (always below 1).
pub const SPEED_MULT: f32 = 0.8;
pub const SPIT_EVERY: f32 = 3.0;
pub const TELEGRAPH: f32 = 0.8;
pub const TEAR_SPEED_MULT: f32 = 0.4;
pub const TEAR_LIFE: f32 = 6.0;
pub const MAX_TEARS: usize = 2;
pub const TEAR_TOUCH: f32 = 0.6;
/// It only spits when a tear could get to you.
pub const SPIT_RANGE: f32 = 14.0;
/// How far it strays from its post.
pub const LEASH: f32 = 6.0 * CELL;
pub const SCALE_PULSE: f32 = 0.2;

/// A wall as a box: (min, max).
pub type Wall = (Vec3, Vec3);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tear {
    pub pos: Vec3,
    dir: Vec3,
    pub life: f32,
}

fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

/// Is `p` inside any wall box?
pub fn in_wall(p: Vec3, walls: &[Wall]) -> bool {
    walls
        .iter()
        .any(|&(lo, hi)| p.cmpge(lo).all() && p.cmple(hi).all())
}

#[derive(Clone, Debug)]
pub struct Weeper {
    home: Vec3,
    player_speed: f32,
    /// Seconds until the next telegraph may start.
    gap: f32,
    /// Seconds of telegraph left, when telling.
    tell: Option<f32>,
    facing: Vec3,
    pub tears: Vec<Tear>,
}

impl Weeper {
    pub fn new(home: Vec3, player_speed: f32) -> Self {
        Self {
            home,
            player_speed,
            gap: SPIT_EVERY,
            tell: None,
            facing: Vec3::Z,
            tears: Vec::new(),
        }
    }

    pub fn speed(&self) -> f32 {
        self.player_speed * SPEED_MULT
    }

    pub fn tear_speed(&self) -> f32 {
        self.player_speed * TEAR_SPEED_MULT
    }

    pub fn telling(&self) -> bool {
        self.tell.is_some()
    }

    pub fn facing(&self) -> Vec3 {
        self.facing
    }

    /// Does not know you are there (a sneak attack counts double).
    pub fn unaware(&self, pos: Vec3, player: Vec3) -> bool {
        flat(player - pos).length() > KEEP_MAX + 2.0 * CELL
    }

    /// Body scale multiplier while telling (the tint carries it in reduced motion).
    pub fn scale_mult(&self, reduced: bool) -> f32 {
        match self.tell {
            Some(left) if !reduced => {
                let t = 1.0 - left / TELEGRAPH;
                1.0 + SCALE_PULSE * (t * 14.0).sin().abs()
            }
            _ => 1.0,
        }
    }

    /// A strike lands: the spit is spoiled.
    pub fn hurt(&mut self) {
        self.tell = None;
        self.gap = SPIT_EVERY * 0.5;
    }

    /// A tear touches the dreamer.
    pub fn touching(&self, player: Vec3) -> bool {
        self.tears.iter().any(|t| {
            flat(t.pos - player).length() < TEAR_TOUCH + PLAYER_RADIUS * 0.5
                && (t.pos.y - player.y).abs() < 1.2
        })
    }

    #[cfg(test)]
    pub fn update(&mut self, pos: &mut Vec3, player: Vec3, walls: &[Wall], dt: f32) {
        self.update_aimed(pos, player, player, walls, dt);
    }

    /// `update`, but it spits at `aim` (a Decoy draws its fire) while still
    /// keeping its distance from `player`.
    pub fn update_aimed(&mut self, pos: &mut Vec3, player: Vec3, aim: Vec3, walls: &[Wall], dt: f32) {
        let to = flat(player - *pos);
        let dist = to.length();
        if dist > 1e-3 {
            self.facing = to / dist;
        }
        // Tears fly, age, and die on walls.
        let step = self.tear_speed() * dt;
        for t in &mut self.tears {
            t.pos += t.dir * step;
            t.life -= dt;
        }
        self.tears.retain(|t| t.life > 0.0 && !in_wall(t.pos, walls));
        // Keeping its distance (it holds still while telling).
        if self.tell.is_none() {
            let home_gap = flat(self.home - *pos);
            let reach = self.speed() * dt;
            let away = -to.normalize_or_zero();
            let want = if dist < KEEP_MIN {
                away
            } else if dist > KEEP_MAX {
                to.normalize_or_zero()
            } else {
                Vec3::ZERO
            };
            let next = *pos + want * reach;
            let leashed = flat(next - self.home).length() <= LEASH
                || flat(next - self.home).length() < home_gap.length();
            if leashed {
                *pos = next;
            }
        }
        // The spit cycle.
        self.gap = (self.gap - dt).max(0.0);
        match self.tell {
            Some(left) => {
                let left = left - dt;
                if left <= 0.0 {
                    self.tell = None;
                    self.spit(*pos, aim);
                } else {
                    self.tell = Some(left);
                }
            }
            None => {
                let aim_dist = flat(aim - *pos).length();
                if self.gap <= 0.0 && aim_dist <= SPIT_RANGE && self.tears.len() < MAX_TEARS {
                    self.tell = Some(TELEGRAPH);
                    self.gap = SPIT_EVERY - 0.0;
                }
            }
        }
    }

    fn spit(&mut self, from: Vec3, player: Vec3) {
        if self.tears.len() >= MAX_TEARS {
            return;
        }
        let dir = flat(player - from).normalize_or_zero();
        let dir = if dir == Vec3::ZERO { self.facing } else { dir };
        self.tears.push(Tear {
            pos: Vec3::new(from.x, from.y, from.z) + dir * 0.8,
            dir,
            life: TEAR_LIFE,
        });
    }
}

/// What a dream gets: `[]` before depth 4 and in nightmares; else one or
/// two weepers (its own seeded stream).
pub fn spawn_plan(depth: u32, theme: DreamTheme, seed: u64, nightmare: bool) -> Vec<(Kind, usize)> {
    if nightmare || depth < 4 || crate::newfoes::no_wandering_foes(theme) {
        return Vec::new();
    }
    let r = crate::newfoes::stream(seed, 0x3ee_9);
    if r % 100 >= 50 {
        return Vec::new();
    }
    let n = 1 + usize::from(depth >= 8 && r / 100 % 2 == 0);
    vec![(Kind::Weeper, n.min(2))]
}

#[cfg(test)]
mod tests {
    use super::*;

    const PS: f32 = 6.0;
    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn the_numbers_match_the_brief() {
        assert_eq!(HP, 2.0);
        assert_eq!((KEEP_MIN, KEEP_MAX), (12.0, 18.0));
        assert_eq!(SPIT_EVERY, 3.0);
        assert_eq!(TELEGRAPH, 0.8);
        assert_eq!(TEAR_SPEED_MULT, 0.4);
        assert_eq!(TEAR_LIFE, 6.0);
        assert_eq!(MAX_TEARS, 2);
        assert!(SPEED_MULT < 1.0);
        let w = Weeper::new(Vec3::ZERO, PS);
        assert!((w.tear_speed() - 2.4).abs() < 1e-5);
    }

    fn spit_one(w: &mut Weeper, pos: &mut Vec3, player: Vec3) -> (f32, f32) {
        let mut t = 0.0;
        let mut tell_at = None;
        loop {
            w.update(pos, player, &[], DT);
            t += DT;
            if tell_at.is_none() && w.telling() {
                tell_at = Some(t);
            }
            if !w.tears.is_empty() {
                return (tell_at.expect("told first"), t);
            }
            assert!(t < 10.0, "never spat");
        }
    }

    #[test]
    fn the_tell_comes_first_then_the_tear_after_point_eight_seconds() {
        let mut w = Weeper::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        let (tell, spat) = spit_one(&mut w, &mut pos, Vec3::new(0.0, 0.0, 13.0));
        assert!((spat - tell - TELEGRAPH).abs() < 3.0 * DT, "tell {tell} spit {spat}");
        assert!(w.tears[0].life > TEAR_LIFE - 0.1);
    }

    #[test]
    fn it_spits_every_three_seconds_never_more_than_two_tears() {
        let mut w = Weeper::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        let player = Vec3::new(0.0, 0.0, 13.0);
        let mut spits = Vec::new();
        let mut last = 0;
        let mut t = 0.0;
        while t < 13.0 {
            w.update(&mut pos, player, &[], DT);
            t += DT;
            assert!(w.tears.len() <= MAX_TEARS);
            if w.tears.len() > last {
                spits.push(t);
            }
            last = w.tears.len();
        }
        assert!(spits.len() >= 3, "{spits:?}");
        let gap = spits[1] - spits[0];
        assert!((gap - SPIT_EVERY).abs() < 0.1, "gap {gap}");
    }

    #[test]
    fn tears_are_slow_and_expire() {
        let mut w = Weeper::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        let player = Vec3::new(0.0, 0.0, 13.0);
        spit_one(&mut w, &mut pos, player);
        let p0 = w.tears[0].pos;
        for _ in 0..60 {
            w.update(&mut pos, Vec3::new(0.0, 0.0, 13.0), &[], DT);
        }
        let moved = flat(w.tears[0].pos - p0).length();
        assert!((moved - 2.4).abs() < 0.1, "moved {moved} in a second");
        // Far away and out of range of any new spit: let it age out.
        let far = Vec3::new(0.0, 0.0, 400.0);
        w.tell = None;
        for _ in 0..(7.0 / DT) as usize {
            w.update(&mut pos, far, &[], DT);
        }
        assert!(w.tears.is_empty(), "lives only {TEAR_LIFE}s");
    }

    #[test]
    fn tears_die_on_walls() {
        let mut w = Weeper::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        let player = Vec3::new(0.0, 0.0, 13.0);
        spit_one(&mut w, &mut pos, player);
        let wall: Wall = (Vec3::new(-3.0, -1.0, 2.5), Vec3::new(3.0, 4.0, 3.0));
        let mut t = 0.0;
        while !w.tears.is_empty() && t < 3.0 {
            w.update(&mut pos, Vec3::new(0.0, 0.0, 400.0), &[wall], DT);
            t += DT;
        }
        assert!(w.tears.is_empty(), "the wall ate it");
        assert!(t < 3.0);
    }

    #[test]
    fn a_tear_over_a_low_wall_is_not_eaten_and_a_miss_is_not_a_hit() {
        let low: Wall = (Vec3::new(-3.0, -1.0, 2.0), Vec3::new(3.0, 0.2, 3.0));
        assert!(!in_wall(Vec3::new(0.0, 0.9, 2.5), &[low]));
        assert!(in_wall(Vec3::new(0.0, 0.1, 2.5), &[low]));
    }

    #[test]
    fn it_flees_when_you_close_in_but_never_outruns_you() {
        let mut w = Weeper::new(Vec3::new(0.0, 0.0, 0.0), PS);
        let mut pos = Vec3::new(0.0, 0.0, 30.0);
        w.home = pos;
        let mut me = Vec3::new(0.0, 0.0, 30.0 - 6.0);
        let mut gained = 0.0_f32;
        for _ in 0..(3.0 / DT) as usize {
            let before = flat(pos - me).length();
            me.z += PS * DT; // the player charges at full speed
            w.update(&mut pos, me, &[], DT);
            gained = gained.max(flat(pos - me).length() - before);
        }
        assert!(gained < 0.0 + 1e-3, "it opened the gap by {gained}");
        assert!(flat(pos - me).length() < 6.0, "you caught it");
    }

    #[test]
    fn it_backs_off_to_keep_its_distance() {
        let mut w = Weeper::new(Vec3::ZERO, PS);
        w.home = Vec3::ZERO;
        let mut pos = Vec3::ZERO;
        let me = Vec3::new(0.0, 0.0, 6.0);
        for _ in 0..120 {
            w.update(&mut pos, me, &[], DT);
        }
        assert!(pos.z < -1.0, "backed away to {}", pos.z);
        assert!(flat(pos - w.home).length() <= LEASH + 0.1);
    }

    #[test]
    fn it_closes_the_gap_when_you_are_too_far_but_stays_on_its_leash() {
        let mut w = Weeper::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        let me = Vec3::new(0.0, 0.0, 100.0);
        for _ in 0..(30.0 / DT) as usize {
            w.update(&mut pos, me, &[], DT);
        }
        assert!(pos.z > 5.0 && pos.z <= LEASH + 0.1, "{}", pos.z);
    }

    #[test]
    fn a_decoy_draws_its_fire() {
        let mut w = Weeper::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        let me = Vec3::new(0.0, 0.0, 15.0); // in the keep band, within range
        let decoy = Vec3::new(10.0, 0.0, 6.0);
        let mut t = 0.0;
        while w.tears.is_empty() && t < 6.0 {
            w.update_aimed(&mut pos, me, decoy, &[], DT);
            t += DT;
        }
        let tear = w.tears[0];
        let want = flat(decoy - pos).normalize();
        assert!(flat(tear.pos - pos).normalize().dot(want) > 0.99, "aimed at the decoy");
    }

    #[test]
    fn a_hit_spoils_the_spit() {
        let mut w = Weeper::new(Vec3::ZERO, PS);
        let mut pos = Vec3::ZERO;
        let me = Vec3::new(0.0, 0.0, 13.0);
        while !w.telling() {
            w.update(&mut pos, me, &[], DT);
        }
        w.hurt();
        assert!(!w.telling());
        for _ in 0..30 {
            w.update(&mut pos, me, &[], DT);
        }
        assert!(w.tears.is_empty());
    }

    #[test]
    fn a_tear_touches_you_only_when_it_is_on_you() {
        let mut w = Weeper::new(Vec3::ZERO, PS);
        w.tears.push(Tear { pos: Vec3::new(0.0, 0.9, 5.0), dir: Vec3::Z, life: 3.0 });
        assert!(!w.touching(Vec3::new(0.0, 1.0, 0.0)));
        assert!(w.touching(Vec3::new(0.0, 1.0, 5.0)));
        assert!(!w.touching(Vec3::new(0.0, 4.0, 5.0)), "jumped over it");
    }
}
