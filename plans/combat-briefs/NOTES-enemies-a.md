# NOTES: ENEMIES-A (Tooth, Weeper, Lullaby)

Branch `feat/combat-enemies-a`. Build with `CARGO_TARGET_DIR=.../target-a`.

## What was built
Three pure modules, each with its own state machine, constants, `spawn_plan(depth, theme, seed, nightmare) -> Vec<(Kind, usize)>` and unit tests:

| File | Numbers |
|---|---|
| `tooth.rs` | hp 2, from depth 3. Creeps at 0.6x player speed (sense 18u, leash 12u). Telegraph 0.5s (scale pulse + red eye texture), one lunge of 7.5u (2.5 cells) in 0.5s, recovery 1.5s (strike x2 via `STAGGERED_MULT`). Only the lunge hurts. |
| `weeper.rs` | hp 2, from depth 4. Keeps 12-18u, walks at 0.8x player (flees inside 12u, leash 18u from its post). Telegraph 0.8s every 3.0s; tear 0.4x player speed, life 6s, max 2 alive, die on wall boxes. Only tears hurt. |
| `lullaby.rs` | hp 6, from depth 5. 0.35x player speed, turns at 1.6 rad/s. Strike from within 120 deg of its facing: x0.5 damage (`FRONT_MULT`) and stagger; from behind: full. Telegraph 0.8s every 5.0s (floor torus marker), then a ring at `boss::RING_SPEED`, up to 9u; jump it (`boss::ring_hits`) or leave. Stagger cancels a telegraph. |
| `newfoes.rs` | shared: `plan()` (caps: 1 Lullaby, 2 Weepers, 3 Teeth, 4 total, Lullaby first), `place()` (seeded, distinct, 9u apart), `forced()` for the dev switch. Own seeded streams (`stream(seed, salt)`), never the dream RNG. |

Other edits:
- `combat.rs`: `Kind::{Tooth, Weeper, Lullaby}` at the top of the enum (+ `base_hp`); `Ctx.guarded` (x0.5) added (existing `Ctx { .. }` literals need `..Default::default()` or `guarded`).
- `nerve.rs`: `damage_from`: `"tooth"` 1, `"tear"` 1, `"slam"` 2 (test updated).
- `dream/build.rs`: `Dream.lairs` (field and `lairs()` fn, marked `// enemies-a`): off-route floor (route + shard detour), at least 3 cells from spawn/portal/shard, not crumbling floor, not on a patrol end. No RNG drawn, so existing dreams are unchanged. Nightmare and secret dreams: empty.
- `main.rs` (blocks marked `// === enemies-a ===`): `NewAI`/`NewFoe` structs, `newfoes`/`foe_walls` fields, `spawn_newfoes()`, `update_newfoes()`, `threat_touching()` names `"slam"`, `"tooth"`, `"tear"`; `resolve_strike()` targets (armour via `Ctx.guarded`, recovery via `staggered`); `unravel()` despawns a foe's tears/plate/marker/ring; `rattle_enemies()` (Slam/Gust) also reels them; `away_from_nearest_threat()` includes them.
- Spawns are skipped in co-op, the prologue, nightmares, and under autopilot (unless the dev switch is set).

## Dev switches
- `DREAMSCAPE_ENEMY=Tooth|Weeper|Lullaby` forces that enemy (2/2/1) into every dream, ignoring depth/theme gates, using the 4 lairs nearest the spawn.
- `DREAMSCAPE_ENEMY_NEAR=1` (with the above) places them just in front of the spawn, for screenshots.

## Verified
Unit tests (all green): `tooth::`, `weeper::`, `lullaby::`, `newfoes::`, `combat::`, `nerve::`, `dream::build::tests::lairs_are_off_the_route_and_three_cells_from_spawn_portal_and_shard`; see the final reply for the guard tests (`every_route_is_physically_traversable`, `a_patient_player_is_never_walled_in*`).
Screenshots (autopilot, `DREAMSCAPE_ENEMY[_NEAR]`): `/home/_l/dreamscape-combat/shots-a/`: `tooth-telegraph.png` (red cone mid-telegraph), `lullaby.png` (purple cube with gold front plate), `weeper.png` (blue pillar; partly hidden behind a prop).
No `screen_smoke` frame added: that module only renders the HUD, and these enemies add no HUD.

## Not verified / caveats for the merger
- Not played by hand: no live test of a real strike on each enemy, the slam ring (marker/ring were not captured in a screenshot), tears in flight, or a nerve loss from them. Only logic tests and static screenshots.
- Autopilot runs do not spawn them (so the end-to-end autopilot tests are unchanged).
- Tears are not consumed on touch (the 1.2s nerve iframes cover repeat hits). Phase/Dash/Ward behave as for other threats because `threat_touching()` feeds the same path.
- A Weeper aims at the Decoy (`self.decoy`) if one is out, else at you; it keeps its distance from you either way.
- Weeper and Lullaby phase through walls like pacers do; only tears die on walls (wall boxes from `BlockKind::Wall`, rotation ignored).
- Texture swap (not a shader tint) is used for the eye/tell; reduced motion drops the scale pulse and keeps the tint.
- Elite variants and `tactics.rs` shouts/packs for these enemies are not done. No new sounds (existing ones only; no hit or telegraph sound is played for the new enemies).
- Not run: the whole suite (owner does that at merge).
