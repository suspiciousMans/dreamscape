# Stream: TACTICS (module `tactics.rs`, branch feat/hard-tactics)
Mechanic: enemies work TOGETHER, so difficulty comes from combinations, not speed (chasers are capped below player speed).
Design (decided):
- Alert chain ("shout"): when a pacer starts chasing, other pacers within SHOUT_RADIUS (6 cells) get alerted for a few seconds
  (cooldown per enemy 8s). Depth >= 3. Sentry sighting already alerts within 2 cells: keep it.
- Jester assist: the jester's teleport (existing) prefers a safe direction that lands within a stalker's/sentry's sight
  radius when one exists (still "safe" = not a wall/void). Depth >= 4.
- Mimic + sentry: mimic path replay is delayed 3s; when the mimic is inside a sentry cone the sentry's rotation speed rises x1.5. Depth >= 5.
- Elites (Fast/Big/Splitter/Shade) may "lead": a pack of 2-3 pacers follows an elite's alert (depth >= 6).
- Pure functions in tactics.rs (positions/radii in, decisions out): `shout_targets(...)`, `jester_dir(...)`, `sentry_boost(...)`,
  `pack_follow(...)` with unit tests incl. determinism and "never chooses a blocked cell". Wire them into specials/enemy_ai call sites.
- Add one eye-voice or none; no HUD text needed. Patient-walker/route tests must still pass.
