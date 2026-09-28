//! What a card *does*: every dream's attribute teaches one active ability
//! and one passive upgrade, tinted with that dream's first accent. A run's
//! loadout (`RunUpgrades::from_loadout`) is built from these.
//! Table approved in `plans/2026-09-28-dreamscape-rest-of-project-plan.md`.

use crate::cards::Attribute;
use crate::dream::DreamTheme;
use crate::upgrades::{Ability, Upgrade};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Power {
    pub active: Ability,
    pub passive: Upgrade,
    pub tint: [u8; 3],
    pub flavour: &'static str,
}

pub fn power(theme: DreamTheme) -> Power {
    use Ability::*;
    use Upgrade::*;
    let (active, passive, flavour) = match Attribute::of(theme) {
        Attribute::Velvet => (Stillness, CalmMind, "the carpet hushes everything"),
        Attribute::Static => (Phase, LongBreath, "you're between channels"),
        Attribute::Void => (Blink, Spring, "the gap is shorter than it looks"),
        Attribute::Bloom => (ShardCall, LuckyMemory, "things grow toward you"),
        Attribute::Rust => (Flare, HeavyAir, "sparks off the old machines"),
        Attribute::Dawn => (Flare, WideEyes, "the light comes early"),
        Attribute::Hex => (Decoy, DreamAnchor, "the trees argue about which one is you"),
        Attribute::Tide => (Float, LongBreath, "hold your breath and drift"),
        Attribute::Aether => (Float, DoubleJump, "one more step than there should be"),
        Attribute::Glass => (Decoy, WideEyes, "your reflection runs the other way"),
        Attribute::Spore => (ShardCall, ShardSense, "the roots know where it is"),
        Attribute::Abyss => (Dash, SwiftFeet, "the far end pulls you in"),
        Attribute::Halo => (Stillness, Quicken, "every room waits for the next"),
        Attribute::Jest => (Blink, DustHoarder, "a small hand moves you along"),
        Attribute::Trace => (Rewind, SwiftFeet, "you left a copy back there"),
        Attribute::Chord => (Dash, Quicken, "move on the beat"),
        Attribute::Hour => (Rewind, DreamAnchor, "it's still a few seconds ago"),
        Attribute::Lumen => (Phase, Spring, "soft as a jellyfish"),
        Attribute::Gaze => (Stillness, ShardSense, "they stop when you look"),
        Attribute::Blank => (Phase, DustHoarder, "less of you to catch"),
    };
    Power {
        active,
        passive,
        tint: theme.spec().accents[0],
        flavour,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::ALL_THEMES;
    use crate::upgrades::{ALL_ABILITIES, PASSIVES};

    #[test]
    fn every_theme_has_a_power_tinted_by_its_accent() {
        for t in ALL_THEMES {
            let p = power(t);
            assert_eq!(p.tint, t.spec().accents[0], "{t:?}");
            assert!(PASSIVES.contains(&p.passive), "{t:?}");
            assert!(
                !matches!(p.passive, Upgrade::LucidHeart | Upgrade::WideChoice),
                "{t:?}: mythic one-offs aren't card passives"
            );
            assert!(!p.flavour.is_empty() && p.flavour.len() <= 48, "{t:?}");
        }
    }

    #[test]
    fn every_ability_is_on_some_card() {
        for a in ALL_ABILITIES {
            assert!(ALL_THEMES.iter().any(|&t| power(t).active == a), "{a:?}");
        }
    }
}
