# Plan: Relic System

Status: draft for sign-off. Nothing here is built yet.

## 1. What this adds

A **relic** is a randomized permanent item gained from a CS:GO-style case opening. Each relic has:
- a deterministic seeded texture built from the existing `TexSpec` system,
- a rarity (`Cracked / Worn / Pristine / Mythic`),
- a run-scoped buff mapped onto existing `QuirkEffect` levers,
- an optional **visible weapon mesh** attached to the dreamer during the run, using the game's existing low-poly procedural shapes,
- cosmetic trail / glow / sound hooks that plug into `combat.rs`, `juice.rs`, and `sounds.rs`.

Odds are shown exactly. Pity is enforced. Nothing is shared with the card lottery. Old saves load unchanged.

## 2. What already exists that we can reuse

| System | What we borrow |
|---|---|
| `lottery.rs` | Pull structure: `state + seed + featured` → deterministic result. Exact shown odds, soft pity, hard pity, foil. |
| `retention.rs` | `press_pull` becomes the model for `press_relic`. |
| `TexSpec` / `pack_art.rs` | Seeded palette, pattern, bands, foil. We add a relic wear mask and an etch glyph on top, no new renderer work. |
| `dream/meshes.rs` | Procedural low-poly shapes (`Cube`, `Octahedron`, `Orb`, `Cone`, `Torus`, `Crescent`, `Island`, `Cloud`, `Arch`, `Stairs`). These are the weapon models — no external assets, flat-shaded, PS2 look. |
| `combat.rs` | `Strike` already has reach, arc, knockback, stagger, damage mults. Relic weapons plug into these constants. |
| `juice.rs` | Hit-stop dip, shake, spark particles, ring ripple. Weapon cosmetics reuse these. |
| `sounds.rs` | One new `Sound` variant per weapon tier for the strike/clash. |
| `store.rs` / `Stash` | Cosmetic equip pattern (`owned`, `equipped`). Relics are permanent and shown in a new pouch tab. |

## 3. Currency: Ash

Ash is a new field on `Stash`:

```rust
#[serde(default)]
pub ash: u32,
```

Ash is **hard to earn**. Sources:
- **+1 Ash per completed dream** (scales slightly with depth: +1 extra per 3 depth, max +2 bonus).
- **+3–5 Ash for beating a nightmare** (scales with nightmare tier).
- Ash is **never** given by tickets, streaks, fortune, or stores.
- Ash is **not** dust. You cannot convert between them.

## 4. Relic data model

```rust
// relics.rs
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Relic {
    pub id: u32,                // booklet-wide, 1-based
    pub theme: DreamTheme,
    pub seed: u64,
    pub rarity: RelicRarity,
    pub foil: bool,
    pub name: String,
    pub flavour: String,
    pub effect: RelicEffect,
    pub art: TexSpec,
    pub weapon: Option<Weapon>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelicRarity { Cracked, Worn, Pristine, Mythic }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum RelicEffect {
    Buff(RelicBuff),
    WeaponTag(WeaponTag),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RelicBuff {
    pub dust_bonus_pct: u32,
    pub speed_pct: u32,
    pub grace_bonus: u32,
    pub calm_pct: u32,
    pub start_shards: u32,
    pub free_rerolls: u32,
    pub memory_bonus: f32,
    pub max_nerve: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponShape {
    Cube, Octahedron, Orb, Cylinder, Cone,
    Torus, Crescent, Island, Cloud, Arch, Stairs,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Weapon {
    pub shape: WeaponShape,
    pub scale: f32,
    pub tint: [u8; 3],
    pub trail: Option<TrailStyle>,
    pub clash: ClashStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrailStyle { Ray, Spark, Glow }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClashStyle {
    pub sound: Sound,
    pub flash_color: [u8; 3],
    pub hit_stop_secs: f32,
    pub knock_add: f32,
}
```

`RelicBuff` maps 1:1 onto `QuirkEffect` levers so `retention::card_effects` can sum them with one `.plus()`. New levers (`max_nerve`) get new fields in `QuirkEffect`.

## 5. Relic pull system

`Booklet` gains:

