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
/// Dust in your pocket when a run starts (spendable at the moth's).
pub const POCKET_DUST: u32 = 8;
/// Percent off every price in the moth's room.
pub const HAGGLER_PCT: u32 = 25;
/// Extra cards on the moth's shelf.
pub const DEEP_SHELVES_STOCK: usize = 1;
/// Extra pick rerolls for the run.
pub const SPARE_DIE_REROLLS: u32 = 1;
/// Extra dust for skipping a pick.
pub const SHRUG_DUST: u32 = 5;
/// Nightmare bounty multiplier.
pub const BOUNTY_MULT: u32 = 2;

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
    /// Start the run with [`POCKET_DUST`] dust.
    PocketDust,
    /// The moth's prices are [`HAGGLER_PCT`]% lower.
    Haggler,
    /// The moth's shelf holds one more card.
    DeepShelves,
    /// One extra reroll of the after-dream pick.
    SpareDie,
    /// Skipping a pick pays [`SHRUG_DUST`] more dust.
    Shrug,
    /// A beaten nightmare pays double dust.
    Bounty,
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
    Sea,
    Blood,
    Rose,
    Storm,
    Sun,
    Moon,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardStyle {
    #[default]
    Plain,
    Gilt,
    Holo,
    Ember,
    Frost,
    Neon,
    Bone,
}

/// The back of a card in the deck stack and the hand (painted by the DECK
/// stream; the store only remembers the choice).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardBack {
    #[default]
    Dusk,
    Moth,
    Eye,
    Stars,
    Checker,
    Ember,
}

