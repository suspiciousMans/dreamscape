//! The roguelike layer: every dream you get through offers three random
//! upgrades for the rest of the run. Most are passive (faster feet, wider
//! sight, calmer enemies); the rarer ones teach an *ability* bound to a key.
//! Everything is pure and unit-tested; `main.rs` reads the multipliers.

use rand::{rngs::StdRng, Rng, SeedableRng};

/// Active abilities. At most `ABILITY_SLOTS` are held at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

    /// Fires if ready. `cooldown_mult` < 1 shortens the recharge;
    /// `duration_mult` stretches the effect.
    pub fn try_use(&mut self, cooldown_mult: f32, duration_mult: f32) -> bool {
        if self.cooldown > 0.0 {
            return false;
        }
        self.active = self.ability.duration() * duration_mult;
        // However fast it recharges, an effect can never be kept on forever.
        self.cooldown = (self.ability.cooldown() * cooldown_mult).max(self.active + 1.0);
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
    /// Mythic: a shard, banked right now.
    LucidHeart,
    /// Mythic: one more card at every pick.
    WideChoice,
    Learn(Ability),
    /// Strong, with a price.
    Cursed(Curse),
}

/// Cursed cards: a boon and a bane in one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Curse {
    /// +30% speed; enemies notice you from 40% further.
    GlassCannon,
    /// +50% dust; enemies 15% faster.
    BloodMoon,
    /// Abilities recharge 40% faster; you see 25% less.
    Insomnia,
    /// Jump 30% higher; half the safety after a respawn.
    Featherfall,
    /// Shards much commoner; one fewer card at every pick.
    Greed,
}

pub const ALL_CURSES: [Curse; 5] = [
    Curse::GlassCannon,
    Curse::BloodMoon,
    Curse::Insomnia,
    Curse::Featherfall,
    Curse::Greed,
];

/// Chance one card of a pick is swapped for a cursed one.
pub const CURSE_CHANCE: f64 = 0.2;

pub const PASSIVES: [Upgrade; 14] = [
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
    Upgrade::LucidHeart,
    Upgrade::WideChoice,
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

/// Card rarity. Rarer tiers show up more the deeper you are, and a beaten
/// nightmare only offers rare and mythic cards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    Common,
    Rare,
    Mythic,
    Cursed,
}

impl Tier {
    pub fn label(self) -> &'static str {
        match self {
            Tier::Common => "common",
            Tier::Rare => "rare",
            Tier::Mythic => "mythic",
            Tier::Cursed => "cursed",
        }
    }

    /// Card frame colour.
    pub fn frame(self) -> [u8; 3] {
        match self {
            Tier::Common => [170, 160, 200],
            Tier::Rare => [90, 190, 255],
            Tier::Mythic => [255, 200, 60],
            Tier::Cursed => [255, 40, 70],
        }
    }

    /// Odds multiplier at `depth`.
    pub fn odds(self, depth: u32) -> f32 {
        let d = depth as f32;
        match self {
            Tier::Common => 1.0,
            Tier::Rare => (0.35 + 0.03 * d).min(0.8),
            Tier::Mythic => (0.05 + 0.012 * d).min(0.3),
            Tier::Cursed => 0.0,
        }
    }
}

pub struct Info {
    pub name: &'static str,
    pub desc: &'static str,
    /// How many times it can be taken in one run.
    pub max: u32,
    /// Relative odds of being offered (within its tier).
    pub weight: u32,
    pub icon: Icon,
    pub color: [u8; 3],
    pub tier: Tier,
}

