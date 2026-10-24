# NOTES: UPGRADES stream (feat/items-upgrades)
All in `// === upgrades2 ===` blocks. Every number is a named const in `upgrades.rs`.

## Passives (17, appended at END of PASSIVES; 21 -> 38)
Common: TAILWIND (+6% speed x3), LIGHT BONES (gravity x0.93 x2), NIGHT OWL (+8% sight x3), DOWSING (+5% shard chance x3), PACK RAT (+4 skip dust x3), QUICK HANDS (strike cd x0.85 x2), NIMBLE DRAW (draw delay x0.75 x3), RESTFUL (ability cd x0.9 x3).
Rare: HUSHED STEPS (alert x0.88 x3), SPARE COIN (+1 reroll x2), MOTH'S FAVOUR (-10% shelf x3), HOMING CARDS (shuffle-back x0.6 x2), FREE RESHUFFLE (first card used returns at once, no cooldown), SLOW BURN (+15% ability duration x2), AFTERGLOW (+0.35s hit safety x2).
Mythic: IRON NERVE (+1 nerve, capped at 5), TITHE (+60% dust).
## Synergies (9 -> 19)
NIGHT RUNNER, DIVINER'S ROD, LEAN TIMES (skip x2), CASCADE (deck x0.5), ENDLESS HAND (2 free reshuffles), REGULAR (-10% more), FLURRY, WHISPER (hush x1.5), BRACE (+0.4s), FEATHERLIGHT (+15% jump).
## Curses (5 -> 11)
HOURGLASS, SLEEPWALKER, THIN SKIN (-1 nerve max, floor 2), MAGPIE, BORROWED TIME, RIGGED DECK.
## Wiring
- Existing accessors (speed/jump/sight/gravity/alert/grace/shard_bonus/dust/memory_bonus/cooldown_for/duration_for/rerolls_left) fold the new terms in.
- New accessors: skip_dust, strike_cooldown, nerve_max, moth_pct_off, afterglow, deck_perks. main.rs: combat passives (nerve max, strike cooldown, hit safety), skip_choice + hint, `moth_terms`, `update_deck` (sets `Deck.perks`; `deck::DeckPerks`, `Deck::play` honours free reshuffle), `Nerve::lower_max`, field `thin_skin_applied`.
- `Stats` gained gravity, skip_dust, rerolls, strike_cooldown, nerve_max, moth_pct_off, deck, afterglow, so `available()` offers them.
## Not verified / caveats
- No screenshots (no GPU). Card text fit covered only by `audit_text_fits_every_screen`.
- Deck perks are inert in co-op/autopilot/resumed runs (no deck there); DeckPerks passives still show but do nothing.
- Seeds: PASSIVES/ALL_CURSES grew, so seeded pick rolls differ from before (unavoidable); same seed still replays the same.
- No new player-facing story lines.
