//! Abilities 3: eight more actives (Wager, Echo, Swap, Mulligan, Recall,
//! Overload, Bargain, Snare). Pure: positions, seconds and seeds in, numbers
//! out. `main.rs` wires them inside `// === abilities3 ===` blocks.
//!
//! Dust and cards only: nothing here touches anything but the run.

use engine::glam::Vec3;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::upgrades::{Ability, ALL_ABILITIES};

/// The eight abilities this module owns.
pub const NEW_ABILITIES: [Ability; 8] = [
    Ability::Wager,
    Ability::Echo,
    Ability::Swap,
    Ability::Mulligan,
    Ability::Recall,
    Ability::Overload,
    Ability::Bargain,
    Ability::Snare,
];

/// Only a deck has a hand to redeal or a discard to take from.
pub fn deck_only(a: Ability) -> bool {
    matches!(a, Ability::Mulligan | Ability::Recall)
}

/// Obtainable only by picks and the moth (no resonant pair carries them).
#[cfg(test)]
pub const PICK_ONLY: [Ability; 2] = [Ability::Bargain, Ability::Snare];

// --- WAGER: a random ability, used now ---
/// Out of 100: a jackpot, then a fizzle. Shown in the ability's description.
pub const WAGER_JACKPOT_PCT: u32 = 10;
pub const WAGER_FIZZLE_PCT: u32 = 10;
/// A jackpot doubles the rolled ability's duration.
pub const WAGER_JACKPOT_MULT: f32 = 2.0;
/// A fizzle gives back this share of Wager's own cooldown.
pub const WAGER_FIZZLE_REFUND: f32 = 0.5;
/// This many wagers without a jackpot and the next one is one.
pub const WAGER_PITY: u32 = 12;
/// Mixed into the dream seed: the wager has its own stream.
pub const WAGER_MIX: u64 = 0x3A6E_B1E5_0001;

/// What Wager never rolls.
pub const WAGER_EXCLUDED: [Ability; 4] =
    [Ability::Wager, Ability::Echo, Ability::Mulligan, Ability::Recall];