impl Upgrade {
    pub fn info(self) -> Info {
        use Tier::*;
        let i = |name, desc, max, weight, icon, color, tier| Info {
            name,
            desc,
            max,
            weight,
            icon,
            color,
            tier,
        };
        match self {
            Upgrade::SwiftFeet => i(
                "SWIFT FEET",
                "move 12% faster",
                3,
                10,
                Icon::Boot,
                [120, 230, 255],
                Common,
            ),
            Upgrade::Spring => i(
                "SPRING",
                "jump 12% higher",
                2,
                7,
                Icon::Wind,
                [170, 255, 170],
                Common,
            ),
            Upgrade::WideEyes => i(
                "WIDE EYES",
                "see further into the dark",
                3,
                9,
                Icon::Eye,
                [255, 240, 150],
                Common,
            ),
            Upgrade::HeavyAir => i(
                "HEAVY AIR",
                "enemies move 10% slower",
                3,
                9,
                Icon::Lid,
                [180, 150, 255],
                Common,
            ),
            Upgrade::LongBreath => i(
                "LONG BREATH",
                "longer safety after a respawn",
                3,
                7,
                Icon::Heart,
                [255, 140, 170],
                Common,
            ),
            Upgrade::DustHoarder => i(
                "DUST HOARDER",
                "+25% dust when you wake",
                4,
                8,
                Icon::Magnet,
                [255, 210, 80],
                Common,
            ),
            Upgrade::ShardSense => i(
                "SHARD SENSE",
                "shards turn up more often",
                2,
                7,
                Icon::Shard,
                [120, 255, 240],
                Rare,
            ),
            Upgrade::DreamAnchor => i(
                "DREAM ANCHOR",
                "keep your shard the next time you're caught",
                3,
                6,
                Icon::Heart,
                [255, 200, 90],
                Rare,
            ),
            Upgrade::LuckyMemory => i(
                "LUCKY MEMORY",
                "remember more dreams as cards",
                3,
                7,
                Icon::Brain,
                [255, 170, 235],
                Rare,
            ),
            Upgrade::CalmMind => i(
                "CALM MIND",
                "the dream warps and dims less",
                2,
                5,
                Icon::Lid,
                [200, 220, 255],
                Rare,
            ),
            Upgrade::Quicken => i(
                "QUICKEN",
                "abilities recharge 20% faster",
                3,
                6,
                Icon::Gem,
                [255, 150, 90],
                Rare,
            ),
            Upgrade::Learn(a) => i(a.name(), a.desc(), 1, 5, Icon::Gem, [255, 120, 255], Rare),
            Upgrade::DoubleJump => i(
                "DOUBLE JUMP",
                "jump again in mid-air",
                1,
                6,
                Icon::Wind,
                [140, 255, 200],
                Mythic,
            ),
            Upgrade::LucidHeart => i(
                "LUCID HEART",
                "a lucidity shard, yours right now",
                2,
                5,
                Icon::Shard,
                [255, 255, 255],
                Mythic,
            ),
            Upgrade::WideChoice => i(
                "WIDE CHOICE",
                "one more card at every pick",
                1,
                4,
                Icon::Brain,
                [255, 230, 120],
                Mythic,
            ),
            Upgrade::Cursed(c) => {
                let (name, desc) = c.text();
                i(name, desc, 1, 1, Icon::Lid, [255, 70, 90], Cursed)
            }
        }
    }

    pub fn is_ability(self) -> bool {
        matches!(self, Upgrade::Learn(_))
    }

    pub fn tier(self) -> Tier {
        self.info().tier
    }
}

impl Curse {
    pub fn text(self) -> (&'static str, &'static str) {
        match self {
            Curse::GlassCannon => (
                "GLASS CANNON",
                "+30% speed · enemies notice you from further away",
            ),
            Curse::BloodMoon => ("BLOOD MOON", "+50% dust · enemies 15% faster"),
            Curse::Insomnia => ("INSOMNIA", "abilities recharge 40% faster · you see less"),
            Curse::Featherfall => (
                "FEATHERFALL",
                "jump 30% higher · half the safety after a respawn",
            ),
            Curse::Greed => ("GREED", "shards turn up far more · one fewer card per pick"),
        }
    }
}

/// Two upgrades that do more together.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Synergy {
    /// DASH + PHASE: every dash leaves you phased for a moment.
    GhostStep,
    /// WIDE EYES + SHARD SENSE: the shard shows through walls.
    ThirdEye,
    /// STILLNESS + HEAVY AIR: the freeze lasts twice as long.
    DeepFreeze,
    /// BLINK + DOUBLE JUMP: a blink gives your air jumps back.
    Skywalk,
    /// DASH + SWIFT FEET x2: dash recharges twice as fast.
    Slipstream,
    /// DREAM ANCHOR + LONG BREATH: respawn safety doubled again.
    SafeHarbour,
}

