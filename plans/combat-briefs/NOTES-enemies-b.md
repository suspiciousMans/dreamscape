# NOTES: ENEMIES-B (Moths, Mask, Leech)

Branch `feat/combat-enemies-b`, base `080745d`. Two commits `combat: ...`.

## Files
- NEW pure modules (all with unit tests): `moths.rs`, `mask.rs`, `leech.rs`, `foespawn.rs` (shared seeded placement).
- `combat.rs`: `Kind::{Moth, Mask, Leech}` at the bottom + `base_hp`. (Stream A also edits `Kind`, `base_hp` and `Ctx`: expect a trivial textual conflict.)
- `dream/theme.rs`: `EnemyKind::{Moths, Mask, Leech}` at the bottom, with hints. NOT in `ALL_ENEMY_KINDS` (they never come from theme lists). `dream/build.rs` got one `=> None` arm.
- `sounds.rs`: `Sound::{Bounce, Latch}`, `ALL_SOUNDS` 43 -> 45 (stream A may also bump it).
- `nerve.rs`: test only (`damage_from` is 1 for moth/mask/leech via the default arm; no match change).
- `main.rs`, all inside `// === combat ===` blocks: `SpecialAI::{Moth(usize), Mask, Leech}` at the bottom; fields `moth_swarm`, `b_tex`, `jump_event`; `spawn_enemies_b`, `update_enemies_b`, `knock_leech`, `shake_leech`, `release_leeches`, `mask_glances`, `mask_glance`, `rattle_b`; hooks in `threat_touching`, `rattle_enemies`, `resolve_strike`, `unravel`, the jump code (two `jump_event = true`), the caught branch, the specials reset, `update_specials` (first line), and after the `// === end shards ===` block (`spawn_enemies_b` must stay after it: the shards block uses `specials.last_mut()`).
- 7 game-level wiring tests in `screen_smoke` (`a_mask*`, `a_slam_cracks_the_shield_so_a_frontal_strike_lands`, `a_leech*`, `a_strike_unlatches*`, `moths_die*`, `a_gust_scatters_the_swarm`).

## Numbers (constants in the modules, each asserted by a test)
- Moths: depth 6, 4 per group, hp 1, ring 2-3 cells, dive every 2.5s round robin, 0.5s tell, dive at 1.0x player speed (`gameplay::MOVE_SPEED`), scatter 1s.
- Mask: depth 7, hp 3, 0.5x speed, turns 90 deg/s, shield half-angle 60 deg, slam tell 0.7s, trigger 3 cells, reach 1.5 cells, gust = 1.5s spin + shield down, slam = shield down 2s.
- Leech: depth 8, hp 2, 0.6x speed, drain 1 nerve / 2s, shake = 3 jumps in 2s, stagger 1.5s.
- Caps: 1 moth group, 1 mask, 1 leech per dream (max 3 total, `foespawn::MAX_NEW`). None in nightmares, the prologue, or co-op. Placement: floor cell off the route, >= 3 cells from spawn/portal/shard, >= 2 cells from each other. Each enemy has its own seed stream.

## Decisions the merger should know
- Distances written "N cells" are multiplied by `CELL` (3.0). Taken literally this makes the moth ring 6-9 units wide and the mask slam reach 4.5 units with a 9 unit trigger. Tune `ORBIT_*_CELLS`, `SLAM_REACH_CELLS`, `SLAM_TRIGGER_CELLS` if it feels too big.
- A moth dive is longer than 2.5s when you are far from the pocket; the one-at-a-time rule then delays the next dive (the clock is a minimum, not a metronome).
- The mask's body is harmless; only the slam hurts. The leech's touch only latches (no damage); the 2s drain goes through `take_nerve_hit("leech")`, so iframes apply, and a WARD absorbs the latch or a drain as usual. Latch is skipped for the autopilot, grace, Dash, Phase.
- Gust (push > 0) scatters moths and spins masks; Slam (push 0) cracks mask shields. This is decided in `rattle_enemies` by `push`, so a new caller with push 0 would crack shields.
- Stillness (`held`) freezes all three (a latched leech stops draining while held).
- Strike unaware bonus is off for these three. A staggered mask takes x2 (existing rule).
- Reduced motion: no scale pulses, no latch/glance flashes. The brighten/tint tells stay.
- Dev: `DREAMSCAPE_ENEMY=Moths|Mask|Leech` forces that one in every dream (skips the depth gate and nightmare gate; not co-op/prologue).
- No elite variants, no `tactics.rs` shouts/packs, no codex/lore entries.

## Verified
- Focused tests, green: `moths::`, `mask::`, `leech::`, `foespawn::`, `nerve::`, `sounds::`, `combat::` (66), the 7 wiring tests, and `every_route_is_physically_traversable`, `a_patient_player_is_never_walled_in`, `a_patient_player_is_never_walled_in_under_pressure`, `specials_come_from_the_dreams_own_list_and_not_too_early`.
- Debug build: only the 2 known warnings.
- Screenshots (`shots-b/Moths.png`, `Mask.png`, seed 5, `DREAMSCAPE_ENEMY=...`): the dream hint shows and the mask slab is visible; only one moth was in frame.

## Not verified
- The leech never showed up in a screenshot (autopilot, dark dream); its visuals are unchecked by eye.
- No hands-on play: moth dive tell, mask slam tell, latch swell and feel are untested by a human. The whole suite was not run (owner does that at merge).
