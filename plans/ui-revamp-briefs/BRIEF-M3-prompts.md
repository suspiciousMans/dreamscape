# Brief M3: input prompts

Goal: when an action is available or required, a pulsing key/button chip says so. Context-only; never names a wrong key.

## Scope
- New `src/prompts.rs` (pure) + `mod prompts;`. Chip painter lives in `hud.rs` under marker `// === prompts ===` (reuse the existing key-prompt text fn at `hud.rs ~224`).
- `HudView` gains `prompts: Vec<prompts::Prompt>` (default empty); update `screen_smoke` fixtures.
- Leave alone: M4 screen files (they consume your chip painter later), card files.

## Types / functions
```rust
pub enum State { Available, Required }
pub enum ActionId { Strike, UseCard(u8), Wake, GoDeeper, PickUpgrade, OpenPack, Interact, Reroll, Confirm, Back, Sort, Export }
pub struct Prompt { pub action: ActionId, pub state: State, pub target: Option<[f32;2] /*screen pos if anchored*/> }
pub struct Ctx { /* door_in_reach, shard_in_reach, strike_ready, cards_ready: [bool;5], pack_unopened, choice_pending, mode: hud::Mode, focused: Option<ActionId> ... */ }
pub fn compute(ctx: &Ctx) -> Vec<Prompt>;       // pure
pub fn dims_rest(prompts: &[Prompt]) -> bool;    // true only if a Required prompt exists
pub fn bob(t: f32, state: State, juice: UiJuice, reduced: bool) -> (f32 /*y px*/, f32 /*glow 0..1*/);
pub fn label(action: ActionId, settings: &Settings, pad: Option<Button>) -> String; // via settings::key_for / pad::button_label ONLY
```
- Rules: Available chips show only when the action can actually be performed this frame (strike off cooldown with a target in reach; card charged; door in reach; shard within grab range; unopened pack). Required only for `Choice` mode (upgrade pick, event, wake door); in open play never Required, and never dims.
- Menus: `compute` returns exactly one chip for the focused action; the screen passes `focused`.
- Pad glyphs: when the last input device was a pad, `label` uses `pad::button_label` through `pad::key_for(button, mode, settings)` inverse; add a pure `pad::button_for(key, mode, settings) -> Option<Button>` if none exists (grep first) with tests.
- Chip look: egui painter, rounded rect with the key text, bob 3 px at 1.2 Hz, glow ramps; Required pulses 1.8 Hz; `reduced_motion` => static with a steady outline; `reduce_flashing` caps pulse amplitude 0.5; Off => static chip, no dimming.

## Tests
- Table test: each `ActionId` appears/doesn't for hand-made `Ctx` values; Required only in Choice; dims_rest only then; label never returns text for an unbound action and matches `key_for` for every default key and every mode in `pad::PROMPTS`; bob bounded and constant when reduced; menus return <= 1 chip.
- `screen_smoke` + `every_screen_stays_inside_its_ui_budget` with 3 chips visible.

## Approval list
Any chip captions (e.g. "WAKE", "GO DEEPER") in `NOTES-prompts.md`.
