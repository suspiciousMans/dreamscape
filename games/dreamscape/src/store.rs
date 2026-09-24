//! The Lucid Store: spend Dream Dust on run perks (armed for your next run,
//! used up when it starts) and cosmetics (bought once, equipped any time).
//! Pure data; lives inside the booklet save so it persists with your cards.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Perk {
    /// Start the run with one lucidity shard.
    FirstLight,
    /// Longer respawn grace, slower enemies.
    SlowHeart,
    /// Better odds of remembering each dream.
    DeepMemory,
    /// Enemies never notice you (no chasing).
    HeavyEyelids,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Item {
    Perk(Perk),
    Eye(EyeStyle),
    Card(CardStyle),
    Hud(HudPalette),
}

pub struct Info {
    pub name: &'static str,
    pub blurb: &'static str,
    pub price: u32,
}

impl Item {
    pub fn info(self) -> Info {
        use Item::*;
        let (name, blurb, price) = match self {
            Perk(self::Perk::FirstLight) => {
                ("FIRST LIGHT", "begin your next dream with 1 shard", 60)
            }
            Perk(self::Perk::SlowHeart) => {
                ("SLOW HEART", "longer grace, slower dreamers (1 run)", 40)
            }
            Perk(self::Perk::DeepMemory) => {
                ("DEEP MEMORY", "+25% chance to remember each dream", 50)
            }
            Perk(self::Perk::HeavyEyelids) => ("HEAVY EYELIDS", "nothing notices you (1 run)", 45),
            Eye(EyeStyle::Dream) => ("DREAMING EYE", "the eye you were born with", 0),
            Eye(EyeStyle::Ember) => ("EMBER EYE", "a coal that won't go out", 40),
            Eye(EyeStyle::Toxic) => ("TOXIC EYE", "glows like something spilled", 40),
            Eye(EyeStyle::Frost) => ("FROST EYE", "cold, clear, patient", 40),
            Eye(EyeStyle::Gold) => ("GILDED EYE", "it has seen the good ending", 90),
            Eye(EyeStyle::Void) => ("VOID EYE", "no iris. just a question", 70),
            Card(CardStyle::Plain) => ("PLAIN FRAMES", "cards as they surface", 0),
            Card(CardStyle::Gilt) => ("GILT FRAMES", "gold leaf on every card", 80),
            Card(CardStyle::Holo) => ("HOLO FRAMES", "every card catches the light", 150),
            Hud(HudPalette::Dream) => ("DREAM INK", "pale violet, red and cyan ghosts", 0),
            Hud(HudPalette::Amber) => ("AMBER CRT", "a terminal left on overnight", 60),
            Hud(HudPalette::Phosphor) => ("PHOSPHOR", "green, like old radar", 60),
            Hud(HudPalette::Vapor) => ("VAPOR", "a mall at 3 am", 60),
        };
        Info { name, blurb, price }
    }
}

