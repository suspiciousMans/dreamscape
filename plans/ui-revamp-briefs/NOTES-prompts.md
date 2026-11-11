# NOTES: prompts (M3)

## For approval (player-facing chip captions)
- `WAKE` (wake door, Wake highlighted)
- `GO DEEPER` (wake door, second option highlighted)

All other chips show only the key/button text (e.g. `B`, `ENTER`, `ESC`), taken from `settings::key_for` / `pad::button_label`.

## Facts
- `Interact` (door / shard in reach) has no key: doors and shards open by touch. `label` returns "" and the painter draws no chip. `compute` still reports it so a later stream can anchor a non-key cue.
- Card chips show only when a card is charged AND a foe is within 12 units (otherwise five idle chips would sit on screen all the time).
- `prompts::label` takes `(action, mode, settings, pad: bool)` instead of `Option<Button>`: the button depends on the screen's mode, which a `Button` argument cannot carry.
- Choice screens, menus and the pack pick are M4 files and return before the chip painter runs. They call `hud::prompt_chip` (one chip) and `hud::prompt_dim` (Required dim) and take `prompts` from `HudView`.
