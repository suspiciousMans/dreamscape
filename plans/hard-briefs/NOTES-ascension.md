# NOTES-ascension (branch feat/hard-ascension)
Pure logic in `progress.rs` (`Ascension::{no_safe_respawn,remembers,loadout_slots,pickup_factor,eye_timer,always_twist}`,
`remembered_spot`, `lose_last_card`, `forced_twist`). MAX_ASCENSION=16. Rules apply from depth 2 (RULES_FROM_DEPTH);
ascension is already 0 for daily/coop/prologue/autopilot. Nightmares skip 12/15/16.
Constants: EYE_SECONDS 90, EYE_SHRINK 3 (15 -> 87s, 16 -> 84s), MEMORY_RADIUS 14, MEMORY_START_CLEAR 6.
Design: 11 = catch halves lucidity, grace 0 (respawn already goes to start). 12 = one extra watcher (sentinel patrol a==b)
at a route point near last dream's catch spot. 13 = loadout slots-1 (applied in begin_run + loadout toggle, not persisted).
14 = shard_bonus and beacon height x0.5. 15 = eye clock (HUD timer), expiry once per dream removes newest non-ability card.
16 = if no twist rolled, seeded fitting twist. Wiring is in main.rs between `// === ascension ===` markers.

## Status (done, 2 commits on feat/hard-ascension; full `cargo test --release` green: 659+ pass)
Wired in main.rs (`// === ascension ===` blocks): run_asc/asc_slots/dream_timer helpers, shard_bonus x factor, beacon height x factor,
catch hook (halve lucidity, remember spot), respawn grace 0, eye expiry in update, load_dream twist force + remembered reset,
extra sentinel patrol (a==b) appended after existing patrols (existing indices/seeds unchanged), loadout truncation (not persisted).
Audit: title picker drawn at every level 0..16 in audit_text.
Unverified: no in-game playtest; rule 12 uses a lucid-route point as the spawn (previous dream's coords map by nearest, layouts differ);
rule 11 "back to start" relies on the existing respawn_player; rule 15 expiry also fires with HUD timer shown from existing Hasty HUD.
Knobs: progress.rs consts EYE_SECONDS, EYE_SHRINK, MEMORY_RADIUS, MEMORY_START_CLEAR, RULES_FROM_DEPTH.
