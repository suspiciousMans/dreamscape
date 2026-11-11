//! Relic weapons (R5): the held mesh and what it does to a strike.
//!
//! Pure: no GL, no clock, no global RNG. `weapon_for` rolls a relic's weapon
//! from its seed; `strike_mods` turns it into small, capped combat numbers;
//! `HeldSway` + `held_pose` place it lower-right in the first-person view.

use crate::dream::{DreamTheme, Shape, ALL_SHAPES};
use crate::relic_types::{RelicRarity, Weapon};
use crate::ui_motion::{effective, Spring, UiJuice, SOFT_DAMPING, SOFT_STIFFNESS};
use crate::worth::mix;
use glam::Vec3;

/// Held mesh size, in world units (`scale_pct` / 100).
pub const SCALE_MIN: u8 = 15;
pub const SCALE_MAX: u8 = 35;
/// Number of trail and clash styles.
pub const TRAILS: u8 = 4;
pub const CLASHES: u8 = 4;

/// Caps on what any relic adds to a strike.
pub const MAX_DAMAGE_BONUS: u32 = 1;
pub const MAX_REACH_MULT: f32 = 1.25;
pub const MAX_ARC_RELAX: f32 = 0.15;
pub const MAX_HIT_STOP: f32 = 0.05;

fn rank(r: RelicRarity) -> u32 {
    match r {
        RelicRarity::Cracked => 0,
        RelicRarity::Worn => 1,
        RelicRarity::Pristine => 2,
        RelicRarity::Mythic => 3,
    }
}

/// The weapon a relic of this rarity, dream and seed carries. Rarer relics
/// run larger (Cracked 15-26, Mythic 24-35).
pub fn weapon_for(rarity: RelicRarity, theme: DreamTheme, seed: u64) -> Weapon {
    let shape = (mix(seed, 0x5EA9_0001) % ALL_SHAPES.len() as u64) as u8;
    let size = 15 + 3 * rank(rarity) + (mix(seed, 0x5EA9_0002) % 12) as u32;
    let spec = theme.spec();
    let tint = spec
        .accents
        .first()
        .or(spec.prop_colors.first())
        .copied()
        .unwrap_or([255, 255, 255]);
    Weapon {
        shape,
        scale_pct: size.clamp(SCALE_MIN as u32, SCALE_MAX as u32) as u8,
        tint,
        trail: (mix(seed, 0x5EA9_0003) % TRAILS as u64) as u8,
        clash: (mix(seed, 0x5EA9_0004) % CLASHES as u64) as u8,
    }
}

/// The mesh shape for a weapon (an out-of-range index wraps).
pub fn shape_of(w: &Weapon) -> Shape {
    ALL_SHAPES[w.shape as usize % ALL_SHAPES.len()]
}

/// The strike's ring for a tier.
pub fn strike_sound(r: RelicRarity) -> crate::sounds::Sound {
    use crate::sounds::Sound;
    match r {
        RelicRarity::Cracked => Sound::RelicStrikeCracked,
        RelicRarity::Worn => Sound::RelicStrikeWorn,
        RelicRarity::Pristine => Sound::RelicStrikePristine,
        RelicRarity::Mythic => Sound::RelicStrikeMythic,
    }
}

/// Mythic relics keep a tiny cosmetic companion circling the weapon.
pub fn orbiter_shape(w: &Weapon) -> Shape {
    if shape_of(w) == Shape::Crescent {
        Shape::Orb
    } else {
        Shape::Crescent
    }
}

/// What a relic changes about a strike. `IDENTITY` changes nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrikeMods {
    pub damage_bonus: u32,
    pub reach_mult: f32,
    /// Subtracted from the arc's half-angle cosine (wider fan).
    pub arc_relax: f32,
    pub knock_add: f32,
    /// Replaces the strike's time dip length when > 0 (still <= 0.05 s).
    pub hit_stop_secs: f32,
}

impl StrikeMods {
    pub const IDENTITY: StrikeMods = StrikeMods {
        damage_bonus: 0,
        reach_mult: 1.0,
        arc_relax: 0.0,
        knock_add: 0.0,
        hit_stop_secs: 0.0,
    };
}

/// Capped strike numbers for a weapon of this rarity.
pub fn strike_mods(w: &Weapon, rarity: RelicRarity) -> StrikeMods {
    let r = rank(rarity) as f32;
    let size = (w.scale_pct.clamp(SCALE_MIN, SCALE_MAX) - SCALE_MIN) as f32
        / (SCALE_MAX - SCALE_MIN) as f32;
    let clash = w.clash.min(CLASHES - 1);
    StrikeMods {
        damage_bonus: u32::from(rarity == RelicRarity::Mythic).min(MAX_DAMAGE_BONUS),
        reach_mult: (1.0 + 0.05 * r + 0.10 * size).min(MAX_REACH_MULT),
        arc_relax: (0.05 * r).min(MAX_ARC_RELAX),
        knock_add: 0.1 * r,
        hit_stop_secs: if clash == 0 {
            0.0
        } else {
            (0.02 + 0.01 * clash as f32).min(MAX_HIT_STOP)
        },
    }
}

