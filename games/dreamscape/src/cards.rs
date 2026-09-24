//! Dream trading cards and the booklet they're pressed into. Pure data + RON
//! persistence; drawing lives in `booklet_ui.rs`.
//!
//! Save-format note: any field added to `Card`/`SavedRun`/`Booklet` later MUST
//! carry `#[serde(default)]`, or existing booklets stop parsing (they'd be
//! quarantined to `.ron.corrupt`, not lost — but still).

use crate::dream::{DreamTheme, TexSpec};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Rarity {
    Faint,
    Hazy,
    Vivid,
    Lucid,
    Prophetic,
}

impl Rarity {
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s < 0.55 => Rarity::Faint,
            s if s < 0.80 => Rarity::Hazy,
            s if s < 0.95 => Rarity::Vivid,
            s if s < 1.10 => Rarity::Lucid,
            _ => Rarity::Prophetic,
        }
    }

    pub fn bumped(self) -> Self {
        match self {
            Rarity::Faint => Rarity::Hazy,
            Rarity::Hazy => Rarity::Vivid,
            Rarity::Vivid => Rarity::Lucid,
            Rarity::Lucid | Rarity::Prophetic => Rarity::Prophetic,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Rarity::Faint => "FAINT",
            Rarity::Hazy => "HAZY",
            Rarity::Vivid => "VIVID",
            Rarity::Lucid => "LUCID",
            Rarity::Prophetic => "PROPHETIC",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Attribute {
    Velvet,
    Static,
    Void,
    Bloom,
    Rust,
    Dawn,
}

impl Attribute {
    pub fn of(theme: DreamTheme) -> Self {
        match theme {
            DreamTheme::Lobby => Attribute::Velvet,
            DreamTheme::LiminalOffice => Attribute::Static,
            DreamTheme::VoidPlatforms => Attribute::Void,
            DreamTheme::Garden => Attribute::Bloom,
            DreamTheme::NightmareFactory => Attribute::Rust,
            DreamTheme::Awakening => Attribute::Dawn,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Attribute::Velvet => "VELVET",
            Attribute::Static => "STATIC",
            Attribute::Void => "VOID",
            Attribute::Bloom => "BLOOM",
            Attribute::Rust => "RUST",
            Attribute::Dawn => "DAWN",
        }
    }
}

/// What happened in one dream of the current run (collected while playing).
#[derive(Clone, Debug, PartialEq)]
pub struct DreamRecord {
    pub theme: DreamTheme,
    pub seed: u64,
    pub depth: u32,
    pub name: String,
    pub whisper: String,
    pub strangeness: f32,
    pub enemies: u32,
    pub shard_taken: bool,
    pub art: TexSpec,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Card {
    /// Booklet-wide, 1-based; assigned when the run is saved.
    pub number: u32,
    pub theme: DreamTheme,
    pub seed: u64,
    pub depth: u32,
    pub name: String,
    pub description: String,
    pub rarity: Rarity,
    pub attribute: Attribute,
    pub dread: u8,
    pub drift: u8,
    pub recurring: bool,
    pub art: TexSpec,
}

pub fn rarity_roll(seed: u64) -> f32 {
    StdRng::seed_from_u64(seed ^ 0xCA2D_5EED).gen::<f32>()
}

pub fn rarity_bonus(r: &DreamRecord) -> f32 {
    0.15 * r.strangeness.clamp(0.0, 1.0)
        + 0.01 * r.depth.min(10) as f32
        + if r.shard_taken { 0.05 } else { 0.0 }
}

/// 0..=1 → 1..=9.
pub fn stat(x: f32) -> u8 {
    (1.0 + x.clamp(0.0, 1.0) * 8.0).round() as u8
}

pub fn card_from(r: &DreamRecord) -> Card {
    let mut rarity = Rarity::from_score(rarity_roll(r.seed) + rarity_bonus(r));
    if r.theme == DreamTheme::Awakening {
        rarity = rarity.max(Rarity::Vivid);
    }
    Card {
        number: 0,
        theme: r.theme,
        seed: r.seed,
        depth: r.depth,
        name: r.name.clone(),
        description: r.whisper.clone(),
        rarity,
        attribute: Attribute::of(r.theme),
        dread: stat(0.2 * r.enemies as f32 + 0.5 * r.strangeness),
        drift: stat(r.strangeness),
        recurring: false,
        art: r.art.clone(),
    }
}

pub const CARDS_PER_PAGE: usize = 3;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SavedRun {
    pub run_seed: u64,
    pub deepest: u32,
    pub cards: Vec<Card>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Booklet {
    pub runs: Vec<SavedRun>,
}

impl Booklet {
    pub fn cards(&self) -> impl Iterator<Item = &Card> {
        self.runs.iter().flat_map(|r| r.cards.iter())
    }

    pub fn card_count(&self) -> usize {
        self.runs.iter().map(|r| r.cards.len()).sum()
    }

    pub fn has_run(&self, run_seed: u64) -> bool {
        self.runs.iter().any(|r| r.run_seed == run_seed)
    }

    /// Presses a finished run into the booklet. Returns how many cards were added.
    pub fn add_run(&mut self, run_seed: u64, records: &[DreamRecord]) -> usize {
        let mut next = self.card_count() as u32 + 1;
        let mut seen: HashSet<String> = self.cards().map(|c| c.name.clone()).collect();
        let cards: Vec<Card> = records
            .iter()
            .map(|r| {
                let mut c = card_from(r);
                c.number = next;
                next += 1;
                if !seen.insert(c.name.clone()) {
                    c.recurring = true;
                    c.rarity = c.rarity.bumped();
                }
                c
            })
            .collect();
        let added = cards.len();
        let deepest = records.iter().map(|r| r.depth).max().unwrap_or(0);
        self.runs.push(SavedRun {
            run_seed,
            deepest,
            cards,
        });
        added
    }
}

pub fn page_count(total: usize) -> usize {
    total.div_ceil(CARDS_PER_PAGE).max(1)
}

pub fn page_range(total: usize, page: usize) -> std::ops::Range<usize> {
    let start = page.min(page_count(total) - 1) * CARDS_PER_PAGE;
    start.min(total)..(start + CARDS_PER_PAGE).min(total)
}

pub const BOOKLET_PATH: &str = "games/dreamscape/booklet.ron";

pub fn booklet_path() -> PathBuf {
    std::env::var_os("DREAMSCAPE_BOOKLET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(BOOKLET_PATH))
}

/// Missing file = empty booklet. A file that won't parse is moved aside to
/// `<file>.corrupt` and never silently overwritten: it's someone's collection.
pub fn load(path: &Path) -> Booklet {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Booklet::default();
    };
    match ron::from_str(&text) {
        Ok(b) => b,
        Err(e) => {
            let aside = path.with_extension("ron.corrupt");
            log::error!("booklet {path:?} is unreadable ({e}); moved to {aside:?}");
            let _ = std::fs::rename(path, &aside);
            Booklet::default()
        }
    }
}

/// Written to a temp file first, then renamed, so a crash mid-save can't
/// truncate the booklet.
pub fn save(path: &Path, booklet: &Booklet) -> anyhow::Result<()> {
    let text = ron::ser::to_string_pretty(booklet, ron::ser::PrettyConfig::default())?;
    let tmp = path.with_extension("ron.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::DreamTheme::*;

    pub(crate) fn record(
        theme: DreamTheme,
        seed: u64,
        depth: u32,
        strangeness: f32,
        shard: bool,
    ) -> DreamRecord {
        DreamRecord {
            theme,
            seed,
            depth,
            name: crate::dream::dream_name(theme, seed),
            whisper: crate::dream::whisper(theme, seed),
            strangeness,
            enemies: 1,
            shard_taken: shard,
            art: crate::dream::portal_surface(theme, seed),
        }
    }

    #[test]
    fn rarity_thresholds() {
        assert_eq!(Rarity::from_score(0.0), Rarity::Faint);
        assert_eq!(Rarity::from_score(0.549), Rarity::Faint);
        assert_eq!(Rarity::from_score(0.55), Rarity::Hazy);
        assert_eq!(Rarity::from_score(0.80), Rarity::Vivid);
        assert_eq!(Rarity::from_score(0.95), Rarity::Lucid);
        assert_eq!(Rarity::from_score(1.10), Rarity::Prophetic);
        assert_eq!(Rarity::Prophetic.bumped(), Rarity::Prophetic);
        assert_eq!(Rarity::Faint.bumped(), Rarity::Hazy);
    }

    #[test]
    fn shallow_calm_dreams_are_mostly_faint_and_never_prophetic() {
        let n = 20_000;
        let mut faint = 0;
        for seed in 0..n {
            let r = card_from(&record(LiminalOffice, seed, 0, 0.0, false)).rarity;
            assert_ne!(r, Rarity::Prophetic, "seed {seed}");
            faint += (r == Rarity::Faint) as u32;
        }
        let frac = faint as f32 / n as f32;
        assert!((0.52..0.58).contains(&frac), "faint fraction {frac}");
    }

    #[test]
    fn deeper_stranger_dreams_never_lower_rarity() {
        for seed in 0..2000 {
            let low = card_from(&record(Garden, seed, 0, 0.0, false)).rarity;
            let high = card_from(&record(Garden, seed, 10, 1.0, true)).rarity;
            assert!(high >= low, "seed {seed}: {low:?} -> {high:?}");
        }
        assert!((0..200)
            .any(|s| card_from(&record(Garden, s, 12, 1.0, true)).rarity == Rarity::Prophetic));
    }

    #[test]
    fn waking_is_at_least_vivid() {
        for seed in 0..500 {
            assert!(card_from(&record(Awakening, seed, 6, 0.0, false)).rarity >= Rarity::Vivid);
        }
    }

    #[test]
    fn stats_and_attributes() {
        for s in [0.0, 0.3, 1.0] {
            let c = card_from(&record(NightmareFactory, 3, 4, s, false));
            assert!((1..=9).contains(&c.dread) && (1..=9).contains(&c.drift));
        }
        let attrs: HashSet<Attribute> = [
            Lobby,
            LiminalOffice,
            VoidPlatforms,
            Garden,
            NightmareFactory,
            Awakening,
        ]
        .into_iter()
        .map(Attribute::of)
        .collect();
        assert_eq!(attrs.len(), 6);
    }

    #[test]
    fn card_carries_the_dream() {
        let r = record(VoidPlatforms, 77, 5, 0.8, true);
        let c = card_from(&r);
        assert_eq!(
            (c.name.as_str(), c.description.as_str(), c.depth),
            (r.name.as_str(), r.whisper.as_str(), 5)
        );
        assert_eq!(c.attribute, Attribute::Void);
        assert_eq!(c.art, r.art);
    }

    #[test]
    fn numbering_continues_across_runs() {
        let mut b = Booklet::default();
        assert_eq!(
            b.add_run(
                1,
                &[
                    record(Lobby, 10, 0, 0.0, false),
                    record(Garden, 11, 1, 0.3, false)
                ]
            ),
            2
        );
        assert_eq!(b.add_run(2, &[record(Lobby, 20, 0, 0.0, false)]), 1);
        let numbers: Vec<u32> = b.cards().map(|c| c.number).collect();
        assert_eq!(numbers, vec![1, 2, 3]);
        assert_eq!(b.runs[0].deepest, 1);
        assert_eq!(b.card_count(), 3);
    }

    #[test]
    fn dreaming_the_same_dream_again_makes_it_recurring() {
        let mut b = Booklet::default();
        let r = record(Garden, 5, 2, 0.4, false);
        b.add_run(1, &[r.clone()]);
        b.add_run(2, &[r.clone()]);
        let first = &b.runs[0].cards[0];
        let again = &b.runs[1].cards[0];
        assert!(!first.recurring && again.recurring);
        assert_eq!(again.rarity, first.rarity.bumped());
    }

    #[test]
    fn a_run_is_only_pressed_once() {
        let mut b = Booklet::default();
        assert!(!b.has_run(9));
        b.add_run(9, &[record(Lobby, 1, 0, 0.0, false)]);
        assert!(b.has_run(9) && !b.has_run(10));
    }

    #[test]
    fn paging() {
        assert_eq!(page_count(0), 1);
        assert_eq!(page_count(3), 1);
        assert_eq!(page_count(4), 2);
        assert_eq!(page_range(7, 0), 0..3);
        assert_eq!(page_range(7, 2), 6..7);
        assert_eq!(page_range(7, 99), 6..7, "clamps to the last page");
        assert_eq!(page_range(0, 0), 0..0);
    }

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ds_booklet_{name}_{}.ron", std::process::id()))
    }

    #[test]
    fn save_then_load_round_trips() {
        let path = temp("rt");
        let mut b = Booklet::default();
        b.add_run(
            4,
            &[
                record(VoidPlatforms, 8, 3, 0.9, true),
                record(Awakening, 9, 4, 0.0, false),
            ],
        );
        save(&path, &b).unwrap();
        assert_eq!(load(&path), b);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_is_an_empty_booklet() {
        assert_eq!(load(Path::new("no/such/booklet.ron")), Booklet::default());
    }

    #[test]
    fn corrupt_booklet_is_moved_aside_not_destroyed() {
        let path = temp("bad");
        std::fs::write(&path, "this is not ron ((").unwrap();
        assert_eq!(load(&path), Booklet::default());
        let aside = path.with_extension("ron.corrupt");
        assert!(!path.exists() && aside.exists());
        assert_eq!(
            std::fs::read_to_string(&aside).unwrap(),
            "this is not ron (("
        );
        let _ = std::fs::remove_file(&aside);
    }
}
