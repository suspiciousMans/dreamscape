//! Merging cards at the moth: two cards become one fused card, with the
//! first card's shape (theme, pattern, attribute, ability) and the second
//! card's colours and mood. A resonant pair makes a Resonant card instead.
//! Pairs approved in `plans/2026-09-28-dreamscape-rest-of-project-plan.md`.

use crate::cards::{Attribute, Booklet, Card, Rarity};
use crate::dream::{DreamTheme, TexSpec, ALL_THEMES};

/// Each dream appears in exactly one pair; order doesn't matter.
pub const RESONANT: [(DreamTheme, DreamTheme); 10] = {
    use DreamTheme::*;
    [
        (Garden, MyceliumGrove),
        (DrownedLibrary, JellyfishSky),
        (MirrorHall, WatchingWallpaper),
        (TheTunnel, Awakening),
        (MeltingClockworks, AfterimageFields),
        (SkyStairs, VoidPlatforms),
        (FractalCathedral, Elfworks),
        (SynesthesiaHall, Lobby),
        (NightmareFactory, LiminalOffice),
        (CursedForest, WhiteDissolve),
    ]
};

pub fn is_resonant(a: DreamTheme, b: DreamTheme) -> bool {
    RESONANT
        .iter()
        .any(|&(x, y)| (x, y) == (a, b) || (y, x) == (a, b))
}

/// A recipe as stored in the booklet: sorted indices into `ALL_THEMES`.
pub fn recipe(a: DreamTheme, b: DreamTheme) -> (usize, usize) {
    let i = |t| ALL_THEMES.iter().position(|&x| x == t).unwrap_or(0);
    let (x, y) = (i(a), i(b));
    (x.min(y), x.max(y))
}

/// Two cards can merge if they're different dreams.
pub fn can_merge(a: &Card, b: &Card) -> bool {
    a.number != b.number && a.theme != b.theme
}

