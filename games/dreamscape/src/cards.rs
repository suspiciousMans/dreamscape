//! Dream trading cards and the booklet they're pressed into. Pure data + RON
//! persistence; drawing lives in `booklet_ui.rs`.
//!
//! Waking doesn't keep every dream: each one is *remembered* with a chance
//! that grows with its rarity, depth, whether you were lucid, and the
//! DEEP MEMORY perk. Dreams that fade leave Dream Dust behind instead.
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
    /// Two cards merged at the moth, or a dream that blended two themes.
    Fused,
    /// A fused card from one of the resonant pairs (`fusion::resonant`).
    Resonant,
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
            // Made, not rolled: a recurring dream doesn't change them.
            Rarity::Fused => Rarity::Fused,
            Rarity::Resonant => Rarity::Resonant,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Rarity::Faint => "FAINT",
            Rarity::Hazy => "HAZY",
            Rarity::Vivid => "VIVID",
            Rarity::Lucid => "LUCID",
            Rarity::Prophetic => "PROPHETIC",
            Rarity::Fused => "FUSED",
            Rarity::Resonant => "RESONANT",
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
    Hex,
    Tide,
    Aether,
    Glass,
    Spore,
    Abyss,
    Halo,
    Jest,
    Trace,
    Chord,
    Hour,
    Lumen,
    Gaze,
    Blank,
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
            DreamTheme::CursedForest => Attribute::Hex,
            DreamTheme::DrownedLibrary => Attribute::Tide,
            DreamTheme::SkyStairs => Attribute::Aether,
            DreamTheme::MirrorHall => Attribute::Glass,
            DreamTheme::MyceliumGrove => Attribute::Spore,
            DreamTheme::TheTunnel => Attribute::Abyss,
            DreamTheme::FractalCathedral => Attribute::Halo,
            DreamTheme::Elfworks => Attribute::Jest,
            DreamTheme::AfterimageFields => Attribute::Trace,
            DreamTheme::SynesthesiaHall => Attribute::Chord,
            DreamTheme::MeltingClockworks => Attribute::Hour,
            DreamTheme::JellyfishSky => Attribute::Lumen,
            DreamTheme::WatchingWallpaper => Attribute::Gaze,
            DreamTheme::WhiteDissolve => Attribute::Blank,
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
            Attribute::Hex => "HEX",
            Attribute::Tide => "TIDE",
            Attribute::Aether => "AETHER",
            Attribute::Glass => "GLASS",
            Attribute::Spore => "SPORE",
            Attribute::Abyss => "ABYSS",
            Attribute::Halo => "HALO",
            Attribute::Jest => "JEST",
            Attribute::Trace => "TRACE",
            Attribute::Chord => "CHORD",
            Attribute::Hour => "HOUR",
            Attribute::Lumen => "LUMEN",
            Attribute::Gaze => "GAZE",
            Attribute::Blank => "BLANK",
        }
    }
}

/// What happened in one dream of the current run (collected while playing).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// The second theme, when this dream was a fused one.
    #[serde(default)]
    pub blend: Option<DreamTheme>,
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
    /// The second dream in a fused card (colours and mood come from it).
    #[serde(default)]
    pub fused: Option<DreamTheme>,
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
    if r.blend.is_some() {
        rarity = Rarity::Fused;
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
        fused: r.blend,
    }
}

pub const CARDS_PER_PAGE: usize = 3;

/// Base chance to remember a dream of each rarity.
pub fn base_memory(r: Rarity) -> f32 {
    match r {
        Rarity::Faint => 0.25,
        Rarity::Hazy => 0.40,
        Rarity::Vivid => 0.60,
        Rarity::Lucid => 0.80,
        Rarity::Prophetic => 0.95,
        Rarity::Fused | Rarity::Resonant => 1.0,
    }
}

/// Everything that nudges the odds besides rarity.
#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryBoost {
    pub lucid_wake: bool,
    pub deep_memory: bool,
    /// Run upgrades (LUCKY MEMORY).
    pub extra: f32,
}

pub fn memory_chance(r: Rarity, depth: u32, boost: MemoryBoost) -> f32 {
    let mut p = base_memory(r) + 0.02 * depth.min(10) as f32;
    if boost.lucid_wake {
        p += 0.10;
    }
    if boost.deep_memory {
        p += crate::store::DEEP_MEMORY_BONUS;
    }
    p += boost.extra;
    p.min(1.0)
}

