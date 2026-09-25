//! The Lucid Store: spend Dream Dust on run perks (each purchase is a bundle
//! of charges; one charge is used per run) and cosmetics (bought once,
//! equipped any time). Pure data; lives inside the booklet save so it
//! persists with your cards.

use serde::{Deserialize, Serialize};

/// Runs of effect you get per perk purchase.
pub const PERK_RUNS: u32 = 3;
/// You can stock up to this many runs of any one perk.
pub const MAX_CHARGES: u32 = 9;

/// Perk strengths, in one place so balance is one edit.
pub const SLOW_HEART_ENEMY_SPEED: f32 = 0.75;
pub const SLOW_HEART_GRACE: f32 = 2.5;
pub const DEEP_MEMORY_BONUS: f32 = 0.30;
pub const DUST_MAGNET: f32 = 1.5;
pub const LONG_STRIDE: f32 = 1.15;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Perk {
    /// Start the run with one lucidity shard.
    FirstLight,
    /// Much longer respawn grace, slower enemies.
    SlowHeart,
    /// Better odds of remembering each dream.
    DeepMemory,
    /// Enemies never notice you (no chasing).
    HeavyEyelids,
    /// The first time you are caught each dream, you keep your shard.
    SecondWind,
    /// +50% dust from the run.
    DustMagnet,
    /// Move 15% faster.
    LongStride,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EyeStyle {
    #[default]
    Dream,
    Ember,
    Toxic,
    Frost,
    Gold,
    Void,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardStyle {
    #[default]
    Plain,
    Gilt,
    Holo,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HudPalette {
    #[default]
    Dream,
    Amber,
    Phosphor,
    Vapor,
}

/// The dreamer's own crystal colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Crystal {
    #[default]
    Pearl,
    Ruby,
    Jade,
    Obsidian,
    Prism,
}

impl Crystal {
    /// RGBA for the 1x1 player texture. Prism is animated by the caller.
    pub fn rgba(self) -> [u8; 4] {
        match self {
            Crystal::Pearl => [255, 255, 255, 255],
            Crystal::Ruby => [235, 40, 70, 255],
            Crystal::Jade => [60, 220, 140, 255],
            Crystal::Obsidian => [40, 25, 60, 255],
            Crystal::Prism => [255, 255, 255, 255],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Item {
    Perk(Perk),
    Eye(EyeStyle),
    Card(CardStyle),
    Hud(HudPalette),
    Crystal(Crystal),
}

pub struct Info {
    pub name: &'static str,
    pub blurb: &'static str,
    pub price: u32,
}

/// Shelves, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shelf {
    Perks,
    Eyes,
    Crystals,
    Frames,
    Palettes,
}

impl Shelf {
    pub const ALL: [Shelf; 5] = [
        Shelf::Perks,
        Shelf::Eyes,
        Shelf::Crystals,
        Shelf::Frames,
        Shelf::Palettes,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Shelf::Perks => "RUN PERKS",
            Shelf::Eyes => "EYES",
            Shelf::Crystals => "CRYSTALS",
            Shelf::Frames => "CARD FRAMES",
            Shelf::Palettes => "HUD INK",
        }
    }

    pub fn items(self) -> Vec<Item> {
        CATALOG
            .iter()
            .copied()
            .filter(|i| i.shelf() == self)
            .collect()
    }
}

impl Item {
    pub fn shelf(self) -> Shelf {
        match self {
            Item::Perk(_) => Shelf::Perks,
            Item::Eye(_) => Shelf::Eyes,
            Item::Crystal(_) => Shelf::Crystals,
            Item::Card(_) => Shelf::Frames,
            Item::Hud(_) => Shelf::Palettes,
        }
    }

    pub fn info(self) -> Info {
        use Item::*;
        let (name, blurb, price) = match self {
            Perk(self::Perk::FirstLight) => {
                ("FIRST LIGHT", "begin each dream run with 1 shard", 45)
            }
            Perk(self::Perk::SlowHeart) => ("SLOW HEART", "long grace, dreamers crawl", 30),
            Perk(self::Perk::DeepMemory) => ("DEEP MEMORY", "+30% chance to remember dreams", 40),
            Perk(self::Perk::HeavyEyelids) => ("HEAVY EYELIDS", "nothing notices you", 35),
            Perk(self::Perk::SecondWind) => ("SECOND WIND", "first catch each dream is free", 40),
            Perk(self::Perk::DustMagnet) => ("DUST MAGNET", "+50% dream dust", 50),
            Perk(self::Perk::LongStride) => ("LONG STRIDE", "move 15% faster", 35),
            Eye(EyeStyle::Dream) => ("DREAMING EYE", "the eye you were born with", 0),
            Eye(EyeStyle::Ember) => ("EMBER EYE", "a coal that won't go out", 30),
            Eye(EyeStyle::Toxic) => ("TOXIC EYE", "glows like something spilled", 30),
            Eye(EyeStyle::Frost) => ("FROST EYE", "cold, clear, patient", 30),
            Eye(EyeStyle::Gold) => ("GILDED EYE", "it has seen the good ending", 80),
            Eye(EyeStyle::Void) => ("VOID EYE", "no iris. just a question", 60),
            Crystal(self::Crystal::Pearl) => ("PEARL", "the shape you fell asleep in", 0),
            Crystal(self::Crystal::Ruby) => ("RUBY", "a heartbeat you can see", 35),
            Crystal(self::Crystal::Jade) => ("JADE", "calm, green, lucky", 35),
            Crystal(self::Crystal::Obsidian) => ("OBSIDIAN", "a hole shaped like you", 55),
            Crystal(self::Crystal::Prism) => ("PRISM", "every colour at once", 90),
            Card(CardStyle::Plain) => ("PLAIN FRAMES", "cards as they surface", 0),
            Card(CardStyle::Gilt) => ("GILT FRAMES", "gold leaf on every card", 60),
            Card(CardStyle::Holo) => ("HOLO FRAMES", "every card catches the light", 120),
            Hud(HudPalette::Dream) => ("DREAM INK", "pale violet, red and cyan ghosts", 0),
            Hud(HudPalette::Amber) => ("AMBER CRT", "a terminal left on overnight", 45),
            Hud(HudPalette::Phosphor) => ("PHOSPHOR", "green, like old radar", 45),
            Hud(HudPalette::Vapor) => ("VAPOR", "a mall at 3 am", 45),
        };
        Info { name, blurb, price }
    }
}

/// Everything on the shelves, in display order.
pub const CATALOG: [Item; 25] = [
    Item::Perk(Perk::FirstLight),
    Item::Perk(Perk::SlowHeart),
    Item::Perk(Perk::DeepMemory),
    Item::Perk(Perk::HeavyEyelids),
    Item::Perk(Perk::SecondWind),
    Item::Perk(Perk::DustMagnet),
    Item::Perk(Perk::LongStride),
    Item::Eye(EyeStyle::Dream),
    Item::Eye(EyeStyle::Ember),
    Item::Eye(EyeStyle::Toxic),
    Item::Eye(EyeStyle::Frost),
    Item::Eye(EyeStyle::Gold),
    Item::Eye(EyeStyle::Void),
    Item::Crystal(Crystal::Pearl),
    Item::Crystal(Crystal::Ruby),
    Item::Crystal(Crystal::Jade),
    Item::Crystal(Crystal::Obsidian),
    Item::Crystal(Crystal::Prism),
    Item::Card(CardStyle::Plain),
    Item::Card(CardStyle::Gilt),
    Item::Card(CardStyle::Holo),
    Item::Hud(HudPalette::Dream),
    Item::Hud(HudPalette::Amber),
    Item::Hud(HudPalette::Phosphor),
    Item::Hud(HudPalette::Vapor),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Buy(u32),
    /// A perk with `runs` charges left; `price` buys PERK_RUNS more.
    Stocked {
        runs: u32,
        price: u32,
    },
    Owned,
    Equipped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Bought,
    Stocked { runs: u32 },
    Equipped,
    AlreadyEquipped,
    Full,
    TooPoor { need: u32 },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Stash {
    #[serde(default)]
    pub dust: u32,
    /// Bought cosmetics (defaults are implicitly owned).
    #[serde(default)]
    pub owned: Vec<Item>,
    /// Old saves: perks bought for exactly one run. Migrated into `charges`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub armed: Vec<Perk>,
    /// Perk -> runs left.
    #[serde(default)]
    pub charges: Vec<(Perk, u32)>,
    #[serde(default)]
    pub eye: EyeStyle,
    #[serde(default)]
    pub card: CardStyle,
    #[serde(default)]
    pub hud: HudPalette,
    #[serde(default)]
    pub crystal: Crystal,
    /// Lifetime dust earned (shown in the shop, and a little brag).
    #[serde(default)]
    pub dust_earned: u32,
}

impl Stash {
    fn owns(&self, item: Item) -> bool {
        item.info().price == 0 || self.owned.contains(&item)
    }

    fn equipped(&self, item: Item) -> bool {
        match item {
            Item::Eye(e) => self.eye == e,
            Item::Card(c) => self.card == c,
            Item::Hud(h) => self.hud == h,
            Item::Crystal(c) => self.crystal == c,
            Item::Perk(_) => false,
        }
    }

    fn equip(&mut self, item: Item) {
        match item {
            Item::Eye(e) => self.eye = e,
            Item::Card(c) => self.card = c,
            Item::Hud(h) => self.hud = h,
            Item::Crystal(c) => self.crystal = c,
            Item::Perk(_) => {}
        }
    }

    /// Folds old one-run `armed` perks into `charges`.
    pub fn migrate(&mut self) {
        for p in std::mem::take(&mut self.armed) {
            self.add_charges(p, 1);
        }
    }

    pub fn runs_left(&self, p: Perk) -> u32 {
        self.charges
            .iter()
            .find(|(q, _)| *q == p)
            .map_or(0, |&(_, n)| n)
    }

    fn add_charges(&mut self, p: Perk, n: u32) {
        match self.charges.iter_mut().find(|(q, _)| *q == p) {
            Some((_, have)) => *have = (*have + n).min(MAX_CHARGES),
            None => self.charges.push((p, n.min(MAX_CHARGES))),
        }
    }

    pub fn earn(&mut self, dust: u32) {
        self.dust += dust;
        self.dust_earned += dust;
    }

    pub fn state(&self, item: Item) -> State {
        let price = item.info().price;
        match item {
            Item::Perk(p) => match self.runs_left(p) {
                0 => State::Buy(price),
                runs => State::Stocked { runs, price },
            },
            _ if self.equipped(item) => State::Equipped,
            _ if self.owns(item) => State::Owned,
            _ => State::Buy(price),
        }
    }

    /// Buy (or top up), or equip, depending on what `item` is.
    pub fn select(&mut self, item: Item) -> Outcome {
        let price = match self.state(item) {
            State::Equipped => return Outcome::AlreadyEquipped,
            State::Owned => {
                self.equip(item);
                return Outcome::Equipped;
            }
            State::Stocked { runs, .. } if runs + PERK_RUNS > MAX_CHARGES => return Outcome::Full,
            State::Stocked { price, .. } | State::Buy(price) => price,
        };
        if price > self.dust {
            return Outcome::TooPoor {
                need: price - self.dust,
            };
        }
        self.dust -= price;
        match item {
            Item::Perk(p) => {
                self.add_charges(p, PERK_RUNS);
                Outcome::Stocked {
                    runs: self.runs_left(p),
                }
            }
            _ => {
                self.owned.push(item);
                self.equip(item);
                Outcome::Bought
            }
        }
    }

    /// Called when a run begins: every stocked perk spends one charge and is
    /// active for that run. Empty perks drop off the list.
    pub fn take_for_run(&mut self) -> Vec<Perk> {
        self.migrate();
        let active: Vec<Perk> = self
            .charges
            .iter()
            .filter(|(_, n)| *n > 0)
            .map(|&(p, _)| p)
            .collect();
        for (_, n) in self.charges.iter_mut() {
            *n = n.saturating_sub(1);
        }
        self.charges.retain(|(_, n)| *n > 0);
        active
    }
}

/// Dust for a finished run. `deepest` depth, `shards` collected, `kept` cards
/// remembered, `new_names` cards whose name wasn't already in the booklet.
pub fn run_dust(deepest: u32, shards: u32, new_names: u32, magnet: bool) -> u32 {
    let base = 3 * deepest.min(50) + 6 * shards.min(50) + 4 * new_names.min(50);
    // Reaching depth 5 and 10 pays a milestone bonus.
    let milestone = match deepest {
        d if d >= 10 => 40,
        d if d >= 5 => 15,
        _ => 0,
    };
    let total = base + milestone;
    if magnet {
        (total as f32 * DUST_MAGNET).round() as u32
    } else {
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_is_unique_and_only_defaults_are_free() {
        let set: HashSet<Item> = CATALOG.iter().copied().collect();
        assert_eq!(set.len(), CATALOG.len());
        let free: Vec<Item> = CATALOG
            .iter()
            .copied()
            .filter(|i| i.info().price == 0)
            .collect();
        assert_eq!(
            free,
            vec![
                Item::Eye(EyeStyle::Dream),
                Item::Crystal(Crystal::Pearl),
                Item::Card(CardStyle::Plain),
                Item::Hud(HudPalette::Dream)
            ]
        );
        for i in CATALOG {
            let info = i.info();
            assert!(
                info.name.len() <= 16 && info.blurb.len() <= 34,
                "{} / {}",
                info.name,
                info.blurb
            );
        }
    }

    #[test]
    fn every_item_sits_on_exactly_one_shelf_in_catalog_order() {
        let flat: Vec<Item> = Shelf::ALL.iter().flat_map(|s| s.items()).collect();
        assert_eq!(flat, CATALOG.to_vec());
        assert!(Shelf::ALL.iter().all(|s| !s.items().is_empty()));
    }

    #[test]
    fn defaults_start_equipped() {
        let s = Stash::default();
        assert_eq!(s.state(Item::Eye(EyeStyle::Dream)), State::Equipped);
        assert_eq!(s.state(Item::Crystal(Crystal::Pearl)), State::Equipped);
        assert_eq!(s.state(Item::Eye(EyeStyle::Gold)), State::Buy(80));
    }

    #[test]
    fn buying_a_cosmetic_charges_once_then_equips_for_free() {
        let mut s = Stash {
            dust: 100,
            ..Default::default()
        };
        assert_eq!(s.select(Item::Crystal(Crystal::Ruby)), Outcome::Bought);
        assert_eq!((s.dust, s.crystal), (65, Crystal::Ruby));
        assert_eq!(s.select(Item::Crystal(Crystal::Pearl)), Outcome::Equipped);
        assert_eq!(s.select(Item::Crystal(Crystal::Ruby)), Outcome::Equipped);
        assert_eq!(s.dust, 65, "re-equipping is free");
        assert_eq!(
            s.select(Item::Crystal(Crystal::Ruby)),
            Outcome::AlreadyEquipped
        );
    }

    #[test]
    fn too_poor_changes_nothing() {
        let mut s = Stash {
            dust: 10,
            ..Default::default()
        };
        assert_eq!(
            s.select(Item::Card(CardStyle::Holo)),
            Outcome::TooPoor { need: 110 }
        );
        assert_eq!(
            s,
            Stash {
                dust: 10,
                ..Default::default()
            }
        );
    }

    #[test]
    fn a_perk_lasts_several_runs_and_can_be_topped_up_to_a_cap() {
        let mut s = Stash {
            dust: 1000,
            ..Default::default()
        };
        let fl = Item::Perk(Perk::FirstLight);
        assert_eq!(s.select(fl), Outcome::Stocked { runs: PERK_RUNS });
        assert_eq!(
            s.select(fl),
            Outcome::Stocked {
                runs: 2 * PERK_RUNS
            }
        );
        assert_eq!(
            s.select(fl),
            Outcome::Stocked {
                runs: 3 * PERK_RUNS
            }
        );
        let before = s.dust;
        assert_eq!(s.select(fl), Outcome::Full);
        assert_eq!(s.dust, before, "a full perk costs nothing");
        for run in 0..MAX_CHARGES {
            assert_eq!(s.take_for_run(), vec![Perk::FirstLight], "run {run}");
        }
        assert!(s.take_for_run().is_empty());
        assert_eq!(s.state(fl), State::Buy(45));
    }

    #[test]
    fn old_saves_with_armed_perks_become_one_charge() {
        let mut s = Stash {
            armed: vec![Perk::SlowHeart],
            ..Default::default()
        };
        assert_eq!(s.take_for_run(), vec![Perk::SlowHeart]);
        assert!(s.take_for_run().is_empty());
        let old = "(dust: 5, armed: [FirstLight])";
        let mut s: Stash = ron::from_str(old).unwrap();
        s.migrate();
        assert_eq!(s.runs_left(Perk::FirstLight), 1);
        let saved = ron::to_string(&s).unwrap();
        assert!(!saved.contains("armed"), "{saved}");
    }

    #[test]
    fn earning_tracks_lifetime_dust() {
        let mut s = Stash::default();
        s.earn(30);
        s.dust -= 10;
        s.earn(5);
        assert_eq!((s.dust, s.dust_earned), (25, 35));
    }

    #[test]
    fn run_dust_rewards_depth_shards_new_cards_and_milestones() {
        assert_eq!(run_dust(0, 0, 0, false), 0);
        assert!(run_dust(4, 3, 2, false) > run_dust(4, 3, 0, false));
        assert_eq!(run_dust(5, 0, 0, false) - run_dust(4, 0, 0, false), 3 + 15);
        assert_eq!(run_dust(10, 0, 0, false), 30 + 40);
        assert_eq!(run_dust(10, 0, 0, true), 105);
        // A typical 6-deep, 3-shard run with a few new cards buys a perk.
        let typical = run_dust(6, 3, 4, false);
        assert!(
            typical >= Item::Perk(Perk::FirstLight).info().price,
            "{typical}"
        );
    }
}
