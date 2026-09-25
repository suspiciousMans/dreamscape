# Dreamscape roadmap

The run is now a roguelike: an upgrade after every dream, short and long runs, going deeper at the wake door, 10 dream types, and twists on individual dreams. This is what comes next, in phases that can each ship on their own. Each item names the code it touches.

---

## Phase 1: Run structure and feedback

| # | Feature | Notes / code |
|---|---|---|
| 1.1 | **Boss dreams** every 5 depths | A small arena with one large hunter that tires after a lunge and then recovers. You grab 3 shards while it hunts you, and it drops a rare upgrade. Needs a new `DreamTheme::Nightmare` (or a boss twist), `LayoutKind::Arena`, and a `BossAI` next to `enemy_ai.rs`. |
| 1.2 | **Run summary screen** | Shown before the pack. It lists upgrades taken, peak difficulty, twists survived, times caught, and how the dust was worked out (base × hoarder × nightmare + bonuses). New `summary_ui.rs`, fed from `RunUpgrades` plus a `RunStats` struct. |
| 1.3 | **Cursed upgrades** | About 1 in 5 cards has a drawback, e.g. "+30% speed, enemies notice you from further away." Add `Upgrade::Cursed(Boon, Bane)` in `upgrades.rs` and a red card frame. |
| 1.4 | **Reroll and skip** | One reroll per run (more from an upgrade). Skipping a pick gives 5 dust. Keys R and X on the choice screen. |
| 1.5 | **Synergies** | Dash + Phase lets you dash through walls. Wide Eyes + Shard Sense makes the beacon visible anywhere. Stillness + Heavy Air makes the freeze last longer. Blink + Double Jump lets you blink in mid-air. Owning both halves of a pair shows the combo name on the HUD. |
| 1.6 | **Rarity tiers on cards** | Common, rare and mythic, with the frame colour and odds shifting as depth increases. |

## Phase 2: Enemies and threats

| # | Enemy / hazard | Behaviour |
|---|---|---|
| 2.1 | **Stalker** | Only moves while it's outside the tunnel-vision radius or behind the camera, like the classic statue monster. |
| 2.2 | **Mimic** | Replays your exact path 3 seconds behind you. Stopping gets you caught, so it forces you to keep moving. |
| 2.3 | **Sentry** | A rotating cone of light. Being seen by it alerts every enemy within 2 cells. |
| 2.4 | **Drifter** | Floats over the void in platform and spiral dreams, where it blocks jumps. |
| 2.5 | **Jester** (from the "machine elf" reports) | Doesn't catch you. It teleports you 2 cells in a random safe direction, and it's harmless but disorienting. |
| 2.6 | **Elite modifiers** in long runs | Fast, big (larger touch radius), splits in two when it lunges, or invisible when far away. |
| 2.7 | **Hazards** | Crumbling tiles (they fall 1s after you step on them and respawn later), closing platforms, slow fog pockets, and floor tiles that shift to a beat (see 3.6). |

Every new enemy must keep the existing guarantee: the patient-walker test proves the route is never permanently blocked.

---

## Phase 3: New dream types

Each new dream is built around phenomena that people commonly describe in psilocybin, LSD and DMT trip reports:
- breathing, flowing surfaces;
- fractal and geometric patterns;
- the "form constants" (tunnels, spirals, lattices, cobwebs);
- tracers;
- synesthesia;
- intense colour;
- encounters with entities;
- time distortion;
- ego dissolution;
- a sense of connection to nature.

These stay abstract and dreamlike: no drug use is shown, only the look and feel.

### 3.1 New texture patterns (`dream/texture.rs`)
- **Lattice**: a honeycomb or checker lattice that slowly rotates (form constant).
- **Cobweb**: radial threads with rings (form constant).
- **Tunnel**: concentric rings that recede toward the centre.
- **Veins**: branching, glowing lines (mycelium).
- **Mandala**: rotational symmetry of order 6 to 12.
- **Faces**: pareidolia, where blobs and features arrange into faint faces.

