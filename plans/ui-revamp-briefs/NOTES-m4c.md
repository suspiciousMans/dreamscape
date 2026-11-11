# NOTES: M4c (shop + tickets + lottery)

## For approval (chip captions)
- None. Three chips, all key text only (`prompts::label(Confirm, ..)`, caption empty):
  - Shop: right of the action line in the details panel, only when the action can be taken (affordable buy / top-up, or equip).
  - Tickets: above the ticket, only while `tickets > 0`.
  - Lottery: right of the pull strip, only while a pull is affordable.

## Facts
- Reuses `screen_anim` unchanged (`advance`, `enter`, `tick`, `GridMotion`); state ids `m4c_shop`, `m4c_tickets`, `m4c_lottery`. `main.rs`: `m4c` test block only. No hud.rs edits (painter's `ctx()` is used).
- Shop: shelves and items ease in (stagger 30 ms, items drop in from above), cursor item springs 1.0 -> 1.04 and lifts, 80 ms squash when a purchase flips its state, dust / earned tick up on open, details fade in. The old sine bob now runs only when motion is on. Relic tab untouched.
- Tickets: header counters tick up on open; prize rows ease in. Foil scratch untouched.
- Lottery: header dust / pulls tick up on open; odds rows (label, bar, %) ease in. Card stage and shuffle untouched.
- Guardrails: odds, pity and spark numbers are never ticked or animated (test `m4c_odds_text_is_exact...`); no bar climbs, no near-miss or countdown motion; the pity bars are drawn at their value from frame 1 (fade only).
- Off / reduced_motion: final state on frame 1 (test).
- Screens unseen (no display).