/// Dust a faded dream leaves behind.
pub fn fade_dust(r: Rarity) -> u32 {
    match r {
        Rarity::Faint => 3,
        Rarity::Hazy => 5,
        Rarity::Vivid => 8,
        Rarity::Lucid => 12,
        Rarity::Prophetic => 20,
        Rarity::Fused => 30,
        Rarity::Resonant => 45,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Recalled {
    pub card: Card,
    pub remembered: bool,
}

/// One slot in the run's pack: the card as it would be pressed, and whether
/// the memory held. The waking dream is always remembered: you're awake.
/// Deterministic per dream seed so the reveal can be replayed exactly.
pub fn recall(records: &[DreamRecord], boost: MemoryBoost) -> Vec<Recalled> {
    records
        .iter()
        .map(|r| {
            let card = card_from(r);
            let roll = StdRng::seed_from_u64(r.seed ^ 0x3E30_12AA).gen::<f32>();
            let remembered = r.theme == DreamTheme::Awakening
                || roll < memory_chance(card.rarity, r.depth, boost);
            Recalled { card, remembered }
        })
        .collect()
}

/// Cards until the booklet stops shaping packs for variety.
pub const FIRST_CARDS: usize = 5;

/// A new player's first cards teach different things: until the booklet
/// holds `FIRST_CARDS`, a remembered dream whose ability is already held
/// fades instead (the waking dream always stays). And the very first pack
/// always keeps one dream to build a loadout from, the Garden if it's there.
pub fn shape_first_pack(pack: &mut [Recalled], owned: &[Card]) {
    use crate::powers::power;
    let mut held: Vec<crate::upgrades::Ability> =
        owned.iter().map(|c| power(c.theme).active).collect();
    let mut count = owned.len();
    for r in pack.iter_mut().filter(|r| r.remembered) {
        if count >= FIRST_CARDS {
            break;
        }
        let a = power(r.card.theme).active;
        if held.contains(&a) && r.card.theme != DreamTheme::Awakening {
            r.remembered = false;
            continue;
        }
        held.push(a);
        count += 1;
    }
    let playable = |r: &Recalled| r.card.theme != DreamTheme::Awakening;
    if owned.is_empty() && !pack.iter().any(|r| r.remembered && playable(r)) {
        let pick = pack
            .iter()
            .position(|r| r.card.theme == DreamTheme::Garden)
            .or_else(|| pack.iter().position(playable));
        if let Some(i) = pick {
            pack[i].remembered = true;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SavedRun {
    pub run_seed: u64,
    pub deepest: u32,
    pub cards: Vec<Card>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Booklet {
    pub runs: Vec<SavedRun>,
    /// Dream Dust and Lucid Store purchases.
    #[serde(default)]
    pub stash: crate::store::Stash,
    /// Dreams and enemies met, ascension, daily bests.
    #[serde(default)]
    pub codex: crate::progress::Codex,
    /// The story: dreamers, notes found, loadout, prologue.
    #[serde(default)]
    pub lore: crate::lore::Lore,
}

impl Booklet {
    pub fn cards(&self) -> impl Iterator<Item = &Card> {
        self.runs.iter().flat_map(|r| r.cards.iter())
    }

    pub fn card_count(&self) -> usize {
        self.runs.iter().map(|r| r.cards.len()).sum()
    }

    pub fn card(&self, number: u32) -> Option<&Card> {
        self.cards().find(|c| c.number == number)
    }

    /// The cards this run brings: the saved loadout (cards no longer in the
    /// booklet drop out), up to the unlocked slots. With nothing chosen,
    /// the newest cards with different abilities fill the slots.
    pub fn loadout_cards(&self) -> Vec<Card> {
        let slots = self.lore.slots();
        let chosen: Vec<Card> = self
            .lore
            .loadout
            .iter()
            .filter_map(|&n| self.card(n).cloned())
            .take(slots)
            .collect();
        if !chosen.is_empty() {
            return chosen;
        }
        let mut out: Vec<Card> = Vec::new();
        for c in self.cards().collect::<Vec<_>>().into_iter().rev() {
            let a = crate::powers::power(c.theme).active;
            if out.len() < slots
                && !out
                    .iter()
                    .any(|o| crate::powers::power(o.theme).active == a)
            {
                out.push(c.clone());
            }
        }
        out
    }

    pub fn has_run(&self, run_seed: u64) -> bool {
        self.runs.iter().any(|r| r.run_seed == run_seed)
    }

    /// Presses the remembered dreams of a finished run into the booklet and
    /// banks the dust. Returns (cards added, dust earned).
    pub fn press(
        &mut self,
        run_seed: u64,
        pack: &[Recalled],
        shards: u32,
        magnet: bool,
    ) -> (usize, u32) {
        let deepest = pack.iter().map(|r| r.card.depth).max().unwrap_or(0);
        let known: HashSet<&str> = self.cards().map(|c| c.name.as_str()).collect();
        let new_names = pack
            .iter()
            .filter(|r| r.remembered && !known.contains(r.card.name.as_str()))
            .count() as u32;
        let dust = crate::store::run_dust(deepest, shards, new_names, magnet)
            + pack
                .iter()
                .filter(|r| !r.remembered)
                .map(|r| fade_dust(r.card.rarity))
                .sum::<u32>();
        let cards: Vec<Card> = pack
            .iter()
            .filter(|r| r.remembered)
            .map(|r| r.card.clone())
            .collect();
        let added = self.push_cards(run_seed, deepest, cards);
        self.stash.earn(dust);
        (added, dust)
    }

    /// Presses every dream (no fading).
    #[cfg(test)]
    pub fn add_run(&mut self, run_seed: u64, records: &[DreamRecord]) -> usize {
        let deepest = records.iter().map(|r| r.depth).max().unwrap_or(0);
        let cards = records.iter().map(card_from).collect();
        self.push_cards(run_seed, deepest, cards)
    }

    fn push_cards(&mut self, run_seed: u64, deepest: u32, cards: Vec<Card>) -> usize {
        // Max, not count: merging removes cards, and numbers never repeat.
        let mut next = self.cards().map(|c| c.number).max().unwrap_or(0) + 1;
        let mut seen: HashSet<String> = self.cards().map(|c| c.name.clone()).collect();
        let cards: Vec<Card> = cards
            .into_iter()
            .map(|mut c| {
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

pub fn booklet_path() -> PathBuf {
    crate::dev::var_os("DREAMSCAPE_BOOKLET")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::paths::save_file("booklet.ron"))
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
pub(crate) mod tests {
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
            blend: None,
        }
    }

    #[test]
    fn loadout_keeps_known_cards_and_fills_itself_when_empty() {
        let mut b = Booklet::default();
        b.add_run(
            1,
            &[
                record(TheTunnel, 1, 1, 0.1, false),
                record(SynesthesiaHall, 2, 2, 0.1, false),
                record(Garden, 3, 3, 0.1, false),
            ],
        );
        // One slot at first: the newest card.
        assert_eq!(
            b.loadout_cards()
                .iter()
                .map(|c| c.number)
                .collect::<Vec<_>>(),
            vec![3]
        );
        b.lore.loadout = vec![99, 1];
        assert_eq!(
            b.loadout_cards()
                .iter()
                .map(|c| c.number)
                .collect::<Vec<_>>(),
            vec![1]
        );
        b.lore.loadout = vec![99];
        assert_eq!(
            b.loadout_cards().len(),
            1,
            "a vanished card falls back to the default"
        );
        assert!(Booklet::default().loadout_cards().is_empty());
    }

    fn recalled(theme: DreamTheme, seed: u64, remembered: bool) -> Recalled {
        Recalled {
            card: card_from(&record(theme, seed, 1, 0.1, false)),
            remembered,
        }
    }

    #[test]
    fn the_first_pack_always_keeps_a_dream_to_play_with() {
        let mut pack = vec![
            recalled(Lobby, 1, false),
            recalled(Garden, 2, false),
            recalled(Awakening, 3, true),
        ];
        shape_first_pack(&mut pack, &[]);
        assert!(pack[1].remembered, "the Garden is kept");
        assert!(!pack[0].remembered);
    }

    #[test]
    fn the_first_five_cards_teach_different_abilities() {
        // TheTunnel and SynesthesiaHall both teach DASH.
        let mut pack = vec![
            recalled(TheTunnel, 1, true),
            recalled(SynesthesiaHall, 2, true),
            recalled(Garden, 3, true),
            recalled(Awakening, 4, true),
        ];
        shape_first_pack(&mut pack, &[]);
        let kept: Vec<_> = pack
            .iter()
            .filter(|r| r.remembered)
            .map(|r| r.card.theme)
            .collect();
        assert_eq!(kept, vec![TheTunnel, Garden, Awakening]);
        // Once five are held, duplicates are kept again.
        let owned: Vec<Card> = [Lobby, LiminalOffice, VoidPlatforms, Garden, TheTunnel]
            .iter()
            .enumerate()
            .map(|(i, &t)| card_from(&record(t, i as u64, 1, 0.1, false)))
            .collect();
        let mut pack = vec![recalled(SynesthesiaHall, 9, true)];
        shape_first_pack(&mut pack, &owned);
        assert!(pack[0].remembered);
    }

    #[test]
    fn fused_and_resonant_are_never_rolled() {
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..10_000 {
            let r = Rarity::from_score(rng.gen_range(-0.5..2.0));
            assert!(!matches!(r, Rarity::Fused | Rarity::Resonant), "{r:?}");
        }
        assert_eq!(Rarity::Fused.bumped(), Rarity::Fused);
        assert_eq!(Rarity::Resonant.bumped(), Rarity::Resonant);
        assert_eq!(base_memory(Rarity::Fused), 1.0);
        assert_eq!(fade_dust(Rarity::Resonant), 45);
    }

    #[test]
    fn a_blended_dream_presses_a_fused_card_that_round_trips() {
        let mut r = record(Garden, 3, 4, 0.5, true);
        assert_eq!(card_from(&r).fused, None);
        r.blend = Some(MyceliumGrove);
        let c = card_from(&r);
        assert_eq!(c.rarity, Rarity::Fused);
        assert_eq!(c.fused, Some(MyceliumGrove));
        let text = ron::to_string(&c).unwrap();
        assert_eq!(ron::from_str::<Card>(&text).unwrap(), c);
    }

    #[test]
    fn old_booklets_load_with_empty_lore() {
        let b: Booklet = ron::from_str("(runs: [])").unwrap();
        assert!(b.lore.notes.is_empty());
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

    #[test]
    fn memory_odds_rise_with_rarity_depth_lucidity_and_the_perk() {
        let none = MemoryBoost::default();
        let all = MemoryBoost {
            lucid_wake: true,
            deep_memory: true,
            extra: 0.0,
        };
        let mut prev = 0.0;
        for r in [
            Rarity::Faint,
            Rarity::Hazy,
            Rarity::Vivid,
            Rarity::Lucid,
            Rarity::Prophetic,
        ] {
            let p = memory_chance(r, 0, none);
            assert!(p > prev, "{r:?}");
            prev = p;
            assert!(memory_chance(r, 8, none) >= p);
            assert!(memory_chance(r, 0, all) >= p);
            assert!(memory_chance(r, 99, all) <= 1.0);
        }
    }

    #[test]
    fn some_dreams_fade_and_waking_never_does() {
        let records: Vec<DreamRecord> = (0..60)
            .map(|s| {
                record(
                    if s % 6 == 5 { Awakening } else { Garden },
                    s,
                    (s % 6) as u32,
                    0.3,
                    false,
                )
            })
            .collect();
        let pack = recall(&records, MemoryBoost::default());
        let kept = pack.iter().filter(|r| r.remembered).count();
        assert!(kept > 10 && kept < 55, "kept {kept}/60");
        for r in &pack {
            if r.card.theme == Awakening {
                assert!(r.remembered);
            }
        }
        assert_eq!(
            pack,
            recall(&records, MemoryBoost::default()),
            "reveal replays exactly"
        );
        let boosted = recall(
            &records,
            MemoryBoost {
                lucid_wake: true,
                deep_memory: true,
                extra: 0.0,
            },
        );
        assert!(boosted.iter().filter(|r| r.remembered).count() >= kept);
    }

    #[test]
    fn pressing_keeps_the_remembered_and_turns_the_rest_into_dust() {
        let records = [
            record(Lobby, 1, 0, 0.0, false),
            record(Garden, 2, 1, 0.3, true),
            record(Awakening, 3, 2, 0.0, false),
        ];
        let mut pack = recall(&records, MemoryBoost::default());
        pack[0].remembered = false;
        pack[1].remembered = true;
        let mut b = Booklet::default();
        let (added, dust) = b.press(7, &pack, 1, false);
        assert_eq!(added, 2);
        assert_eq!(
            dust,
            crate::store::run_dust(2, 1, 2, false) + fade_dust(pack[0].card.rarity)
        );
        assert_eq!(b.stash.dust, dust);
        assert_eq!(b.cards().map(|c| c.number).collect::<Vec<_>>(), vec![1, 2]);
        assert!(b.has_run(7));
    }

    #[test]
    fn old_booklets_without_a_stash_still_load() {
        let old = "(runs: [(run_seed: 1, deepest: 0, cards: [])])";
        let b: Booklet = ron::from_str(old).unwrap();
        assert_eq!(b.stash, crate::store::Stash::default());
    }
}
