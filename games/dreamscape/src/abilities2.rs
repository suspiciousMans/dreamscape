//! Items wave 2: eight more active abilities (Tether, Mirror, Quake, Veil,
//! Surge, Pulse, Haste, Bind). Pure: positions and seconds in, numbers out.
//! `main.rs` wires them inside `// === abilities2 ===` blocks.

use engine::glam::Vec3;

use crate::upgrades::Ability;

/// The eight abilities this module owns.
pub const NEW_ABILITIES: [Ability; 8] = [
    Ability::Tether,
    Ability::Mirror,
    Ability::Quake,
    Ability::Veil,
    Ability::Surge,
    Ability::Pulse,
    Ability::Haste,
    Ability::Bind,
];

// --- TETHER: yank the nearest shard or calm mote to you ---
pub const TETHER_RADIUS: f32 = 16.0;
pub const TETHER_SPEED: f32 = 14.0;
/// It stops this close to you (so it lands in reach, not inside you).
pub const TETHER_STOP: f32 = 0.6;

/// Index of the nearest point within [`TETHER_RADIUS`] of `from` (flat).
pub fn tether_pick(from: Vec3, points: &[Vec3]) -> Option<usize> {
    let flat = |p: Vec3| Vec3::new(p.x - from.x, 0.0, p.z - from.z).length();
    points
        .iter()
        .enumerate()
        .filter(|(_, &p)| flat(p) <= TETHER_RADIUS)
        .min_by(|a, b| flat(*a.1).total_cmp(&flat(*b.1)))
        .map(|(i, _)| i)
}

/// One frame of the pull: `at` moves toward `to` and never overshoots
/// [`TETHER_STOP`].
pub fn tether_step(at: Vec3, to: Vec3, dt: f32) -> Vec3 {
    let d = Vec3::new(to.x - at.x, 0.0, to.z - at.z);
    let gap = (d.length() - TETHER_STOP).max(0.0);
    at + d.normalize_or_zero() * (TETHER_SPEED * dt).min(gap)
}

// --- MIRROR: the next hit is thrown back and everything near reels ---
pub const MIRROR_REACH: f32 = 6.0;
pub const MIRROR_STUN: f32 = 2.5;
/// Grace after the mirror breaks (the shards of it still hurt them).
pub const MIRROR_GRACE: f32 = 0.8;

// --- QUAKE: the floor ripples; everything on it staggers ---
pub const QUAKE_REACH: f32 = 8.0;
pub const QUAKE_STAGGER: f32 = 2.2;
/// Only a dreamer standing on something can shake it.
pub fn quake_allowed(grounded: bool) -> bool {
    grounded
}

// --- VEIL: chasers lose you; a blow on one that did not see you is a sneak attack ---
/// Where the enemies think you are: nowhere near.
pub const VEIL_FAR: f32 = 10_000.0;

pub fn seen_from(real: Vec3, veiled: bool) -> Vec3 {
    if veiled {
        real + Vec3::new(VEIL_FAR, 0.0, VEIL_FAR)
    } else {
        real
    }
}

// --- SURGE: the next jump and the next dash are doubled ---
pub const SURGE_MULT: f32 = 2.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Surge {
    jump: bool,
    dash: bool,
}

impl Surge {
    pub fn arm(&mut self) {
        self.jump = true;
        self.dash = true;
    }

    pub fn clear(&mut self) {
        *self = Surge::default();
    }

    #[cfg(test)]
    pub fn armed(&self) -> bool {
        self.jump || self.dash
    }

    /// The multiplier for a jump happening now; spends the surge's jump.
    pub fn take_jump(&mut self) -> f32 {
        if std::mem::take(&mut self.jump) {
            SURGE_MULT
        } else {
            1.0
        }
    }

    /// The multiplier for a dash starting now; spends the surge's dash.
    pub fn take_dash(&mut self) -> f32 {
        if std::mem::take(&mut self.dash) {
            SURGE_MULT
        } else {
            1.0
        }
    }
}

// --- PULSE: see through walls for a moment ---
/// A Shade shows from this far while the pulse lasts.
pub const PULSE_SHADE_SHOWS: f32 = 40.0;
/// Height of the pillar marking something hidden behind a wall.
pub const PULSE_PILLAR_HEIGHT: f32 = 24.0;

// --- HASTE: a zone where you are quick and chasers drag ---
pub const HASTE_RADIUS: f32 = 6.0;
pub const HASTE_PLAYER: f32 = 1.5;
pub const HASTE_FOE: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HasteField {
    pub at: Vec3,
    pub left: f32,
}

impl HasteField {
    pub fn new(at: Vec3, secs: f32) -> Self {
        Self { at, left: secs }
    }