impl CardBack {
    /// (field, pattern) colours for a simple painted back.
    pub fn colors(self) -> ([u8; 3], [u8; 3]) {
        match self {
            CardBack::Dusk => ([30, 18, 56], [150, 130, 210]),
            CardBack::Moth => ([40, 20, 60], [255, 210, 80]),
            CardBack::Eye => ([10, 10, 30], [120, 230, 255]),
            CardBack::Stars => ([8, 6, 34], [255, 255, 200]),
            CardBack::Checker => ([60, 20, 40], [240, 200, 220]),
            CardBack::Ember => ([50, 14, 8], [255, 130, 40]),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HudPalette {
    #[default]
    Dream,
    Amber,
    Phosphor,
    Vapor,
    Ice,
    Rose,
    Blood,
    Mono,
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
    Sapphire,
    Topaz,
    Amethyst,
    Opal,
    Garnet,
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
            Crystal::Sapphire => [60, 110, 255, 255],
            Crystal::Topaz => [255, 190, 50, 255],
            Crystal::Amethyst => [170, 90, 230, 255],
            Crystal::Opal => [200, 240, 230, 255],
            Crystal::Garnet => [150, 20, 40, 255],
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
    Back(CardBack),
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
    Trinkets,
}

impl Shelf {
    pub const ALL: [Shelf; 6] = [
        Shelf::Perks,
        Shelf::Eyes,
        Shelf::Crystals,
        Shelf::Frames,
        Shelf::Palettes,
        Shelf::Trinkets,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Shelf::Perks => "RUN PERKS",
            Shelf::Eyes => "EYES",
            Shelf::Crystals => "CRYSTALS",
            Shelf::Frames => "CARD FRAMES",
            Shelf::Palettes => "HUD INK",
            Shelf::Trinkets => "CARD BACKS",
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
            Item::Back(_) => Shelf::Trinkets,
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
            Perk(self::Perk::PocketDust) => ("POCKET DUST", "start each run with 8 dust", 30),
            Perk(self::Perk::Haggler) => ("HAGGLER", "the moth's prices are 25% off", 40),
            Perk(self::Perk::DeepShelves) => ("DEEP SHELVES", "the moth stocks 1 more card", 35),
            Perk(self::Perk::SpareDie) => ("SPARE DIE", "one extra pick reroll", 30),
            Perk(self::Perk::Shrug) => ("SHRUG", "skipping a pick pays +5 dust", 25),
            Perk(self::Perk::Bounty) => ("BOUNTY", "nightmares pay double dust", 45),
            Eye(EyeStyle::Dream) => ("DREAMING EYE", "the eye you were born with", 0),
            Eye(EyeStyle::Ember) => ("EMBER EYE", "a coal that won't go out", 30),
            Eye(EyeStyle::Toxic) => ("TOXIC EYE", "glows like something spilled", 30),
            Eye(EyeStyle::Frost) => ("FROST EYE", "cold, clear, patient", 30),
            Eye(EyeStyle::Gold) => ("GILDED EYE", "it has seen the good ending", 80),
            Eye(EyeStyle::Void) => ("VOID EYE", "no iris. just a question", 60),
            Eye(EyeStyle::Sea) => ("SEA EYE", "a tide pool, looking back", 35),
            Eye(EyeStyle::Blood) => ("BLOOD EYE", "it has not slept in days", 40),
            Eye(EyeStyle::Rose) => ("ROSE EYE", "soft, and a little sorry", 35),
            Eye(EyeStyle::Storm) => ("STORM EYE", "lightning behind the lid", 55),
            Eye(EyeStyle::Sun) => ("SUN EYE", "too warm for a dream", 50),
            Eye(EyeStyle::Moon) => ("MOON EYE", "pale, round, borrowed", 45),
            Crystal(self::Crystal::Pearl) => ("PEARL", "the shape you fell asleep in", 0),
            Crystal(self::Crystal::Ruby) => ("RUBY", "a heartbeat you can see", 35),
            Crystal(self::Crystal::Jade) => ("JADE", "calm, green, lucky", 35),
            Crystal(self::Crystal::Obsidian) => ("OBSIDIAN", "a hole shaped like you", 55),
            Crystal(self::Crystal::Prism) => ("PRISM", "every colour at once", 90),
            Crystal(self::Crystal::Sapphire) => ("SAPPHIRE", "deep water, held still", 40),
            Crystal(self::Crystal::Topaz) => ("TOPAZ", "a streetlamp in a stone", 40),
            Crystal(self::Crystal::Amethyst) => ("AMETHYST", "quiet, violet, sure of itself", 45),
            Crystal(self::Crystal::Opal) => ("OPAL", "milk and a little fire", 60),
            Crystal(self::Crystal::Garnet) => ("GARNET", "old blood, well kept", 50),
            Card(CardStyle::Plain) => ("PLAIN FRAMES", "cards as they surface", 0),
            Card(CardStyle::Gilt) => ("GILT FRAMES", "gold leaf on every card", 60),
            Card(CardStyle::Holo) => ("HOLO FRAMES", "every card catches the light", 120),
            Card(CardStyle::Ember) => ("EMBER FRAMES", "edges that smoulder", 50),
            Card(CardStyle::Frost) => ("FROST FRAMES", "rimed, cold to hold", 50),
            Card(CardStyle::Neon) => ("NEON FRAMES", "tubes humming round each card", 75),
            Card(CardStyle::Bone) => ("BONE FRAMES", "carved, yellowed, patient", 55),
            Hud(HudPalette::Dream) => ("DREAM INK", "pale violet, red and cyan ghosts", 0),
            Hud(HudPalette::Amber) => ("AMBER CRT", "a terminal left on overnight", 45),
            Hud(HudPalette::Phosphor) => ("PHOSPHOR", "green, like old radar", 45),
            Hud(HudPalette::Vapor) => ("VAPOR", "a mall at 3 am", 45),
            Hud(HudPalette::Ice) => ("ICE INK", "breath on a cold window", 40),
            Hud(HudPalette::Rose) => ("ROSE INK", "a letter never sent", 40),
            Hud(HudPalette::Blood) => ("BLOOD INK", "red text, no apology", 50),
            Hud(HudPalette::Mono) => ("MONO INK", "a photograph of a screen", 35),
            Back(CardBack::Dusk) => ("DUSK BACK", "the back you were dealt", 0),
            Back(CardBack::Moth) => ("MOTH BACK", "one wing on every card", 25),
            Back(CardBack::Eye) => ("EYE BACK", "the deck watches you draw", 35),
            Back(CardBack::Stars) => ("STAR BACK", "a sky nobody named", 30),
            Back(CardBack::Checker) => ("CHECKER BACK", "a floor you know from somewhere", 30),
            Back(CardBack::Ember) => ("EMBER BACK", "warm to the touch", 45),
        };
        Info { name, blurb, price }
    }
}

/// Everything on the shelves, in display order.
pub const CATALOG: [Item; 56] = [
    Item::Perk(Perk::FirstLight),
    Item::Perk(Perk::SlowHeart),
    Item::Perk(Perk::DeepMemory),
    Item::Perk(Perk::HeavyEyelids),
    Item::Perk(Perk::SecondWind),
    Item::Perk(Perk::DustMagnet),
    Item::Perk(Perk::LongStride),
    Item::Perk(Perk::PocketDust),
    Item::Perk(Perk::Haggler),
    Item::Perk(Perk::DeepShelves),
    Item::Perk(Perk::SpareDie),
    Item::Perk(Perk::Shrug),
    Item::Perk(Perk::Bounty),
    Item::Eye(EyeStyle::Dream),
    Item::Eye(EyeStyle::Ember),
    Item::Eye(EyeStyle::Toxic),
    Item::Eye(EyeStyle::Frost),
    Item::Eye(EyeStyle::Gold),
    Item::Eye(EyeStyle::Void),
    Item::Eye(EyeStyle::Sea),
    Item::Eye(EyeStyle::Blood),
    Item::Eye(EyeStyle::Rose),
    Item::Eye(EyeStyle::Storm),
    Item::Eye(EyeStyle::Sun),
    Item::Eye(EyeStyle::Moon),
    Item::Crystal(Crystal::Pearl),
    Item::Crystal(Crystal::Ruby),
    Item::Crystal(Crystal::Jade),
    Item::Crystal(Crystal::Obsidian),
    Item::Crystal(Crystal::Prism),
    Item::Crystal(Crystal::Sapphire),
    Item::Crystal(Crystal::Topaz),
    Item::Crystal(Crystal::Amethyst),
    Item::Crystal(Crystal::Opal),
    Item::Crystal(Crystal::Garnet),
    Item::Card(CardStyle::Plain),
    Item::Card(CardStyle::Gilt),
    Item::Card(CardStyle::Holo),
    Item::Card(CardStyle::Ember),
    Item::Card(CardStyle::Frost),
    Item::Card(CardStyle::Neon),
    Item::Card(CardStyle::Bone),
    Item::Hud(HudPalette::Dream),
    Item::Hud(HudPalette::Amber),
    Item::Hud(HudPalette::Phosphor),
    Item::Hud(HudPalette::Vapor),
    Item::Hud(HudPalette::Ice),
    Item::Hud(HudPalette::Rose),
    Item::Hud(HudPalette::Blood),
    Item::Hud(HudPalette::Mono),
    Item::Back(CardBack::Dusk),
    Item::Back(CardBack::Moth),
    Item::Back(CardBack::Eye),
    Item::Back(CardBack::Stars),
    Item::Back(CardBack::Checker),
    Item::Back(CardBack::Ember),
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
    /// The chosen card back (old saves: the default).
    #[serde(default)]
    pub card_back: CardBack,
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
            Item::Back(b) => self.card_back == b,
            Item::Perk(_) => false,
        }
    }