/// Where the held weapon sits, in view space (x right, y up, -z ahead).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeldPose {
    pub offset: Vec3,
    /// Euler radians (pitch, yaw, roll), applied YXZ.
    pub rot: Vec3,
    /// The idle bob alone (vertical metres), for tests and the orbiter.
    pub bob: f32,
}

/// Resting place: lower-right, forward of the eye.
pub const BASE_OFFSET: Vec3 = Vec3::new(0.42, -0.36, -0.85);
/// Roll toward the screen centre (about 15 degrees).
pub const TILT: f32 = 0.262;
pub const BOB_HZ: f32 = 0.6;
pub const BOB_AMP: f32 = 0.015;
/// Most the camera-turn lag shifts the weapon sideways.
pub const LAG_MAX: f32 = 0.06;

/// The camera-turn lag: a spring chasing the turn rate, so a quick look
/// drags the weapon behind and it eases back.
#[derive(Clone, Copy, Debug)]
pub struct HeldSway {
    lag: Spring,
}

impl HeldSway {
    pub const fn new() -> Self {
        Self { lag: Spring::at(0.0) }
    }

    /// `turn_rate`: yaw change in radians per second (right positive).
    /// Returns the lag in -1..1.
    pub fn step(&mut self, dt: f32, turn_rate: f32, juice: UiJuice, reduced: bool) -> f32 {
        if effective(juice, reduced) <= 0.0 {
            self.lag = Spring::at(0.0);
            return 0.0;
        }
        let target = (-turn_rate * 0.25).clamp(-1.0, 1.0);
        self.lag.step(target, dt, SOFT_STIFFNESS, SOFT_DAMPING);
        self.lag.pos.clamp(-1.0, 1.0)
    }
}

impl Default for HeldSway {
    fn default() -> Self {
        Self::new()
    }
}

/// The pose at time `t` with the sway's `lag` (-1..1). Reduced motion or
/// juice Off give the static resting pose (the tilt stays).
pub fn held_pose(t: f32, lag: f32, juice: UiJuice, reduced: bool) -> HeldPose {
    let k = effective(juice, reduced);
    let bob = (t * BOB_HZ * std::f32::consts::TAU).sin() * BOB_AMP * k;
    let lag = lag.clamp(-1.0, 1.0) * k.min(1.0);
    HeldPose {
        offset: BASE_OFFSET + Vec3::new(lag * LAG_MAX, bob, 0.0),
        rot: Vec3::new(0.0, -lag * 0.1, TILT),
        bob,
    }
}

/// Mythic orbiter offset from the weapon centre. Static when motion is off.
pub fn orbiter_offset(t: f32, juice: UiJuice, reduced: bool) -> Vec3 {
    const R: f32 = 0.16;
    let a = if effective(juice, reduced) > 0.0 { t * 1.4 } else { 0.8 };
    Vec3::new(a.cos() * R, a.sin() * R * 0.6, a.sin() * R * 0.5)
}

