# NOTES: HANDSCORE (branch feat/handscore)

## Built
- `handscore.rs` (pure): `classify`, `best_in_deck`, `scale_dust`, `HandRank` (name/pct/label), all multipliers are `*_PCT` consts (hundredths), `SCORE_MAX_PCT = 180`.
- Single best multiplier wins (no stacking). Tool cards are wild for the attribute only, one per hand (a second is inert); they never help FLUSH/RAINBOW. Hands under 5 score on what exists; FIVE OF A KIND/FLUSH/RAINBOW need a full 5.
- Rounding: `(base * pct + 50) / 100` (half up); a rank above NONE with base >= 1 pays at least +1. Never below base.
- Dust scaled AT EARNING by the hand then (`earn_dust` in main.rs, `// === handscore ===`): enemy loot, WINDFALL, undone nightmare, high cache, dream bonus, nightmare portal dust, and the pack dust at run end (`save_journal`: extra = scaled - base; the run's own `bonus_dust` is not scaled twice).
- NOT scaled: banked stash, prices, lottery/coin/tickets, skip dust, event/pact dust, start dust, record dust. Calm motes give nerve, not dust, so nothing to scale there.
- Inert (None, tag hidden): no deck (fixed loadout, resumed run), co-op, autopilot.
- HUD tag above the hand (hand.rs `draw_score_tag`), soft pulse on rank change (`score_pulse`, off with reduced motion); hidden at NONE. Recomputed every `update_deck` tick, so draws/discards update it.
- Deck builder: line "best hand with this deck: X" (needs >= 3 chosen cards), centred under the title.
- Booklet `best_hand` (`#[serde(default)]`); summary row "best hand" = best of THIS run; achievement `full_house` (45 total, ACHIEVEMENTS.md wave 3) from the saved best.

## Not wired / unverified
- `HandFace` has no `foil` field: the game has no foil cards. Add one when foils exist.
- No screenshots (no display): tag and deck-builder line position checked only via `audit_text_fits_every_screen` (HUD tag with FIVE OF A KIND, summary row, loadout with 4 chosen).
- Tests: `handscore` (13), `achievements`, `audit_text_fits`, `screen_smoke` green; full suite not run.

## For approval (new player-facing text)
- Rank names: NONE (never shown), PAIR, TWO PAIR, TRIO, FULL HOUSE, QUAD, FIVE OF A KIND, FLUSH, RAINBOW, RESONANT PAIR
- HUD tag: `<RANK>  x1.30`
- Deck builder: `best hand with this deck: <RANK>`
- Summary row: `best hand` / `<RANK>`
- Achievement: `Full House`
