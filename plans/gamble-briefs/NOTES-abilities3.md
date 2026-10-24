# NOTES: ABILITIES3 (feat/abilities3)

## Built: 8 actives (Ability 24 -> 32, `ALL_ABILITIES` 32, `ALL_SOUNDS` 80 -> 91, 11 new sounds)
Pure logic + tests in `abilities3.rs`; wiring in `main.rs` inside `// === abilities3 ===` (`fire_ability`, `abilities3_ready`, `abilities3_before_play`, `abilities3_after_play`, `fire_borrowed`, `update_abilities3`, `plant_snare`, `swap_target`, `root_foe`). Deck helpers (`mulligan`, `recall`, `recall_candidate`) in `deck.rs`; `Nerve::spend` in `nerve.rs`.

| Ability | cd | Effect / numbers (all consts in abilities3.rs) |
|---|---|---|
| WAGER | 20 | random from `ALL_ABILITIES` minus Wager/Echo/Mulligan/Recall; own stream `dream_seed ^ WAGER_MIX` + wager index (reload cannot re-roll). 10% jackpot (duration x2; instant ones refund Wager's whole cooldown), 10% fizzle (nothing, half cooldown back), pity: 12th wager without a jackpot is one. Desc states the odds. HUD hint line shows `WAGER: NAME` for 3.5s. If the pick has no target ("nothing to use it on") it is shown, cooldown stays. |
| ECHO | 20 | repeats the last real ability (never Echo/Wager shell) free; original's recharge is not touched; denied if none |
| SWAP | 12 | nearest foe or decoy within 12 (floor within 3): positions exchanged, foe stunned/staggered 2s |
| MULLIGAN | 25 | deck only: hand thrown back, pile shuffled (deck rng), full fresh hand; old cards flick out |
| RECALL | 14 | deck only: most recent played card (not Recall's own, not already in hand) leaves discard/draw pile into the first empty slot, keeps HALF its cooldown (`RECALL_COOLDOWN_LEFT`); property-style test: no card lost/doubled |
| OVERLOAD | 16 | hold key up to 2.0s (translucent ring at feet grows; card fills), release: stagger radius 3..10, 0.8..3.0s. Via Wager/Echo it fires a 1.0s-charge blast |
| BARGAIN | 18 | spends 1 nerve (refused at 1; not a hit, no iframes); next 3 strikes x3 damage, ends after 3 swings or 10s |
| SNARE | 12 | trap at feet, max 2 (3rd takes up the oldest); first foe within 1.2 is rooted 3s (pacer stunned, others staggered); trap spent |

Mechanics worth knowing: WAGER/ECHO fire the target through a temporary `AbilityState` pushed past the hand (`fire_borrowed`, same "ghost" idea decks use); it is `spent`, removed when its effect ends (`RunUpgrades::tick` now also does this in fixed-loadout runs; `hand_view` hides it). Key release is tracked by `ability_held` (set before the playing-mode early return).

## Getting them
- Fixed loadout: the moth's ability offer now draws from abilities2 + abilities3 (Mulligan/Recall excluded: `available` refuses them without a deck). Side effect: the shelf offer changes for the same seed (14 candidates instead of 8).
- Deck runs, resonant pairs (`PAIR_ABILITIES` 4 -> 10; only pairs that were free, no existing card changes): Echo = Synesthesia+Lobby, Recall = DrownedLibrary+Jellyfish, Swap = SkyStairs+Void, Mulligan = Fractal+Elfworks, Wager = CursedForest+WhiteDissolve, Overload = Garden+Mycelium.
- BARGAIN and SNARE have no card (`PICK_ONLY`): fixed-loadout moth only (or `DREAMSCAPE_GIVE=Snare`). Owner decision: add a dream/pair or sell them in deck mode.

## Owner approval: new player-facing lines (powers.rs pair flavours)
- "the hall says it again, softer" (Echo)
- "what sank comes drifting back up" (Recall)
- "up and down trade places" (Swap)
- "the pattern deals itself again" (Mulligan)
- "the forest keeps no promises" (Wager)
- "the roots gather, then let go" (Overload)
Ability descs (upgrades.rs) and the wager line ("WAGER: DASH (jackpot)", "(fizzled)", "(nothing to use it on)") are plain item text.

## Not verified / limits
- No display: icons, rings, trap discs, sounds' feel unseen; `screen_smoke` (120) passed. Ring/trap entities need `player_tex` and a mesh (absent in tests), so only their logic is tested.
- Deck-hand animation of Mulligan/Recall relies on existing `Drawn`/leaving paths; not seen.
- Wager is not inert in co-op/autopilot beyond the existing rule that abilities fire there if pressed (no stakes, no persistence). No combos defined. OVERLOAD's key-release path is tested by setting `ability_held`, not by SDL events (pad buttons go through the same key path).
- Echo/Wager of ANCHOR drops a marker with no second stage unless Anchor is held; Echo of Overload/Mulligan/Recall works (free).
- Full suite not run (owner does at merge); ran: abilities3, deck, powers, sounds, pixels, upgrades, moth_shop, nerve, secrets, achievements, hand, combat, abilities2, screen_smoke.
