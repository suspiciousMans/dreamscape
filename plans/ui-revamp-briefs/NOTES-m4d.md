# NOTES: M4d (pack reveal + merge)

## For approval (chip captions)
- None. Both chips show only the key text from `prompts::label(Confirm, ..)` (`ENTER` / pad button), caption empty:
  - Merge: beside the instruction ribbon, only when the cursor's card can be picked/merged (hidden on "same dream"; skipped if it would leave the screen).
  - Pack pick: above the chosen pack.
- No new player-facing strings.

## Facts
- Reuses `screen_anim` (advance, enter, tick, GridMotion) and `booklet_ui::card_hover` (card_warp); no new animation module. Pure helpers in `reveal_ui.rs` (`card_enter`, `land_pop`, `land_hover`, `settled_for`, `count_up`, `pick_slide`) and `merge_ui.rs` (`result_key`, `result_scale`), each in a `// === m4d ===` block.
- Merge: grid uses `draw_grid_motion` (cursor identified by card number, list is the `sorting::order`ed view list); altar slots ease in staggered; result card eases 0.92 -> 1.0 each time the preview changes; RESONANT PAIRS count ticks up; slot cards get warp hover for the cursor's card. Hint bar text untouched, test `m4d_merge_keeps_its_hint_bar_keys` pins its keys.
- Reveal: `beat`/`reveal_times`/`effects` untouched (test pins tiers, shake, hold). Added: staggered face-down entrance (only when there was no tear), rarity-scaled landing settle (`land_pop`, tier 0 none), warp hover fading 1 -> 0 after landing, counts ("N of M stayed", cards/dust/held) tick up once the last card is in. Reduce-flashing now also removes the white flash and the near-miss stutter (face fades steadily). Juice Off removes shake/flash/burst/pop.
- Pack pick: each pack slides + fades in 80 ms after its left neighbour; chip above the chosen pack.
- Layout fix: after pressing, when the report/goal lines show, the card area shrinks (14/42 px) so card faces no longer sit under those lines in small windows (was overlapping at 640x360).
- Budgets (Full juice, measured): Merge 297 shapes / 2.5-2.6k tris at both sizes (budget 500/7200); Reveal 12 cards at 1280x720 585 / 10.2k tris (Off: 585 / 9.9k), gated at 700/12000 by the new test; Pack pick ~3.0k shapes / 26k tris at 640x360 is the pack art, unchanged and not budgeted.
- Test-harness note: the text-fit scanner flags "BECOMES" (label outside the result glow ring) on the unchanged merge screen too; `m4d_fits` filters that one false positive, and the reveal goal line over a rarity halo.
- Screens unseen (no display).
