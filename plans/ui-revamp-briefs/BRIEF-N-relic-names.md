# Brief N: relic names (highly randomised, role-fitting)

Goal: `relic_names::name(&RelicSpec) -> String` and `flavour(...)`; deterministic per seed, near-unique, always reads like the item it is.

## Scope
- New `src/relic_names.rs` (pure) + `mod relic_names;` (`#[allow(dead_code)] // R6` until wired). Reads `crate::dream::vocab::{vocab, ABSTRACTS, CODAS}` (grep `pub fn vocab` / make `vocab` module items `pub(crate)` if private; that is the only edit outside your file) and `relics::{wear_tier, WearTier}`.
- Leave alone: `relics.rs` logic, UI, art.

## Types
```rust
pub struct RelicSpec<'a> { pub theme: DreamTheme, pub seed: u64, pub rarity: RelicRarity, pub wear: WearTier, pub foil: bool, pub shape: Shape, pub effect: RelicEffect, pub taken: &'a [String] }
pub const MAX_NAME: usize = 40; pub const MAX_FLAVOUR: usize = 56;
pub fn name(s: &RelicSpec) -> String;      // UPPERCASE
pub fn flavour(s: &RelicSpec) -> String;   // lowercase whisper
pub fn head_nouns(shape: Shape) -> &'static [&'static str];   // 8-12 per shape
pub fn effect_hint(e: &RelicEffect) -> Option<&'static [&'static str]>;  // strongest lever -> word list
```

## Design (see PLAN-ui-revamp.md "Relic naming design")
- Head noun from `shape` (the role): Crescent SICKLE/HORN/SMILE/PARING..., Cube DIE/RELIQUARY/BRICK..., Torus HOOP/HALO/RING..., etc. All 11 shapes.
- Modifiers from the theme's own vocab (adjectives, abstracts, relatives); 25% mashup borrows one slot from a donor theme (same `MASHUP_CHANCE` rule as `dream/names.rs`).
- Rarity chooses template family: Cracked plain/ruined; Worn "THE {ADJ} {NOUN}"; Pristine "{NOUN} OF {ABSTRACT}"; Mythic grand ("{ABSTRACT}'S {ADJ} {NOUN}", "THE {NOUN} THAT {REL}").
- Wear prefix for Heavy/Ruined (`HEAVILY-WORN`, `RUINED`); foil adds a sheen word (GILT, PRISMATIC, ...); strongest effect lever adds a suffix/prefix word hinting its power (dust OF PLENTY, speed FLEETING, calm HUSHED, memory REMEMBERING, rerolls SECOND-GUESSING, shards SHARD-BORN, grace FORGIVING, max_nerve STEADY).
- Uniqueness: seeded retry with salt counter against `taken` (12 tries), then numeral suffix.
- Must fit `MAX_NAME`: choose among template outputs by length, never truncate mid-word.
- Flavour: `{subject} {predicate}` from theme vocab plus shape-aware fragments ("it hums when held", ...); rarity-weighted.
- Every word list non-empty and all-caps/ASCII-safe for the pixel font (check the font glyph set used by `pixel_ui`/dream names; avoid unsupported punctuation beyond ' and -).

## Tests
- Determinism; shape word (or listed synonym) always present in `name`; len <= MAX_NAME for 20k seeds x all themes x all rarities; unique rate >= 97% per (theme, rarity) over 5000 seeds; names with `taken` never collide; every `Shape` x `effect` lever has a non-empty list; wear prefix present for Ruined; Cracked and Mythic template families differ (sample test); charset test.
- Print 40 sample names per rarity into `NOTES-names.md` ("for approval": the word lists, not outputs).
