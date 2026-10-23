# NOTES pressure (branch feat/hard-pressure, worktree dreamscape-hard/pressure)
Design (consts at top of pressure.rs): depth>=2, normal dreams only, skipped for HASTY.
grace = max(75-5*depth, 35)s; stirring 30s (stir 0..1: alert +40%, speed +10%); warning 20s; then collapse.
Shard refund 12s. Collapse -> existing wake melt (Pending::Wake); the dream's run dust earned is halved
(`collapse_dust`); summary row says so. HUD: thin lid bar (top centre) + eye lines at Stirring / Warning.
Enemy multipliers applied live via `EnemyAI::set_pressure(speed_m, alert_m, cap)` (enemy_ai.rs, small addition);
cap keeps pacers under 0.92*player_speed/CHASE_BOOST. Specials (stalker etc.) are not scaled.
Status: DONE, 2 commits on feat/hard-pressure; full `cargo test --release` (dreamscape crate) 668 pass.
Wiring (all in `// === pressure ===` blocks): main.rs fields/reset in load_dream (after dream_age=0), update_pressure() in update,
refund after collect_shard, summary row, pressure_applies() (not prologue/coop/nightmare/Awakening/Lobby, not HASTY);
hud.rs pressure_bar + HudView.pressure_lid; enemy_ai.rs set_pressure. Collapse = begin_melt(Pending::Wake) (same as wake door).
Dust: run bonus dust earned during the collapsed dream halved (pack dust untouched).
Unverified: no live playthrough; secret rooms/Paused simply don't tick (not Playing); specials (stalker/mimic/sentry) unscaled;
pacers spawned by Splitter are not re-stirred until next frame set (they are, set each frame). Knobs: consts at top of pressure.rs.
