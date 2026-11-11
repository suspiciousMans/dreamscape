# NOTES: M4e (in-run HUD + hand + summary + upgrade picks)

## For approval (chip captions)
- None. No chips added and no new player-facing strings. The wake-door / upgrade-pick Required prompt stays M3's (`prompts.rs`).

## Facts
- Reuses `screen_anim` (`advance`, `enter`, `tick`, `GridMotion`) and `ui_motion`; no new animation module.
- HUD (kept calm): the last heart's throb is steady at Off / reduced motion (`hud::heart_beat`); the HURRIED timer's 2 Hz red blink is a steady red with reduce-flashing (`hud::timer_urgent`). Full juice does not beat harder. No tick-up or entrance in the playing HUD.
- Hand: Off now equals reduced (no draw arc, sway, ready lift, pop, ring growth). Recharge strip: the filling cell eases in (`hand::charge_cell_alpha`, monotone), a ready card's cells pulse softly, steady when still. `draw_pose`/`leave_pose` signatures and curves unchanged. Warp parts untouched. Hand layout and first-person/death_fx code untouched.
- Summary: panel eases in with a 16 px rise (`enter`), rows appear whole at Off / reduced motion (`typed_for`), a row value that starts with a plain integer counts up from 0 once the row has typed (`ticked`; "x1.5", "3.2", "1,204" are left alone), dust ribbon eases in after the rows. Row text, order and layout unchanged.
- Upgrade picks: deal-in uses ease-out-cubic (`deal_eased`, everything at once at Off), the selected card springs 1.0 -> 1.04 and floats up via `GridMotion` (state in egui memory keyed by `choice.age`), the old sine bob is gone. A still selection sits 10 px up as before.
- Tests: 5 new unit tests (hand, hud, summary x2, upgrade) and 4 `m4e_` screen tests (choice, wake choice, summary at Off/Soft/Full +/- reduced, 1/3/5-card hand at both sizes and all juices, budgets at Full). `audit_text_fits_every_screen` and `every_screen_stays_inside_its_ui_budget` pass.
- Budgets measured at Full juice (shapes / triangles): Playing with a 5-card deck hand 273 / 2509 at 640x360 and 274 / 2509 at 1280x720 (gated 340 / 3300); Summary 53 / 438 and 53 / 498 (gated 80 / 600); Choice 122 / 782 at both (gated 160 / 1100). The shipped Playing budget (160 / 800) covers the HUD without a hand.
- Screens unseen (no display).
