# Plan: juice and interaction (reasons to come back, and to feel good while here)

Written by Rei, 2026-10-05, after playing the build at `ed4543f`.
Builds on `PLAN-dopamine.md` (lottery, worth, streaks, reveal beats, eye voice).
That plan covers what players *want*; this one covers how a run *feels* and
what players can *do*. Same principles: no real money, honest odds, missing a
day is never punished, no dark patterns.

## What the code audit found (grep, not memory)

- No hit-stop anywhere. Screen shake exists only in the co-op, lottery and
  reveal screens, not in the run itself.
- Between-dream events (ROADMAP 3.5) are not implemented. Neither are secret
  rooms (3.6).
- Adaptive music (5.4/5.5): the only "intensity" hit is in `dream/build.rs`,
  so the music does not react to danger.
- Seeds exist (`DREAMSCAPE_SEED`, daily seed) but nothing lets a player share
  one. No photo mode.
- The pack reveal and lottery already have good beats. The *run* is the
  flat part: the loop between "pick upgrade" and "wake door" has little
  moment-to-moment feedback.

## Wave A: juice (small, parallel, each shippable alone)

| # | Feature | Touches | Done when |
|---|---|---|---|
| A1 | **Hit-stop + shake on catch and shard grab**: 60-90 ms freeze, scaled shake, bigger for the last shard | `main.rs` (frame step), `fpv.rs` | Reduced-motion setting zeroes it; screenshot burst shows the freeze |
| A2 | **Shard chain feedback**: pickup pitch climbs a note per shard grabbed within 4 s, a ring pulse, dust motes flying to the HUD counter | `sounds.rs`, `hud.rs` | Sound test: pitch strictly rises across a chain and resets |
| A3 | **Near-miss slow-mo**: an enemy lunge that misses by under half a cell dips time to 0.6x for 0.3 s and the eye blinks | `enemy_ai.rs`, `main.rs`, `eye.rs` | Unit test on the miss detector; reduced-motion disables the time dip |
| A4 | **Adaptive music layers**: intensity from nearest-enemy distance + difficulty + shards left, crossfading a calm/tense/chase stem set | `music.rs`, `AUDIO_SYSTEM.md` | Intensity fn is pure and tested; stems crossfade without clicks |
| A5 | **Wake-door fanfare**: the door gets a build-up (light rays, hum rising as you approach) and a distinct "go deeper" vs "wake" stinger | `hud.rs`, `sounds.rs` | Both stingers audible; screenshots of both choices |
| A6 | **Number and card juice**: dust counts up with ticks on the summary, cards tilt toward the cursor/stick in the booklet, foil shimmer follows the tilt | `summary_ui.rs`, `booklet_ui.rs`, `worth.rs` | Pure tick schedule tested; no per-frame allocation regressions |

All of Wave A must honour `reduced motion` and `reduce flashing`. Each item
ships with a headless draw smoke test like the existing ones.

## Wave B: interactive features (the new things to do)

| # | Feature | Why it brings people back | Touches |
|---|---|---|---|
| B1 | **Between-dream events** (ROADMAP 3.5): 1 in 6 gaps is a short choice ("A door hums. Open it?") with a boon, a curse, or a trade | Variance between runs; stories players retell | new `events.rs`, `upgrade_ui.rs` choice screen, `eye.rs` lines |
| B2 | **Secret rooms** (ROADMAP 3.6): cracked walls hidden in layouts, reached by Blink/Dash/Phase, leading to a moth shop dream | Rewards mastery and curiosity | `dream/layout.rs`, `dream/build.rs`; must keep the patient-walker route guarantee |
| B3 | **Shareable seeds**: a copyable code (`DS-XXXX-XXXX`) on the summary screen and a "enter a seed" field on the title screen | Friends can race the same dream, no servers needed | `progress.rs`, `summary_ui.rs`, `title_ui.rs` |
| B4 | **Dream photo mode**: pause then `[f]` freezes, free-orbit camera, saves a PNG with seed + dream name watermark (cards already export PNGs via `[p]`, reuse it) | Screenshots are free marketing; players make their own content | `main.rs`, `booklet_ui.rs` export path |
| B5 | **Ghost runs**: record your best run's path per daily seed, replay it as a faint dreamer next time (and a friend's via file import) | Beat your own ghost; async competition without a backend | new `ghost.rs`; deterministic seeds already make it cheap (inputs + seed) |
| B6 | **Companion rituals**: feed, name and pet your companion from the booklet; it comments on your runs and remembers what you did together | Emotional attachment, a reason to open the game on a bad run | `companion.rs`, `eye.rs`, `booklet_ui.rs` |
| B7 | **Weekly dream remix**: the weekly challenge (`streaks::weekly`) gets a modifier set with its own card back as the reward | Gives a week-scale rhythm on top of the daily one | `streaks.rs`, `title_ui.rs`, `cards.rs` |

## Wave C: social and long tail (after A and B)

- **Co-op juice pass** (`coop_game.rs`): shared shard chain bonus, revive
  pulse, ping marker. Steam lobbies wait on the App ID.
- **Steam hooks**: achievements bridge exists. Add rich presence ("Depth 7,
  the Tunnel") and cloud saves for the booklet.
- **Achievements for the new stuff** (`ACHIEVEMENTS.md`): clean chain of 10
  shards, find 3 secret rooms, beat your ghost.

## Order and rules

1. A1, A2, A5 first. They are the cheapest and change how every single run
   feels. Verify with the same screenshot-burst harness used for the eye
   entrance (`burst.ps1`: scancode input, 250 ms frames).
2. B3 and B4 next. They are tiny and give marketing material before Early
   Access.
3. B1 and B2 together, since both change level generation and need the
   generation test harness (ROADMAP 6.5) extended first.
4. B5, B6, B7, then Wave C.

Rules: one PR per item. New persisted fields use `#[serde(default)]`. New RNG
draws use their own seeded streams so existing seeds replay. Every item is
checked on screen before it is called done. "488 tests pass" was true for the
last commit and the new screens still had a layout bug nobody had seen.

## Decisions (James, 2026-10-05)

1. Steam work (Wave C Steam hooks, co-op lobbies) is deferred. Build the single-player B-wave first.
2. Everything stays code/file based. No servers: seeds are copyable codes, ghosts are files.
3. Keep juice soft: short hit-stop (about 40 ms), gentle shake, slow-mo no deeper than 0.75x. Dreamy, not arcade.