    /// Ticks the zone; false once it has faded.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.left -= dt;
        self.left > 0.0
    }

    pub fn contains(&self, p: Vec3) -> bool {
        Vec3::new(p.x - self.at.x, 0.0, p.z - self.at.z).length() <= HASTE_RADIUS
    }

    /// Your speed multiplier standing at `p`.
    pub fn player_mult(&self, p: Vec3) -> f32 {
        if self.contains(p) {
            HASTE_PLAYER
        } else {
            1.0
        }
    }

    /// An enemy's time multiplier standing at `p`.
    pub fn foe_mult(&self, p: Vec3) -> f32 {
        if self.contains(p) {
            HASTE_FOE
        } else {
            1.0
        }
    }
}

// --- BIND: roots the nearest enemy ---
pub const BIND_REACH: f32 = 12.0;
pub const BIND_SECS: f32 = 5.0;

/// Index of the nearest of `points` within [`BIND_REACH`] (flat).
pub fn bind_pick(from: Vec3, points: &[Vec3]) -> Option<usize> {
    let flat = |p: Vec3| Vec3::new(p.x - from.x, 0.0, p.z - from.z).length();
    points
        .iter()
        .enumerate()
        .filter(|(_, &p)| flat(p) <= BIND_REACH)
        .min_by(|a, b| flat(*a.1).total_cmp(&flat(*b.1)))
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    const O: Vec3 = Vec3::ZERO;

    #[test]
    fn tether_takes_the_nearest_in_range_and_lands_in_reach() {
        let pts = [Vec3::new(30.0, 0.0, 0.0), Vec3::new(9.0, 0.0, 0.0), Vec3::new(4.0, 0.0, 3.0)];
        assert_eq!(tether_pick(O, &pts), Some(2));
        assert_eq!(tether_pick(O, &pts[..1]), None, "out of range");
        let mut at = pts[1];
        for _ in 0..600 {
            at = tether_step(at, O, 1.0 / 60.0);
        }
        assert!((at.length() - TETHER_STOP).abs() < 1e-3, "{at:?}");
        let one = tether_step(pts[1], O, 1.0 / 60.0);
        assert!((pts[1] - one).length() <= TETHER_SPEED / 60.0 + 1e-4);
    }

    #[test]
    fn veil_hides_you_only_while_it_lasts() {
        let me = Vec3::new(1.0, 1.0, 2.0);
        assert_eq!(seen_from(me, false), me);
        assert!((seen_from(me, true) - me).length() > 1000.0);
    }

    #[test]
    fn surge_doubles_one_jump_and_one_dash() {
        let mut s = Surge::default();
        assert_eq!((s.take_jump(), s.take_dash()), (1.0, 1.0));
        s.arm();
        assert!(s.armed());
        assert_eq!(s.take_jump(), SURGE_MULT);
        assert_eq!(s.take_jump(), 1.0, "only the next jump");
        assert!(s.armed(), "the dash is still waiting");
        assert_eq!(s.take_dash(), SURGE_MULT);
        assert!(!s.armed());
        s.arm();
        s.clear();
        assert_eq!(s.take_dash(), 1.0, "it faded");
    }

    #[test]
    fn haste_speeds_you_and_drags_chasers_only_inside() {
        let mut f = HasteField::new(O, 2.0);
        let (inside, outside) = (Vec3::new(2.0, 0.0, 2.0), Vec3::new(HASTE_RADIUS + 1.0, 0.0, 0.0));
        assert_eq!(f.player_mult(inside), HASTE_PLAYER);
        assert_eq!(f.foe_mult(inside), HASTE_FOE);
        assert_eq!((f.player_mult(outside), f.foe_mult(outside)), (1.0, 1.0));
        assert!(f.tick(1.0));
        assert!(!f.tick(1.5), "the zone fades");
    }

    #[test]
    fn bind_picks_the_nearest_enemy_in_reach() {
        let pts = [Vec3::new(8.0, 0.0, 0.0), Vec3::new(3.0, 5.0, 0.0), Vec3::new(40.0, 0.0, 0.0)];
        assert_eq!(bind_pick(O, &pts), Some(1));
        assert_eq!(bind_pick(O, &pts[2..]), None);
        assert_eq!(bind_pick(O, &[]), None);
    }

    #[test]
    fn quake_needs_ground() {
        assert!(quake_allowed(true) && !quake_allowed(false));
    }

    #[test]
    fn the_eight_are_distinct_and_new() {
        for (i, a) in NEW_ABILITIES.iter().enumerate() {
            assert!(!NEW_ABILITIES[i + 1..].contains(a));
            assert!(crate::upgrades::ALL_ABILITIES.contains(a));
        }
    }
}