pub const ALL_SYNERGIES: [Synergy; 6] = [
    Synergy::GhostStep,
    Synergy::ThirdEye,
    Synergy::DeepFreeze,
    Synergy::Skywalk,
    Synergy::Slipstream,
    Synergy::SafeHarbour,
];

/// How long GHOST STEP keeps you phased after a dash.
pub const GHOST_STEP_SECONDS: f32 = 1.0;

impl Synergy {
    pub fn name(self) -> &'static str {
        match self {
            Synergy::GhostStep => "GHOST STEP",
            Synergy::ThirdEye => "THIRD EYE",
            Synergy::DeepFreeze => "DEEP FREEZE",
            Synergy::Skywalk => "SKYWALK",
            Synergy::Slipstream => "SLIPSTREAM",
            Synergy::SafeHarbour => "SAFE HARBOUR",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Synergy::GhostStep => "dashing leaves you phased",
            Synergy::ThirdEye => "the shard shows through walls",
            Synergy::DeepFreeze => "stillness lasts twice as long",
            Synergy::Skywalk => "blinking gives back your air jumps",
            Synergy::Slipstream => "dash recharges twice as fast",
            Synergy::SafeHarbour => "respawn safety doubled",
        }
    }

    /// The two halves: (upgrade, how many of it).
    pub fn parts(self) -> [(Upgrade, u32); 2] {
        use Upgrade::*;
        match self {
            Synergy::GhostStep => [(Learn(Ability::Dash), 1), (Learn(Ability::Phase), 1)],
            Synergy::ThirdEye => [(WideEyes, 1), (ShardSense, 1)],
            Synergy::DeepFreeze => [(Learn(Ability::Stillness), 1), (HeavyAir, 1)],
            Synergy::Skywalk => [(Learn(Ability::Blink), 1), (DoubleJump, 1)],
            Synergy::Slipstream => [(Learn(Ability::Dash), 1), (SwiftFeet, 2)],
            Synergy::SafeHarbour => [(DreamAnchor, 1), (LongBreath, 1)],
        }
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
    /// Rerolls spent this run.
    pub rerolls_used: u32,
    /// Ascension: cards taken away from every pick.
    pub penalty_cards: usize,
    /// Ascension: no free reroll.
    pub no_free_reroll: bool,
}

/// Free rerolls per run.
pub const REROLLS_PER_RUN: u32 = 1;
/// Dust for skipping a pick.
pub const SKIP_DUST: u32 = 5;

/// What a pick is being rolled for.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Roll {
    pub depth: u32,
    /// Reward for beating a nightmare: only rare and mythic cards, no curses.
    pub boss: bool,
}

impl RunUpgrades {
    pub fn count(&self, u: Upgrade) -> u32 {
        self.taken.iter().filter(|&&t| t == u).count() as u32
    }

    fn n(&self, u: Upgrade) -> i32 {
        self.count(u) as i32
    }