```rust
pub relics: Vec<Relic>,
pub relic_lottery: RelicState,
```

`RelicState` tracks only relic-specific counters:

```rust
pub struct RelicState {
    pub total_pulls: u32,
    pub since_mythic: u32,
    pub since_pristine: u32,
    pub ash_spent: u32,
}
```

### Per-rarity pull costs

```rust
pub const RELIC_COST: [u32; 4] = [40, 60, 80, 120]; // Cracked..Mythic
```

A pull always costs the amount for the **lowest rarity** it can possibly yield, shown on the button. The player spends that Ash and the rarity is rolled normally.

### Rarity odds (independent of card rarity)

```rust
pub const BASE_RELIC_ODDS: [f32; 4] = [0.50, 0.30, 0.17, 0.03]; // Cracked..Mythic
pub const RELIC_FOIL_ODDS: f32 = 1.0 / 36.0;
pub const SOFT_PITY_PR: u32 = 14;   // Pristine+ chance ramps
pub const SOFT_PRY_STEP: f32 = 0.05;
pub const HARD_PRY_MYTHIC: u32 = 24; // Mythic guaranteed
pub const FEATURED_RELIC_THEME_WEIGHT: u32 = 2;
```

Draw order: rarity → theme → foil → wear → etch → weapon shape.

Theme selection mirrors `lottery::roll_theme` but uses its own featured weight. A relic's theme is mostly cosmetic; it tints the weapon mesh and the texture palette.

## 6. Deterministic appearance (the CS:GO part)

All randomness comes from `seed ^ salt` via the existing `mix` / `percent_roll` helpers in `worth.rs`.

### Texture

Reuse `TexSpec { pattern, palette, bands, seed }` from `pack_art.rs`. Add two relic-specific layers computed at draw time:

1. **Wear mask** (`WEAR_SALT`): selects one of four wear tiers. Higher tiers increase noise/spatter on one palette slot and dim the base pattern. `Cracked` is most worn; `Mythic` is pristine.
2. **Etch glyph** (`ETCH_SALT`): picks one of 16 glyph shapes (small procedural marks — lines, dots, arcs — drawn in `relic_art.rs` using the same pixel-grid helpers as `pack_art.rs`). The glyph is overlaid in the relic's accent colour at low alpha.

Everything is computed from `seed`, so the same relic always looks identical — no serialization needed beyond `seed`.

### Weapon mesh

Weapon models are drawn as a child entity of the dreamer with a `Transform` offset to the right hand. The mesh is one of the procedural shapes from `dream/meshes.rs`, tinted by the relic's theme accent, scaled to ~0.15–0.35 world units.

`Mythic` weapons get a second tiny orbiting mesh (an `Orb` or `Crescent`) that floats and rotates — purely cosmetic, no gameplay effect.

Mesh and texture are generated once when the relic is first equipped for the run and cached in a `RelicArtCache` keyed by `(WeaponShape, seed)`.

## 7. Buffs

Every buff plugs into existing run systems:

| Buff lever | Existing consumer |
|---|---|
| `dust_bonus_pct` | `run_dust` in `store.rs` |
| `speed_pct` | player stride multiplier |
| `grace_bonus` | respawn grace beats |
| `calm_pct` | enemy aggression/speed |
| `start_shards` | run init |
| `free_rerolls` | upgrade offer rerolls |
| `memory_bonus` | `memory_chance` |
| `max_nerve` | nerve cap in `nerve.rs` |

New relic-only buffs can extend `RelicBuff` later; the first pass stays inside these levers so nothing breaks.

## 8. Weapon effects

Weapons modify the existing strike pipeline in `combat.rs`:

| Effect | Implementation |
|---|---|
| `+1 damage` | `DAMAGE` const becomes `1 + weapon_damage_bonus` |
| Wider arc | `ARC_COS` is relaxed while the weapon is active |
| Longer reach | `REACH` multiplied by weapon reach bonus |
| Extra knockback | `KNOCKBACK` additive bonus |
| Trail / slash shape change | `slash_ray` and `slash_spark` in `combat.rs` read an optional `WeaponStyle` from run state and draw relic-specific rays/sparks |
| Clash sound | One new `Sound` per weapon tier, played on impact |
| Hit-stop tweak | `juice::Hit::Strike` dip floor / duration overridden by `ClashStyle.hit_stop_secs` |

