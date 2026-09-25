//! Dream variants: a twist rolled onto a dream on top of its theme. The
//! same garden can come back weightless, flooded, pitch black or upside
//! down. Deeper dreams twist more often; long runs stack two twists.
//! Everything here is a pure multiplier the game reads; nothing is drawn.

use super::theme::{DreamTheme, LayoutKind};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Variant {
    /// Half gravity, higher jumps.
    LowGravity,
    /// Knee-deep water: slower walking.
    Flooded,
    /// You can see much less, but the shard's beacon burns brighter.
    Blackout,
    /// The dream is upside down (controls follow the screen).
    Inverted,
    /// Twice the enemies, but slower ones.
    Swarm,
    /// Enemies freeze for a second every few seconds.
    Frozen,
    /// Reach the portal before the timer runs out for bonus dust.
    Hasty,
    /// A guaranteed shard and double dust, but the enemies are faster.
    Gilded,
}

pub const ALL_VARIANTS: [Variant; 8] = [
    Variant::LowGravity,
    Variant::Flooded,
    Variant::Blackout,
    Variant::Inverted,
    Variant::Swarm,
    Variant::Frozen,
    Variant::Hasty,
    Variant::Gilded,
];

/// Seconds a HASTY dream gives you.
pub const HASTY_SECONDS: f32 = 45.0;
/// Dust for beating a HASTY timer.
pub const HASTY_DUST: u32 = 12;
/// FROZEN: enemies stand still for this long out of every period.
pub const FROZEN_HOLD: f32 = 1.0;
pub const FROZEN_PERIOD: f32 = 3.0;

impl Variant {
    pub fn label(self) -> &'static str {
        match self {
            Variant::LowGravity => "WEIGHTLESS",
            Variant::Flooded => "FLOODED",
            Variant::Blackout => "BLACKOUT",
            Variant::Inverted => "UPSIDE DOWN",
            Variant::Swarm => "SWARMING",
            Variant::Frozen => "STUTTERING",
            Variant::Hasty => "HURRIED",
            Variant::Gilded => "GILDED",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Variant::LowGravity => "you weigh almost nothing",
            Variant::Flooded => "the floor is under water",
            Variant::Blackout => "the lights went out",
            Variant::Inverted => "everything is the wrong way up",
            Variant::Swarm => "there are more of them, but they're tired",
            Variant::Frozen => "they keep forgetting to move",
            Variant::Hasty => "the portal is closing",
            Variant::Gilded => "everything here is worth more",
        }
    }

    fn fits(self, theme: DreamTheme) -> bool {
        let layout = theme.spec().layout;
        let has_void = matches!(layout, LayoutKind::PlatformChain | LayoutKind::Spiral);
        let has_enemies = theme.spec().enemies.1 > 0;
        match self {
            // Water would hide the gaps you need to jump.
            Variant::Flooded => !has_void,
            Variant::Swarm | Variant::Frozen => has_enemies,
            _ => true,
        }
    }
}

/// The twists on one dream, as multipliers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Twists(pub Vec<Variant>);

impl Twists {
    pub fn has(&self, v: Variant) -> bool {
        self.0.contains(&v)
    }

    fn product(&self, f: impl Fn(Variant) -> f32) -> f32 {
        self.0.iter().map(|&v| f(v)).product()
    }

    pub fn gravity(&self) -> f32 {
        self.product(|v| if v == Variant::LowGravity { 0.5 } else { 1.0 })
    }

    pub fn jump(&self) -> f32 {
        self.product(|v| if v == Variant::LowGravity { 1.15 } else { 1.0 })
    }

    pub fn walk(&self) -> f32 {
        self.product(|v| if v == Variant::Flooded { 0.8 } else { 1.0 })
    }

    pub fn sight(&self) -> f32 {
        self.product(|v| if v == Variant::Blackout { 0.7 } else { 1.0 })
    }

    pub fn enemy_count(&self) -> f32 {
        self.product(|v| if v == Variant::Swarm { 2.0 } else { 1.0 })
    }

    pub fn enemy_speed(&self) -> f32 {
        self.product(|v| match v {
            Variant::Swarm => 0.8,
            Variant::Gilded => 1.2,
            _ => 1.0,
        })
    }

    pub fn dust(&self) -> f32 {
        self.product(|v| if v == Variant::Gilded { 2.0 } else { 1.0 })
    }

    pub fn timer(&self) -> Option<f32> {
        self.has(Variant::Hasty).then_some(HASTY_SECONDS)
    }

    /// FROZEN dreams: are the enemies stuck at this moment?
    pub fn enemies_frozen(&self, age: f32) -> bool {
        self.has(Variant::Frozen) && age.rem_euclid(FROZEN_PERIOD) < FROZEN_HOLD
    }

