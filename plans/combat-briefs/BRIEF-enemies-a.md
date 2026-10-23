# Stream: ENEMIES-A (Tooth, Weeper, Lullaby)
Worktree `/home/_l/dreamscape-combat/enemies-a`, branch `feat/combat-enemies-a`, `CARGO_TARGET_DIR=/home/_l/dreamscape-combat/target-a`.
Read `CONTEXT.md` and plan section 8 first. Put variants you add to enums (`combat::Kind`, `SpecialAI`, `EnemyKind` if needed) at the **top** of the list; stream B adds at the bottom.

Build three enemies, each a pure module (`tooth.rs`, `weeper.rs`, `lullaby.rs`) with its own AI/state machine, plus wiring, spawn gating, textures, and tests:

| Enemy | From depth | hp | Behaviour |
|---|---|---|---|
| **Tooth** | 3 | 2 | Fast, short-reach lunge out of the dark. Idles in a pocket, 0.5s telegraph (scale pulse + eye tint), then one fast lunge of about 2.5 cells, then 1.5s recovery during which it is staggered-vulnerable (damage x2). Slower than the player when chasing; the lunge is the danger. Dash through it, or strike its recovery. |
| **Weeper** | 4 | 2 | Keeps 4-6 cells from you (flees if you close in, never faster than you), and every 3s telegraphs 0.8s then spits a slow tear (speed 0.4 x player, life 6s, dodgeable, 1 nerve). Tears die on walls. Counters: Blink/Dash in; Decoy draws its fire. Max 2 tears alive. |
| **Lullaby** | 5 | 6 | Slow (0.35 x player) and big. Front is armoured: a strike from the front staggers (poise) but deals half damage; from behind (angle from its facing > 120 degrees) full damage. Every 5s it telegraphs 0.8s (a ring marker on the floor) then slams: a damaging ring (like `boss.rs` ring, 2 nerve) you must jump or leave. Counters: Blink/Hush behind it, Slam. |

Requirements:
- **Spawning**: a pure `spawn_plan(depth, theme, seed, nightmare: bool) -> Vec<(Kind, count)>` per enemy (own seeded RNG stream; `[]` for depth < gate, nightmares, prologue, co-op). At most 1 Lullaby, 2 Weepers, 3 Teeth per dream; total of new enemies at most 4. Use the existing pocket/route-safe placement used by pacers (look at `dream/build.rs` and where `self.enemies.push(Pacer {...})` happens in `load_dream`); never place on the route or within 3 cells of spawn/portal/shard.
- **Damage**: add their touches/projectiles/rings to `threat_touching()` with distinct names (`"tooth"`, `"tear"`, `"slam"`) and give `nerve::damage_from` the amounts (tooth 1, tear 1, slam 2). Update its test.
- **Strike**: extend `combat::Kind` (+ `base_hp`), `resolve_strike()` target list (armour and recovery bonus via `Ctx` or a new field), and `unravel()` cleanup (despawn tears owned by a dead Weeper).
- **Visuals**: reuse meshes (`Shape::Cube/Octahedron/...`) and `self.texture(gl, rgba)`; each enemy needs a distinct silhouette and colour; the telegraph must be visible without text.
- **Tests**: every constant above, telegraph-before-damage ordering, tears dying on walls, the Weeper never outrunning you, Lullaby armour angle, spawn plan gating and caps, and a headless `screen_smoke` frame if one applies.
- Dev switch `DREAMSCAPE_ENEMY=Tooth|Weeper|Lullaby`.
- Update `NOTES-enemies-a.md`.
