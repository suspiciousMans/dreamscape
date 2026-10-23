//! COMBAT: the dreamer's STRIKE and what enemies can take.
//!
//! Pure rules, no GL and no clock: the game feeds `dt` and asks what to do.
//! A strike winds up for [`WINDUP`], lands once on everything in its arc,
//! then recovers (you walk slowly) until [`RECOVERY`] more has passed.
//! Enemies unravel at zero hp: nothing here kills, it undoes.

use crate::specials::Elite;
use engine::glam::Vec3;

/// How far a strike reaches (world units), and the cosine of its half-arc
/// (0.5 = 60° each way, a 120° fan in front).
pub const REACH: f32 = 2.4;
pub const ARC_COS: f32 = 0.5;
/// Targets this far above or below you are out of reach.
pub const REACH_HEIGHT: f32 = 2.0;
/// WIDE EYES widens the fan (cosine of the half-arc); LONG REACH stretches it.
pub const ARC_COS_WIDE: f32 = 0.35;
pub const LONG_REACH_PER: f32 = 0.25;
/// ECHO STRIKE: every this-many strikes sends a second wave, this much longer.
pub const ECHO_EVERY: u32 = 4;
pub const ECHO_REACH: f32 = 1.7;
/// HEAVY AIR: strikes shove further (per stack, up to two).
pub const HEAVY_KNOCK_PER: f32 = 0.5;
/// SPRING: how long after leaving the ground a strike still counts as aerial.
pub const SPRING_AIR_GRACE: f32 = 0.3;
/// CALM MIND: seconds of calm for one nerve, shorter with more stacks.
pub const CALM_REGEN_SECS: f32 = 25.0;
pub const CALM_REGEN_STACK: f32 = 0.8;
/// LUCKY MEMORY: extra chance of a calm mote per stack.
pub const LUCKY_MOTE_PER: f32 = 0.06;
/// WIDE EYES shows hit points on any enemy this near.
pub const EYES_PIP_REACH: f32 = 10.0;
/// COUNTER: how far the attacker can be, and the pause between counters.
pub const COUNTER_REACH: f32 = 2.5;
pub const COUNTER_COOLDOWN: f32 = 1.0;
pub const COUNTER_STAGGER: f32 = 1.2;
pub const RIPOSTE_STAGGER: f32 = 2.0;
/// THORNS bite whatever is this close when it hurts you.
pub const THORNS_REACH: f32 = 2.5;
/// CALM SPORE: a mote's reach on what is near.
pub const SPORE_REACH: f32 = 4.0;
pub const WINDUP: f32 = 0.10;
pub const RECOVERY: f32 = 0.25;
/// Minimum time between the starts of two strikes.
pub const COOLDOWN: f32 = 0.45;
/// Fraction of your speed while recovering from a strike (SWIFT FEET: better).
pub const RECOVERY_WALK: f32 = 0.6;
pub const RECOVERY_WALK_SWIFT: f32 = 0.8;
/// A struck enemy reels for this long and is shoved this far.
pub const STAGGER_SECS: f32 = 0.7;
pub const KNOCKBACK: f32 = 0.9;
/// Damage multipliers.
pub const UNAWARE_MULT: f32 = 2.0;
pub const AIRBORNE_MULT: f32 = 1.5;
pub const STAGGERED_MULT: f32 = 2.0;
pub const HUSH_MULT: f32 = 3.0;
pub const GHOST_MULT: f32 = 2.0;
pub const HIGH_GROUND_MULT: f32 = 1.5;
/// How much higher than the target counts as high ground.
pub const HIGH_GROUND_RISE: f32 = 0.8;
/// BLINK: the next strike is a sneak attack for this long.
pub const BLINK_BACKSTAB_SECS: f32 = 2.0;
/// DASH cuts what it passes through within this reach.
pub const DASH_CUT_REACH: f32 = 1.4;
/// FLARE: reach and stagger of the light.
pub const FLARE_REACH: f32 = 7.0;
pub const FLARE_STAGGER: f32 = 1.0;
/// DECOY bursts when it ends; ANCHOR leaves a wave behind; LEAP and SLAM shock on landing.
pub const DECOY_BURST_REACH: f32 = 3.0;
pub const ANCHOR_WAVE_REACH: f32 = 3.0;
pub const LEAP_SHOCK_REACH: f32 = 2.5;
/// WARD reflects onto the nearest enemy within this reach.
pub const WARD_REFLECT_REACH: f32 = 3.0;
/// SHARD CALL draws calm motes in at this speed.
pub const MOTE_PULL_SPEED: f32 = 5.0;
/// Seconds an unravelling enemy takes to come apart.
pub const UNRAVEL_SECS: f32 = 0.45;
/// Chance an unravelled enemy leaves a calm mote, and the mote's lifetime.
pub const MOTE_CHANCE: f32 = 0.12;
pub const MOTE_LIFE: f32 = 12.0;
pub const MOTE_PICKUP: f32 = 1.3;

