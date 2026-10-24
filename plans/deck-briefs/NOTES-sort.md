# NOTES: SORT stream (feat/sort)

## Built
- `sorting.rs` (commit 1): `SortKey{Rarity,Dream,Number,Ability}`, `Dir`, `Sort{key,dir}`, `cycle_key()`, `flip()`, `label()` ("RARITY ▼"; ▼ = Desc), `order(&[Card], Sort) -> Vec<usize>`. Stable; ties break by card number in the sort's direction (flip = exact reverse). Dream compares names case-insensitively; Ability compares `powers::power(theme).active.name()`. 8 unit tests.
- **Default deviation**: `Sort::default()` = Number **Asc** (the brief says Desc "= today's order", but `Booklet::cards()` is oldest-first, so Asc preserves today's order and keeps existing tests green). Desc = newest first, as specified.
- Wiring (main.rs `// === sort ===`): `sorted_cards(sort)`, `sort_press(cycle)`; booklet (paging, hud view, [p] export), merge (`merge_sel` indexes the sorted list, `merge_first` stays a card number, mouse via sorted view cards), loadout (`loadout_sel` same). `[t]` cycle field, `[r]` flip. Cursor/page stays on the same card after a change.
- Persisted per screen in `Settings` (`sort_booklet`, `sort_merge`, `sort_loadout`, serde default; old saves load).
- Labels in hint bars (`[t] sort: RARITY ▼  [r] flip`); `HudView.booklet_sort`, `MergeView.sort`, `LoadoutView.sort`.
- Pad: Booklet Y=[t], Back=[r]; Merge/Loadout RB=[t], LB=[r].
- Tests: `sorting_keeps_the_cursor_on_the_card_and_the_mouse_follows_the_order`, `sorted_screens_draw_and_old_settings_still_load` (frame check per screen with non-default sort). `screen_smoke::game()` now uses default Settings and a temp settings path (sort presses persist; tests must not write the shared settings.ron).

## Run
sorting::, screen_smoke (81 pass), settings, loadout, pad, merge, mouse filters. Full suite NOT run.

## Not verified / not wired
- Visual check of the ▲▼ glyphs and hint-bar width: no screenshot (Vulkan init fails here). `audit_text_fits_every_screen` passes.
- `deck_ui.rs` (DECK stream) must call `sorting::order` itself; no deck screen sort is wired here.
- Codex/lottery screens not sorted (out of scope).
