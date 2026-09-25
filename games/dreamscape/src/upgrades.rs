//! The roguelike layer: every dream you get through offers three random
//! upgrades for the rest of the run. Most are passive (faster feet, wider
//! sight, calmer enemies); the rarer ones teach an *ability* bound to a key.
//! Everything is pure and unit-tested; `main.rs` reads the multipliers.

use rand::{rngs::StdRng, Rng, SeedableRng};

/// Active abilities. At most `ABILITY_SLOTS` are held at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ability {
    /// A short burst of speed you can't be caught during.
    Dash,
    /// Teleport a cell and a half forward, if there's floor to land on.
    Blink,
    /// Every enemy freezes for a moment.
    Stillness,
    /// The next few seconds, enemies pass straight through you.
    Phase,
    /// The shard drifts toward you.
    ShardCall,
}

pub const ALL_ABILITIES: [Ability; 5] = [
    Ability::Dash,
    Ability::Blink,
    Ability::Stillness,
    Ability::Phase,
    Ability::ShardCall,
];

/// Shift and E.
pub const ABILITY_SLOTS: usize = 2;
pub const SLOT_KEYS: [&str; ABILITY_SLOTS] = ["shift", "e"];

pub const DASH_SPEED_MULT: f32 = 3.0;
/// Blink distance, in cells.
pub const BLINK_CELLS: f32 = 1.5;
/// ShardCall pull speed (world units / second).
pub const SHARD_CALL_SPEED: f32 = 2.5;

impl Ability {
    pub fn name(self) -> &'static str {
        match self {
            Ability::Dash => "DASH",
            Ability::Blink => "BLINK",
            Ability::Stillness => "STILLNESS",
            Ability::Phase => "PHASE",
            Ability::ShardCall => "SHARD CALL",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Ability::Dash => "burst forward; nothing can catch you mid-dash",
            Ability::Blink => "teleport ahead onto solid floor",
            Ability::Stillness => "freeze every enemy for a moment",
            Ability::Phase => "enemies pass through you for 3s",
            Ability::ShardCall => "the shard drifts toward you",
        }
    }

    /// Seconds between uses.
    pub fn cooldown(self) -> f32 {
        match self {
            Ability::Dash => 2.5,
            Ability::Blink => 5.0,
            Ability::Stillness => 12.0,
            Ability::Phase => 10.0,
            Ability::ShardCall => 20.0,
        }
    }

    /// Seconds the effect lasts (0 = instant).
    pub fn duration(self) -> f32 {
        match self {
            Ability::Dash => 0.18,
            Ability::Blink => 0.0,
            Ability::Stillness => 2.5,
            Ability::Phase => 3.0,
            Ability::ShardCall => 3.0,
        }
    }
}

/// One held ability and its timers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AbilityState {
    pub ability: Ability,
    /// Seconds until it can be used again.
    pub cooldown: f32,
    /// Seconds of effect left.
    pub active: f32,
}

