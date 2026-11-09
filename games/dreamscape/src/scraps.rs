//! Scraps: small disposable cards that come with a pack. One is added per
//! opened pack, kept in the booklet's satchel until used, and carried (up to
//! [`SATCHEL_CARRY`]) into a deck run as run-only cards that are gone once
//! played.
//!
//! Pure: no clock, no GL. The kind is a seeded roll; `main.rs` only wires.

use serde::{Deserialize, Serialize};

/// Most scraps the loadout lets a run carry.
pub const SATCHEL_CARRY: usize = 2;
/// Most of one kind the satchel holds (a quiet cap, never shown as a penalty).
pub const SATCHEL_CAP: u32 = 9;
/// Mixed into the run seed (and the pack's index) for the kind roll.
pub const ROLL_SALT: u64 = 0x5C4A_9B5E_D15C_A8D5;

/// Mend: nerve points refilled.
pub const MEND_NERVE: u32 = 99;
/// Lull: seconds the enemies are calmed.
pub const LULL_SECS: f32 = 6.0;
/// Lull: reach in world units (the whole dream).
pub const LULL_REACH: f32 = 1000.0;
/// Lantern: seconds the shard's pillar of light stays.
pub const LANTERN_SECS: f32 = 10.0;
/// Windfall: dust added to the run's bonus, paid with the run's dust.
pub const WINDFALL_DUST: u32 = 25;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScrapKind {
    Mend,
    Lull,
    Lantern,
    Windfall,
    SafeStep,
    Redraw,
}

pub const ALL_SCRAPS: [ScrapKind; 6] = [
    ScrapKind::Mend,
    ScrapKind::Lull,
    ScrapKind::Lantern,
    ScrapKind::Windfall,
    ScrapKind::SafeStep,
    ScrapKind::Redraw,
];

impl ScrapKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Mend => "Mend",
            Self::Lull => "Lull",
            Self::Lantern => "Lantern",
            Self::Windfall => "Windfall",
            Self::SafeStep => "Safe Step",
            Self::Redraw => "Redraw",
        }
    }

    /// One short line, for the loadout and the hand.
    pub fn desc(self) -> &'static str {
        match self {
            Self::Mend => "Refill your nerve.",
            Self::Lull => "Calm every enemy for a few seconds.",
            Self::Lantern => "Light up the nearest shard.",
            Self::Windfall => "Bonus dust, paid when the run ends.",
            Self::SafeStep => "The next hit does nothing.",
            Self::Redraw => "Throw back your hand and draw a fresh one.",
        }
    }

    /// A very short label for the small card in the hand.
    pub fn label(self) -> &'static str {
        match self {
            Self::Mend => "MEND",
            Self::Lull => "LULL",
            Self::Lantern => "LIGHT",
            Self::Windfall => "DUST",
            Self::SafeStep => "SAFE",
            Self::Redraw => "REDRAW",
        }
    }

    /// The card's colour.
    pub fn color(self) -> [u8; 3] {
        match self {
            Self::Mend => [120, 220, 150],
            Self::Lull => [140, 160, 240],
            Self::Lantern => [250, 220, 120],
            Self::Windfall => [240, 190, 90],
            Self::SafeStep => [200, 200, 230],
            Self::Redraw => [210, 150, 230],
        }
    }
}

/// The kind a pack gives: a seeded roll, so the same run and pack always
/// give the same scrap. `pack_index` counts the packs opened this run.
pub fn roll_scrap(seed: u64, pack_index: u32) -> ScrapKind {
    let h = crate::worth::mix(seed ^ ROLL_SALT, pack_index as u64);
    ALL_SCRAPS[(h % ALL_SCRAPS.len() as u64) as usize]
}

/// The scraps kept in the booklet until they are used, and which the
/// player chose to carry.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Satchel {
    #[serde(default)]
    pub counts: Vec<(ScrapKind, u32)>,
    /// The loadout's picks (a kind may repeat up to its count).
    #[serde(default)]
    pub chosen: Vec<ScrapKind>,
}

impl Satchel {
    pub fn count(&self, kind: ScrapKind) -> u32 {
        self.counts.iter().find(|c| c.0 == kind).map_or(0, |c| c.1)
    }

    #[cfg(test)]
    pub fn total(&self) -> u32 {
        self.counts.iter().map(|c| c.1).sum()
    }

    /// Adds one; false (nothing added) at [`SATCHEL_CAP`].
    pub fn add(&mut self, kind: ScrapKind) -> bool {
        match self.counts.iter_mut().find(|c| c.0 == kind) {
            Some(c) if c.1 >= SATCHEL_CAP => false,
            Some(c) => {
                c.1 += 1;
                true
            }
            None => {
                self.counts.push((kind, 1));
                true
            }
        }
    }

    /// Uses one; false (nothing changes) at zero.
    pub fn spend(&mut self, kind: ScrapKind) -> bool {
        let Some(i) = self.counts.iter().position(|c| c.0 == kind && c.1 > 0) else {
            return false;
        };
        self.counts[i].1 -= 1;
        if self.counts[i].1 == 0 {
            self.counts.remove(i);
        }
        let held = self.count(kind) as usize;
        // Keep the picks honest: never more of a kind than is held.
        let mut seen = 0;
        self.chosen.retain(|&k| {
            if k != kind {
                return true;
            }
            seen += 1;
            seen <= held
        });
        true
    }

