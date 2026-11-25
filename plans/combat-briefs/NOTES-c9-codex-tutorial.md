# NOTES: combat codex entries + strike tutorial (C9 leftovers)

## What was built
- `dream::theme::EnemyKind` gains `Tooth`, `Weeper`, `Lullaby` (spawned by
  `newfoes`, never from a theme list, so they stay out of `ALL_ENEMY_KINDS`).
- `dream::ALL_CODEX_ENEMIES` (11): the five theme enemies, the three
  `foespawn` kinds (Moths, Mask, Leech) and the three combat foes. Display order
  only; `ALL_ENEMY_KINDS` is unchanged, so no spawn table moved.
- The codex's STRANGE THINGS panel reads `ALL_CODEX_ENEMIES` (it used
  `ALL_ENEMY_KINDS`, which is why Moths/Mask/Leech were never listed either).
- Sightings are recorded where foes actually spawn: `spawn_specials` (theme
  specials), `spawn_enemies_b` (moths/mask/leech) and `spawn_newfoes` (tooth/
  weeper/lullaby, via `DreamscapeGame::codex_kind`). Before this only the theme
  lists recorded, so nothing outside `ALL_ENEMY_KINDS` could ever unlock.
- `codex_ui`: the strange-things list scales its type to the slot and clips or
  drops the hint on small windows, so 11 entries can no longer overlap. New pure
  helper `codex_ui::clip_lines`.
- Strike tutorial: `tutorial::strike_prompt(key)` plus
  `DreamscapeGame::say_strike_tutorial`, called once per launch from
  `load_dream` after the spawns. It writes one line onto the dream's title-card
  hint the first time a dream has anything that can hit back. Skipped in the
  prologue and in co-op; the key comes from `prompts::label(Strike, ..)`, so a
  pad shows its own button.

## For approval (new player-facing text)
Codex hints (`EnemyKind::hint`, shown in STRANGE THINGS once met):
- tooth: "a tooth. it lunges from the dark. dash through it, then strike."
- weeper: "a weeper keeps its distance and spits tears. close the gap."
- lullaby: "a lullaby. its face is armoured. hit it from behind, or jump its ring."

Tutorial line (first dream with a foe, appended to the title-card hint):
- keyboard: "something here can hurt you. [F] strikes."
- pad: "something here can hurt you. [B] strikes."
- no binding found: "something here can hurt you. strike it back."

The six existing codex hints are unchanged.

## Tests
- `dream::theme::tests::the_codex_lists_every_foe_once_and_each_says_something`
- `codex_ui::tests::long_hints_are_clipped_to_the_rows_they_have`
- `tutorial::tests::the_strike_line_names_the_key_and_has_a_fallback`
- `screen_smoke::the_first_fighting_dream_says_how_to_strike_once`
- `screen_smoke::every_combat_foe_has_a_codex_name`

## Unverified
The author's sandbox has no cmake, so the SDL2 bundled build (and therefore
`cargo test`) could not run here. Nothing was built or run, and the codex's
eleven-row layout was not seen on screen; it is arithmetic from the existing
panel geometry (slot height, scaled fonts, clipped hints), covered only by the
`clip_lines` unit test and the unchanged screen budget/audit tests.