/// The dreamer's strike: a cooldown and a swing in progress.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Strike {
    cooldown: f32,
    /// Seconds into the swing; `None` when idle.
    age: Option<f32>,
    landed: bool,
}

impl Strike {
    /// Begins a swing if idle and off cooldown. `cooldown_mult` < 1 is faster.
    pub fn try_start(&mut self, cooldown_mult: f32) -> bool {
        if self.cooldown > 0.0 || self.age.is_some() {
            return false;
        }
        self.age = Some(0.0);
        self.landed = false;
        self.cooldown = COOLDOWN * cooldown_mult.max(0.2);
        true
    }

    /// Advances by `dt`. True on the one frame the swing lands.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.cooldown = (self.cooldown - dt).max(0.0);
        let Some(age) = self.age else {
            return false;
        };
        let now = age + dt;
        let impact = !self.landed && now >= WINDUP;
        if impact {
            self.landed = true;
        }
        self.age = (now < WINDUP + RECOVERY).then_some(now);
        impact
    }

    /// Past the impact and still recovering: you move slowly.
    pub fn recovering(&self) -> bool {
        self.age.is_some_and(|a| a >= WINDUP)
    }

    /// 0..1 through the whole swing, or `None` when idle.
    #[cfg(test)]
    pub fn progress(&self) -> Option<f32> {
        self.age.map(|a| (a / (WINDUP + RECOVERY)).clamp(0.0, 1.0))
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Strike reach with LONG REACH stacks.
pub fn reach_with(stacks: u32) -> f32 {
    REACH * (1.0 + LONG_REACH_PER * stacks.min(2) as f32)
}

/// Seconds of calm that earn one nerve with CALM MIND stacks (0 = never).
pub fn calm_interval(stacks: u32) -> Option<f32> {
    (stacks > 0).then(|| CALM_REGEN_SECS * CALM_REGEN_STACK.powi(stacks as i32 - 1))
}

/// Is `target` inside the fan in front of `from` (facing a flat unit vector)?
pub fn in_arc(from: Vec3, facing: Vec3, target: Vec3, reach: f32, arc_cos: f32) -> bool {
    if (target.y - from.y).abs() > REACH_HEIGHT {
        return false;
    }
    let flat = Vec3::new(target.x - from.x, 0.0, target.z - from.z);
    let d = flat.length();
    if d > reach {
        return false;
    }
    if d < 0.3 {
        return true; // on top of you: always hit
    }
    let f = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    f == Vec3::ZERO || flat.normalize().dot(f) >= arc_cos
}

/// What made this strike special.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ctx {
    /// The target has not noticed you (a sneak attack).
    pub unaware: bool,
    /// You are in the air.
    pub airborne: bool,
    /// The target is already reeling (or frozen).
    pub staggered: bool,
    /// PHASE: you strike as a ghost.
    pub ghost: bool,
    /// HUSH: an unaware target takes a deeper cut.
    pub hush: bool,
    /// STEP: you stand above the target.
    pub high_ground: bool,
    /// Struck on armour (a Lullaby's front): damage is halved.
    pub guarded: bool,
}

pub fn damage(base: f32, ctx: Ctx) -> f32 {
    let mut d = base;
    if ctx.unaware {
        d *= if ctx.hush { HUSH_MULT } else { UNAWARE_MULT };
    }
    if ctx.ghost {
        d *= GHOST_MULT;
    }
    if ctx.high_ground {
        d *= HIGH_GROUND_MULT;
    }
    if ctx.airborne {
        d *= AIRBORNE_MULT;
    }
    if ctx.staggered {
        d *= STAGGERED_MULT;
    }
    if ctx.guarded {
        d *= crate::lullaby::FRONT_MULT;
    }
    d
}

/// Kinds of enemy that can be struck.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    // === enemies-a ===
    Tooth,
    Weeper,
    Lullaby,
    // === end enemies-a ===
    /// A pacer; `child` is a splitter's offspring.
    Pacer { elite: Option<Elite>, child: bool },
    Stalker,
    Sentry,
    Drifter,
    Jester,
}

