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
    /// A way of fighting some dreams also teach (see `edges.rs`).
    pub edge: Option<crate::edges::Edge>,
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
        Attribute::Dawn => (Leap, WideEyes, "the light lifts you early"),
        Attribute::Hex => (Decoy, DreamAnchor, "the trees argue about which one is you"),
        Attribute::Tide => (Float, LongBreath, "hold your breath and drift"),
        Attribute::Aether => (Step, DoubleJump, "one more step than there should be"),
        Attribute::Glass => (Ward, WideEyes, "your reflection takes the hit"),
        Attribute::Spore => (Tether, ShardSense, "the roots haul it up to you"),
        Attribute::Abyss => (Dash, SwiftFeet, "the far end pulls you in"),
        Attribute::Halo => (Hush, Quicken, "every room waits for the next"),
        Attribute::Jest => (Gust, DustHoarder, "a small hand shoves them back"),
        Attribute::Trace => (Anchor, SwiftFeet, "you left a copy back there"),
        Attribute::Chord => (Slam, Quicken, "land on the beat"),
        Attribute::Hour => (Rewind, DreamAnchor, "it's still a few seconds ago"),
        Attribute::Lumen => (Veil, Spring, "soft light, and nobody sees you in it"),
        Attribute::Gaze => (Bind, ShardSense, "they stop when you look"),
        Attribute::Blank => (Pulse, DustHoarder, "you hear where the walls are not"),
    };
    Power {
        active,
        passive,
        edge: crate::edges::edge(theme),
        tint: theme.spec().accents[0],
        flavour,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::ALL_THEMES;
    use crate::upgrades::{Ability, ALL_ABILITIES, PASSIVES};

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
            // MIRROR, QUAKE, SURGE and HASTE have no card: picks and the moth only.
            if [Ability::Mirror, Ability::Quake, Ability::Surge, Ability::Haste].contains(&a) {
                continue;
            }
            assert!(ALL_THEMES.iter().any(|&t| power(t).active == a), "{a:?}");
        }
    }
}
