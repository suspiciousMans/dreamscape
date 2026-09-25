# Dreamscape

A PS2-style dream-escape roguelike built on the Jame engine (Rust). You fall asleep and sink through an endless run of procedurally generated dreams, picking up an upgrade after every one. Collect lucidity shards to wake up (or refuse the wake door and go deeper), then keep the dreams you remember as trading cards.

## Run it

```bash
cargo run -p dreamscape          # from the repo root
cargo test -p dreamscape         # 228 tests
```

## Controls

| Key | Action |
|---|---|
| WASD | move |
| Space | jump (again in mid-air with DOUBLE JUMP) |
| Shift / E | use ability slot 1 / 2 |
| 1 / 2 / 3 or A/D + Enter | pick an upgrade (or WAKE / GO DEEPER at the wake door) |
| R / X (on a pick) | reroll the cards (once a run) / skip for dust |
| C / T / X / O (title) | continue a saved run / today's dream / codex / settings |
| A / D (title, on ascension) | pick an ascension level |
| P (booklet) | save the cards on this page as PNG images (`games/dreamscape/cards/`) |
| controller | left stick moves, A jumps, shoulders / X / Y use abilities, Start pauses, d-pad drives menus |

Every movement and ability key can be rebound in **settings**, which also has music and sound volume, grain and vignette strength, reduced motion and fullscreen.
| R | title screen: toggle run length (short / long) |
| Esc | pause (Q to quit from pause) |
| L / B | Lucid Store / dream booklet (title screen, pause, after waking) |
| F1 | overhead camera |
| F2 | toggle tunnel vision + film grain |

## What's in it

- **Fully procedural dreams.** There are no hand-built levels. Fourteen dream types each have their own layout rules, props, palette, music and "mood" (breathing walls, glow, beat pulse, motion trails). Levels grow with depth. Four are built from things people describe in psychedelic trip reports:
  - **Mycelium Grove**: clearings over the void joined by root bridges; glowing veins lead to the shard, and you run faster on them.
  - **The Tunnel**: one long corridor toward a white light; gates across it open and close in a travelling wave.
  - **Fractal Cathedral**: square rooms nested inside each other, a door on a different side of each.
  - **Elfworks**: a jewelled toy workshop full of jesters, where floor tiles sink and rise.
- **Meta-progression.** Quitting saves your run (continue it from the title). **Today's dream** is the same seed for everyone that day, with its own best. Beating a long run unlocks the next of ten **ascension** levels, each adding one more rule. The **codex** records every dream type and strange thing you've met.
- **Synthesized sound.** Every sound effect is generated in code (no audio files), one per action, enemy and event.
- **A roguelike run.** Every dream you get through offers three random upgrades for the rest of the run: faster feet, higher jumps, wider sight, slower enemies, shard sense, dream anchors, more dust, and more. Rarer cards teach an ability: DASH, BLINK, STILLNESS, PHASE or SHARD CALL. You can hold two abilities at once.
- **Rarity, curses and combos.** Cards come in common, rare and mythic tiers (rarer ones show up more the deeper you go). About one pick in five swaps a card for a cursed one: strong, with a price. Some pairs combine into combos, like DASH + PHASE = GHOST STEP. You get one reroll per run, or you can skip a pick for dust.
- **Special enemies and hazards.** From depth 3, each dream type brings its own threats on top of the pacers:
  - **stalkers** only move while you can't see them;
  - **mimics** walk your own path 3 seconds behind you;
  - **sentries** sweep a beam, and being seen calls the nearby pacers;
  - **drifters** swing across the gaps you jump;
  - **jesters** don't catch you, they throw you somewhere else.

  In long runs some pacers are **elite**: FAST, BIG, SPLITTER (splits in two when it lunges) or SHADE (invisible until close). Open-air dreams have **crumbling tiles** (a different pattern) that fall a second after you step on them, and the forest, library and garden have **purple fog** that slows you down. Each new threat is explained the first time you meet it.