The weapon is equipped at run start from the player's equipped relic (or the highest-rarity owned relic if none is equipped). **Only one weapon mesh is drawn at a time.**

## 9. UI

New `relic_ui.rs` module, wired under `// === relics ===` in `main.rs`.

### Pouch
- Scrollable list of owned relics.
- Each row shows: relic icon (small `TexSpec` preview), name, rarity label, flavour.
- Equip button sets the run weapon.

### Pull screen
- Centered case (painted with `pixel_ui` helpers, chunky grid, dither shading).
- Cost: **40 Ash for Cracked, 60 Ash for Worn, 80 Ash for Pristine, 120 Ash for Mythic** (the button shows the cost for the roll being attempted).
- Exact odds breakdown by rarity, plus foil chance.
- Pity counters shown ("Mythic in 3 pulls").
- Pull animation: case shakes, opens, relic card slides out with foil sweep if foil.

### Where it lives
**A new tab in the existing moth shop** (`shop_ui.rs`), beside Tickets and Abilities.

## 10. Tests required

- `relics.rs`: deterministic pull for `(state, seed, odds)`, pity hits exact pulls, odds sum to 1, wear/foil/etch deterministic.
- `retention.rs`: `press_relic` appends without corrupting booklet; old saves without `relics`/`ash` load empty.
- `screen_smoke`: relic pouch and pull screen at 640x360 and 1280x720.
- `combat.rs` / `juice.rs`: weapon damage bonus, arc/reach multipliers, trail style, hit-stop override all unit-tested.

## 11. Milestones

| # | Milestone | Size | Depends on |
|---|---|---|---|
| R0 | Sign-off + briefs in `plans/relic-briefs/` | S | you |
| R1 | **Ash + Booklet**: `ash` field on `Stash`, earn rules, save compat, `relics` vec on `Booklet`, `press_relic` | M | R0 |
| R2 | **Relic pull engine**: `relics.rs` pure module (odds, pull, pity, foil, wear, etch), tests | M | R1 |
| R3 | **Texture + art**: `relic_art.rs` (wear mask, etch glyph, foil sweep on `TexSpec`), `PackArt`-style cache | M | R2 |
| R4 | **Buffs**: `RelicBuff` mapped onto `QuirkEffect`, summed in `retention::card_effects`, nerve cap hook | M | R2 |
| R5 | **Weapon meshes + combat hooks**: `dream/meshes.rs` enum already exists; add `weapon_mesh(shape, tint, scale)` builder, attach to dreamer entity, plug damage/arc/reach/trail/sound into `combat.rs` and `juice.rs` | L | R2 |
| R6 | **UI**: `relic_ui.rs` (pouch + pull screen), moth shop tab, equip flow | M | R3 |
| R7 | **Polish**: new `Sound` variants, `screen_smoke`, settings audit, `audit_text_fits_every_screen`, balance pass | M | R4–R6 |
| R8 | **Bug-testing round**: full tests, clippy, screenshots of every rarity/weapon combo, save compat, autopilot seeds | S | R7 |

Dispatch: R1 and R2 go first, one stream each. After R2, R3, R4, and R5 run **in parallel** as subagents with their own worktrees. R6 waits on R3 (art cache). R7 and R8 after.

## 12. Decisions locked in

1. **Pull cost:** per-rarity: Cracked 40 Ash, Worn 60 Ash, Pristine 80 Ash, Mythic 120 Ash.
2. **Ash earn:** +1 Ash per completed dream (+1 bonus per 3 depth, max +2); +3–5 Ash per beaten nightmare.
3. **Weapon cap:** exactly one equipped weapon at a time.
4. **Relic UI location:** moth shop tab.
5. **Mythic orbits:** tiny floating cosmetic mesh, kept.
6. **Approval list:** relic names and flavour text reviewed before merge.

---

*Decisions are locked. Next step: write briefs in `plans/relic-briefs/` and spin up R1 + R2.*
