# Dreamscape — plan from Phase 19 Stanza 2 to Early Access + co-op

## Context
Stage 1 of Phase 19 (lore, eye, objectives, prologue, `DreamDirector.forced`) is on `main` (`d768ee9`, 321 tests). What remains: Stanzas 2–3 (cards with power, merging, companions, ending), then balance, Steam, packaging, achievements, and 5-player co-op. The only spec in the repo is `plans/…phase19-claude-prompt-stanzas-2-3.md`; it has no exact tables, so I draft them below for sign-off. **Gap found:** `Rarity::Fused/Resonant`, `Card.fused` and `DreamRecord.blend` are *not* in `cards.rs` (Rarity stops at Prophetic). They're built in M1.

## Working rules (from your answers)
- Branch `claude/nifty-bardeen-8px7fn`; one PR per milestone against `main`; keep going while you playtest.
- TDD per task, commit per green task (`dreamscape: <what>`). Unit tests during build; screenshots/E2E/Windows checks happen in one **bug-testing round at the end** (M9).
- Never touch the real booklet; temp booklet via `DREAMSCAPE_BOOKLET`.
- No new story text except the 6 ending lines below (need your approval).
- Existing seeds must replay: new RNG draws use their own seeded streams.

## M0 — Tables for your approval (before any code)
Committed as `plans/2026-09-28-phase19-tables.md`. Drafts:

**Powers (attribute → active + passive; tint = `theme.spec().accents[0]`)**
| Theme / Attr | Active | Passive |
|---|---|---|
| Lobby / Velvet | Stillness | CalmMind |
| LiminalOffice / Static | Phase | LongBreath |
| VoidPlatforms / Void | Blink | Spring |
| Garden / Bloom | ShardCall | LuckyMemory |
| NightmareFactory / Rust | Flare | HeavyAir |
| Awakening / Dawn | Flare | WideEyes |
| CursedForest / Hex | Decoy | DreamAnchor |
| DrownedLibrary / Tide | Float | LongBreath |
| SkyStairs / Aether | Float | DoubleJump |
| MirrorHall / Glass | Decoy | WideEyes |
| MyceliumGrove / Spore | ShardCall | ShardSense |
| TheTunnel / Abyss | Dash | SwiftFeet |
| FractalCathedral / Halo | Stillness | Quicken |
| Elfworks / Jest | Blink | DustHoarder |
| AfterimageFields / Trace | Rewind | SwiftFeet |
| SynesthesiaHall / Chord | Dash | Quicken |
| MeltingClockworks / Hour | Rewind | DreamAnchor |
| JellyfishSky / Lumen | Phase | Spring |
| WatchingWallpaper / Gaze | Stillness | ShardSense |
| WhiteDissolve / Blank | Phase | DustHoarder |
All 9 abilities reachable; mythics (LucidHeart, WideChoice) excluded. Flavour lines: one short dry line per theme, written with the tables.

**Resonant pairs (each theme exactly once):** Garden+Mycelium · DrownedLibrary+JellyfishSky · MirrorHall+WatchingWallpaper · TheTunnel+Awakening · MeltingClockworks+AfterimageFields · SkyStairs+VoidPlatforms · FractalCathedral+Elfworks · SynesthesiaHall+Lobby · NightmareFactory+LiminalOffice · CursedForest+WhiteDissolve.

**Companions (boon = 2 stacks of a passive; burden ≈ 10–15% on one stat)**
| Weight | Boon | Burden |
|---|---|---|
| Grief | DreamAnchor | lower sight |
| Burnout | LongBreath | slower movement |
| Anxiety | CalmMind | enemies notice sooner |
| Loneliness | LuckyMemory | −1 card at each pick |
| Guilt | ShardSense | shorter grace |
| Insomnia | HeavyAir | faster enemies |
| Heartbreak | SwiftFeet | lower jump |
| Pressure | DustHoarder | smaller shard bonus |

**Natural fuse rate:** depth ≥ 5, not nightmare, not a card dream: `p = min(0.02 + 0.004·(depth−5), 0.14)` → 3000-descent test lands in 4–16%.

**Ending lines (draft, ≤90 chars; reuse `eye::Moment::Ending` lines if already present):**
1. you came all this way down. i was never trying to keep you here.
2. every nightmare was me, holding on too tight. i'm sorry it hurt.
3. the notes were yours. the others' too. none of you were alone in it.
4. you don't have to be fixed to wake up. you just have to wake up.
5. i'll still be here. i'm the part of you that noticed.
6. open your eyes. it's morning, or close enough.

## M1 — Stanza 2a: power data + rarities (PR 1)
- `cards.rs`: add `Rarity::Fused` ("FUSED", base_memory 1.0, dust 30, colour cycles both accents) and `Rarity::Resonant` ("RESONANT", dust 45, `hue(t*0.8)` + white pulse); excluded from `from_score`, `bumped()` no-op. Add `#[serde(default)] fused: Option<DreamTheme>` on `Card`, `blend` on `DreamRecord`; `card_from` → Fused when `blend.is_some()`. Tests: 10k scores never special; RON round-trip; old booklets load.
- New `powers.rs` (table above). Tests: every theme has a power, all abilities reachable, tint = accents[0].
- `upgrades.rs`: `RunUpgrades::from_loadout(&[Card])` — cards 1–2 give actives (existing ability slots), all cards give passives, fused +1 passive of the fused theme, Resonant +1 extra. Remove abilities from `roll_choices`; test `abilities_come_from_cards_not_picks`; passives still respect caps.