impl AbilityState {
    pub fn new(ability: Ability) -> Self {
        Self {
            ability,
            cooldown: 0.0,
            active: 0.0,
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.cooldown = (self.cooldown - dt).max(0.0);
        self.active = (self.active - dt).max(0.0);
    }

    /// Fires if ready. `cooldown_mult` < 1 shortens the recharge.
    pub fn try_use(&mut self, cooldown_mult: f32) -> bool {
        if self.cooldown > 0.0 {
            return false;
        }
        self.cooldown = self.ability.cooldown() * cooldown_mult;
        self.active = self.ability.duration();
        true
    }

    pub fn is_active(&self) -> bool {
        self.active > 0.0
    }

    /// 1 = ready, 0 = just used.
    pub fn charge(&self, cooldown_mult: f32) -> f32 {
        let full = (self.ability.cooldown() * cooldown_mult).max(1e-3);
        (1.0 - self.cooldown / full).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Upgrade {
    SwiftFeet,
    Spring,
    WideEyes,
    HeavyAir,
    LongBreath,
    ShardSense,
    DreamAnchor,
    DustHoarder,
    LuckyMemory,
    CalmMind,
    /// Passive: one extra jump in mid-air.
    DoubleJump,
    /// Abilities recharge faster.
    Quicken,
    Learn(Ability),
}

pub const PASSIVES: [Upgrade; 12] = [
    Upgrade::SwiftFeet,
    Upgrade::Spring,
    Upgrade::WideEyes,
    Upgrade::HeavyAir,
    Upgrade::LongBreath,
    Upgrade::ShardSense,
    Upgrade::DreamAnchor,
    Upgrade::DustHoarder,
    Upgrade::LuckyMemory,
    Upgrade::CalmMind,
    Upgrade::DoubleJump,
    Upgrade::Quicken,
];

/// Which pixel icon an upgrade card shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Boot,
    Wind,
    Eye,
    Heart,
    Shard,
    Lid,
    Magnet,
    Brain,
    Gem,
}

pub struct Info {
    pub name: &'static str,
    pub desc: &'static str,
    /// How many times it can be taken in one run.
    pub max: u32,
    /// Relative odds of being offered.
    pub weight: u32,
    pub icon: Icon,
    pub color: [u8; 3],
}

impl Upgrade {
    pub fn info(self) -> Info {
        let i = |name, desc, max, weight, icon, color| Info {
            name,
            desc,
            max,
            weight,
            icon,
            color,
        };
        match self {
            Upgrade::SwiftFeet => i(
                "SWIFT FEET",
                "move 12% faster",
                3,
                10,
                Icon::Boot,
                [120, 230, 255],
            ),
            Upgrade::Spring => i(
                "SPRING",
                "jump 12% higher",
                2,
                7,
                Icon::Wind,
                [170, 255, 170],
            ),
            Upgrade::WideEyes => i(
                "WIDE EYES",
                "see further into the dark",
                3,
                9,
                Icon::Eye,
                [255, 240, 150],
            ),
            Upgrade::HeavyAir => i(
                "HEAVY AIR",
                "enemies move 10% slower",
                3,
                9,
                Icon::Lid,
                [180, 150, 255],
            ),
            Upgrade::LongBreath => i(
                "LONG BREATH",
                "longer safety after a respawn",
                3,
                7,
                Icon::Heart,
                [255, 140, 170],
            ),
            Upgrade::ShardSense => i(
                "SHARD SENSE",
                "shards turn up more often",
                2,
                7,
                Icon::Shard,
                [120, 255, 240],
            ),
            Upgrade::DreamAnchor => i(
                "DREAM ANCHOR",
                "keep your shard the next time you're caught",
                3,
                6,
                Icon::Heart,
                [255, 200, 90],
            ),
            Upgrade::DustHoarder => i(
                "DUST HOARDER",
                "+25% dust when you wake",
                4,
                8,
                Icon::Magnet,
                [255, 210, 80],
            ),
            Upgrade::LuckyMemory => i(
                "LUCKY MEMORY",
                "remember more dreams as cards",
                3,
                7,
                Icon::Brain,
                [255, 170, 235],
            ),
            Upgrade::CalmMind => i(
                "CALM MIND",
                "the dream warps and dims less",
                2,
                5,
                Icon::Lid,
                [200, 220, 255],
            ),
            Upgrade::DoubleJump => i(
                "DOUBLE JUMP",
                "jump again in mid-air",
                1,
                4,
                Icon::Wind,
                [140, 255, 200],
            ),
            Upgrade::Quicken => i(
                "QUICKEN",
                "abilities recharge 20% faster",
                3,
                4,
                Icon::Gem,
                [255, 150, 90],
            ),
            Upgrade::Learn(a) => i(a.name(), a.desc(), 1, 4, Icon::Gem, [255, 120, 255]),
        }
    }

    pub fn is_ability(self) -> bool {
        matches!(self, Upgrade::Learn(_))
    }
}

/// Everything picked up this run.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunUpgrades {
    pub taken: Vec<Upgrade>,
    /// Held abilities, slot order (shift, e).
    pub abilities: Vec<AbilityState>,
    /// DREAM ANCHOR charges spent.
    pub anchors_used: u32,
}

impl RunUpgrades {
    pub fn count(&self, u: Upgrade) -> u32 {
        self.taken.iter().filter(|&&t| t == u).count() as u32
    }

    fn n(&self, u: Upgrade) -> i32 {
        self.count(u) as i32
    }

    /// Takes `u`. A new ability goes into a free slot, or replaces the
    /// oldest one when both are full.
    pub fn take(&mut self, u: Upgrade) {
        if let Upgrade::Learn(a) = u {
            if self.abilities.len() >= ABILITY_SLOTS {
                let old = self.abilities.remove(0);
                self.taken.retain(|&t| t != Upgrade::Learn(old.ability));
            }
            self.abilities.push(AbilityState::new(a));
        }
        self.taken.push(u);
    }

