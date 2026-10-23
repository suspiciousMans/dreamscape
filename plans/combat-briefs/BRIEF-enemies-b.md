# Stream: ENEMIES-B (Moths, Mask, Leech)
Worktree `/home/_l/dreamscape-combat/enemies-b`, branch `feat/combat-enemies-b`, `CARGO_TARGET_DIR=/home/_l/dreamscape-combat/target-b`.
Read `CONTEXT.md` and plan section 8 first. Put variants you add to enums (`combat::Kind`, `SpecialAI`) at the **bottom** of the list; stream A adds at the top.

Build three enemies, each a pure module (`moths.rs`, `mask.rs`, `leech.rs`) with its own state machine, plus wiring, spawn gating, textures and tests:

| Enemy | From depth | hp | Behaviour |
|---|---|---|---|
| **Moths** | 6 | 1 each, groups of 4 | Orbit a point (a pocket) at 2-3 cells in a loose ring. Every 2.5s one moth (round robin) telegraphs 0.5s (it brightens) then dives at your position-at-telegraph-time at 1.0 x player speed, 1 nerve on touch, then returns. Never more than one diving. One STRIKE kills one; Stillness freezes the ring; Gust scatters them (stagger 1s). |
| **Mask** | 7 | 3 | Slow patroller (0.5 x player) with a frontal shield: strikes from the front deal 0 (a spark, a bounce sound and a flash; the Mask is not staggered), from the side or back full damage. Turns slowly (90 deg/s). Slams its forehead (0.7s telegraph, 1 nerve, reach 1.5 cells) when you stand in front within 3 cells. Counters: Blink behind it, Gust (spins it: 1.5s stagger with shield down), Slam (shield down 2s). |
| **Leech** | 8 | 2 | Crawls toward you slowly (0.6 x player). On touch it **latches**: it rides you and drains 1 nerve every 2s until shaken. Shake: three jumps within 2s (each jump counts once, ground jump or air jump) or a STRIKE (it has 2 hp; a strike while latched also removes it from you). WARD blocks the latch (consumes the ward as usual). A latched leech does not stop you moving. After it is shaken off it is staggered 1.5s then resumes. |

Requirements:
- **Spawning**: a pure `spawn_plan(depth, theme, seed, nightmare: bool)` per enemy (own seeded RNG stream; empty below the gate, in nightmares, the prologue, and co-op). Caps per dream: 1 Moth group, 1 Mask, 1 Leech; total of new enemies at most 3 (stream A's enemies are budgeted separately; the merger will cap the combined number). Use the existing pocket/route-safe placement used by pacers; never on the route or within 3 cells of spawn/portal/shard.
- **Damage**: moth dives, mask slam and leech drain via `threat_touching()` names (`"moth"`, `"mask"`, `"leech"`) and `nerve::damage_from` (all 1). The leech drain calls `take_nerve_hit("leech")` on its own 2s timer, so it still respects invulnerability.
- **Strike**: extend `combat::Kind` (+ `base_hp`), `resolve_strike()` (shield angle test as a pure fn in `mask.rs`; the leech latched state), and `unravel()` cleanup (a dead leech releases you).
- **Visuals**: distinct silhouettes and colours; telegraphs visible without text; moth dive tell = brighten + scale.
- **Tests**: every constant above, round-robin dive order, one-at-a-time rule, shield angle table, latch timing and the three-jump shake rule, spawn plan gating and caps.
- Dev switch `DREAMSCAPE_ENEMY=Moths|Mask|Leech`.
- Write `NOTES-enemies-b.md`.
