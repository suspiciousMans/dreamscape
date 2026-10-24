//! === perf === Cached card lists for the menu views. `hud_view()` runs every
//! frame; the loadout, merge and booklet screens used to clone every card
//! (Strings and art specs) each time. The sorted list is now built once per
//! (booklet contents, sort, slice) and shared as an `Arc<[Card]>`.
use crate::cards::Card;
use crate::sorting::Sort;
use std::sync::Arc;

/// Cheap fingerprint of the booklet's cards: changes whenever a card is added,
/// removed, merged or edited in any field the screens show.
pub fn fingerprint<'a>(cards: impl Iterator<Item = &'a Card>) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        h ^= h >> 29;
    };
    for c in cards {
        mix(c.number as u64);
        mix(c.seed);
        mix(c.depth as u64);
        mix(c.dread as u64 | (c.drift as u64) << 8 | (c.recurring as u64) << 16);
        mix(c.name.len() as u64 ^ (c.description.len() as u64) << 20);
        mix(c.rarity as u64);
        mix(c.fused.map_or(0, |t| 1 + t as u64));
    }
    h
}

#[derive(Default)]
pub struct CardCache {
    key: Option<(u64, Sort, usize, usize)>,
    list: Arc<[Card]>,
    /// How many times the list was rebuilt (tests, profiling).
    pub rebuilds: u32,
}

impl CardCache {
    /// The cards in `order` (indices into `all`), limited to `range`, cloned
    /// only when `print`/`sort`/`range` differ from the last call.
    pub fn get(
        &mut self,
        print: u64,
        sort: Sort,
        range: std::ops::Range<usize>,
        all: impl FnOnce() -> Vec<Card>,
    ) -> Arc<[Card]> {
        let key = (print, sort, range.start, range.end);
        if self.key != Some(key) {
            let cards = all();
            let order = crate::sorting::order(&cards, sort);
            let end = range.end.min(order.len());
            let start = range.start.min(end);
            self.list = order[start..end].iter().map(|&i| cards[i].clone()).collect();
            self.key = Some(key);
            self.rebuilds += 1;
        }
        self.list.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{card_from, tests::record};
    use crate::dream::DreamTheme;
    use crate::sorting::{Dir, SortKey};

    fn cards(n: u32) -> Vec<Card> {
        (1..=n)
            .map(|i| {
                let mut c = card_from(&record(DreamTheme::Garden, i as u64, i, 0.3, false));
                c.number = i;
                c
            })
            .collect()
    }

    #[test]
    fn rebuilds_only_when_the_cards_the_sort_or_the_slice_change() {
        let mut c = CardCache::default();
        let all = cards(10);
        let s = Sort::default();
        let p = fingerprint(all.iter());
        let a = c.get(p, s, 0..usize::MAX, || all.clone());
        let b = c.get(p, s, 0..usize::MAX, || panic!("must not rebuild"));
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(c.rebuilds, 1);
        let desc = Sort { key: SortKey::Number, dir: Dir::Desc };
        let d = c.get(p, desc, 0..usize::MAX, || all.clone());
        assert_eq!(d[0].number, 10);
        let page = c.get(p, desc, 2..4, || all.clone());
        assert_eq!(page.iter().map(|x| x.number).collect::<Vec<_>>(), vec![8, 7]);
        assert_eq!(c.rebuilds, 3);
    }

    #[test]
    fn the_fingerprint_sees_added_removed_and_edited_cards() {
        let mut all = cards(5);
        let p = fingerprint(all.iter());
        assert_eq!(p, fingerprint(all.iter()));
        let mut edited = all.clone();
        edited[2].depth += 1;
        assert_ne!(p, fingerprint(edited.iter()));
        let last = all.pop().unwrap();
        assert_ne!(p, fingerprint(all.iter()));
        all.push(last);
        all.swap(0, 1);
        assert_ne!(p, fingerprint(all.iter()), "order matters");
    }
}