    /// Can this still be offered?
    pub fn available(&self, u: Upgrade) -> bool {
        match u {
            Upgrade::Quicken if self.abilities.is_empty() => false,
            _ => self.count(u) < u.info().max,
        }
    }

    pub fn speed(&self) -> f32 {
        1.0 + 0.12 * self.n(Upgrade::SwiftFeet) as f32
    }
    pub fn jump(&self) -> f32 {
        1.0 + 0.12 * self.n(Upgrade::Spring) as f32
    }
    /// Extra sight radius, world units.
    pub fn sight_bonus(&self) -> f32 {
        1.8 * self.n(Upgrade::WideEyes) as f32
    }
    pub fn enemy_speed(&self) -> f32 {
        0.9_f32.powi(self.n(Upgrade::HeavyAir))
    }
    pub fn grace(&self) -> f32 {
        1.0 + 0.6 * self.n(Upgrade::LongBreath) as f32
    }
    pub fn shard_bonus(&self) -> f64 {
        0.12 * self.n(Upgrade::ShardSense) as f64
    }
    pub fn anchors_left(&self) -> u32 {
        self.count(Upgrade::DreamAnchor)
            .saturating_sub(self.anchors_used)
    }
    pub fn dust(&self) -> f32 {
        1.0 + 0.25 * self.n(Upgrade::DustHoarder) as f32
    }
    pub fn memory_bonus(&self) -> f32 {
        0.06 * self.n(Upgrade::LuckyMemory) as f32
    }
    /// Multiplies how strange the dream looks (not how it's built).
    pub fn calm(&self) -> f32 {
        0.8_f32.powi(self.n(Upgrade::CalmMind))
    }
    pub fn air_jumps(&self) -> u32 {
        self.count(Upgrade::DoubleJump)
    }
    pub fn cooldown(&self) -> f32 {
        0.8_f32.powi(self.n(Upgrade::Quicken))
    }

    pub fn tick(&mut self, dt: f32) {
        for a in &mut self.abilities {
            a.tick(dt);
        }
    }

    pub fn active(&self, ability: Ability) -> bool {
        self.abilities
            .iter()
            .any(|a| a.ability == ability && a.is_active())
    }