/// Dev: the relic to hold, from `DREAMSCAPE_GIVE_RELIC=<cracked|worn|pristine|mythic>[:seed]`.
pub fn parse_give(s: &str) -> Option<(RelicRarity, u64)> {
    let (name, seed) = s.split_once(':').unwrap_or((s, "1"));
    let rarity = match name.trim().to_ascii_lowercase().as_str() {
        "cracked" | "0" => RelicRarity::Cracked,
        "worn" | "1" => RelicRarity::Worn,
        "pristine" | "2" => RelicRarity::Pristine,
        "mythic" | "3" => RelicRarity::Mythic,
        _ => return None,
    };
    Some((rarity, seed.trim().parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::ALL_THEMES;

    const RARITIES: [RelicRarity; 4] = [
        RelicRarity::Cracked,
        RelicRarity::Worn,
        RelicRarity::Pristine,
        RelicRarity::Mythic,
    ];

    #[test]
    fn weapon_for_is_deterministic_and_in_range() {
        for &r in &RARITIES {
            for theme in ALL_THEMES {
                for seed in 0..60u64 {
                    let w = weapon_for(r, theme, seed);
                    assert_eq!(w, weapon_for(r, theme, seed));
                    assert!((w.shape as usize) < ALL_SHAPES.len());
                    assert!((SCALE_MIN..=SCALE_MAX).contains(&w.scale_pct));
                    assert!(w.trail < TRAILS && w.clash < CLASHES);
                    let spec = theme.spec();
                    let accent = spec.accents.first().or(spec.prop_colors.first()).copied();
                    assert_eq!(Some(w.tint), accent, "{theme:?}");
                }
            }
        }
    }

    #[test]
    fn rarer_weapons_run_larger_and_every_shape_turns_up() {
        let mean = |r| {
            (0..400u64)
                .map(|s| weapon_for(r, DreamTheme::Lobby, s).scale_pct as f32)
                .sum::<f32>()
                / 400.0
        };
        assert!(mean(RelicRarity::Mythic) > mean(RelicRarity::Cracked) + 4.0);
        let mut seen = [false; 11];
        for s in 0..400u64 {
            seen[weapon_for(RelicRarity::Worn, DreamTheme::Lobby, s).shape as usize] = true;
        }
        assert!(seen.iter().all(|&b| b));
    }

    #[test]
    fn strike_mods_stay_inside_the_caps() {
        for &r in &RARITIES {
            for theme in ALL_THEMES {
                for seed in 0..80u64 {
                    let w = weapon_for(r, theme, seed);
                    let m = strike_mods(&w, r);
                    assert!(m.damage_bonus <= MAX_DAMAGE_BONUS);
                    assert!(m.damage_bonus == 0 || r == RelicRarity::Mythic);
                    assert!((1.0..=MAX_REACH_MULT).contains(&m.reach_mult));
                    assert!((0.0..=MAX_ARC_RELAX).contains(&m.arc_relax));
                    assert!((0.0..=MAX_HIT_STOP).contains(&m.hit_stop_secs));
                    assert!(m.knock_add >= 0.0 && m.knock_add <= 0.31);
                }
            }
        }
        // Even a hand-made extreme weapon cannot break a cap.
        let w = Weapon { shape: 0, scale_pct: 255, tint: [0; 3], trail: 99, clash: 99 };
        let m = strike_mods(&w, RelicRarity::Mythic);
        assert!(m.reach_mult <= MAX_REACH_MULT && m.hit_stop_secs <= MAX_HIT_STOP);
    }

    #[test]
    fn identity_changes_nothing() {
        let i = StrikeMods::IDENTITY;
        assert_eq!((i.damage_bonus, i.reach_mult, i.arc_relax, i.knock_add, i.hit_stop_secs), (0, 1.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn pose_is_bounded_and_static_when_reduced_or_off() {
        for step in 0..400 {
            let t = step as f32 * 0.05;
            for lag in [-1.0, 0.0, 1.0, 9.0] {
                let p = held_pose(t, lag, UiJuice::Full, false);
                assert!((p.offset - BASE_OFFSET).abs().max_element() <= LAG_MAX + BOB_AMP * 1.4 + 1e-4);
                assert!(p.rot.abs().max_element() < 0.4);
            }
            let a = held_pose(t, 0.7, UiJuice::Soft, true);
            let b = held_pose(0.0, -0.7, UiJuice::Soft, true);
            assert_eq!(a, b, "reduced motion is static");
            assert_eq!(held_pose(t, 1.0, UiJuice::Off, false), a);
            assert_eq!(orbiter_offset(t, UiJuice::Soft, true), orbiter_offset(0.0, UiJuice::Soft, true));
        }
        // The tilt is toward the centre and about 15 degrees.
        let p = held_pose(0.0, 0.0, UiJuice::Soft, true);
        assert!((p.rot.z.to_degrees() - 15.0).abs() < 0.5);
        assert!(p.offset.x > 0.0 && p.offset.y < 0.0);
    }

    #[test]
    fn idle_bob_runs_at_about_point_six_hertz() {
        // Peaks 1/0.6 s apart: the first quarter-period peak is at 0.4167 s.
        let at = |t| held_pose(t, 0.0, UiJuice::Soft, false).bob;
        assert!(at(0.4167) > BOB_AMP * 0.99);
        assert!(at(1.0 / BOB_HZ).abs() < 1e-3);
    }

    #[test]
    fn sway_lags_a_turn_then_settles() {
        let mut s = HeldSway::new();
        let mut peak = 0.0f32;
        for _ in 0..30 {
            peak = peak.max(s.step(0.016, 3.0, UiJuice::Soft, false).abs());
        }
        assert!(peak > 0.1 && peak <= 1.0);
        let mut last = 1.0;
        for _ in 0..300 {
            last = s.step(0.016, 0.0, UiJuice::Soft, false);
        }
        assert!(last.abs() < 0.01);
        assert_eq!(HeldSway::new().step(0.016, 5.0, UiJuice::Soft, true), 0.0);
    }

    #[test]
    fn old_weapon_saves_load() {
        let w: Weapon = ron::from_str("(shape: 3, scale_pct: 22)").unwrap();
        assert_eq!((w.shape, w.scale_pct, w.tint, w.trail, w.clash), (3, 22, [0; 3], 0, 0));
    }

    #[test]
    fn give_relic_env_parses() {
        assert_eq!(parse_give("mythic"), Some((RelicRarity::Mythic, 1)));
        assert_eq!(parse_give("Worn:42"), Some((RelicRarity::Worn, 42)));
        assert_eq!(parse_give("2:7"), Some((RelicRarity::Pristine, 7)));
        assert_eq!(parse_give("nope"), None);
        assert_eq!(parse_give("worn:x"), None);
    }
}
