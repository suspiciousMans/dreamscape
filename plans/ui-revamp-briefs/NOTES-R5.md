# NOTES-R5: relic weapons

## Dev
- `DREAMSCAPE_GIVE_RELIC=<cracked|worn|pristine|mythic>[:seed]` (also 0-3) puts a relic weapon in hand at startup (theme Lobby). Pair with `DREAMSCAPE_FP=1` for first person. R6's equip flow should set `DreamscapeGame.held_relic` instead.

## Deviations from the brief
- `held_pose(t, lag, juice, reduced)` takes the spring-stepped `lag` (-1..1) rather than `turn_rate`; the spring lives in `HeldSway::step(dt, turn_rate, ..)` (called from `update_combat`).
- Held mesh is not a world entity: it is drawn in `render()` straight after the world passes (depth cleared in first person), same program, same `uDissolve`/`uDissolveColor` as the dreamer (only on a run-ending death).
- Clash styles 1-3 set the strike's time-dip to 30/40/50 ms (style 0 = unchanged 60 ms default); trail styles scale ray width / tip reach.

## For approval
- No player-facing strings added. If the pouch/UI shows trail/clash labels, they are R6's to name.
