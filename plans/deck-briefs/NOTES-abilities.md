# NOTES: ABILITIES stream (feat/items-abilities)

## Built: 8 new actives (Ability 16 -> 24, ALL_ABILITIES 24)
Pure logic + tests in `abilities2.rs`; wiring in `main.rs` inside `// === abilities2 ===` blocks (`abilities2_ready`, `abilities2_on_use`, `update_abilities2`, `break_mirror`, `pulse_reveal`, plus hooks in the enemy loop, `strike_ctx`, `player_speed`, dash velocity, jump, caught flow, leech latch).

| Ability | Effect | cd / dur | Hooks |
|---|---|---|---|
| TETHER | yanks nearest shard or calm mote (<=16) to you at 14 u/s | 12 / 2.5 | denied (charge kept) with nothing in range |
| MIRROR | next hit (touch or leech) is thrown back: attacker takes 1, all within 6 stagger 2.5s, 0.8s grace | 22 / 8 | beside WARD in the caught flow |
| QUAKE | floor ripple: everything within 8 staggers 2.2s (no damage); needs ground | 10 / 0 | via `shock` |
| VEIL | chasers drop the chase and cannot see you; every strike counts as unaware (sneak attack) | 16 / 5 | enemy loop `seen_from`, `strike_ctx` |
| SURGE | next jump and next dash x2 | 14 / 6 | `Surge` consumed by jump / dash |
| PULSE | Shades show from 40 away; tall pillar over the cracked wall and each Shade | 15 / 4 | enemy loop, `pulse_marks` |
| HASTE | zone r6 at your feet: you x1.5, chasers' time x0.5 | 18 / 6 | `player_speed`, enemy loop |
| BIND | roots nearest enemy within 12 for 5s (pacer stunned, others staggered so strikes cut deeper) | 13 / 0 | denied with no target |

All numbers are named consts in `abilities2.rs`. Each has icon (`pixels.rs`), burst colour, `Sound` (9 new: TetherPull, MirrorUp, MirrorBreak, QuakeRumble, VeilOn, SurgeUp, PulsePing, HasteOn, BindClamp; `ALL_SOUNDS` 71 -> 80), name/desc/cooldown/duration. Surge duration is 6 (a test needs cooldown > 2x duration).

## Getting them
- Fixed-loadout runs: `Learn(Ability)` works as for the old ones; the moth now always includes one new-ability offer (`moth_shop::ability_offer`, Rare price, deterministic per seed/reroll; costs one ordinary slot). Deck runs never see it (`available` refuses abilities there by design).
- Deck runs get abilities only from cards. Four existing duplicate-ability attributes now teach a new one (existing seeds keep their cards; only these four powers changed):
  - Spore ShardCall -> TETHER, Lumen Phase -> VEIL, Blank Phase -> PULSE, Gaze Stillness -> BIND.
  - No two attributes share an active any more (Bloom keeps ShardCall, Static keeps Phase, Lobby keeps Stillness). Two tests that relied on a shared active (`a_shared_active_is_learned_once`, `the_first_five_cards_teach_different_abilities`) now use two Lobby cards.
- MIRROR, QUAKE, SURGE, HASTE have NO card: deck-mode players cannot obtain them (only fixed loadouts via the moth, or `DREAMSCAPE_GIVE=Quake,...`). Decision for the owner: map them to attributes (changes seeds) or let the moth sell them in deck mode.

## Owner approval: new player-facing flavour lines (powers.rs)
- Spore: "the roots haul it up to you"
- Lumen: "soft light, and nobody sees you in it"
- Blank: "you hear where the walls are not"
(Gaze keeps "they stop when you look".)

## Not verified / limits
- No screenshots (no GPU): icons, pillars, haste disc, burst colours unseen. Pillars/disc use the player texture (Translucent + Lit); they need `player_tex` (absent in tests).
- PULSE's pillars and Shade reveal have no game-level test (needs meshes); pure parts and the others are tested.
- MIRROR only answers the touch catch and leech latch, not other nerve sources.
- Co-op/autopilot: abilities fire the same way if held; nothing special.
- No combos defined for the new abilities.
