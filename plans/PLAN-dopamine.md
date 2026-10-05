# Plan: making dreams worth wanting (retention loops)

Principles: reward discovery and mastery; no real-money anything; odds are shown honestly;
missing a day is never punished (no loss-aversion streaks); every dry spell is bounded (pity).

Existing pieces this builds on (do not duplicate): `cards.rs` (Rarity, Card, recall, memory_chance,
fade_dust, shape_first_pack, Booklet), `store.rs` (Stash, Dream Dust, Perk), `progress.rs`
(Codex, daily_seed, ascension), `reveal_ui.rs` (pack reveal), `eye.rs` (Moment lines),
`achievements.rs`, `fusion.rs` / `merge_ui.rs` (moth).

## Wave 1: pure, tested modules (parallel, disjoint files)

| Stream | Owns | Summary |
|---|---|---|
| A lottery | `lottery.rs` | Seeded dust-funded pull + pity + sparks + pack pity |
| B worth | `worth.rs` | Quirks, foil, worth, mastery, echoes: dreams that mean something |
| C streaks | `streaks.rs` | Kind daily streak, clean-dream streak, goal gradient, weekly challenge |
| D reveal beats | `reveal_ui.rs` (pure fns only) | Reveal order/pacing: best last, near-miss, rarity-scaled beats |
| E eye voice | `eye.rs` | Eye lines that unlock with run count / streaks / foil finds |

All new persisted fields need `#[serde(default)]` (booklet save compat).
Each module is pure + unit-tested, `#![allow(dead_code)]` until wired in wave 2.

## Wave 2: wiring (serial, by the main session)

main.rs / hud.rs / title_ui.rs / booklet_ui.rs / shop_ui.rs / summary_ui.rs:
lottery screen in the store, quirk/foil/mastery on cards, streak toast + goal line on title,
"one more dream" teaser on the wake summary, pack pity in `open_pack`, eye lines on events.
