# Brief: Relic Pull Engine (R2)

Goal: implement the pure `relics.rs` module with deterministic pulls, pity, foil, wear, and etch.

## Scope
- New file `games/dreamscape/src/relics.rs`.
- No GL, no clock, no global RNG. Pure functions of `(RelicState, seed, featured)`.

## Constants
```rust
pub const BASE_RELIC_ODDS: [f32; 4] = [0.50, 0.30, 0.17, 0.03];
pub const RELIC_FOIL_ODDS: f32 = 1.0 / 36.0;
pub const SOFT_PITY_PR: u32 = 14;
pub const SOFT_PRY_STEP: f32 = 0.05;
pub const HARD_PRY_MYTHIC: u32 = 24;
pub const FEATURED_RELIC_THEME_WEIGHT: u32 = 2;
pub const RELIC_COST: [u32; 4] = [40, 60, 80, 120];
```

## Types
```rust
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RelicState { total_pulls: u32, since_mythic: u32, since_pristine: u32, ash_spent: u32 }

#[derive(Clone, Debug, PartialEq)]
pub struct RelicPull { pub rarity: RelicRarity, pub theme: DreamTheme, pub foil: bool, pub pity: bool, pub near_miss: Option<RelicRarity>, pub cost: u32 }
```

## Functions
- `pullable_themes()` — all themes except Awakening.
- `featured_relic_theme(day: u64) -> DreamTheme` — stable per day, cycling.
- `rarity_tables(state: &RelicState) -> ([f32; 4], [f32; 4])` — natural and final odds with soft/hard pity.
- `pick_index(table: &[f32; 4], u: f32) -> usize`.
- `roll_theme(rng, featured) -> DreamTheme` — weighted by `FEATURED_RELIC_THEME_WEIGHT`.
- `pull(state: &mut RelicState, seed: u64, featured: Option<DreamTheme>) -> RelicPull` — rarity → theme → foil.
- `odds(state: &RelicState, featured: Option<DreamTheme>) -> RelicOdds` — exact shown probabilities.
- `cost_for(rarity: RelicRarity) -> u32` — from `RELIC_COST`.

## Seeded appearance helpers (no GL)
- `wear_tier(seed) -> WearTier` via `percent_roll(seed, WEAR_SALT)`.
- `etch_glyph(seed) -> u8` via `percent_roll(seed, ETCH_SALT)`.
- These are consumed by `relic_art.rs` (R3); this brief only defines the pure functions.

## Tests
- 100 pulls hit hard pity at exactly 24 for Mythic.
- Odds sum to 1.0 at every call.
- Featured theme gets its weight bump.
- Foil is independent of rarity.
- Same `(state, seed, featured)` always returns the same pull.
- Old `RelicState` loads from empty RON.

## Approval list
- No new player-facing text in this brief.
