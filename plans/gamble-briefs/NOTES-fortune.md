# NOTES: FORTUNE stream (feat/fortune)

## Built
- `fortune.rs` (pure): once-per-day free pull (`claim`), `FortuneState` (reuses `streaks::DailyStreak`, so rest tokens apply), nudge = `min(prior days, FORTUNE_STREAK_CAP=6) * FORTUNE_STREAK_BONUS_PER_DAY=0.005` added to Lucid+ (max +3.0 points). Seed `fortune_seed(save_seed, day)`: same day = same card, saved on claim. A free pull uses the player's current pity position but restores the pity/spark/pull counters afterwards (neither spends nor breaks pity).
- `lottery.rs`: additive `pull_nudged` / `odds_nudged` (`pull`/`odds` unchanged = nudge 0; existing seeds replay).
- `tickets.rs` (pure): table (weights/100): 20 dust 40, 40 dust 24, 80 dust 14, 150 dust 6, lottery pull 8, unowned cosmetic 6, card-pick hint 2 (fills sparks to a spark pick). `TICKET_PITY=4` smallest-prize scratches in a row remove the smallest prize from the next table. No unowned cosmetic left: its 6 points are paid as the 40-dust prize and the table shown says so. Seed `ticket_seed(save_seed, saved scratch count)`.
- Earning: +1 per clean dream (no nerve lost, not caught) up to `TICKETS_PER_RUN_CAP=5` per run; +1 per new achievement; +2 per weekly challenge. Bank cap 99. Solo only (inert in co-op, autopilot, prologue).
- `tickets_ui.rs` + store tab 3 (tab cycles perks, lottery, tickets). Two-step: Enter asks, Enter confirms, any other key cancels. Prize written with `persist_booklet` before the foil animation; reduced motion = instant. Dev hook `DREAMSCAPE_SCREEN=tickets|tickets_scratch` (gives 3 tickets; use a temp booklet).
- Lottery tab: `[f]` claims; the free pull's exact odds + streak nudge are listed under the title while one waits. Title bottom line: "a fortune is waiting in the Lucid Store" (plain, joined to the goal line).
- Saves: `Booklet.fortune`, `Stash.tickets/ticket_scratches/ticket_dry`, all `#[serde(default)]`; old-save tests pass.

## Tests
`fortune::`, `tickets::`, `tickets_ui::`, 3 new `screen_smoke` tests (fortune day, two-step scratch + saved-before-reveal + reduced motion, clean-dream cap/autopilot), audit_text_fits_every_screen covers the new tab. Passed: fortune, ticket, audit_text_fits, lottery, title, screen_smoke, store, booklet. Not run: full suite. No screenshots (no display): unseen.

## For owner approval (new player-facing text)
- Title: "a fortune is waiting in the Lucid Store"
- Lottery: "FREE FORTUNE [f]: ... (streak +x%, N days)", "today's fortune is already yours", "[f] free fortune"
- Tickets screen: "DREAM TICKETS", "WHAT A TICKET CAN BE", prize labels ("A LOTTERY PULL", "A STORE COSMETIC", "A DREAM OF YOUR CHOOSING"), "THE SMALLEST PRIZE LEAVES THE TABLE IF IT COMES N MORE TIMES", "THE SMALLEST PRIZE IS OFF THE TABLE FOR THE NEXT SCRATCH", "earned: a clean dream, a new achievement, a weekly challenge", "scratch one ticket? press enter again to confirm", "no tickets yet: clean dreams, achievements and weekly challenges earn them", reveal details ("added to your stash", "a spark pick is ready in the lottery tab").

## Judgement calls
- "Resonant hint" built as a spark pick (existing `[p]` flow), not a new Resonant mechanic.
- A veteran's first launch may grant a burst of tickets (one per achievement newly unlocked by `achievements::check`), capped at 99.
- The ticket "lottery pull" is a normal pull (counts toward pity, no cost).
