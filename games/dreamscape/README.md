# Dreamscape

A PS2-style dream-escape game built on the Jame engine (Rust). You fall asleep and sink through an endless run of procedurally generated dreams. Collect three lucidity shards to wake up, then keep the dreams you remember as trading cards.

## Run it

```bash
cargo run -p dreamscape          # from the repo root
cargo test -p dreamscape         # 156 tests
```

## Controls

| Key | Action |
|---|---|
| WASD | move |
| Space | jump |
| Esc | pause (Q to quit from pause) |
| L / B | Lucid Store / dream booklet (title screen, pause, after waking) |
| F1 | overhead camera |
| F2 | toggle tunnel vision + film grain |

## What's in it

- **Fully procedural dreams.** There are no hand-built levels. Six dream types (lobby, liminal office, void platforms, garden, nightmare factory, awakening) each have their own layout rules, props, palette and music. Levels grow with depth.
- **Endless descent.** Cyan portals go deeper. Collect 3 lucidity shards, then take the white wake door.
- **Acid-trip textures and shaders.** Generated patterns (plasma, swirl, eyes, kaleidoscope…) melt, breathe and shift hue more as the dream gets stranger.
- **Dreams melt into each other.** Each level slumps into a fog puddle and the next one rises out of it.
- **Tunnel vision.** The world only exists near you; the rest sinks into dark film grain.
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

## Credits

- Font: [VT323](https://fonts.google.com/specimen/VT323) by Peter Hull, under the SIL Open Font License (`assets/fonts/OFL.txt`).
- Engine: Jame (this repository's `engine/` crate).
