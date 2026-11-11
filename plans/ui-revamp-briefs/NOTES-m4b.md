# NOTES: M4b (loadout + deck builder + booklet)

## For approval (chip captions)
- None. The one chip (deck builder, beside the selected tile) shows only the key text from `prompts::label(Confirm, ..)` (`ENTER` / pad button), caption empty.

## Facts
- New `screen_anim.rs` (pure + one egui-memory helper `advance`): entrance stagger, focus spring, press squash, meter spring, tick. State lives in egui temp memory: no game-struct changes. `main.rs`: one `mod` line + `m4b` test block.
- `loadout_ui::draw_grid` keeps its signature (merge_ui still calls it, unchanged look); new `draw_grid_motion(.., &GridMotion)`. `booklet_ui::card` keeps its signature; new `card_hover(.., hover)`.
- Deck builder: pips + tiles ease in (diagonal stagger 30 ms, 240 ms), selected tile 1.0 -> 1.04 and floats, 80 ms squash on add/remove, meter bar climbs by spring, `DECK n/12` ticks up on open, detail card warp hover = 1 (springs up on cursor move). Hint bar text untouched (`[t]`/`[r]` kept).
- Cursor identified by card number (the view's cards are already `sorting::order`ed); the focus spring follows the number across a re-sort.
- Booklet: cards rise + fade in per page (restarts on page turn), "N cards" ticks up on open. No chip (nothing is focused there); booklet cards keep hover 0.
- Off / reduced_motion: everything at its final state on frame 1 (hover warp still 1 for the selected card; Off has a plain quad).
- Screens unseen (no display).