    /// Short names for the HUD, stacked ("SWIFT FEET x2").
    pub fn summary(&self) -> Vec<String> {
        let mut seen: Vec<Upgrade> = Vec::new();
        for &u in &self.taken {
            if !u.is_ability() && !seen.contains(&u) {
                seen.push(u);
            }
        }
        seen.iter()
            .map(|&u| match self.count(u) {
                1 => u.info().name.to_string(),
                n => format!("{} x{n}", u.info().name),
            })
            .collect()
    }
}

/// Three different upgrades to choose from, weighted by rarity, never one
/// that's maxed out. Deterministic per `seed`.
pub fn roll_choices(seed: u64, run: &RunUpgrades) -> Vec<Upgrade> {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x0F_FE12);
    let mut pool: Vec<Upgrade> = PASSIVES
        .iter()
        .copied()
        .chain(ALL_ABILITIES.iter().map(|&a| Upgrade::Learn(a)))
        .filter(|&u| run.available(u))
        .collect();
    let mut out = Vec::new();
    while out.len() < 3 && !pool.is_empty() {
        let total: u32 = pool.iter().map(|u| u.info().weight).sum();
        let mut pick = rng.gen_range(0..total);
        let idx = pool
            .iter()
            .position(|u| {
                let w = u.info().weight;
                if pick < w {
                    true
                } else {
                    pick -= w;
                    false
                }
            })
            .expect("pick < total");
        out.push(pool.remove(idx));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn choices_are_three_distinct_and_deterministic() {
        let run = RunUpgrades::default();
        for seed in 0..200 {
            let c = roll_choices(seed, &run);
            assert_eq!(c.len(), 3);
            assert_eq!(c.iter().collect::<HashSet<_>>().len(), 3);
            assert_eq!(c, roll_choices(seed, &run));
        }
        let variety: HashSet<Vec<Upgrade>> = (0..100).map(|s| roll_choices(s, &run)).collect();
        assert!(variety.len() > 50);
    }

    #[test]
    fn every_upgrade_is_offered_sometimes() {
        let mut run = RunUpgrades::default();
        run.take(Upgrade::Learn(Ability::Dash)); // so QUICKEN is allowed
        let seen: HashSet<Upgrade> = (0..3000).flat_map(|s| roll_choices(s, &run)).collect();
        for u in PASSIVES {
            assert!(seen.contains(&u), "{u:?} never offered");
        }
        for a in ALL_ABILITIES.iter().skip(1) {
            assert!(seen.contains(&Upgrade::Learn(*a)), "{a:?} never offered");
        }
        assert!(
            !seen.contains(&Upgrade::Learn(Ability::Dash)),
            "already held"
        );
    }

    #[test]
    fn maxed_upgrades_are_never_offered() {
        let mut run = RunUpgrades::default();
        for _ in 0..Upgrade::SwiftFeet.info().max {
            run.take(Upgrade::SwiftFeet);
        }
        for seed in 0..500 {
            assert!(!roll_choices(seed, &run).contains(&Upgrade::SwiftFeet));
            assert!(
                !roll_choices(seed, &run).contains(&Upgrade::Quicken),
                "quicken with no abilities"
            );
        }
        // Take everything: the pool runs dry gracefully.
        let mut all = RunUpgrades::default();
        for u in PASSIVES {
            for _ in 0..u.info().max {
                all.take(u);
            }
        }
        for a in ALL_ABILITIES {
            all.take(Upgrade::Learn(a));
        }
        assert!(roll_choices(1, &all).len() <= 3);
    }

    #[test]
    fn multipliers_stack_the_right_way() {
        let mut r = RunUpgrades::default();
        assert_eq!(
            (r.speed(), r.jump(), r.dust(), r.grace()),
            (1.0, 1.0, 1.0, 1.0)
        );
        assert_eq!((r.enemy_speed(), r.calm(), r.cooldown()), (1.0, 1.0, 1.0));
        r.take(Upgrade::SwiftFeet);
        r.take(Upgrade::SwiftFeet);
        r.take(Upgrade::HeavyAir);
        r.take(Upgrade::WideEyes);
        r.take(Upgrade::CalmMind);
        assert!((r.speed() - 1.24).abs() < 1e-5);
        assert!(r.enemy_speed() < 1.0 && r.calm() < 1.0);
        assert!(r.sight_bonus() > 0.0);
        assert_eq!(
            r.summary(),
            vec!["SWIFT FEET x2", "HEAVY AIR", "WIDE EYES", "CALM MIND"]
        );
    }

    #[test]
    fn anchors_are_spent() {
        let mut r = RunUpgrades::default();
        assert_eq!(r.anchors_left(), 0);
        r.take(Upgrade::DreamAnchor);
        r.take(Upgrade::DreamAnchor);
        assert_eq!(r.anchors_left(), 2);
        r.anchors_used += 1;
        assert_eq!(r.anchors_left(), 1);
    }

    #[test]
    fn two_ability_slots_oldest_is_replaced() {
        let mut r = RunUpgrades::default();
        r.take(Upgrade::Learn(Ability::Dash));
        r.take(Upgrade::Learn(Ability::Blink));
        r.take(Upgrade::Learn(Ability::Phase));
        let held: Vec<Ability> = r.abilities.iter().map(|a| a.ability).collect();
        assert_eq!(held, vec![Ability::Blink, Ability::Phase]);
        assert!(r.available(Upgrade::Learn(Ability::Dash)), "can relearn it");
        assert!(!r.available(Upgrade::Learn(Ability::Blink)));
    }

    #[test]
    fn abilities_cool_down_and_run_for_their_duration() {
        let mut a = AbilityState::new(Ability::Stillness);
        assert_eq!(a.charge(1.0), 1.0);
        assert!(a.try_use(1.0));
        assert!(a.is_active());
        assert!(!a.try_use(1.0), "still cooling down");
        a.tick(Ability::Stillness.duration() + 0.01);
        assert!(!a.is_active());
        assert!(a.charge(1.0) < 1.0);
        a.tick(Ability::Stillness.cooldown());
        assert!(a.try_use(0.5));
        assert!((a.cooldown - Ability::Stillness.cooldown() * 0.5).abs() < 1e-5);
    }

    #[test]
    fn dash_is_brief_and_every_ability_recharges() {
        assert!(Ability::Dash.duration() < 0.5);
        for a in ALL_ABILITIES {
            assert!(
                a.cooldown() > a.duration(),
                "{a:?} could be kept on forever"
            );
        }
    }
}