    fn cursed(&self, c: Curse) -> bool {
        self.count(Upgrade::Cursed(c)) > 0
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

    pub fn has(&self, s: Synergy) -> bool {
        s.parts().iter().all(|&(u, n)| self.count(u) >= n)
    }

    pub fn synergies(&self) -> Vec<Synergy> {
        ALL_SYNERGIES
            .iter()
            .copied()
            .filter(|&s| self.has(s))
            .collect()
    }

    pub fn rerolls_left(&self) -> u32 {
        if self.no_free_reroll {
            return 0;
        }
        REROLLS_PER_RUN.saturating_sub(self.rerolls_used)
    }

    /// Cards offered at each pick.
    pub fn choice_count(&self) -> usize {
        (3 + self.n(Upgrade::WideChoice)
            - self.cursed(Curse::Greed) as i32
            - self.penalty_cards as i32)
            .max(2) as usize
    }

    pub fn speed(&self) -> f32 {
        1.0 + 0.12 * self.n(Upgrade::SwiftFeet) as f32
            + if self.cursed(Curse::GlassCannon) {
                0.3
            } else {
                0.0
            }
    }
    pub fn jump(&self) -> f32 {
        1.0 + 0.12 * self.n(Upgrade::Spring) as f32
            + if self.cursed(Curse::Featherfall) {
                0.3
            } else {
                0.0
            }
    }
    /// Extra sight radius, world units.
    pub fn sight_bonus(&self) -> f32 {
        1.8 * self.n(Upgrade::WideEyes) as f32
            + if self.has(Synergy::ThirdEye) {
                1.8
            } else {
                0.0
            }
    }
    /// Multiplies the sight radius.
    pub fn sight(&self) -> f32 {
        if self.cursed(Curse::Insomnia) {
            0.75
        } else {
            1.0
        }
    }
    /// Multiplies how far away enemies notice you.
    pub fn alert(&self) -> f32 {
        if self.cursed(Curse::GlassCannon) {
            1.4
        } else {
            1.0
        }
    }
    pub fn enemy_speed(&self) -> f32 {
        0.9_f32.powi(self.n(Upgrade::HeavyAir))
            * if self.cursed(Curse::BloodMoon) {
                1.15
            } else {
                1.0
            }
    }
    pub fn grace(&self) -> f32 {
        (1.0 + 0.6 * self.n(Upgrade::LongBreath) as f32)
            * if self.has(Synergy::SafeHarbour) {
                2.0
            } else {
                1.0
            }
            * if self.cursed(Curse::Featherfall) {
                0.5
            } else {
                1.0
            }
    }
    pub fn shard_bonus(&self) -> f64 {
        0.12 * self.n(Upgrade::ShardSense) as f64
            + if self.cursed(Curse::Greed) { 0.25 } else { 0.0 }
    }
    pub fn anchors_left(&self) -> u32 {
        self.count(Upgrade::DreamAnchor)
            .saturating_sub(self.anchors_used)
    }
    pub fn dust(&self) -> f32 {
        1.0 + 0.25 * self.n(Upgrade::DustHoarder) as f32
            + if self.cursed(Curse::BloodMoon) {
                0.5
            } else {
                0.0
            }
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
    /// Cooldown multiplier for `ability`.
    pub fn cooldown_for(&self, ability: Ability) -> f32 {
        let mut m = 0.8_f32.powi(self.n(Upgrade::Quicken));
        if self.cursed(Curse::Insomnia) {
            m *= 0.6;
        }
        if ability == Ability::Dash && self.has(Synergy::Slipstream) {
            m *= 0.5;
        }
        m
    }
    /// Duration multiplier for `ability`.
    pub fn duration_for(&self, ability: Ability) -> f32 {
        if ability == Ability::Stillness && self.has(Synergy::DeepFreeze) {
            2.0
        } else {
            1.0
        }
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

/// The cards for one pick, weighted by rarity (rarer tiers grow with depth),
/// never one that's maxed out; sometimes one is swapped for a cursed card.
/// Deterministic per `seed`.
pub fn roll_choices(seed: u64, run: &RunUpgrades, roll: Roll) -> Vec<Upgrade> {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x0F_FE12);
    let mut pool: Vec<Upgrade> = PASSIVES
        .iter()
        .copied()
        .chain(ALL_ABILITIES.iter().map(|&a| Upgrade::Learn(a)))
        .filter(|&u| run.available(u))
        .filter(|&u| !roll.boss || u.tier() >= Tier::Rare)
        .collect();
    let weight = |u: &Upgrade| {
        let odds = if roll.boss {
            1.0
        } else {
            u.tier().odds(roll.depth)
        };
        ((u.info().weight as f32 * odds * 100.0) as u32).max(1)
    };
    let n = run.choice_count();
    let mut out = Vec::new();
    while out.len() < n && !pool.is_empty() {
        let total: u32 = pool.iter().map(weight).sum();
        let mut pick = rng.gen_range(0..total);
        let idx = pool
            .iter()
            .position(|u| {
                let w = weight(u);
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
    let curses: Vec<Curse> = ALL_CURSES
        .iter()
        .copied()
        .filter(|&c| run.available(Upgrade::Cursed(c)))
        .collect();
    if !roll.boss && !out.is_empty() && !curses.is_empty() && rng.gen_bool(CURSE_CHANCE) {
        let slot = rng.gen_range(0..out.len());
        out[slot] = Upgrade::Cursed(curses[rng.gen_range(0..curses.len())]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn roll(seed: u64, run: &RunUpgrades) -> Vec<Upgrade> {
        roll_choices(
            seed,
            run,
            Roll {
                depth: 5,
                boss: false,
            },
        )
    }

    #[test]
    fn choices_are_distinct_and_deterministic() {
        let run = RunUpgrades::default();
        for seed in 0..200 {
            let c = roll(seed, &run);
            assert_eq!(c.len(), 3);
            assert_eq!(c.iter().collect::<HashSet<_>>().len(), 3);
            assert_eq!(c, roll(seed, &run));
        }
        let variety: HashSet<Vec<Upgrade>> = (0..100).map(|s| roll(s, &run)).collect();
        assert!(variety.len() > 50);
    }

    #[test]
    fn every_upgrade_is_offered_sometimes() {
        let mut run = RunUpgrades::default();
        run.take(Upgrade::Learn(Ability::Dash)); // so QUICKEN is allowed
        let seen: HashSet<Upgrade> = (0..5000).flat_map(|s| roll(s, &run)).collect();
        for u in PASSIVES {
            assert!(seen.contains(&u), "{u:?} never offered");
        }
        for a in ALL_ABILITIES.iter().skip(1) {
            assert!(seen.contains(&Upgrade::Learn(*a)), "{a:?} never offered");
        }
        for c in ALL_CURSES {
            assert!(seen.contains(&Upgrade::Cursed(c)), "{c:?} never offered");
        }
        assert!(
            !seen.contains(&Upgrade::Learn(Ability::Dash)),
            "already held"
        );
    }

    #[test]
    fn rarer_tiers_show_up_more_the_deeper_you_go() {
        let run = RunUpgrades::default();
        let rare = |depth| {
            (0..2000)
                .flat_map(|s| roll_choices(s, &run, Roll { depth, boss: false }))
                .filter(|u| matches!(u.tier(), Tier::Rare | Tier::Mythic))
                .count()
        };
        assert!(rare(1) < rare(15));
    }

    #[test]
    fn nightmare_rewards_are_rare_or_better_and_never_cursed() {
        let run = RunUpgrades::default();
        for seed in 0..500 {
            for u in roll_choices(
                seed,
                &run,
                Roll {
                    depth: 5,
                    boss: true,
                },
            ) {
                assert!(matches!(u.tier(), Tier::Rare | Tier::Mythic), "{u:?}");
            }
        }
    }

    #[test]
    fn about_one_pick_in_five_is_cursed() {
        let run = RunUpgrades::default();
        let cursed = (0..2000)
            .filter(|&s| roll(s, &run).iter().any(|u| u.tier() == Tier::Cursed))
            .count();
        let rate = cursed as f64 / 2000.0;
        assert!((rate - CURSE_CHANCE).abs() < 0.04, "{rate}");
    }

    #[test]
    fn wide_choice_and_greed_change_the_hand_size() {
        let mut r = RunUpgrades::default();
        assert_eq!(r.choice_count(), 3);
        r.take(Upgrade::WideChoice);
        assert_eq!(r.choice_count(), 4);
        assert_eq!(roll(1, &r).len(), 4);
        r.take(Upgrade::Cursed(Curse::Greed));
        assert_eq!(r.choice_count(), 3);
        assert!(r.shard_bonus() > 0.2);
    }

    #[test]
    fn curses_cut_both_ways() {
        let mut r = RunUpgrades::default();
        r.take(Upgrade::Cursed(Curse::GlassCannon));
        assert!(r.speed() > 1.0 && r.alert() > 1.0);
        r.take(Upgrade::Cursed(Curse::BloodMoon));
        assert!(r.dust() > 1.0 && r.enemy_speed() > 1.0);
        r.take(Upgrade::Cursed(Curse::Insomnia));
        assert!(r.cooldown_for(Ability::Blink) < 1.0 && r.sight() < 1.0);
        r.take(Upgrade::Cursed(Curse::Featherfall));
        assert!(r.jump() > 1.0 && r.grace() < 1.0);
        for seed in 0..300 {
            let c = roll(seed, &r);
            assert!(
                !c.contains(&Upgrade::Cursed(Curse::GlassCannon)),
                "a curse is only taken once"
            );
        }
    }

    #[test]
    fn maxed_upgrades_are_never_offered() {
        let mut run = RunUpgrades::default();
        for _ in 0..Upgrade::SwiftFeet.info().max {
            run.take(Upgrade::SwiftFeet);
        }
        for seed in 0..500 {
            assert!(!roll(seed, &run).contains(&Upgrade::SwiftFeet));
            assert!(
                !roll(seed, &run).contains(&Upgrade::Quicken),
                "quicken with no abilities"
            );
        }
        let mut all = RunUpgrades::default();
        for u in PASSIVES {
            for _ in 0..u.info().max {
                all.take(u);
            }
        }
        for a in ALL_ABILITIES {
            all.take(Upgrade::Learn(a));
        }
        assert!(roll(1, &all).len() <= all.choice_count());
    }

    #[test]
    fn synergies_need_both_halves() {
        let mut r = RunUpgrades::default();
        assert!(r.synergies().is_empty());
        r.take(Upgrade::Learn(Ability::Dash));
        r.take(Upgrade::SwiftFeet);
        assert!(!r.has(Synergy::Slipstream), "needs SWIFT FEET x2");
        r.take(Upgrade::SwiftFeet);
        assert!(r.has(Synergy::Slipstream));
        assert!(r.cooldown_for(Ability::Dash) < r.cooldown_for(Ability::Blink));
        r.take(Upgrade::Learn(Ability::Phase));
        assert_eq!(r.synergies(), vec![Synergy::GhostStep, Synergy::Slipstream]);
        r.take(Upgrade::Learn(Ability::Stillness)); // replaces DASH
        assert!(!r.has(Synergy::GhostStep), "losing a half loses the combo");
        r.take(Upgrade::HeavyAir);
        assert_eq!(r.duration_for(Ability::Stillness), 2.0);
        let mut h = RunUpgrades::default();
        h.take(Upgrade::DreamAnchor);
        let before = h.grace();
        h.take(Upgrade::LongBreath);
        assert!(h.grace() > before * 2.0);
        for s in ALL_SYNERGIES {
            assert!(!s.name().is_empty() && !s.desc().is_empty());
        }
    }

    #[test]
    fn multipliers_stack_the_right_way() {
        let mut r = RunUpgrades::default();
        assert_eq!(
            (r.speed(), r.jump(), r.dust(), r.grace()),
            (1.0, 1.0, 1.0, 1.0)
        );
        assert_eq!(
            (r.enemy_speed(), r.calm(), r.sight(), r.alert()),
            (1.0, 1.0, 1.0, 1.0)
        );
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
    fn rerolls_run_out() {
        let mut r = RunUpgrades::default();
        assert_eq!(r.rerolls_left(), REROLLS_PER_RUN);
        r.rerolls_used = REROLLS_PER_RUN;
        assert_eq!(r.rerolls_left(), 0);
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
        assert!(a.try_use(1.0, 1.0));
        assert!(a.is_active());
        assert!(!a.try_use(1.0, 1.0), "still cooling down");
        a.tick(Ability::Stillness.duration() + 0.01);
        assert!(!a.is_active());
        assert!(a.charge(1.0) < 1.0);
        a.tick(Ability::Stillness.cooldown());
        assert!(a.try_use(0.5, 2.0));
        assert!((a.cooldown - Ability::Stillness.cooldown() * 0.5).abs() < 1e-5);
        assert!((a.active - Ability::Stillness.duration() * 2.0).abs() < 1e-5);
    }

    #[test]
    fn dash_is_brief_and_every_ability_recharges() {
        assert!(Ability::Dash.duration() < 0.5);
        for a in ALL_ABILITIES {
            assert!(
                a.cooldown() > a.duration() * 2.0,
                "{a:?} could be kept on forever"
            );
        }
    }
}
