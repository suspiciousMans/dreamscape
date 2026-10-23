# Stream: PRESSURE (module `pressure.rs`, branch feat/hard-pressure)
Mechanic: the dream does not wait for you. After a grace period the eye "stirs"; linger and the dream collapses.
Design (decided; tune via consts at top of pressure.rs):
- Applies at depth >= 2, normal dreams only (not boss/secret/prologue/tutorial/co-op/Paused).
- `Pressure` state ticks with dream time. Phases: Calm (grace = max(75s - 5s*depth, 35s)), Stirring (next 30s: a 0..1
  `stir` value rises; enemy chase radius and alert grow up to +40%, enemy speed up to +10% (still under the player-speed cap)),
  Collapse (after that +20s warning: the dream dissolves: you wake early; the run ENDS like waking, cards kept, dust x0.5
  for that dream's share, say so in the summary line).
- Picking up a shard pushes the timer back (refund ~ 12s) so progress is rewarded. A clock/eye cue on the HUD: a thin bar or
  eye-lid creeping shut (use existing eye; no new assets) + one eye-voice line when Stirring starts, one at Collapse warning.
- Twist HASTY already has a timer: Pressure must not stack with it (skip Pressure when HASTY).
- Pure API in pressure.rs: `Pressure::new(depth)`, `tick(dt)`, `refund_on_shard()`, `phase()`, `stir()` (0..1), `chase_mult()`,
  `speed_mult()`, `collapse_in()` -> Option<f32>; all unit-tested (phase boundaries, refund, no stacking, depth scaling).
- Wiring: reset per dream; feed multipliers into the existing pressured_* call sites; HUD bar; collapse -> existing wake path.