- **Nightmares.** Every fifth dream is a walled arena with a hunter in it. It stalks, lunges, then has to rest (it shrinks while it rests). Gather three sigils to open the portal, and you get a pick of rare and mythic cards.
- **Run summary.** On waking, see your depth, time, nightmares beaten, times caught, twists survived, upgrades, combos and how your dust multiplier was worked out.
- **Short or long runs.** A short run needs 3 shards. A long run needs 6, and every dream is exponentially harder than the last (faster, more alert enemies, bigger dreams), with rewards scaled to match. At any wake door you can also refuse to wake and GO DEEPER: 3 more shards, the climb starts (or steepens), and the payout grows.
- **Dream twists.** Deeper dreams can come WEIGHTLESS, FLOODED, BLACKOUT, UPSIDE DOWN, SWARMING, STUTTERING, HURRIED (beat the timer for dust) or GILDED (a sure shard and double dust). Long runs stack two.
- **Dreamlike scenery.** Floating rock islands under every platform, drifting clouds, hoops and crystals, glowing motes, a moon at the far end, and new props: doorways to nowhere, stairs to nothing, clouds, moons, ring gates, bookshelves, mushrooms.
- **Endless descent.** Cyan portals go deeper. Collect your lucidity shards, then take the white wake door.
- **Acid-trip textures and shaders.** Generated patterns (plasma, swirl, eyes, kaleidoscope…) melt, breathe and shift hue more as the dream gets stranger.
- **Dreams melt into each other.** Each level slumps into a fog puddle and the next one rises out of it.
- **Tunnel vision.** The world only exists near you; the rest sinks into darkness under light film grain. The dreamer is always drawn self-lit, outlined, and as a silhouette through anything in front of it.
- **Enemies you can time.** Each one paces between a spot on your path and a side pocket, so the way is open half the time. A test walks every route to prove no dream can wall you in.
- **Dream cards.** On waking, dreams flip over one by one. Some are remembered and become cards (rarity, attribute, DREAD and DRIFT stats); the rest fade into Dream Dust.
- **The Lucid Store.** A pixel-art shop run by a sleepy moth. It sells multi-run perks (First Light, Slow Heart, Deep Memory, Heavy Eyelids, Second Wind, Dust Magnet, Long Stride) and cosmetics (eyes, crystals, card frames, HUD ink).
- **Names from a 1,000+ word grammar.** For example, *THE PIER THAT ISN'T THERE — your footprint is falling with you.*

## Dev switches

| Variable | Effect |
|---|---|
| `DREAMSCAPE_SEED=n` | replay a run exactly |
| `DREAMSCAPE_AUTOPILOT=1` | walks the generated route (end-to-end test) |
| `DREAMSCAPE_BOOKLET=path` | use a different save file |
| `DREAMSCAPE_MELT_SCALE=6` | slow-motion dream transitions |
| `DREAMSCAPE_NO_TUNNEL=1` | start with tunnel vision off |
| `DREAMSCAPE_RUN=long` | default the title screen to a long run |
| `DREAMSCAPE_THEME=SkyStairs` | start one dream deep in a given dream type |
| `DREAMSCAPE_NIGHTMARE=1` | start in a nightmare arena |
| `DREAMSCAPE_SCREEN=codex` | open the codex (or `settings`) on start |
| `DREAMSCAPE_CONTINUE=1` | continue the saved run on start (`DREAMSCAPE_SAVE` picks the file) |
| `DREAMSCAPE_SETTINGS=path` / `DREAMSCAPE_CARDS=dir` | a different settings file / card export folder |
| `DREAMSCAPE_SPECIAL=Stalker` | put that special enemy (Stalker, Mimic, Sentry, Drifter, Jester) in every dream |
| `DREAMSCAPE_SHOT=out.png` | save a screenshot after `DREAMSCAPE_SHOT_AT` seconds (default 6), then quit |

`tools/screenshots.sh OUT_DIR` shoots every dream type, a nightmare and each special enemy under Xvfb and fails on a blank frame. CI runs it on Linux and uploads the images.

## Credits

- Font: [VT323](https://fonts.google.com/specimen/VT323) by Peter Hull, under the SIL Open Font License (`assets/fonts/OFL.txt`).
- Engine: Jame (this repository's `engine/` crate).
