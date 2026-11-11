# NOTES: M4a (title + settings + codex/memories/code)

## For approval (chip captions)
- None. Two chips (title menu, settings list), each beside the focused row, show only the key text from `prompts::label(Confirm, ..)` with an empty caption. Drawn only if the chip fits right of the panel (always at 1280x720; skipped when the panel leaves no room, e.g. narrow windows).

## Facts
- Reuses `screen_anim` (M4b). Additive only, in an `// === m4a ===` block: `chip_beside(right, y, key, screen_right)` and `list_motion(painter, id, time, sel, settings) -> GridMotion` (state in egui temp memory, keyed per screen).
- Title: menu rows rise + fade in (30 ms stagger), focused row springs (1.0 -> 1.04 text, small lift), stats strip (dust / cards / deepest) ticks up (strip sized by the final numbers, so no layout shift).
- Settings: rows ease in, focused row springs; no chip while a key is being captured. The UI-juice row was already wired by M0.
- Codex: dream cells and strange-thing panels ease in; best depths tick up. Memories: dreamers ease in, focused springs, the notes list re-fades on each dreamer change. Code: the three lines drift in.
- Off / reduced motion: final state on frame 1 (tests assert it). No new Action/Row/Sound/settings field.
- Memories had no budget entry; measured 126 shapes / 1748 tris at 640x360 Full, test cap 140 / 2200.
- Existing, untouched: the code screen's cursor blink (`_`) still blinks under reduced motion.
- Screens unseen (no display).
