//! Card sorting for the booklet, merge menu and loadout/deck screens.
//! Pure logic: no GL, no clock.

use crate::cards::Card;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SortKey {
    Rarity,
    Dream,
    Number,
    Ability,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Dir {
    Asc,
    Desc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Sort {
    pub key: SortKey,
    pub dir: Dir,
}

impl Default for Sort {
    /// Today's order: the booklet lists cards in the order they were made (oldest first).
    fn default() -> Self {
        Sort { key: SortKey::Number, dir: Dir::Asc }
    }
}

impl SortKey {
    pub fn next(self) -> SortKey {
        match self {
            SortKey::Number => SortKey::Rarity,
            SortKey::Rarity => SortKey::Dream,
            SortKey::Dream => SortKey::Ability,
            SortKey::Ability => SortKey::Number,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SortKey::Rarity => "RARITY",
            SortKey::Dream => "DREAM",
            SortKey::Number => "NUMBER",
            SortKey::Ability => "ABILITY",
        }
    }
}

impl Sort {
    /// Next field (Number -> Rarity -> Dream -> Ability -> Number); direction is kept.
    pub fn cycle_key(&mut self) {
        self.key = self.key.next();
    }

    pub fn flip(&mut self) {
        self.dir = match self.dir {
            Dir::Asc => Dir::Desc,
            Dir::Desc => Dir::Asc,
        };
    }

    /// e.g. "RARITY ▼" (▼ = Desc, ▲ = Asc).
    pub fn label(&self) -> String {
        let arrow = match self.dir {
            Dir::Asc => '▲',
            Dir::Desc => '▼',
        };
        format!("{} {}", self.key.name(), arrow)
    }
}

/// Indices into `cards`, sorted. Stable; ties break by card number (in the sort's direction,
/// so flipping always reverses the whole list). Number Desc = newest first.
/// Asc = Faint..Prophetic.. / A..Z / A..Z by ability name / oldest first.
pub fn order(cards: &[Card], sort: Sort) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..cards.len()).collect();
    idx.sort_by(|&a, &b| {
        let (ca, cb) = (&cards[a], &cards[b]);
        let primary = match sort.key {
            SortKey::Rarity => ca.rarity.cmp(&cb.rarity),
            SortKey::Dream => ca.name.to_lowercase().cmp(&cb.name.to_lowercase()),
            SortKey::Number => std::cmp::Ordering::Equal,
            SortKey::Ability => crate::powers::power_of(ca).active.name().cmp(crate::powers::power_of(cb).active.name()),
        };
        let o = primary.then(ca.number.cmp(&cb.number));
        match sort.dir {
            Dir::Asc => o,
            Dir::Desc => o.reverse(),
        }
    });
    idx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{card_from, Rarity};
    use crate::dream::DreamTheme;

    fn card(number: u32, theme: DreamTheme, name: &str, rarity: Rarity) -> Card {
        let mut c = card_from(&crate::cards::tests::record(theme, number as u64, 1, 0.1, false));
        c.number = number;
        c.name = name.to_string();
        c.rarity = rarity;
        c
    }

    fn nums(cards: &[Card], s: Sort) -> Vec<u32> {
        order(cards, s).into_iter().map(|i| cards[i].number).collect()
    }

    fn sample() -> Vec<Card> {
        vec![
            card(1, DreamTheme::Garden, "Moss", Rarity::Vivid),
            card(2, DreamTheme::Lobby, "apple", Rarity::Faint),
            card(3, DreamTheme::Garden, "Zinc", Rarity::Vivid),
            card(4, DreamTheme::Lobby, "Brine", Rarity::Lucid),
        ]
    }

    #[test]
    fn number_both_directions() {
        let c = sample();
        let mut s = Sort { key: SortKey::Number, dir: Dir::Desc };
        assert_eq!(nums(&c, s), vec![4, 3, 2, 1]);
        s.flip();
        assert_eq!(nums(&c, s), vec![1, 2, 3, 4]);
        assert_eq!(Sort::default(), Sort { key: SortKey::Number, dir: Dir::Asc });
    }

    #[test]
    fn rarity_both_directions_ties_by_number() {
        let c = sample();
        let asc = Sort { key: SortKey::Rarity, dir: Dir::Asc };
        assert_eq!(nums(&c, asc), vec![2, 1, 3, 4]);
        let desc = Sort { key: SortKey::Rarity, dir: Dir::Desc };
        assert_eq!(nums(&c, desc), vec![4, 3, 1, 2]);
    }

    #[test]
    fn dream_is_case_insensitive_name_order() {
        let c = sample();
        let asc = Sort { key: SortKey::Dream, dir: Dir::Asc };
        assert_eq!(nums(&c, asc), vec![2, 4, 1, 3]);
        let desc = Sort { key: SortKey::Dream, dir: Dir::Desc };
        assert_eq!(nums(&c, desc), vec![3, 1, 4, 2]);
    }

    #[test]
    fn ability_groups_by_active_name() {
        let c = sample();
        let name = |i: usize| crate::powers::power_of(&c[i]).active.name();
        for dir in [Dir::Asc, Dir::Desc] {
            let o = order(&c, Sort { key: SortKey::Ability, dir });
            let names: Vec<_> = o.iter().map(|&i| name(i)).collect();
            let mut want = names.clone();
            want.sort();
            if dir == Dir::Desc {
                want.reverse();
            }
            assert_eq!(names, want);
        }
        // Same-theme cards tie and fall back to number.
        let o = nums(&c, Sort { key: SortKey::Ability, dir: Dir::Asc });
        let pos = |n| o.iter().position(|&x| x == n).unwrap();
        assert!(pos(1) < pos(3));
        assert!(pos(2) < pos(4));
    }

    #[test]
    fn stable_and_a_permutation() {
        let c = sample();
        for key in [SortKey::Rarity, SortKey::Dream, SortKey::Number, SortKey::Ability] {
            for dir in [Dir::Asc, Dir::Desc] {
                let mut o = order(&c, Sort { key, dir });
                assert_eq!(o, order(&c, Sort { key, dir }));
                o.sort();
                assert_eq!(o, vec![0, 1, 2, 3]);
            }
        }
        assert!(order(&[], Sort::default()).is_empty());
    }

    #[test]
    fn flip_reverses_everything() {
        let c = sample();
        for key in [SortKey::Rarity, SortKey::Dream, SortKey::Number, SortKey::Ability] {
            let mut s = Sort { key, dir: Dir::Asc };
            let mut a = order(&c, s);
            s.flip();
            a.reverse();
            assert_eq!(a, order(&c, s));
        }
    }

    #[test]
    fn cycle_and_label() {
        let mut s = Sort { key: SortKey::Rarity, dir: Dir::Desc };
        assert_eq!(s.label(), "RARITY ▼");
        s.flip();
        assert_eq!(s.label(), "RARITY ▲");
        let start = Sort::default().key;
        let mut k = start;
        for _ in 0..4 {
            k = k.next();
        }
        assert_eq!(k, start);
        s.cycle_key();
        assert_eq!(s.key, SortKey::Dream);
    }
}