/// Half of one name, half of the other ("VELVET" + "GARDEN OF DOORS").
fn fused_name(a: &str, b: &str) -> String {
    let aw: Vec<&str> = a.split_whitespace().collect();
    let bw: Vec<&str> = b.split_whitespace().collect();
    let head = &aw[..aw.len().div_ceil(2).max(1).min(aw.len())];
    let tail = &bw[bw.len() / 2..];
    head.iter()
        .chain(tail.iter())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The merged card (numbered like `a`; the booklet decides the rest).
pub fn merge(a: &Card, b: &Card) -> Card {
    Card {
        number: a.number,
        theme: a.theme,
        seed: a.seed.rotate_left(17) ^ b.seed,
        depth: a.depth.max(b.depth),
        name: fused_name(&a.name, &b.name),
        description: b.description.clone(),
        rarity: if is_resonant(a.theme, b.theme) {
            Rarity::Resonant
        } else {
            Rarity::Fused
        },
        attribute: Attribute::of(a.theme),
        dread: a.dread.max(b.dread),
        drift: a.drift.max(b.drift),
        recurring: false,
        art: TexSpec {
            pattern: a.art.pattern,
            palette: b.art.palette.clone(),
            bands: a.art.bands,
            seed: a.art.seed,
        },
        fused: Some(b.theme),
    }
}

impl Booklet {
    /// Merges card `b` into card `a`: `a` becomes the fused card (keeping its
    /// number and place), `b` is used up. Returns the new card.
    pub fn merge(&mut self, a: u32, b: u32) -> Option<Card> {
        let (ca, cb) = (self.card(a)?.clone(), self.card(b)?.clone());
        if !can_merge(&ca, &cb) {
            return None;
        }
        let fused = merge(&ca, &cb);
        for run in &mut self.runs {
            run.cards.retain(|c| c.number != b);
            if let Some(c) = run.cards.iter_mut().find(|c| c.number == a) {
                *c = fused.clone();
            }
        }
        self.runs.retain(|r| !r.cards.is_empty());
        self.lore.loadout.retain(|&n| n != b);
        let r = recipe(ca.theme, cb.theme);
        if !self.lore.recipes.contains(&r) {
            self.lore.recipes.push(r);
        }
        Some(fused)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{card_from, tests::record};
    use DreamTheme::*;

    fn booklet(themes: &[DreamTheme]) -> Booklet {
        let mut b = Booklet::default();
        let recs: Vec<_> = themes
            .iter()
            .enumerate()
            .map(|(i, &t)| record(t, i as u64 + 1, 1, 0.3, false))
            .collect();
        b.add_run(1, &recs[..1]);
        b.add_run(2, &recs[1..]);
        b
    }

    #[test]
    fn every_dream_is_in_exactly_one_resonant_pair() {
        for t in ALL_THEMES {
            let n = RESONANT.iter().filter(|&&(x, y)| x == t || y == t).count();
            assert_eq!(n, 1, "{t:?}");
        }
        assert!(is_resonant(MyceliumGrove, Garden), "order doesn't matter");
        assert!(!is_resonant(Garden, Lobby));
    }

    #[test]
    fn merging_uses_both_cards_and_keeps_a_shape_and_b_colours() {
        let mut b = booklet(&[Garden, TheTunnel, Lobby]);
        let (a0, b0) = (b.card(1).unwrap().clone(), b.card(2).unwrap().clone());
        b.lore.loadout = vec![2];
        let f = b.merge(1, 2).unwrap();
        assert_eq!(b.card_count(), 2);
        assert!(b.card(2).is_none());
        assert!(
            b.lore.loadout.is_empty(),
            "a used-up card leaves the loadout"
        );
        assert_eq!(f.number, 1);
        assert_eq!((f.theme, f.fused), (Garden, Some(TheTunnel)));
        assert_eq!(f.rarity, Rarity::Fused);
        assert_eq!(f.art.pattern, a0.art.pattern);
        assert_eq!(f.art.palette, b0.art.palette);
        assert_eq!(f.attribute, Attribute::Bloom);
        assert_eq!(b.lore.recipes, vec![recipe(Garden, TheTunnel)]);
        assert!(!b.runs[0].cards.is_empty());
        let text = ron::to_string(&b).unwrap();
        assert_eq!(ron::from_str::<Booklet>(&text).unwrap(), b);
    }

    #[test]
    fn a_resonant_pair_makes_a_resonant_card_and_empty_runs_go() {
        let mut b = booklet(&[MyceliumGrove, Garden]);
        let f = b.merge(2, 1).unwrap();
        assert_eq!(f.rarity, Rarity::Resonant);
        assert_eq!(b.runs.len(), 1, "card 1's run is empty now");
        assert!(!b.runs[0].cards.is_empty());
    }

    #[test]
    fn a_card_cant_merge_with_itself_or_its_own_dream() {
        let mut b = booklet(&[Garden, Garden]);
        assert!(b.merge(1, 1).is_none());
        assert!(b.merge(1, 2).is_none());
        assert!(b.merge(1, 9).is_none());
        assert_eq!(b.card_count(), 2);
    }

    #[test]
    fn new_cards_after_a_merge_get_fresh_numbers() {
        let mut b = booklet(&[Garden, TheTunnel, Lobby]);
        b.merge(1, 2).unwrap();
        b.add_run(9, &[record(Elfworks, 50, 1, 0.3, false)]);
        let mut nums: Vec<u32> = b.cards().map(|c| c.number).collect();
        nums.sort();
        nums.dedup();
        assert_eq!(nums.len(), b.card_count(), "numbers stay unique");
    }

    #[test]
    fn fused_names_take_half_of_each() {
        assert_eq!(
            fused_name("VELVET HALL", "GARDEN OF DOORS"),
            "VELVET OF DOORS"
        );
        assert_eq!(fused_name("A", "B"), "A B");
        let c = card_from(&record(Garden, 1, 1, 0.1, false));
        assert!(!merge(&c, &c).name.is_empty());
    }
}
