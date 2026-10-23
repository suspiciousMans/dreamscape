# NOTES: hazards (branch feat/hard-hazards, module theme_rules.rs)

Design: one `Kind` per theme from `rule_for`. Gate: depth >= 3 (`MIN_DEPTH`), normal dreams only
(not nightmare/prologue/coop). `strength(depth)` = 0.25 at depth 3, +0.1/depth, clamp 1.
Kinds/effects (at strength s): RisingTide speed -40%*s*ramp (ramp 0..1 over 60s since last shard, shard resets);
StickyFloor speed -20%*s; GustWind 1.6s push every 9s, 3.0*s u/s, own hashed direction (no shared RNG);
DimmingLight visibility -60%*s*ramp; Silence hearing x(1+0.8s); DriftingFog drift 0.4+0.6s;
BeatFloor hot every other 1.2s beat; ShrinkingFloor `shrunk_cells` (outer first, route +1-cell margin never, max half).

Pairings: Lobby sticky, LiminalOffice dimming, VoidPlatforms shrink, Garden fog, NightmareFactory beat,
Awakening dimming, CursedForest silence, DrownedLibrary tide, SkyStairs gust, MirrorHall silence,
MyceliumGrove fog, TheTunnel dimming, FractalCathedral shrink, Elfworks sticky, AfterimageFields silence,
SynesthesiaHall beat, MeltingClockworks tide, JellyfishSky gust, WatchingWallpaper silence, WhiteDissolve shrink.

Status: see end of file.

## Status (done)
- theme_rules.rs: table, strength, RuleState/Effect, shrunk_cells + 7 unit tests. Full `cargo test --release` green (662 + others).
- Wired in main.rs (`// === hazards ===` blocks, field `theme_rule`): speed mult (tide/sticky) and gust push on the player body,
  tick per frame, `on_shard` reset, rule line prepended to `dream_hint` (title card). Skips nightmare/prologue/coop/Awakening.
- NOT wired (pure logic + tests only; Effect fields exist): ShrinkingFloor cells, DimmingLight visibility, Silence hearing
  (enemy chase radius on dash/blink), DriftingFog drift, BeatFloor hot tiles. Each needs a render/AI hook in main.rs.
- Unverified: feel/tuning in play; hint line fit on narrow windows relies on existing `fit` (audit_text passes, but it does not render depth>=3 dreams).
- Knobs: MIN_DEPTH, strength(), FULL_BITE_SECS, GUST_*, factors in RuleState::effect.

## HAZARDS-2 (branch feat/hard-hazards2) — wired list
All 8 kinds now wired (`// === hazards ===` blocks in main.rs; pure fns + tests in theme_rules.rs):
- Silence: `alert_enemies` radius x `heard_radius` (slam/gust/dash noise carry further).
- DimmingLight: `dimmed_sight` scales render sight radius and the `clear` radius in update_specials; floor 40%; shard resets.
- DriftingFog: `fog_center` loop (radius 4u, speed 0.4+0.6s u/s) moves `self.fog` and the disc entities (`fog_discs`).
- ShrinkingFloor: floor runs split into one-cell slabs (`ShrinkFloor`), `shrink_order`/`shrink_count`; shake 1s then despawn, never route/margin/spawn/portal/shard/fragments/sigils/crumbles; skipped for Chase/HoldOn objectives.
- BeatFloor: `beat_hazard_cells` adds 2+6s route tiles to `beat_tiles` (existing bpm clock, existing catch check, crossing off-beat safe).
Unverified: feel/tuning; no GL test (needs a window): dreams were not spawned headless. No new text so screen_smoke unchanged.
Known gaps: WhiteDissolve keeps its own dissolve tiles (shrink skips them, so its rule is mostly cosmetic); beat plates use route y=0 (terraced floors could hide them); enemies on shrunk cells are not handled.
