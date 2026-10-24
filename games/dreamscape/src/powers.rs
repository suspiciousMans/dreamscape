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

/// A resonant pair carries a way of moving neither dream teaches alone:
/// the fused card's ability (and flavour) become the pair's own.
pub const PAIR_ABILITIES: [(DreamTheme, DreamTheme, Ability, &str); 4] = [
    (
        DreamTheme::MirrorHall,
        DreamTheme::WatchingWallpaper,
        Ability::Mirror,
        "the glass and the eyes agree: not you",
    ),
    (
        DreamTheme::NightmareFactory,
        DreamTheme::LiminalOffice,
        Ability::Quake,
        "the whole floor remembers being a machine",
    ),
    (
        DreamTheme::TheTunnel,
        DreamTheme::Awakening,
        Ability::Surge,
        "the far end and the morning pull together",
    ),
    (
        DreamTheme::MeltingClockworks,
        DreamTheme::AfterimageFields,
        Ability::Haste,
        "every hour leaves a faster copy behind",
    ),
];

/// The power of a card as it stands: its dream's, except that a resonant
/// pair's fusion takes the pair's own ability and flavour.
pub fn power_of(card: &crate::cards::Card) -> Power {
    let mut p = power(card.theme);
    if card.rarity == crate::cards::Rarity::Resonant {
        if let Some(b) = card.fused {
            let pair = PAIR_ABILITIES.iter().find(|&&(x, y, _, _)| {
                (x, y) == (card.theme, b) || (y, x) == (card.theme, b)
            });
            if let Some(&(_, _, active, flavour)) = pair {
                p.active = active;
                p.flavour = flavour;
            }
        }
    }
    p
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

    fn card_of(theme: DreamTheme, fused: Option<DreamTheme>, rarity: crate::cards::Rarity) -> crate::cards::Card {
        let mut b = crate::cards::Booklet::default();
        b.add_run(1, &[crate::cards::tests::record(theme, 1, 1, 0.3, false)]);
        let mut c = b.cards().next().cloned().expect("a card");
        c.fused = fused;
        c.rarity = rarity;
        c
    }

    #[test]
    fn every_ability_is_on_some_card() {
        for a in ALL_ABILITIES {
            let on_a_dream = ALL_THEMES.iter().any(|&t| power(t).active == a);
            let on_a_pair = PAIR_ABILITIES.iter().any(|p| p.2 == a);
            assert!(on_a_dream || on_a_pair, "{a:?}");
        }
    }

    #[test]
    fn pair_abilities_sit_on_real_resonant_pairs_and_only_there() {
        use crate::cards::Rarity;
        for &(x, y, a, flavour) in &PAIR_ABILITIES {
            assert!(crate::fusion::is_resonant(x, y), "{x:?}+{y:?}");
            assert!(!flavour.is_empty() && flavour.len() <= 48, "{a:?}");
            for (t, f) in [(x, y), (y, x)] {
                let c = card_of(t, Some(f), Rarity::Resonant);
                assert_eq!(power_of(&c).active, a, "{t:?}+{f:?}");
                assert_eq!(power_of(&c).flavour, flavour);
                assert_eq!(power_of(&c).passive, power(t).passive, "passive unchanged");
                // A plain fusion of the same dreams keeps the first dream's own ability.
                let plain = card_of(t, Some(f), Rarity::Fused);
                assert_eq!(power_of(&plain).active, power(t).active);
            }
            assert_eq!(power_of(&card_of(x, None, Rarity::Vivid)).active, power(x).active);
        }
    }
}