/// Hit points by kind (see the combat plan).
pub fn base_hp(kind: Kind) -> f32 {
    match kind {
        Kind::Tooth => crate::tooth::HP,
        Kind::Weeper => crate::weeper::HP,
        Kind::Lullaby => crate::lullaby::HP,
        Kind::Pacer { child: true, .. } => 1.0,
        Kind::Pacer { elite: Some(Elite::Big), .. } => 4.0,
        Kind::Pacer { .. } => 2.0,
        Kind::Stalker => 3.0,
        Kind::Sentry => 4.0,
        Kind::Drifter => 2.0,
        Kind::Jester => 1.0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Foe {
    pub hp: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Still standing.
    Hurt,
    /// Undone.
    Unravel,
}

impl Foe {
    pub fn new(kind: Kind) -> Self {
        let hp = base_hp(kind);
        Self { hp, max: hp }
    }

    pub fn hit(&mut self, dmg: f32) -> Outcome {
        self.hp = (self.hp - dmg).max(0.0);
        if self.hp <= 0.0 {
            Outcome::Unravel
        } else {
            Outcome::Hurt
        }
    }
}

/// What an unravelled enemy leaves behind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loot {
    pub dust: u32,
    pub mote: bool,
}

fn mix(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^ (x >> 33)
}

/// Loot from a seed (its own stream: it never touches the dream's RNG).
/// `bonus_dust` adds to the dust (DUST HOARDER); `mote_bonus` to the mote
/// chance (LUCKY MEMORY).
pub fn loot(seed: u64, bonus_dust: u32, mote_bonus: f32) -> Loot {
    let r = mix(seed ^ 0x100_7);
    let dust = 1 + (r % 3) as u32 + bonus_dust;
    let mote = (mix(seed ^ 0x40_7e) % 10_000) as f32 / 10_000.0 < MOTE_CHANCE + mote_bonus;
    Loot { dust, mote }
}

/// Scale of an unravelling enemy `age` seconds in: a small swell, then gone.
pub fn unravel_scale(age: f32) -> f32 {
    let t = (age / UNRAVEL_SECS).clamp(0.0, 1.0);
    if t < 0.2 {
        1.0 + 0.25 * (t / 0.2)
    } else {
        1.25 * (1.0 - (t - 0.2) / 0.8)
    }
}

/// Bob height of a calm mote.
pub fn mote_bob(age: f32) -> f32 {
    0.9 + 0.15 * (age * 3.0).sin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_strike_winds_up_lands_once_and_recovers() {
        let mut s = Strike::default();
        assert!(s.try_start(1.0));
        assert!(!s.try_start(1.0), "no second swing mid-swing");
        let mut impacts = 0;
        let mut t = 0.0;
        while t < 1.0 {
            if s.tick(0.01) {
                impacts += 1;
                assert!(t >= WINDUP - 0.011, "not before the windup, t = {t}");
            }
            t += 0.01;
        }
        assert_eq!(impacts, 1);
        assert_eq!(s.progress(), None, "idle again");
    }

    #[test]
    fn the_cooldown_gates_the_next_swing_and_speeds_up() {
        let mut s = Strike::default();
        s.try_start(1.0);
        for _ in 0..40 {
            s.tick(0.01);
        }
        assert!(!s.try_start(1.0), "0.4s in, cooldown 0.45");
        for _ in 0..10 {
            s.tick(0.01);
        }
        assert!(s.try_start(0.5), "ready");
        let mut quick = Strike::default();
        quick.try_start(0.5);
        for _ in 0..40 {
            quick.tick(0.01);
        }
        assert!(quick.try_start(1.0), "half the cooldown: the swing itself is the limit");
    }

    #[test]
    fn recovering_means_after_the_impact_only() {
        let mut s = Strike::default();
        assert!(!s.recovering());
        s.try_start(1.0);
        assert!(!s.recovering(), "winding up: not yet");
        s.tick(WINDUP + 0.01);
        assert!(s.recovering());
        s.tick(RECOVERY);
        assert!(!s.recovering());
    }

    #[test]
    fn the_arc_is_a_fan_in_front_with_a_reach_and_a_height() {
        let me = Vec3::new(0.0, 1.0, 0.0);
        let f = Vec3::Z;
        let at = |x: f32, y: f32, z: f32| Vec3::new(x, y, z);
        assert!(in_arc(me, f, at(0.0, 1.0, 2.0), REACH, ARC_COS), "dead ahead");
        assert!(!in_arc(me, f, at(0.0, 1.0, 2.6), REACH, ARC_COS), "too far");
        assert!(!in_arc(me, f, at(0.0, 1.0, -1.0), REACH, ARC_COS), "behind you");
        assert!(in_arc(me, f, at(1.5, 1.0, 1.5), REACH, ARC_COS), "45 degrees");
        assert!(!in_arc(me, f, at(2.0, 1.0, 0.5), REACH, ARC_COS), "off to the side");
        assert!(!in_arc(me, f, at(0.0, 4.0, 1.0), REACH, ARC_COS), "far above");
        assert!(in_arc(me, f, at(0.1, 1.0, 0.1), REACH, ARC_COS), "on top of you");
    }

    #[test]
    fn damage_multipliers_stack() {
        let plain = Ctx::default();
        assert_eq!(damage(1.0, plain), 1.0);
        assert_eq!(damage(1.0, Ctx { unaware: true, ..plain }), 2.0);
        assert_eq!(damage(1.0, Ctx { airborne: true, ..plain }), 1.5);
        assert_eq!(damage(1.0, Ctx { staggered: true, ..plain }), 2.0);
        let all = Ctx { unaware: true, airborne: true, staggered: true, ..plain };
        assert_eq!(damage(1.0, all), 6.0);
        assert_eq!(damage(1.0, Ctx { guarded: true, ..plain }), 0.5, "armour halves it");
        assert_eq!(damage(1.0, Ctx { guarded: true, ..all }), 3.0);
    }

    #[test]
    fn long_reach_and_calm_mind_scale_with_stacks() {
        assert_eq!(reach_with(0), REACH);
        assert!((reach_with(1) - REACH * 1.25).abs() < 1e-5);
        assert!((reach_with(2) - REACH * 1.5).abs() < 1e-5);
        assert_eq!(reach_with(9), reach_with(2), "two stacks at most");
        assert_eq!(calm_interval(0), None);
        assert_eq!(calm_interval(1), Some(25.0));
        assert!(calm_interval(3).unwrap() < calm_interval(2).unwrap());
    }

    #[test]
    fn hush_ghost_and_high_ground_change_the_cut() {
        let plain = Ctx::default();
        let unaware = Ctx { unaware: true, ..plain };
        assert_eq!(damage(1.0, Ctx { hush: true, ..unaware }), 3.0, "hush: x3 on the unaware");
        assert_eq!(damage(1.0, Ctx { hush: true, ..plain }), 1.0, "hush needs an unaware target");
        assert_eq!(damage(1.0, Ctx { ghost: true, ..plain }), 2.0);
        assert_eq!(damage(1.0, Ctx { high_ground: true, ..plain }), 1.5);
        assert_eq!(
            damage(1.0, Ctx { ghost: true, high_ground: true, ..plain }),
            3.0
        );
    }

    #[test]
    fn hp_follows_the_plan() {
        let pacer = Kind::Pacer { elite: None, child: false };
        assert_eq!(base_hp(pacer), 2.0);
        assert_eq!(base_hp(Kind::Pacer { elite: Some(Elite::Big), child: false }), 4.0);
        assert_eq!(base_hp(Kind::Pacer { elite: Some(Elite::Splitter), child: true }), 1.0);
        assert_eq!(base_hp(Kind::Pacer { elite: Some(Elite::Fast), child: false }), 2.0);
        assert_eq!(base_hp(Kind::Stalker), 3.0);
        assert_eq!(base_hp(Kind::Sentry), 4.0);
        assert_eq!(base_hp(Kind::Jester), 1.0);
        assert_eq!(base_hp(Kind::Tooth), 2.0);
        assert_eq!(base_hp(Kind::Weeper), 2.0);
        assert_eq!(base_hp(Kind::Lullaby), 6.0);
    }

    #[test]
    fn two_hits_unravel_a_pacer_and_overkill_is_fine() {
        let mut f = Foe::new(Kind::Pacer { elite: None, child: false });
        assert_eq!(f.hit(1.0), Outcome::Hurt);
        assert_eq!(f.hit(1.0), Outcome::Unravel);
        let mut g = Foe::new(Kind::Jester);
        assert_eq!(g.hit(6.0), Outcome::Unravel);
        assert_eq!(g.hp, 0.0);
    }

    #[test]
    fn loot_is_deterministic_small_and_motes_are_rare() {
        assert_eq!(loot(7, 0, 0.0), loot(7, 0, 0.0));
        let mut motes = 0;
        for seed in 0..5000u64 {
            let l = loot(seed, 0, 0.0);
            assert!((1..=3).contains(&l.dust), "{l:?}");
            motes += l.mote as u32;
        }
        let rate = motes as f32 / 5000.0;
        assert!((0.08..0.16).contains(&rate), "mote rate {rate}");
        assert_eq!(loot(7, 2, 0.0).dust, loot(7, 0, 0.0).dust + 2, "dust hoarder adds");
        let lucky = (0..5000u64).filter(|&s| loot(s, 0, 0.12).mote).count();
        let plain = (0..5000u64).filter(|&s| loot(s, 0, 0.0).mote).count();
        assert!(lucky > plain + 200, "lucky memory: more motes ({plain} -> {lucky})");
    }

    #[test]
    fn unravelling_swells_then_vanishes() {
        assert_eq!(unravel_scale(0.0), 1.0);
        assert!(unravel_scale(UNRAVEL_SECS * 0.2) > 1.2);
        assert!(unravel_scale(UNRAVEL_SECS * 0.6) < unravel_scale(UNRAVEL_SECS * 0.3));
        assert_eq!(unravel_scale(UNRAVEL_SECS), 0.0);
        assert_eq!(unravel_scale(9.0), 0.0);
    }
}