/// Everything on the shelf, in display order.
pub const CATALOG: [Item; 17] = [
    Item::Perk(Perk::FirstLight),
    Item::Perk(Perk::SlowHeart),
    Item::Perk(Perk::DeepMemory),
    Item::Perk(Perk::HeavyEyelids),
    Item::Eye(EyeStyle::Dream),
    Item::Eye(EyeStyle::Ember),
    Item::Eye(EyeStyle::Toxic),
    Item::Eye(EyeStyle::Frost),
    Item::Eye(EyeStyle::Gold),
    Item::Eye(EyeStyle::Void),
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
    Armed,
    Owned,
    Equipped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Bought,
    Armed,
    Equipped,
    AlreadyArmed,
    AlreadyEquipped,
    TooPoor { need: u32 },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Stash {
    #[serde(default)]
    pub dust: u32,
    /// Bought cosmetics (defaults are implicitly owned).
    #[serde(default)]
    pub owned: Vec<Item>,
    /// Perks bought for the next run.
    #[serde(default)]
    pub armed: Vec<Perk>,
    #[serde(default)]
    pub eye: EyeStyle,
    #[serde(default)]
    pub card: CardStyle,
    #[serde(default)]
    pub hud: HudPalette,
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
            Item::Perk(_) => false,
        }
    }

    fn equip(&mut self, item: Item) {
        match item {
            Item::Eye(e) => self.eye = e,
            Item::Card(c) => self.card = c,
            Item::Hud(h) => self.hud = h,
            Item::Perk(_) => {}
        }
    }

    pub fn state(&self, item: Item) -> State {
        match item {
            Item::Perk(p) if self.armed.contains(&p) => State::Armed,
            Item::Perk(_) => State::Buy(item.info().price),
            _ if self.equipped(item) => State::Equipped,
            _ if self.owns(item) => State::Owned,
            _ => State::Buy(item.info().price),
        }
    }

    /// Buy, arm or equip, depending on what `item` is and whether you own it.
    pub fn select(&mut self, item: Item) -> Outcome {
        match self.state(item) {
            State::Armed => Outcome::AlreadyArmed,
            State::Equipped => Outcome::AlreadyEquipped,
            State::Owned => {
                self.equip(item);
                Outcome::Equipped
            }
            State::Buy(price) if price > self.dust => Outcome::TooPoor {
                need: price - self.dust,
            },
            State::Buy(price) => {
                self.dust -= price;
                match item {
                    Item::Perk(p) => {
                        self.armed.push(p);
                        Outcome::Armed
                    }
                    _ => {
                        self.owned.push(item);
                        self.equip(item);
                        Outcome::Bought
                    }
                }
            }
        }
    }

    /// Called when a run begins: the armed perks are used up.
    pub fn take_armed(&mut self) -> Vec<Perk> {
        std::mem::take(&mut self.armed)
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
                Item::Card(CardStyle::Plain),
                Item::Hud(HudPalette::Dream)
            ]
        );
        for i in CATALOG {
            let info = i.info();
            assert!(
                info.name.len() <= 16 && info.blurb.len() <= 40,
                "{}",
                info.name
            );
        }
    }

    #[test]
    fn defaults_start_equipped() {
        let s = Stash::default();
        assert_eq!(s.state(Item::Eye(EyeStyle::Dream)), State::Equipped);
        assert_eq!(s.state(Item::Hud(HudPalette::Dream)), State::Equipped);
        assert_eq!(s.state(Item::Eye(EyeStyle::Gold)), State::Buy(90));
    }

    #[test]
    fn buying_a_cosmetic_charges_once_then_equips_for_free() {
        let mut s = Stash {
            dust: 100,
            ..Default::default()
        };
        assert_eq!(s.select(Item::Eye(EyeStyle::Ember)), Outcome::Bought);
        assert_eq!((s.dust, s.eye), (60, EyeStyle::Ember));
        assert_eq!(s.select(Item::Eye(EyeStyle::Dream)), Outcome::Equipped);
        assert_eq!(s.select(Item::Eye(EyeStyle::Ember)), Outcome::Equipped);
        assert_eq!(s.dust, 60, "re-equipping is free");
        assert_eq!(
            s.select(Item::Eye(EyeStyle::Ember)),
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
            Outcome::TooPoor { need: 140 }
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
    fn perks_arm_once_and_are_used_up_by_the_next_run() {
        let mut s = Stash {
            dust: 200,
            ..Default::default()
        };
        assert_eq!(s.select(Item::Perk(Perk::FirstLight)), Outcome::Armed);
        assert_eq!(
            s.select(Item::Perk(Perk::FirstLight)),
            Outcome::AlreadyArmed
        );
        assert_eq!(s.select(Item::Perk(Perk::SlowHeart)), Outcome::Armed);
        assert_eq!(s.dust, 100);
        assert_eq!(s.take_armed(), vec![Perk::FirstLight, Perk::SlowHeart]);
        assert!(s.take_armed().is_empty());
        assert_eq!(s.state(Item::Perk(Perk::FirstLight)), State::Buy(60));
    }
}