### 3.2 New shader effects (`mesh.vert` / `mesh.frag` / post)
- **`uBreathe`**: per-theme amplitude for surface breathing (today it's tied only to strangeness).
- **Tracers**: blend the previous frame into the post pass (a feedback texture in `renderer/post.rs`), with strength set per theme.
- **`uPulse`**: brightness and hue pulses synced to the music's beat, for synesthesia.
- **Emissive veins**: a glow mask for the Veins pattern, so fungal dreams light the dark on their own.
- **Dissolve**: surfaces dither away into noise where nobody is looking, for the ego-dissolution dream.

### 3.3 The dreams

| Dream | Inspired by | Layout | Look | Mechanic |
|---|---|---|---|---|
| **Mycelium Grove** | Connection to nature; the living, breathing forest | New `Network` layout: clearings joined by root bridges | Dark ground, glowing Veins pattern, giant mushroom and toadstool props, spores drifting upward (Sky motes) | Glowing threads lead toward the shard. Standing on veins speeds you up. |
| **Fractal Cathedral** | Fractals, mandalas, self-similarity | New `Recursive` layout: a room that holds a smaller copy of itself, which holds a smaller copy again | Mandala and Lattice patterns, stained-glass colour, very high symmetry | The portal sits in the smallest room. Scenery scales as you go in. |
| **The Tunnel** | The form-constant tunnel and the light at the end | New `Corridor` layout: one long winding tube with side alcoves | Tunnel pattern on the walls, hoops (Torus) every few cells, bright white light at the portal end | Hoops close and open in sequence. Time your run through them. |
| **Elfworks** (the "hyperspace nursery") | Machine-elf and jester reports; jewelled, self-transforming machines | OpenHall full of machine props | Saturated jewel palette, Kaleido pattern, props that spin and change shape (swapping meshes) | Jester enemies (2.5). Floor tiles rearrange every few seconds, with the route kept valid. |
| **Afterimage Fields** | Tracers and trails | ScatterField | Pale pastel colours; everything leaves colour trails (tracer effect at maximum) | Mimic enemies (2.2). Your own trail shows where you've already been. |
| **Synesthesia Hall** | Seeing sound and hearing colour | OpenHall with a stage | Floor tiles light up with the music, and a spectrum shows on the walls | Hazard tiles pulse on the beat (2.7). Crossing on the off-beat is safe. |
| **Melting Clockworks** | Time distortion; loops; Dalí clocks | Maze | Draped clock props (a new mesh), amber and brass, the Rings pattern | Every 20s the dream resets to its starting state. Only what you've collected stays. The HUD shows a loop counter. |
| **Jellyfish Sky** | Floating, drifting, the "ocean" of visions | PlatformChain | Bioluminescent jellyfish entities (a new mesh: dome plus tendrils) drifting past, deep blue gradient | Low gravity always on. Drifters (2.4) are jellyfish. |
| **The Watching Wallpaper** | Pareidolia: faces in the patterns | Maze | Faces pattern on every surface; faces turn to follow the player (shader billboard offset) | Stalker enemies (2.1). The faces "look" toward the shard. |
| **White Dissolve** | Ego dissolution, which usually comes before waking | Spiral inward | Almost pure white, dissolve shader; the dreamer's outline fades | The floor dissolves behind you while you move and rebuilds when you stop. A rare dream that is only reached through GO DEEPER. |

Each dream needs a `ThemeSpec`, vocabulary (at least 18 of each word list, nouns unique across dreams), an `Attribute` for its cards, a profile `.ron`, weighted exits, and possibly a new `PropKind` or `Shape`. The existing tests enforce all of this.

### 3.4 New twists
- **Mirrored**: left and right controls are swapped (the one twist that does *not* follow the screen).
- **Tiny**: the camera pulls back 2×, so the dreamer is small in a huge dream.
- **Echo**: a ghost of you from 5s ago also collects shards. Enemies chase it too.
- **Night Shift**: the lights turn off completely every 8s for 2s, and only Lit things show.
- **Kaleidoscope**: a post-effect with 4-way mirror symmetry, while gameplay stays normal.
- **Breathing**: walls physically shift by up to half a cell. The route stays valid because only non-route walls move.

### 3.5 Between-dream events
Short text choices shown between dreams, instead of an upgrade, about 1 time in 6. For example: "A door hums. Open it?" leads to an upgrade or a curse. "A voice offers to carry a shard" gives +1 shard and the next dream is gilded, but costs one upgrade. They reuse the choice screen.

### 3.6 Secret rooms
A cracked wall hides a door, and the Blink ability or a DASH through reaches it. It leads to a small **shop dream** run by the moth, where you spend shards on upgrades.

---

## Phase 4: Meta-progression and replay value

| # | Feature | Notes |
|---|---|---|
| 4.1 | **Daily dream** | A seed made from the date. It keeps its own best depth and shows on the title screen. |
| 4.2 | **Booklet unlocks** | Collecting 5 cards of one attribute unlocks an upgrade or a starting ability. They show in the booklet as "resonance". |
| 4.3 | **Ascension levels 1–10** | Beating a long run unlocks the next level. Each adds a modifier: +1 shard needed, faster enemies, fewer choices per pick, and so on. |
| 4.4 | **Dreamers (starting characters)** | Crystal (default); Moth (starts with Blink, 1 fewer upgrade choice); Ember (Dash, but hotter enemies); Pearl (sees further, is slower). Selected on the title screen. |
| 4.5 | **Achievements and records** | Fastest wake, deepest long run, wake without taking any upgrades, survive every twist. Saved in the booklet. |
| 4.6 | **Dream codex** | A booklet page per dream type, filled in as you visit, with its vocabulary, attribute and best depth. |

## Phase 5: Feel and polish

| # | Feature | Notes |
|---|---|---|
| 5.1 | **Ability effects** | Afterimages when you dash, a burst when you blink, frozen enemies turning blue under Stillness, a see-through player during Phase. |
| 5.2 | **Sound for abilities, twists and enemies** | Replace the sine beeps with short synthesized sounds, plus water ambience for Flooded, wind for Sky Stairs, and an echo for Mirror Hall. |
| 5.3 | **Distant scenery** | The moon, clouds and islands show through the tunnel-vision darkness as dim silhouettes, via a `Silhouette` marker in `mesh.frag`. |
| 5.4 | **Screen shake and hit-stop** | When you're caught, with a bigger effect when a shard drops. |
| 5.5 | **Adaptive music** | Layers of intensity driven by difficulty and how close enemies are. |
| 5.6 | **Accessibility** | Sliders for grain and vignette, colour-blind shard and portal markers, key remapping, reduced motion (which caps the breathing, melting and tracer effects). |
| 5.7 | **Title screen for runs** | Pick the dreamer, run length, ascension level and daily seed on one panel. |

## Phase 6: Infrastructure

| # | Feature | Notes |
|---|---|---|
| 6.1 | **Linux CI with tests** | Link `advapi32` only on Windows (`engine/build.rs`: check `CARGO_CFG_TARGET_OS`). Add an Ubuntu job that runs `cargo test` and `cargo clippy`. |
| 6.2 | **Headless screenshot tests** | Under Xvfb, run `DREAMSCAPE_THEME=<each>` with autopilot, capture frames, and compare them roughly against saved references. |
| 6.3 | **Save the run mid-way** | Quitting saves the director, upgrades and depth to the booklet file. The title screen offers Continue. |
| 6.4 | **Balance log** | `DREAMSCAPE_BALANCE=path.csv` writes depth, difficulty, time, catches and upgrades per dream. |
| 6.5 | **Theme test harness** | Extend the existing generation tests to every new layout and twist combination: traversable, never blocked, shard reachable. |

---

## Suggested order

1. **Phase 6.1 and 6.2** first, so everything after it gets CI and screenshots.
2. **Phase 1** (1.1, 1.2, 1.5): the biggest change to how a run feels.
3. **Phase 3.1 and 3.2** (patterns and shader effects), then the dreams in pairs: Mycelium Grove + The Tunnel, then Fractal Cathedral + Elfworks, then Afterimage Fields + Synesthesia Hall, then Melting Clockworks + Jellyfish Sky, then Watching Wallpaper + White Dissolve.
4. **Phase 2** enemies alongside the dreams that use them.
5. **Phase 4**, then **Phase 5** polish throughout.

## References (dream-type research)
- [Visions and Imagery During Psilocybin Experiences (OmTerra)](https://www.omterra.org/articles-resources/visions-and-imagery-during-psilocybin-experiences-a-comprehensive-guide)
- [Understanding Mushroom Trip Visuals and Effects (The Healing Dose)](https://healingdose.com/blog/psychedelic-science/understanding-mushroom-trip-visuals-and-effects/)
- [Quantitative analysis of psychedelic visual experience reports (PMC)](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC11663017/)
- [An Encounter With the Other: DMT experience content analysis (Frontiers)](https://www.frontiersin.org/articles/10.3389/fpsyg.2021.720717/full)
- [Survey of entity encounter experiences (Davis et al., 2020)](https://journals.sagepub.com/doi/full/10.1177/0269881120916143)
- [DMT Hyperspace and its Liminal Aesthetics (St John)](https://chacruna.net/dmt-liminality-and-hyperspace/)
