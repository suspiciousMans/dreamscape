# Dreamscape achievements (design, v1)

Proposed Steam achievements for the Early Access launch. Each one says how it unlocks in code terms. **Tracked today?** says whether the game already records what's needed (yes: it's already in the codex, booklet or run stats; **new**: the unlock needs a small new counter).

Principles:
- Reward seeing the game (dreams, enemies, bosses) and mastering it (depth, no-hit, ascension).
- Nothing grindy beyond ~20 hours.
- Nothing that rewards bad play (no "die 100 times").
- Hidden achievements are marked; they spoil a surprise otherwise.

API names use the `snake_case` Steamworks requires.

## First steps
| API name | Name | Unlocks when | Tracked today? |
|---|---|---|---|
| `wake_once` | Lucid | Wake up from a run for the first time | yes: run completes (`Game complete`) |
| `first_shard` | Something Solid | Collect a lucidity shard | yes: shard collected |
| `first_nightmare` | Face It | Beat a nightmare (all sigils, through the portal) | yes: `codex.nightmares_beaten` |
| `first_card` | Keepsake | Press your first card into the booklet | yes: booklet card count |
| `first_purchase` | Window Shopping, Then Not | Buy anything in the Lucid Store | **new**: cosmetics show in `stash.owned`, but perk charges get used up, so count purchases |

## Depth
| API name | Name | Unlocks when | Tracked today? |
|---|---|---|---|
| `depth_10` | Under | Reach depth 10 in one run | yes: best depth |
| `depth_20` | Deeper Under | Reach depth 20 | yes |
| `depth_35` | Sleep Paralysis | Reach depth 35 | yes |
| `go_deeper` | Not Yet | Choose GO DEEPER at the wake door | **new**: count the choice |
| `white_dissolve` | Blank Page (hidden) | Enter White Dissolve (only reachable by going deeper) | yes: codex dream visits |

## Seeing everything
| API name | Name | Unlocks when | Tracked today? |
|---|---|---|---|
| `all_dreams` | Dream Cartographer | Visit all 20 dream types | yes: `codex.dreams` |
| `trip_dreams` | Afterimages | Visit all six trip dreams (Afterimage Fields, Synesthesia Hall, Melting Clockworks, Jellyfish Sky, Watching Wallpaper, White Dissolve) | yes: codex |
| `all_enemies` | Field Notes | Meet every enemy kind | yes: `codex.enemies` |
| `prophetic_card` | Prophecy | Keep a Prophetic-rarity card | yes: booklet rarities |
| `full_booklet` | Scrapbook | Hold 100 cards in the booklet | yes: booklet card count |

## Bosses
| API name | Name | Unlocks when | Tracked today? |
|---|---|---|---|
| `beat_quake` | Aftershock | Beat The Quake (tier 2) | yes: tier known at nightmare end |
| `beat_swarm_mother` | Mother's Day | Beat The Swarm Mother (tier 3) | yes |
| `beat_eclipse` | Totality | Beat The Eclipse (tier 4, four sigils) | yes |
| `untouched_nightmare` | Didn't Even Blink | Beat any tier-2+ nightmare without being caught in it | **new**: catches-per-dream counter |

## Mastery
| API name | Name | Unlocks when | Tracked today? |
|---|---|---|---|
| `clean_run` | Sound Sleeper | Wake up with 0 catches and 0 falls | yes: `RunStats.caught/fell` |
| `no_abilities` | Bare Hands | Wake up from depth 10+ without ever using an ability | **new**: ability-use count per run (already logged) |
| `synergy` | Better Together | Complete any upgrade synergy | yes: `run.synergies()` |
| `four_synergies` | Harmonics | Hold 3 synergies in one run | yes |
| `ascension_5` | Restless | Beat Ascension 5 | yes: `codex.ascension_unlocked` |
| `ascension_10` | Insomniac | Beat Ascension 10 | yes |
| `daily` | Same Time Tomorrow | Finish a daily dream | yes: `codex.daily_best` |

## Hidden, and silly
| API name | Name | Unlocks when | Tracked today? |
|---|---|---|---|
| `decoy_boss` | Nice Try (hidden) | Use DECOY during a nightmare (the boss ignores it) | **new**: ability use during a nightmare |
| `rewind_loop` | Déjà Vu (hidden) | Use REWIND in Melting Clockworks | **new**: ability + theme check |
| `long_stare` | It Stares Back (hidden) | Look straight at a stalker for 10 seconds in a first-person dream | **new**: gaze timer |

That's 29 achievements. 21 are computable from state the game already saves or tracks; 8 need a small new counter.

## Implementation plan (Phase 3, after the App ID)
- A pure `achievements.rs` with the list above, and `check(&Booklet, &RunStats, &Event) -> Vec<Id>`, tested per achievement. Unlocks are stored in the booklet (`#[serde(default)]`), so they work offline and in non-Steam builds.
- A Steam bridge that calls `set_achievement` + `store_stats` on unlock, and on startup pushes any unlocked while offline.
- Icons: 64×64 pixel art per achievement, locked and unlocked versions, generated the same way as the store icons.
