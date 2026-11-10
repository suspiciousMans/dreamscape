# Brief M4: screen pass (five parallel streams, M4a-M4e)

Goal: every screen feels smooth and rewarding: eased entrances, hover spring, tick-up numbers, a focused-action chip, warped cards.

Common to all streams (needs M0 `ui_motion`, M1 `card_warp`, M3 `prompts` merged):
- Entrance: panels ease in (`ease_out_cubic`, 0.18-0.28 s, staggered 30 ms per item); no entrance when `reduced_motion`/Off.
- Hover/focus: `Spring` scale 1.0 -> 1.04 and a slight lift; selected card gets stronger warp (hover param) and a soft glow ring.
- Numbers (dust, ash, shards, stats) tick up with `ui_motion::tick_up` over <= 0.8 s; the last digits settle with a tiny ease-out-back scale.
- Press feedback: 80 ms squash (0.96) then spring back; tiny, never a bounce loop.
- Focused-action chip (M3 painter) beside the focused control only.
- Keep layouts/behaviour; do not restyle colours or change flow. Add no new text except chip captions. Fix any overflow you cause (panels that fit exactly overflow when a block is added: scale block sizes, do not drop lines).
- `screen_smoke` at 640x360 and 1280x720 for each screen; `audit_text_fits_every_screen`; UI budget test. Menu screens that select by index map the cursor through `sorting::order` and identify cards by number.

| Stream | Screens (files) |
|---|---|
| M4a | title + settings: `title_ui.rs`, `settings_ui.rs` (also add the UI-juice row if M0 did not wire it), codex/memories: `codex_ui.rs`, `memories_ui.rs`, `code_ui.rs` |
| M4b | loadout + deck builder + booklet: `loadout_ui.rs`, `deck_ui.rs`, `booklet_ui.rs` |
| M4c | shop + tickets + lottery: `shop_ui.rs` (not the relic tab, R6 owns it), `tickets_ui.rs`, `lottery_ui.rs` |
| M4d | pack reveal + merge: `reveal_ui.rs`, `merge_ui.rs` (use `reveal` beat functions; keep the rarity-scaled beats) |
| M4e | in-run HUD + hand + summary + upgrade picks: `hud.rs` (not the prompt marker), `hand.rs` (non-warp parts: draw pose easing, charge ring), `summary_ui.rs`, `upgrade_ui.rs` (Required prompt for the wake door/upgrade pick comes from M3) |

Conflict rule: streams own disjoint files. `main.rs` edits are limited to adding view fields; hint bars on merged screens get read by the parent after merge (a past merge lost the deck builder's `[t]/[r]` sort keys).

## Tests (each stream)
- Pure timing fns for its screens (entrance delays, tick schedule), monotone and bounded; `Off` => final state at t=0.
- No per-frame allocation regressions in loops over cards (reuse buffers).

## Approval list
Only chip captions, to `NOTES-m4<x>.md`.
