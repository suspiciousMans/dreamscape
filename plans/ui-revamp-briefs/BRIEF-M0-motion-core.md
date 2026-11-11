# Brief M0: ui_motion + ui_juice setting (done by the parent, wave 0)

Goal: one shared pure motion module and the UI-juice setting so every later stream builds against the same types.

## Scope
- New `src/ui_motion.rs` (pure, tested): `UiJuice { Off, Soft, Full }` (+ `scale()` 0.0/1.0/1.4, `Default = Soft`), `smoothstep`, `ease_out_cubic`, `ease_out_back` (overshoot <= 1.1), `Spring { pos, vel }::step(target, dt, stiffness, damping)` (critically-damped-ish, stable for dt up to 0.1), `tick_up(from, to, t)`, `stagger(i, per_item, t)`, `effective(juice, reduced) -> f32` (0 when reduced or Off).
- `settings.rs`: `Settings.ui_juice: UiJuice` `#[serde(default)]`; an `Action`/Row to cycle it (bump `ALL_ACTIONS` length and the `rows().len()` assert; `default_key` + label); `settings_ui.rs` row and font-floor check at 640x360.
- `relic_types.rs`: `Weapon { tint: [u8;3], trail: u8, clash: u8 }` and `RelicEffect.max_nerve` with `#[serde(default)]`; fix literals with `..Default::default()`.
- Branch `feat/ui-revamp` off `main`; commit; then cut worktrees.

## Tests
- Easing endpoints (0 -> 0, 1 -> 1), monotone smoothstep, back-ease overshoot bound, spring converges and never explodes, tick_up monotone and exact at ends, stagger bounds, Off/reduced => 0 effective; old `settings.ron` loads with `Soft`.
