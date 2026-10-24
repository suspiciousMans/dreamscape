# NOTES: RISK stream (branch feat/risk)

## Built
- `gamble.rs` (pure, 13 tests): `STAKES=[5,10,20]` (never > half run dust, min 5), `GAMBLE_LOSS_CAP=2` then a guaranteed win ("the moth owes you one"), `MAX_PER_VISIT=3`, 50% coin; `RunState` (saved: `flips`, `losses`, `wager_next/active`, `pick_owed`). Flip seed = `dream_seed ^ 0x6A4B_1E00 ^ flips*0x9E3779B9`; `flips` is saved with the dust in `save_run` before the result shows, so reload cannot re-roll.
- Moth shop: new row GAMBLE after REROLL, key `[g]`. Screens: stakes (odds line shown) -> confirm (default = NOT NOW) -> result -> back to shelf. `[x]` steps back. Run dust (`bonus_dust`) only.
- Wager portal: `WAGER_PORTAL_CHANCE` 1/6, depth >= 4, normal dream only (`pressure_applies`, not autopilot/co-op/prologue), next dream not a nightmare/sanctuary; rolled from `dream_seed ^ 0x57A6_E200`, placed on a dream `lairs` cell (off route, >= 3 cells from spawn/portal/shard; pink orb, no collider). Touching it opens the deal screen (default = NOT NOW). Taking it melts like the normal portal; next dream is wagered: enemies x1.3 (`pressure_base`), hazard strength read at depth+1 (`hazard_rule_state`), dust earned in it x2 on leaving by portal, one extra Rare+ pick after the next normal pick (`moth_after_nightmare`, boss floor, own seed salt). Waking/new run clears it. `SavedRun.gamble` is `#[serde(default)]`.
- Markers: `// === risk ===` in main.rs and progress.rs, `// === wager portal ===` in gamble.rs.

## Tests
`gamble::` (odds, caps, guarantee, determinism, placement off-route over all themes), main.rs coin/deal/wager/save tests, audit_text_fits_every_screen covers shop+coin screens and deal. The existing shop card-count assertion went from +1 to +2 (reroll + coin).

## Player-facing text for approval
ROW: GAMBLE / "put some run dust on a coin. win and the stake is doubled, lose and it is gone." / tag "double or nothing" / "enough for this visit" / "needs 10 dust". Stakes: "{n} DUST", "win: you get {2n} · lose: {n} gone", "keep your dust. nothing happens.", "BACK", "NOT NOW", "FLIP FOR {n}". Odds: "50% double · 50% lose the stake" / "the moth owes you one: this coin is a win". Results: "THE COIN FELL YOUR WAY" / "THE COIN FELL HER WAY". Deal: "A SECOND DOOR", "harder dream, double reward", "TAKE THE DEAL", "NOT NOW", plus descriptions in gamble.rs.

## Not wired / unverified
- No screenshots (no display): wager orb look and coin screens unseen.
- Wager dust start is in memory: a quit inside a wagered dream resumes with current dust as the baseline (loses part of the x2). Visit count (3 per visit) is in memory too.
- If the post-dream pick is an event instead of an upgrade, the owed extra pick waits for the next upgrade pick.
- A second wager door can appear inside a wagered dream (chaining is opt-in each time).
- No sound/animation for the coin beyond existing Pick/Denied sounds.