    pub fn label(&self) -> String {
        self.0
            .iter()
            .map(|v| v.label())
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

/// Chance a dream at `depth` gets a twist: none in the first dream, then
/// climbing to 70%.
pub fn twist_chance(depth: u32) -> f64 {
    if depth < 2 {
        0.0
    } else {
        (0.25 + 0.06 * depth as f64).min(0.7)
    }
}

/// Rolls the twists for one dream. Long (hard) runs past depth 8 may stack two.
#[cfg(test)]
pub fn roll(theme: DreamTheme, seed: u64, depth: u32, hard: bool) -> Twists {
    roll_with(theme, seed, depth, hard, 0.0)
}

/// `roll`, with `bonus` added to the twist chance (ascension).
pub fn roll_with(theme: DreamTheme, seed: u64, depth: u32, hard: bool, bonus: f64) -> Twists {
    if matches!(theme, DreamTheme::Lobby | DreamTheme::Awakening) {
        return Twists::default();
    }
    let mut rng = StdRng::seed_from_u64(seed ^ 0x7715_7ED0);
    let mut out = Vec::new();
    let count = if hard && depth >= 8 { 2 } else { 1 };
    for _ in 0..count {
        let chance = if depth < 2 {
            0.0
        } else {
            (twist_chance(depth) + bonus).min(0.9)
        };
        if !rng.gen_bool(chance) {
            continue;
        }
        let options: Vec<Variant> = ALL_VARIANTS
            .iter()
            .copied()
            .filter(|v| v.fits(theme) && !out.contains(v))
            .collect();
        if let Some(&v) = options.choose(&mut rng) {
            out.push(v);
        }
    }
    Twists(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::theme::ALL_THEMES;
    use std::collections::HashSet;

    #[test]
    fn calm_dreams_never_twist() {
        for seed in 0..200 {
            assert!(roll(DreamTheme::Lobby, seed, 9, true).0.is_empty());
            assert!(roll(DreamTheme::Awakening, seed, 9, true).0.is_empty());
            assert!(roll(DreamTheme::Garden, seed, 1, true).0.is_empty());
        }
    }

    #[test]
    fn twists_get_commoner_with_depth_and_all_show_up() {
        let rate = |depth| {
            (0..400)
                .filter(|&s| !roll(DreamTheme::Garden, s, depth, false).0.is_empty())
                .count()
        };
        assert!(rate(3) < rate(10));
        let seen: HashSet<Variant> = ALL_THEMES
            .iter()
            .flat_map(|&t| (0..300).flat_map(move |s| roll(t, s, 10, true).0))
            .collect();
        assert_eq!(seen.len(), ALL_VARIANTS.len(), "seen {seen:?}");
    }

    #[test]
    fn twists_fit_their_dream() {
        for t in ALL_THEMES {
            for s in 0..200 {
                let tw = roll(t, s, 12, true);
                assert!(tw.0.len() <= 2);
                let unique: HashSet<_> = tw.0.iter().collect();
                assert_eq!(unique.len(), tw.0.len(), "{t:?} doubled a twist");
                for v in &tw.0 {
                    assert!(v.fits(t), "{t:?} got {v:?}");
                }
            }
        }
        assert!(!Variant::Flooded.fits(DreamTheme::VoidPlatforms));
        assert!(!Variant::Swarm.fits(DreamTheme::Lobby));
    }

    #[test]
    fn only_long_runs_stack_twists() {
        let stacked = |hard| {
            (0..400)
                .filter(|&s| roll(DreamTheme::NightmareFactory, s, 12, hard).0.len() == 2)
                .count()
        };
        assert_eq!(stacked(false), 0);
        assert!(stacked(true) > 50);
    }

    #[test]
    fn multipliers() {
        let none = Twists::default();
        assert_eq!(
            (none.gravity(), none.walk(), none.sight(), none.dust()),
            (1.0, 1.0, 1.0, 1.0)
        );
        assert_eq!(none.timer(), None);
        let t = Twists(vec![Variant::LowGravity, Variant::Gilded]);
        assert!(t.gravity() < 1.0 && t.jump() > 1.0);
        assert_eq!(t.dust(), 2.0);
        assert!(t.enemy_speed() > 1.0);
        let swarm = Twists(vec![Variant::Swarm]);
        assert_eq!(swarm.enemy_count(), 2.0);
        assert!(swarm.enemy_speed() < 1.0);
        assert!(Twists(vec![Variant::Blackout]).sight() < 1.0);
        assert!(Twists(vec![Variant::Flooded]).walk() < 1.0);
        assert_eq!(Twists(vec![Variant::Hasty]).timer(), Some(HASTY_SECONDS));
        assert_eq!(t.label(), "WEIGHTLESS · GILDED");
    }

    #[test]
    fn frozen_enemies_stutter() {
        let f = Twists(vec![Variant::Frozen]);
        assert!(f.enemies_frozen(0.5));
        assert!(!f.enemies_frozen(2.0));
        assert!(f.enemies_frozen(FROZEN_PERIOD + 0.2));
        assert!(!Twists::default().enemies_frozen(0.5));
    }

    #[test]
    fn low_gravity_jump_still_lands_on_the_next_platform() {
        // Airtime grows with jump / gravity; horizontal reach must stay sane.
        let t = Twists(vec![Variant::LowGravity]);
        let airtime = 2.0 * crate::gameplay::JUMP_SPEED * t.jump() / (9.8 * t.gravity());
        assert!(airtime * crate::gameplay::MOVE_SPEED > 2.0 * crate::gameplay::CELL);
    }
}