## M2 — Stanza 2b: schedule, loadout screen, visuals (PR 2)
- `dream/director.rs`: `plan_cards(themes)` at depths `3+3i`, slide past nightmares, never back-to-back; sets `blend` for fused cards. Uses `forced`-style queue, own RNG stream. Test: 5 themes × 20 descents.
- F8: fresh save's first 5 cards distinct attribute + distinct active (bias director choice for card-less saves).
- Prologue presses a guaranteed starter card (Garden → ShardCall) so START always has ≥1 card; 0-card saves skip the loadout.
- `loadout_ui.rs` + `Mode::Loadout` (pattern from `booklet_ui.rs`/`upgrade_ui.rs`): slot row (`Lore::slots()`), card grid, detail panel, keys/pad per spec; persists `lore.loadout`, drops missing cards; companion slot stub (hidden until M4).
- `main.rs`: ability flash tinted by granting card; 8-cube ring burst fading 0.5 s (reuse `dissolve.rs`/particle helpers).

## M3 — Stanza 3a: merging + fused dreams (PR 3)
- `fusion.rs`: `merge(a, b) -> Card` (a's shape, b's colours/mood, consumes both, Resonant on recipe pair, records in `lore.recipes`). RON round-trip tests.
- Moth shop (`shop_ui.rs`): add MERGE tab; shop dream now guaranteed after each beaten nightmare (plus existing secret rooms).
- Natural fused dreams in director (rate above); presses a Fused card; eye speaks `FusedDream`. `dream/names.rs` `fused_name` if missing; blended palette in `dream/theme.rs`.

## M4 — Stanza 3b: companions + ending (PR 4)
- `companion.rs`: boon/burden per weight applied in `RunUpgrades` stat getters; loadout companion slot unlocks at first `lore.dreamer_complete`. Tests: each burden hits its stat.
- Ending: after all 12 own notes, the next descent forces a Bottom dream (`blend` wired); eye speaks 6 lines over 30 s; lid-close animation in `eye.rs`, credits + helpline; `ending_seen` → title shows complete; runs continue.

## M5 — Balance (PR 5) — card power + difficulty curve
- Headless autopilot sweep test (seeds 1–50, each single-card loadout and 5-card loadouts) reporting depth reached / catches; `#[ignore]` long test.
- Tune per-card passive magnitudes, burden %, and `Difficulty`/`Pressure` curve (you've allowed changes); update affected tests deliberately.

## M6 — Packaging: Windows, Linux, Steam Deck (PR 6)
- `package.sh` (Linux mirror of `package.ps1`); Ubuntu CI job (`cargo test`, `clippy`; link `advapi32` only on Windows per roadmap 6.1).
- Steam Deck: controller-only pass on every screen (loadout, shop merge, ending), 1280×800 layout and text size check.

## M7 — Steam + achievements (PR 7)
- `steamworks` crate behind a `steam` feature, App ID 480 placeholder via `steam_appid.txt`; init/shutdown in `main.rs`, no-op when feature off.
- `achievements.rs` exactly per `ACHIEVEMENTS.md` (29, booklet-stored, `check(&Booklet,&RunStats,&Event)`), 8 new counters, Steam bridge + offline sync. No new Phase 19 achievements unless you ask.

## M8 — 5-player co-op over engine TCP (PRs 8a–8c)
- `engine/src/net`: `MAX_PLAYERS` 4→5; extend protocol for dreamscape state (player snapshots, shard/portal events, ghost/revive, choice-ready). Bump `PROTOCOL_VERSION`.
- Host-authoritative: host generates dream from seed, clients regenerate locally from seed; enemies/shards synced by snapshot.
- Rules: shared shards and portal (all living players must enter); own loadouts; caught → ghost, revived by a teammate standing near ~2 s; all ghosts → run wakes; each player picks own upgrade (wait for all); shards needed and enemy count scale per extra player; **no lore/notes in co-op**.
- UI: title HOST / JOIN (IP), lobby list. Direct IP/LAN first; Steam lobbies later once App ID exists.
- 8a net+lobby, 8b shared run rules, 8c ghosts/revive/scaling.

## M9 — Bug-testing round (end)
- Linux: `cargo test -p dreamscape`, `cargo clippy`, Xvfb screenshots (loadout, ability flash, card detail, merged card, fused dream, ending), autopilot seeds 1–5, forced card-dream (20 descents) and fused (3000 descents) runs, 2–5 local co-op clients.
- Windows CI green on every PR; you do hand playtests.

## Critical files
`src/cards.rs`, `src/upgrades.rs`, `src/dream/director.rs`, `src/dream/theme.rs`, `src/dream/names.rs`, `src/lore.rs`, `src/eye.rs`, `src/tutorial.rs`, `src/shop_ui.rs`, `src/main.rs`, new `src/powers.rs`, `src/fusion.rs`, `src/companion.rs`, `src/loadout_ui.rs`, `src/achievements.rs`, `engine/src/net/*`, `package.ps1`, `.github/workflows/ci.yml`.

## Verification per milestone
`cargo test -p dreamscape` green with 0 warnings; the named tests for that milestone exist and pass; autopilot seeds 1–5 finish without errors (headless). Full visual/E2E in M9.