    fn equip(&mut self, item: Item) {
        match item {
            Item::Eye(e) => self.eye = e,
            Item::Card(c) => self.card = c,
            Item::Hud(h) => self.hud = h,
            Item::Crystal(c) => self.crystal = c,
            Item::Back(b) => self.card_back = b,
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

/// What the run's active perks change besides the old hard-wired ones, in one
/// place so `main.rs` only reads fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerkFx {
    /// Dust to start the run with.
    pub start_dust: u32,
    /// Percent off the moth's prices.
    pub moth_pct_off: u32,
    /// Cards added to the moth's shelf.
    pub shelf_bonus: usize,
    /// Extra pick rerolls.
    pub rerolls: u32,
    /// Extra dust when a pick is skipped.
    pub skip_bonus: u32,
    /// Nightmare dust multiplier (1 = none).
    pub nightmare_mult: u32,
}

impl PerkFx {
    pub fn of(perks: &[Perk]) -> PerkFx {
        let has = |p| perks.contains(&p);
        PerkFx {
            start_dust: if has(Perk::PocketDust) { POCKET_DUST } else { 0 },
            moth_pct_off: if has(Perk::Haggler) { HAGGLER_PCT } else { 0 },
            shelf_bonus: if has(Perk::DeepShelves) { DEEP_SHELVES_STOCK } else { 0 },
            rerolls: if has(Perk::SpareDie) { SPARE_DIE_REROLLS } else { 0 },
            skip_bonus: if has(Perk::Shrug) { SHRUG_DUST } else { 0 },
            nightmare_mult: if has(Perk::Bounty) { BOUNTY_MULT } else { 1 },
        }
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
                Item::Hud(HudPalette::Dream),
                Item::Back(CardBack::Dusk)
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
    fn the_catalog_is_big_and_priced_against_a_run() {
        let n = |s: Shelf| s.items().len();
        assert!(n(Shelf::Perks) >= 13 && n(Shelf::Eyes) >= 12 && n(Shelf::Crystals) >= 10);
        assert!(n(Shelf::Frames) >= 7 && n(Shelf::Palettes) >= 8 && n(Shelf::Trinkets) >= 6);
        let typical = run_dust(6, 3, 4, false);
        for i in CATALOG {
            let p = i.info().price;
            assert!(p <= 120, "{:?} costs {p}", i);
            if matches!(i, Item::Perk(_)) {
                assert!(p <= typical, "{:?} is out of reach of one run", i);
            }
        }
    }

    #[test]
    fn each_new_perk_has_a_real_strength() {
        let none = PerkFx::of(&[]);
        assert_eq!(
            none,
            PerkFx {
                start_dust: 0,
                moth_pct_off: 0,
                shelf_bonus: 0,
                rerolls: 0,
                skip_bonus: 0,
                nightmare_mult: 1
            }
        );
        assert_eq!(PerkFx::of(&[Perk::PocketDust]).start_dust, POCKET_DUST);
        assert_eq!(PerkFx::of(&[Perk::Haggler]).moth_pct_off, HAGGLER_PCT);
        assert_eq!(PerkFx::of(&[Perk::DeepShelves]).shelf_bonus, DEEP_SHELVES_STOCK);
        assert_eq!(PerkFx::of(&[Perk::SpareDie]).rerolls, SPARE_DIE_REROLLS);
        assert_eq!(PerkFx::of(&[Perk::Shrug]).skip_bonus, SHRUG_DUST);
        assert_eq!(PerkFx::of(&[Perk::Bounty]).nightmare_mult, BOUNTY_MULT);
        // The old perks change none of these.
        assert_eq!(PerkFx::of(&[Perk::FirstLight, Perk::DustMagnet]), none);
    }

    #[test]
    fn card_backs_are_bought_once_and_equipped() {
        let mut s = Stash {
            dust: 60,
            ..Default::default()
        };
        assert_eq!(s.state(Item::Back(CardBack::Dusk)), State::Equipped);
        assert_eq!(s.select(Item::Back(CardBack::Moth)), Outcome::Bought);
        assert_eq!((s.dust, s.card_back), (35, CardBack::Moth));
        assert_eq!(s.select(Item::Back(CardBack::Dusk)), Outcome::Equipped);
        assert_eq!(s.select(Item::Back(CardBack::Moth)), Outcome::Equipped);
        assert_eq!(s.dust, 35);
        assert_eq!(Shelf::Trinkets.items().len(), 6);
    }

    #[test]
    fn an_old_booklet_stash_loses_nothing() {
        // Written before the Trinkets shelf, the new perks and cosmetics.
        let old = "(dust: 77, owned: [Eye(Gold), Crystal(Ruby), Card(Holo), Hud(Vapor)], \
                   charges: [(FirstLight, 2), (LongStride, 9)], eye: Gold, card: Holo, \
                   hud: Vapor, crystal: Ruby, dust_earned: 300)";
        let mut s: Stash = ron::from_str(old).unwrap();
        s.migrate();
        assert_eq!((s.dust, s.dust_earned), (77, 300));
        assert_eq!(s.card_back, CardBack::Dusk);
        assert_eq!(s.runs_left(Perk::FirstLight), 2);
        assert_eq!(s.runs_left(Perk::LongStride), 9);
        assert_eq!(s.state(Item::Eye(EyeStyle::Gold)), State::Equipped);
        assert_eq!(s.state(Item::Card(CardStyle::Holo)), State::Equipped);
        assert_eq!(s.state(Item::Hud(HudPalette::Vapor)), State::Equipped);
        assert_eq!(s.state(Item::Crystal(Crystal::Ruby)), State::Equipped);
        // Everything bought before is still owned, nothing new is free.
        s.eye = EyeStyle::Dream;
        assert_eq!(s.state(Item::Eye(EyeStyle::Gold)), State::Owned);
        assert!(matches!(s.state(Item::Eye(EyeStyle::Sea)), State::Buy(_)));
        assert!(matches!(s.state(Item::Back(CardBack::Moth)), State::Buy(_)));
        // And it round-trips with the new field.
        let again: Stash = ron::from_str(&ron::to_string(&s).unwrap()).unwrap();
        assert_eq!(again, s);
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
