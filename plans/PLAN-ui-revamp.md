# Plan: UI revamp (warping cards, dissolving death, input prompts, relic pipeline)

Status: decisions locked, briefs written in `plans/ui-revamp-briefs/`. Nothing is built; waiting for the owner's go to dispatch. Written 2026-11-10 after Q&A with the owner.

## Locked by the owner (2026-11-10)

1. **Relics:** finish the whole pipeline R3-R8 (only R1 Ash + R2 pull engine are merged) and fold it into this revamp.
2. **Card warp:** everywhere cards appear (hand, booklet, pack reveal, merge, loadout), one shared effect. Look: slow liquid wobble/melt like the world's `uMelt`/`uStrangeness`, always on and subtle, stronger on hover/selection. Each card warps from its own dream theme.
3. **Death:** the dreamer dissolves into drifting particles with an edge-glow burn (noise threshold), then the summary/wake screen fades in.
4. **Input prompts:** context-only. A glyph appears (pulses/bobs) only when an action is available or required; gamepad glyphs swap automatically via `pad.rs`.
5. **Juice level:** keep the 2026-10-05 soft rule everywhere: dreamy, gentle, short (hit-stop ~40 ms, slow-mo >= 0.75x). "Balatro" here means smoothness, ease curves, tilt, hover response, number tick-ups, not arcade punch.
6. **Scope of screens:** all: title, loadout, hand/HUD, booklet, merge, shop, pack reveal, lottery, summary, upgrade picks, settings.
7. **Relic visibility:** first-person held weapon model, plus the mesh in the pouch/pull screen.
8. **Relic art/reveal:** same warp family plus wear mask, etch glyph, foil sweep, and a case-opening reveal with Balatro-style beats (soft per #5).
9. **Accessibility:** everything respects `reduced_motion` and `reduce_flashing`, plus a new setting **UI juice: Off / Soft / Full**. Default Soft. Full still stays inside rule #5's ceilings (it only lengthens/enlarges, never goes arcade).

## What the code audit found (grep, not memory)

- Cards are painted in **egui** as textured meshes (`hand.rs:465 Mesh::with_texture`, `booklet_ui.rs` and `pack_art.rs` cache `egui::TextureHandle`s from `card.art.rgba()`), not by the GL mesh shader. So the warp is a **CPU vertex-grid UV/position displacement of an egui Mesh**, not GLSL. No new shader flag bit, no `ShaderVariantCache::ALL_FLAGS` change, no ~55 ms variant hitch.
- The player has a body (`Hero`, `main.rs:186`) but it is **hidden in first-person dreams** (`first_person_active`, `FpvDreams`). A mesh dissolve on `Hero` is invisible there. Death needs two paths (see M2).
- "Death" today is the **caught flow** (`main.rs ~12940`): flash, `Sound::Caught`, shard drop, respawn; the run ends in `hud::Mode::Summary`. `nerve::Hit::Down` feeds it.
- `dissolve.rs` already exists and is the **White Dissolve floor tiles** mechanic. Do not reuse that name: the new module is `death_fx.rs`.
- Input hints exist in pieces: `pad.rs` has `PROMPTS`/`key_for`/`button_label`; `hud.rs` has "a key prompt as the player's device would say it". There is no single "required action" system.
- `juice.rs` already has hit-stop dip, shake, pitch chain, ring ripple, all pure and `reduced`-aware. New UI easing should live beside it, not replace it.
- Budget guard: `every_screen_stays_inside_its_ui_budget` caps shapes/triangles. A warped card is a grid mesh (e.g. 8x10 = 160 triangles) per visible card; hand (5) + a booklet page (up to ~12 visible) must fit, so grid density scales with on-screen card size and juice level.
- Relic plan `PLAN-relics.md` and `relic-briefs/` exist; `relics.rs` (352 lines) and `relic_types.rs` exist, **no relic UI, art, buffs, weapon, or shop tab exist**. Note `relic_types::Weapon { shape: u8, scale_pct: u8 }` is thinner than `PLAN-relics.md` section 4 (`tint`, `trail`, `clash`); reconcile in M6.

## Milestones

| # | Milestone | Size | Depends on | Owns (new file first) |
|---|---|---|---|---|
| M0 | **Shared UI-juice core**: `ui_motion.rs` (pure): ease curves (out-back, smoothstep, spring), `Juice level` enum (Off/Soft/Full) with scale factors, hover/press state with spring, number tick-up schedule. Setting `ui_juice` (`#[serde(default)]`, `ALL_ACTIONS`/rows test updates). | M | you | `ui_motion.rs`, `settings.rs`, `settings_ui.rs` |
| M1 | **Card warp**: `card_warp.rs` (pure displacement field + egui Mesh builder). Inputs: `(theme, seed, t, hover 0..1, size)` -> displaced grid. Theme sets frequency/amplitude/direction so a Melt-type dream sags and a Weeper-type ripples. Wire into `hand.rs`, `booklet_ui.rs`, `reveal_ui.rs`, `merge_ui.rs`, `loadout_ui.rs`, `deck_ui.rs`. Warp amplitude clamped so text stays readable: **text/name plate is drawn flat, only the art is warped**. | L | M0 | `card_warp.rs` + one-line swaps in each card painter |
| M2 | **Death dissolve**: `death_fx.rs` (pure timeline: dissolve 0..1, burn-edge colour from the dream theme, particle seeds). Third-person: noise-threshold discard in `mesh.frag` via a `uDissolve` uniform on `Hero` (same program, uniform not flag). First-person: the held hands/relic dissolve and the view desaturates/embers fade, since there's no body to see. Then fade to Summary. Reduced-motion = short fade, no particles. | M | M0 | `death_fx.rs`, `mesh.frag`, `main.rs` caught block marker `// === deathfx ===` |
| M3 | **Input prompts**: `prompts.rs` (pure): a `Prompt { action, target?, state: Available / Required / Cooldown }` list computed from run state each frame (door reachable, shard in grab range, card ready, strike ready, pack unopened, choice pending). HUD draws a glyph chip with a bob/pulse; `Required` pulses faster and dims the rest slightly. Glyph text comes only from `pad::button_label` / `settings::key_for`, so a prompt can never name a wrong key. Show on menus too: the focused action's key chip pulses. | M | M0 | `prompts.rs`, `hud.rs` |
| M4 | **Screen pass** (parallel, one stream per group, all consume M0/M1/M3): title+settings; loadout+deck+booklet; shop+tickets+lottery; reveal+merge; summary+upgrade picks. Per screen: eased entrances, hover spring, tick-up numbers, focused-action chip, `screen_smoke` at 640x360 and 1280x720. | L | M1, M3 | per-screen files |
| R3 | **Relic art**: `relic_art.rs` wear mask + etch glyph + foil sweep on the shared warp; texture cache keyed `(theme, seed)`. | M | M1 | `relic_art.rs` |
| R4 | **Relic buffs**: `RelicEffect::to_quirk` summed in `retention::card_effects`; `max_nerve` hook in `nerve.rs`. | M | - | `retention.rs`, `nerve.rs` |
| R5 | **Relic weapons + combat hooks**: first-person held mesh (`dream/meshes.rs` builder), `Mythic` orbit mesh, damage/arc/reach/knockback/trail/clash into `combat.rs`/`juice.rs`, new `Sound`s (`ALL_SOUNDS` bump). Held mesh goes through the same `uDissolve` as death (M2) so it vanishes with the dreamer. | L | R2, M2 | `combat.rs`, `juice.rs`, `sounds.rs`, `main.rs` marker `// === relics ===` |
| R6 | **Relic UI**: `relic_ui.rs` pouch + pull/case-opening screen (soft Balatro beats: case shake, lid pop, card slides out with eased flip, foil sweep, rarity-scaled hold), moth shop tab, equip flow. Pity counters and exact odds on screen. | L | R3, M1 | `relic_ui.rs`, `shop_ui.rs` |
| N | **Relic names**: `relic_names.rs` grammar generator (see "Relic naming design") | M | R2 | `relic_names.rs` |
| R7 | Polish: sounds, `audit_text_fits_every_screen`, settings audit, balance. | M | all | - |
| R8 | **Bug-testing round**: full suite, clippy, screenshots of every screen and every rarity, save compat, autopilot seeds. | S | R7 | - |

Parallelism (owner prefers full parallel dispatch): M0 first (small, blocks the rest), then M1, M2, M3, R4 in parallel in separate worktrees. After M1/M3 land: M4 streams + R3 + R5 in parallel. R6 after R3. R7/R8 last. Max three streams with `CARGO_BUILD_JOBS=2` each (OOM guard).

## Data model (minimal)

- `Settings.ui_juice: UiJuice { Off, Soft, Full }` with `#[serde(default)]`, default `Soft`. New `Action`/`Row` means `ALL_ACTIONS` length and the `rows().len()` assert, and a row-height font floor check in `settings_ui.rs`.
- `Weapon` (relic_types) grows `tint: [u8;3]`, `trail: u8`, `clash: u8` with `#[serde(default)]`. Each new field has a marker block if more than one stream edits `Booklet`.
- No new persisted state for warp, death or prompts: all run/frame-scoped.

## Hook points

- Hand: `hand.rs::draw` mesh at line ~465 (replace the quad with `card_warp::mesh`).
- Booklet/merge/loadout/reveal: their existing `TextureHandle` caches; swap the quad call only.
- Caught: `main.rs` caught block (~12940), before respawn; summary entry at `Mode::Summary` set (~9533). Death must delay the mode switch by the dissolve length, with input locked and a skip on any key.
- Prompts: `hud.rs` view struct (`HudView`) gains `prompts: Vec<Prompt>`; update `screen_smoke` fixtures.
- Relic Ash earn: already at `main.rs:13168` (`// === relics ===`).

## Tests and save compat

- Pure: warp field is deterministic per `(seed, t)`, zero at amplitude 0, bounded displacement; death timeline monotone, ends at 1.0, reduced-motion path has no particles; prompt state machine (available -> required -> cooldown) and "prompt names the bound key"; `ui_motion` easing endpoints and monotonicity; Off level yields identity everywhere.
- UI budget: `every_screen_stays_inside_its_ui_budget` at both resolutions with the hand full and a full booklet page, at Full juice.
- `screen_smoke` for every touched screen at 640x360 and 1280x720; `audit_text_fits_every_screen`.
- Old saves: `ui_juice`, `Weapon` additions all `#[serde(default)]`; test loads a pre-change `settings.ron` and booklet.
- Verification the subagents cannot do (no display): parent session screenshots every touched screen via `DREAMSCAPE_SCREEN` and `DREAMSCAPE_SHOT`; mid-animation stills with `SHOT_AT` inside effect life (death at `caught + 0.4 s`); animations themselves stay unverifiable by a still, so unit timings carry them.

## Risks

1. **Warp readability and cost**: warped art under hover must not blur the name/rank; mitigated by flat text plate and a grid density cap. If the budget test fails, drop grid density before dropping cards.
2. **First-person death** has no body to dissolve; the plan's fallback (held hands/relic + ember fade of the view) is a design substitution the owner has not seen. See decision 1.
3. **Merge conflicts**: M4 touches ten UI files; each stream owns disjoint files and the shared pieces (M0/M1/M3) land first. Two streams editing one screen (deck builder vs sort) silently dropped UI before; screenshot merged screens and read the hint bars.
4. **Names**: `death_fx.rs` not `dissolve.rs`.

## Decisions locked in (2026-11-10, round 2)

1. First-person death: held hands/weapon dissolve + ember fade of the view. Third-person dreams dissolve the body.
2. Full dissolve plays on the run-ending death only; a mid-run catch + respawn gets a 0.4 s short version.
3. Menu prompts: chip on the focused action only.
4. `Required` state (dims the rest) only for blocking choices: upgrade pick, events, wake door. Never in open play.
5. Held relic placement: owner delegated. Chosen: **lower-right, tilted ~15 degrees toward screen centre, slow idle sway (bob 0.6 Hz, 1.5 cm) and a small lag behind camera turns**. This is the composition that keeps the crosshair and the hand fan (bottom-centre) clear.
6. Warp strength is the same on every card (no rarity scaling). Foil/rarity are shown by sheen and frame, not wobble.
7. Relic names: highly randomised but role-fitting, via a pure grammar generator `relic_names.rs` (stream N, below).
8. Ship everything (M0-M4, R3-R6, N) before the owner plays. R7/R8 (polish + bug round) are done by the parent session.
9. Fresh branches off `main`: integration branch `feat/ui-revamp`; stream branches `feat/ui-<stream>` cut from it after M0 lands. Worktrees in `/home/_l/dreamscape-ui/<stream>`.

### Relic naming design (stream N)

A relic's name must fit its **role** (what it is: weapon shape + what it does + how worn) and still be near-unique. Inputs: `shape` (11 weapon shapes), `theme` (20 dreams), `rarity`, `WearTier`, `foil`, `effect` (its strongest lever), `seed`. Output: `name` (<= 40 chars, uppercase like dream names) and `flavour` (<= 56 chars, lowercase whisper).

- **Head noun by shape** (role): each `Shape` has ~8-12 head nouns (Crescent: SICKLE, MOON-BLADE, HORN, SMILE, PARING; Cube: DIE, RELIQUARY, BRICK, SUGAR LUMP; Torus: HOOP, HALO, RING, NOOSE...). The weapon is always recognisable from the name.
- **Modifier slots from the theme's own `dream::vocab` lists** (`adjectives`, `abstracts`, `relatives`), so a Drowned Library relic reads wet/paper and a Tunnel relic reads dark/echoing. 25% mashup borrows one slot from a donor theme, same rule as `names.rs`.
- **Rarity picks the template family** (Cracked: plain/ruined, "A CHIPPED {NOUN}"; Worn: "THE {ADJ} {NOUN}"; Pristine: "{NOUN} OF {ABSTRACT}"; Mythic: grand/owned, "{ABSTRACT}'S {ADJ} {NOUN}", "THE {NOUN} THAT {REL}"). Wear tier adds a prefix (`RUINED`, `HEAVILY-WORN`, none) so wear is readable in the name.
- **Effect hint**: the strongest `RelicEffect` lever adds a suffix word list (dust: "OF PLENTY", "THE HOARDER'S"; speed: "SWIFT", "FLEETING"; calm: "HUSHED"; memory: "REMEMBERING"; rerolls: "SECOND-GUESSING"; shards: "SHARD-BORN"; grace: "FORGIVING"), so a name hints at what it does.
- **Foil** gets a sheen word ("GILT", "PRISMATIC") in the name.
- **Uniqueness**: seeded retry with a salt counter against the owner's existing relics' names (pouch is small); 12 retries then a numeral suffix ("II").
- **Pure** (`mix`/`percent_roll` from `worth.rs`), deterministic per seed, and tested for: length cap, non-empty lists, no duplicates over 5,000 seeds within a theme above a stated rate (target: >= 97% unique per theme+rarity), shape word always present, no lowercase/unknown glyphs.
- Per the earlier lock, the **word lists** (not each output) go on the approval list in `NOTES-names.md`.

## Open items

None. Briefs: `plans/ui-revamp-briefs/`.