pub fn wager_pool() -> Vec<Ability> {
    ALL_ABILITIES
        .iter()
        .copied()
        .filter(|a| !WAGER_EXCLUDED.contains(a))
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WagerKind {
    Normal,
    Jackpot,
    Fizzle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WagerRoll {
    pub ability: Ability,
    pub kind: WagerKind,
}

/// The `n`th wager of a run in the dream with `seed`; `dry` is how many
/// wagers since the last jackpot. The same inputs always give the same roll,
/// so reloading cannot re-roll it.
pub fn wager_roll(seed: u64, n: u32, dry: u32) -> WagerRoll {
    let mut rng = StdRng::seed_from_u64(
        (seed ^ WAGER_MIX).wrapping_add((n as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)),
    );
    let pool = wager_pool();
    let ability = pool[rng.gen_range(0..pool.len())];
    let pct = rng.gen_range(0..100);
    let kind = if dry + 1 >= WAGER_PITY || pct < WAGER_JACKPOT_PCT {
        WagerKind::Jackpot
    } else if pct < WAGER_JACKPOT_PCT + WAGER_FIZZLE_PCT {
        WagerKind::Fizzle
    } else {
        WagerKind::Normal
    };
    WagerRoll { ability, kind }
}

/// Wager's own cooldown left after the roll (`base` is the full cooldown).
pub fn wager_cooldown(kind: WagerKind, base: f32, rolled_has_duration: bool) -> f32 {
    match kind {
        WagerKind::Normal => base,
        WagerKind::Fizzle => base * (1.0 - WAGER_FIZZLE_REFUND),
        WagerKind::Jackpot if rolled_has_duration => base,
        WagerKind::Jackpot => 0.0,
    }
}

/// Duration multiplier for the rolled ability.
pub fn wager_stretch(kind: WagerKind, has_duration: bool) -> f32 {
    if kind == WagerKind::Jackpot && has_duration {
        WAGER_JACKPOT_MULT
    } else {
        1.0
    }
}

/// The line on the HUD after a wager.
pub fn wager_line(roll: WagerRoll) -> String {
    match roll.kind {
        WagerKind::Normal => format!("WAGER: {}", roll.ability.name()),
        WagerKind::Jackpot => format!("WAGER: {} (jackpot)", roll.ability.name()),
        WagerKind::Fizzle => format!("WAGER: {} (fizzled)", roll.ability.name()),
    }
}
/// Seconds the wager line stays up.
pub const WAGER_LINE_SECS: f32 = 3.5;

// --- ECHO: do the last thing again ---
/// What Echo repeats: the last ability used, never Echo itself.
pub fn echo_target(last: Option<Ability>) -> Option<Ability> {
    last.filter(|&a| a != Ability::Echo)
}

// --- SWAP: trade places with the nearest enemy or the decoy ---
pub const SWAP_RADIUS: f32 = 12.0;
pub const SWAP_STAGGER: f32 = 2.0;
/// You land on their floor, so it can be no higher or lower than this.
pub const SWAP_MAX_DY: f32 = 3.0;

/// Index of the nearest of `points` in reach whose floor is close to yours.
pub fn swap_pick(from: Vec3, points: &[Vec3]) -> Option<usize> {
    let flat = |p: Vec3| Vec3::new(p.x - from.x, 0.0, p.z - from.z).length();
    points
        .iter()
        .enumerate()
        .filter(|(_, &p)| flat(p) <= SWAP_RADIUS && (p.y - from.y).abs() <= SWAP_MAX_DY)
        .min_by(|a, b| flat(*a.1).total_cmp(&flat(*b.1)))
        .map(|(i, _)| i)
}

// --- OVERLOAD: hold to charge, release to blast ---
pub const OVERLOAD_MAX: f32 = 2.0;
/// Used without a key to hold (Wager, Echo): a half-charged blast.
pub const OVERLOAD_AUTO: f32 = 1.0;
pub const OVERLOAD_RADIUS_MIN: f32 = 3.0;
pub const OVERLOAD_RADIUS_MAX: f32 = 10.0;
pub const OVERLOAD_STAGGER_MIN: f32 = 0.8;
pub const OVERLOAD_STAGGER_MAX: f32 = 3.0;

/// A charge in progress.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Charge {
    pub slot: usize,
    pub secs: f32,
}

impl Charge {
    pub fn new(slot: usize) -> Self {
        Self { slot, secs: 0.0 }
    }

    pub fn tick(&mut self, dt: f32) {
        self.secs = (self.secs + dt).min(OVERLOAD_MAX);
    }

    /// 0..=1 of a full charge.
    pub fn fraction(&self) -> f32 {
        (self.secs / OVERLOAD_MAX).clamp(0.0, 1.0)
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Blast radius for `secs` of charge.
pub fn overload_radius(secs: f32) -> f32 {
    lerp(OVERLOAD_RADIUS_MIN, OVERLOAD_RADIUS_MAX, (secs / OVERLOAD_MAX).clamp(0.0, 1.0))
}

/// How long the blast staggers.
pub fn overload_stagger(secs: f32) -> f32 {
    lerp(OVERLOAD_STAGGER_MIN, OVERLOAD_STAGGER_MAX, (secs / OVERLOAD_MAX).clamp(0.0, 1.0))
}

// --- BARGAIN: pay a nerve, your next strikes cut deeper ---
pub const BARGAIN_STRIKES: u32 = 3;
pub const BARGAIN_SECS: f32 = 10.0;
pub const BARGAIN_MULT: f32 = 3.0;

/// Paying needs a nerve to spare: never the last one.
pub fn bargain_allowed(nerve: u32) -> bool {
    nerve > 1
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bargain {
    strikes: u32,
    secs: f32,
}

impl Bargain {
    pub fn arm(&mut self) {
        self.strikes = BARGAIN_STRIKES;
        self.secs = BARGAIN_SECS;
    }

    pub fn active(&self) -> bool {
        self.strikes > 0 && self.secs > 0.0
    }

    pub fn tick(&mut self, dt: f32) {
        self.secs = (self.secs - dt).max(0.0);
        if self.secs <= 0.0 {
            self.strikes = 0;
        }
    }

    /// Damage multiplier for a blow landing now.
    pub fn mult(&self) -> f32 {
        if self.active() {
            BARGAIN_MULT
        } else {
            1.0
        }
    }

    /// A swing is over: one of the strikes is spent.
    pub fn spend_strike(&mut self) {
        self.strikes = self.strikes.saturating_sub(1);
    }

    #[cfg(test)]
    pub fn strikes_left(&self) -> u32 {
        self.strikes
    }
}

// --- SNARE: a trap underfoot ---
pub const SNARE_MAX: usize = 2;
pub const SNARE_RADIUS: f32 = 1.2;
pub const SNARE_ROOT: f32 = 3.0;

/// The live traps, each carrying a handle `T` (its entity).
#[derive(Clone, Debug, Default)]
pub struct Snares<T: Copy> {
    traps: Vec<(Vec3, T)>,
}

impl<T: Copy> Snares<T> {
    pub fn new() -> Self {
        Self { traps: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.traps.len()
    }

    /// Plants a trap; past [`SNARE_MAX`] the oldest is taken up and returned.
    pub fn plant(&mut self, at: Vec3, handle: T) -> Option<T> {
        let old = (self.traps.len() >= SNARE_MAX).then(|| self.traps.remove(0).1);
        self.traps.push((at, handle));
        old
    }

    /// Checks every trap against the enemies; each trap that one stepped on
    /// is spent. Returns (trap handle, enemy index). An enemy trips one trap.
    pub fn step(&mut self, enemies: &[Vec3]) -> Vec<(T, usize)> {
        let mut tripped: Vec<(T, usize)> = Vec::new();
        let mut used: Vec<usize> = Vec::new();
        self.traps.retain(|&(at, h)| {
            let near = enemies.iter().enumerate().find(|(i, p)| {
                !used.contains(i)
                    && Vec3::new(p.x - at.x, 0.0, p.z - at.z).length() <= SNARE_RADIUS
                    && (p.y - at.y).abs() <= 2.0
            });
            match near {
                Some((i, _)) => {
                    used.push(i);
                    tripped.push((h, i));
                    false
                }
                None => true,
            }
        });
        tripped
    }

    #[cfg(test)]
    pub fn clear(&mut self) -> Vec<T> {
        self.traps.drain(..).map(|t| t.1).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const O: Vec3 = Vec3::ZERO;

    #[test]
    fn the_eight_are_distinct_and_new() {
        for (i, a) in NEW_ABILITIES.iter().enumerate() {
            assert!(ALL_ABILITIES.contains(a));
            assert!(!NEW_ABILITIES[..i].contains(a));
            assert!(!crate::abilities2::NEW_ABILITIES.contains(a));
        }
    }

    #[test]
    fn wager_never_rolls_its_excluded_and_is_repeatable() {
        let pool = wager_pool();
        for a in WAGER_EXCLUDED {
            assert!(!pool.contains(&a), "{a:?}");
        }
        assert_eq!(pool.len(), ALL_ABILITIES.len() - 4);
        for n in 0..200 {
            let r = wager_roll(7, n, 0);
            assert!(pool.contains(&r.ability));
            assert_eq!(r, wager_roll(7, n, 0), "same roll when reloaded");
        }
        assert_ne!(
            (0..20).map(|n| wager_roll(7, n, 0).ability).collect::<Vec<_>>(),
            (0..20).map(|n| wager_roll(8, n, 0).ability).collect::<Vec<_>>(),
            "another dream, another stream"
        );
    }

    #[test]
    fn wager_odds_are_about_ten_and_ten() {
        let (mut jack, mut fizz) = (0, 0);
        let total = 20_000;
        for n in 0..total {
            match wager_roll(99, n, 0).kind {
                WagerKind::Jackpot => jack += 1,
                WagerKind::Fizzle => fizz += 1,
                WagerKind::Normal => {}
            }
        }
        let pct = |x: u32| x as f32 * 100.0 / total as f32;
        assert!((pct(jack) - WAGER_JACKPOT_PCT as f32).abs() < 1.0, "{}", pct(jack));
        assert!((pct(fizz) - WAGER_FIZZLE_PCT as f32).abs() < 1.0, "{}", pct(fizz));
    }

    #[test]
    fn a_dry_spell_ends_in_a_jackpot() {
        assert!((0..500).all(|n| wager_roll(3, n, WAGER_PITY - 1).kind == WagerKind::Jackpot));
        assert!((0..500).any(|n| wager_roll(3, n, 0).kind != WagerKind::Jackpot));
    }

    #[test]
    fn wager_cooldown_and_stretch_by_outcome() {
        assert_eq!(wager_cooldown(WagerKind::Normal, 20.0, true), 20.0);
        assert_eq!(wager_cooldown(WagerKind::Fizzle, 20.0, true), 10.0);
        assert_eq!(wager_cooldown(WagerKind::Jackpot, 20.0, false), 0.0, "instant: refunded");
        assert_eq!(wager_cooldown(WagerKind::Jackpot, 20.0, true), 20.0, "timed: doubled instead");
        assert_eq!(wager_stretch(WagerKind::Jackpot, true), 2.0);
        assert_eq!(wager_stretch(WagerKind::Jackpot, false), 1.0);
        assert_eq!(wager_stretch(WagerKind::Normal, true), 1.0);
        assert!(wager_line(WagerRoll { ability: Ability::Dash, kind: WagerKind::Normal }).contains("DASH"));
    }

    #[test]
    fn wager_desc_states_its_odds() {
        let d = Ability::Wager.desc();
        assert!(d.contains(&format!("{WAGER_JACKPOT_PCT}%")) && d.contains(&format!("{WAGER_FIZZLE_PCT}%")), "{d}");
    }

    #[test]
    fn echo_repeats_anything_but_echo() {
        assert_eq!(echo_target(None), None);
        assert_eq!(echo_target(Some(Ability::Echo)), None);
        assert_eq!(echo_target(Some(Ability::Dash)), Some(Ability::Dash));
    }

    #[test]
    fn swap_takes_the_nearest_on_a_reachable_floor() {
        let pts = [
            Vec3::new(8.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 4.0),
            Vec3::new(2.0, 9.0, 0.0), // up on another floor
            Vec3::new(40.0, 0.0, 0.0),
        ];
        assert_eq!(swap_pick(O, &pts), Some(1));
        assert_eq!(swap_pick(O, &pts[2..]), None);
        assert_eq!(swap_pick(O, &[]), None);
    }

    #[test]
    fn overload_scales_with_charge_and_caps() {
        let mut c = Charge::new(1);
        assert_eq!(c.fraction(), 0.0);
        c.tick(1.0);
        assert!((c.fraction() - 0.5).abs() < 1e-5);
        c.tick(10.0);
        assert_eq!(c.secs, OVERLOAD_MAX);
        assert_eq!(overload_radius(0.0), OVERLOAD_RADIUS_MIN);
        assert_eq!(overload_radius(OVERLOAD_MAX), OVERLOAD_RADIUS_MAX);
        assert_eq!(overload_radius(9.0), OVERLOAD_RADIUS_MAX);
        assert!(overload_radius(1.0) > overload_radius(0.5));
        assert_eq!(overload_stagger(0.0), OVERLOAD_STAGGER_MIN);
        assert_eq!(overload_stagger(OVERLOAD_MAX), OVERLOAD_STAGGER_MAX);
        assert!(overload_stagger(1.5) > overload_stagger(0.5));
    }

    #[test]
    fn bargain_cuts_three_times_for_three_strikes_or_ten_seconds() {
        assert!(!bargain_allowed(1) && bargain_allowed(2));
        let mut b = Bargain::default();
        assert_eq!(b.mult(), 1.0);
        b.arm();
        assert_eq!(b.mult(), BARGAIN_MULT);
        b.spend_strike();
        b.spend_strike();
        assert_eq!(b.mult(), BARGAIN_MULT);
        b.spend_strike();
        assert_eq!((b.mult(), b.strikes_left()), (1.0, 0));
        b.arm();
        b.tick(BARGAIN_SECS - 0.1);
        assert!(b.active());
        b.tick(0.2);
        assert!(!b.active() && b.mult() == 1.0, "the bargain lapsed");
    }

    #[test]
    fn snare_keeps_two_and_spends_one_on_the_first_foot() {
        let mut s: Snares<u32> = Snares::new();
        assert_eq!(s.plant(O, 1), None);
        assert_eq!(s.plant(Vec3::new(10.0, 0.0, 0.0), 2), None);
        assert_eq!(s.plant(Vec3::new(20.0, 0.0, 0.0), 3), Some(1), "the oldest is taken up");
        assert_eq!(s.len(), SNARE_MAX);
        let foes = [Vec3::new(5.0, 0.0, 0.0), Vec3::new(10.5, 0.0, 0.5)];
        assert_eq!(s.step(&foes[..1]), vec![]);
        assert_eq!(s.step(&foes), vec![(2, 1)]);
        assert_eq!(s.len(), 1, "that trap is spent");
        assert_eq!(s.step(&foes), vec![]);
        assert_eq!(s.clear(), vec![3]);
    }

    #[test]
    fn one_foe_trips_one_snare() {
        let mut s: Snares<u32> = Snares::new();
        s.plant(O, 1);
        s.plant(Vec3::new(0.5, 0.0, 0.0), 2);
        assert_eq!(s.step(&[Vec3::new(0.2, 0.0, 0.0)]).len(), 1);
        assert_eq!(s.len(), 1);
    }
}