    /// The scraps a run brings: the chosen ones still held, at most `limit`,
    /// in pick order.
    pub fn carry(&self, limit: usize) -> Vec<ScrapKind> {
        let mut out: Vec<ScrapKind> = Vec::new();
        for &k in &self.chosen {
            let have = out.iter().filter(|&&o| o == k).count() as u32;
            if have < self.count(k) && out.len() < limit {
                out.push(k);
            }
        }
        out
    }

    /// Loadout toggle: picks one more of `kind` (if held and under `limit`),
    /// or with none left to add, drops every pick of it. Returns true when
    /// the picks changed.
    pub fn toggle_carry(&mut self, kind: ScrapKind, limit: usize) -> bool {
        let picked = self.chosen.iter().filter(|&&k| k == kind).count() as u32;
        if picked < self.count(kind) && self.carry(usize::MAX).len() < limit {
            self.chosen.push(kind);
            true
        } else if picked > 0 {
            self.chosen.retain(|&k| k != kind);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_roll_is_deterministic_and_reaches_every_kind() {
        assert_eq!(roll_scrap(7, 0), roll_scrap(7, 0));
        let mut seen = std::collections::HashSet::new();
        for s in 0..200u64 {
            seen.insert(roll_scrap(s, 0));
        }
        assert_eq!(seen.len(), ALL_SCRAPS.len());
        let a: Vec<_> = (0..12).map(|i| roll_scrap(5, i)).collect();
        assert!(a.iter().any(|&k| k != a[0]), "the pack index changes the roll");
    }

    #[test]
    fn spend_refuses_at_zero_and_add_stops_at_the_cap() {
        let mut s = Satchel::default();
        assert!(!s.spend(ScrapKind::Mend));
        assert!(s.add(ScrapKind::Mend));
        assert_eq!(s.count(ScrapKind::Mend), 1);
        assert!(s.spend(ScrapKind::Mend));
        assert!(!s.spend(ScrapKind::Mend));
        assert_eq!(s.total(), 0);
        for _ in 0..SATCHEL_CAP {
            assert!(s.add(ScrapKind::Lull));
        }
        assert!(!s.add(ScrapKind::Lull));
        assert_eq!(s.count(ScrapKind::Lull), SATCHEL_CAP);
    }

    #[test]
    fn carry_follows_picks_limit_and_what_is_held() {
        let mut s = Satchel::default();
        s.add(ScrapKind::Mend);
        s.add(ScrapKind::Lull);
        s.add(ScrapKind::Redraw);
        assert!(s.toggle_carry(ScrapKind::Mend, SATCHEL_CARRY));
        assert!(s.toggle_carry(ScrapKind::Lull, SATCHEL_CARRY));
        assert!(!s.toggle_carry(ScrapKind::Redraw, SATCHEL_CARRY), "full, and none picked to drop");
        assert_eq!(s.carry(SATCHEL_CARRY), vec![ScrapKind::Mend, ScrapKind::Lull]);
        assert_eq!(s.carry(1), vec![ScrapKind::Mend]);
        assert!(s.toggle_carry(ScrapKind::Mend, SATCHEL_CARRY), "a held pick toggles off");
        assert_eq!(s.carry(SATCHEL_CARRY), vec![ScrapKind::Lull]);
        // Spending a picked scrap drops the pick.
        assert!(s.spend(ScrapKind::Lull));
        assert!(s.carry(SATCHEL_CARRY).is_empty());
        assert!(s.chosen.is_empty());
    }

    #[test]
    fn a_kind_can_be_carried_twice_only_if_held_twice() {
        let mut s = Satchel::default();
        s.add(ScrapKind::SafeStep);
        assert!(s.toggle_carry(ScrapKind::SafeStep, SATCHEL_CARRY));
        s.add(ScrapKind::SafeStep);
        assert!(s.toggle_carry(ScrapKind::SafeStep, SATCHEL_CARRY));
        assert_eq!(s.carry(SATCHEL_CARRY).len(), 2);
    }

    #[test]
    fn an_old_booklet_loads_with_an_empty_satchel() {
        let b: crate::cards::Booklet = ron::from_str("(runs: [])").expect("old booklet");
        assert_eq!(b.satchel, Satchel::default());
        let s: Satchel = ron::from_str("(counts: [(Mend, 2)])").expect("old satchel");
        assert_eq!(s.count(ScrapKind::Mend), 2);
        let round: Satchel = ron::from_str(&ron::to_string(&s).unwrap()).unwrap();
        assert_eq!(round, s);
    }

    #[test]
    fn every_kind_has_text_and_a_short_label() {
        for k in ALL_SCRAPS {
            assert!(!k.name().is_empty() && !k.desc().is_empty());
            assert!(k.label().len() <= 6);
        }
    }
}
