# Dreamscape roadmap

The run is a roguelike: an upgrade after every dream, short and long runs, going
deeper at the wake door, 20 dream types, twists on individual dreams, combat, and
a relic weapon you carry between runs. This file records what is built and what is
actually left, in phases that can each ship on their own. Each item names the code
it touches.

**Last checked against source: 2026-11-14, `main` at `1e03b41`.** Statuses below
come from reading `src/`, not from earlier versions of this file. When a stream's
own notes disagree with the code, the code wins.

---

## Phase 1: Run structure and feedback ✅ done

| # | Feature | Where it lives now |
|---|---|---|
| 1.1 | **Boss dreams** every 5 depths | `hunter.rs`, `boss.rs`, sigil objective (`objective.rs`). Telegraphed attacks (0.8 s) from tier 2 up. |
| 1.2 | **Run summary** | `summary_ui.rs`, `RunStats` in `main.rs`. Upgrades, peak difficulty, twists, catches, dust breakdown. |
| 1.3 | **Cursed upgrades** | `Upgrade::Cursed(boon, bane)` in `upgrades.rs`, red frame. Roughly 1 in 5 cards. |
| 1.4 | **Reroll and skip** | Rerolls (one per run, more from upgrades), skip pays 5 dust, R/X on the choice screen. |
| 1.5 | **Synergies** | `ALL_SYNERGIES` in `upgrades.rs`: **19** total. The first six were the pairs listed here; the rest came with the items, abilities and combat waves. Links drawn in `hand.rs`. |
| 1.6 | **Rarity tiers** | `cards.rs` `Rarity` (Common/Rare/Mythic by dream score, odds shift with depth) **plus** `Fused` and `Resonant` from Stanza 2 (`fusion.rs`). |

## Phase 2: Enemies and threats ✅ done

| # | Enemy / hazard | Notes |
|---|---|---|
| 2.1 | **Stalker** | `specials.rs`; moves only while unwatched or behind the camera. |
| 2.2 | **Mimic** | Replays your path 3 s behind (`specials.rs`, `Trail`). |
| 2.3 | **Sentry** | Rotating cone; being seen alerts enemies within 2 cells. |
| 2.4 | **Drifter** | Blocks jumps over void/platform dreams. |
| 2.5 | **Jester** | Teleports you 2 cells in a safe direction; harmless. |
| 2.6 | **Elite modifiers** | Fast / Big / Splitter / Shade (`specials::Elite`), with `tactics.rs` shouts and packs. |
| 2.7 | **Hazards** | Grew beyond the original list: one rule per theme (`theme_rules.rs`, 8 kinds, depth 3+): rising tide, sticky floor, gust wind, dimming light, silence, drifting fog, beat floor, shrinking floor. Crumbling floor lives in `dream/build.rs`. All 8 are wired (`// === hazards ===` blocks). |

Every new enemy keeps the standing guarantee: the patient-walker tests prove the
route is never permanently blocked.

## Phase 3: Dream types and twists (3.1, 3.2, 3.3, 3.5, 3.6 ✅; 3.4 partly)

### 3.1 Patterns (`dream/texture.rs`) ✅
14 patterns, including all six form-constant ones: Lattice, Cobweb, Tunnel,
Veins, Mandala, plus Eyes, Kaleido and Faces.

### 3.2 Shader effects ✅
`mesh.vert`/`mesh.frag`: `uBreathe`, `uMelt` (dream transition), `uPulse`
(beat), emissive veins (glow), `uDissolve` (death and White Dissolve). Trails
are a per-theme `Mood.trails` amount handed to the renderer's trail pass
(`Renderer::set_trails`). Vignette and film grain moved into
`post_composite.frag` during the perf pass.

### 3.3 The dreams ✅
All 10 are in, with `ALL_THEMES` at **20** dreams (`dream/theme.rs`): Mycelium
Grove, Fractal Cathedral, The Tunnel, Elfworks, Afterimage Fields, Synesthesia
Hall, Melting Clockworks, Jellyfish Sky, Watching Wallpaper, White Dissolve.

### 3.4 Twists — 6 of 6 built
`Variant` in `dream/variant.rs` holds **14** twists. Of the six this roadmap
asked for:

- **Mirrored** ✅ (actions swap, not keys)
- **Tiny** ✅ (`TINY_ZOOM` 2.0)
- **Echo** ✅ (5 s ghost banks shards, enemies chase it)
- **Night Shift** ✅ (shown as FLICKERING; reduce-flashing softens the cut)
- **Kaleidoscope** ✅ (`kaleido.rs`)
- **Breathing** ✅ (`breathing.rs`)

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

### 3.4 New twists ✅ (all six; Kaleidoscope and Breathing are the last two)
- **Mirrored**: left and right controls are swapped (the one twist that does *not* follow the screen).
- **Tiny**: the camera pulls back 2×, so the dreamer is small in a huge dream.
- **Echo**: a ghost of you from 5s ago also collects shards. Enemies chase it too.
- **Night Shift**: the lights turn off completely every 8s for 2s, and only Lit things show.
- **Kaleidoscope**: a post-effect with 4-way mirror symmetry, while gameplay stays normal.
- **Breathing**: walls physically shift by up to half a cell. The route stays valid because only non-route walls move.

### 3.5 Between-dream events ✅
`events.rs`: **19** events. `events::due` gates them: one in six (`ODDS`), not
before depth 2, never on a boss reward, and never on the dream straight after
another event. Text approved in `plans/APPROVED-TEXT.md`; outcomes reuse
existing currencies only.

### 3.6 Secret rooms ✅
`dream/secret.rs` places a cracked wall off-route; `secrets.rs` decides what
breaks it (Blink or Dash only), the notice hint, and the door. Behind it is the
moth's room (`moth_shop.rs`) selling run upgrades.

Deviation from the original sketch: the room spends **dust**, not shards, and
does not merge (merging stays in the store's `[m]` screen).

## Phase 4: Meta-progression and replay value ✅ done

| # | Feature | Notes |
|---|---|---|
| 4.1 | **Daily dream** | Date seed, own best depth, daily fortune hook. |
| 4.2 | **Booklet unlocks** | Became `attunement.rs`: 5 cards of one attribute → that attribute's passive for every run. |
| 4.3 | **Ascension levels** | `progress.rs`, now **0–16** (was 1–10). Each level has a named rule; the title picker draws all 17 states. |
| 4.4 | **Dreamers** | `form.rs`, **6** forms: Crystal, Moth, Ember, Pearl, Lantern, Hare. |
| 4.5 | **Achievements and records** | `achievements.rs` **45** achievements, with a Steam bridge behind the `steam` feature. Records are tracked in the codex and announced by `record_wake`; breaking one pays nothing yet. |
| 4.6 | **Dream codex** | `codex_ui.rs`: dreams with best depth, and enemies with a one-line description. |

## Phase 5: Feel and polish ✅ done

5.1 ability FX, 5.2 sound (**95** `Sound` variants, adaptive layers in
`music.rs`), 5.3 silhouettes, 5.4 shake and hit-stop (`juice.rs`, honours
reduced motion), 5.5 adaptive music, 5.6 accessibility (grain, vignette,
colour-blind markers, remapping, reduced motion, reduce flashing, UI juice,
gentle nerve), 5.7 title screen with dreamer / length / ascension / daily / seed
code / co-op.

## Phase 6: Infrastructure ✅ done

6.1 Windows + Linux CI (`.github/workflows/ci.yml`: clippy, engine + dreamscape
tests, Steam check, package), 6.2 headless screenshots (`tools/screenshots.sh`
in CI), 6.3 mid-run save (title **Continue**, whole deck state saved), 6.4
balance log (`balance.rs`, `DREAMSCAPE_BALANCE`), 6.5 theme harness (every
theme × layout × twist in `dream/build.rs` tests).

---

## Built after this roadmap was first written

These were not in the phases above. They are done and merged; the plan file that
owns each one is named so a planner reads the right document.

### Combat — `plans/2026-10-23-combat-system-plan.md` (C1–C7 shipped)
Nerve hearts replace instant-catch (`nerve.rs`; 3 start, 5 max, 1.2 s i-frames,
`gentle_nerve` setting). STRIKE on **F** / pad B (`combat.rs`), enemy hp,
stagger, unravel on zero, calm motes, drops. Nightmares have hp too: a strike
on a resting hunter is `boss::Blow`, and the HUD shows `boss_hp` + vulnerability.
All 16 original abilities have combat effects (`abilities2.rs`, `abilities3.rs`
add 16 more, **32 total**). Six new enemies: Tooth, Weeper, Lullaby, Moths, Mask,
Leech (`tooth.rs`, `weeper.rs`, `lullaby.rs`, `moths.rs`, `mask.rs`, `leech.rs`,
spawning via `newfoes.rs` / `foespawn.rs`). C8 (balance sweep) and C9 (polish)
are **partial** — see "What's next".

### Relics — `plans/PLAN-relics.md` + `plans/relic-briefs/` (R1–R6 shipped)
Ash currency and pouch in the booklet (`relic_types.rs`, `relics.rs` with pity,
featured theme and foil), wear/etch/foil art (`relic_art.rs`), buffs through
`card_effects` and a max-nerve hook (`relic_effects.rs`), first-person held
weapon with sway and strike mods (`relic_weapon.rs`), and the case/pouch UI plus
the store's fourth tab (`relic_ui.rs`). Names from a seeded grammar
(`relic_names.rs`). R7/R8 are **not done**.

### UI revamp — `plans/PLAN-ui-revamp.md` (M0–M4e, N, R3–R6 shipped)
`ui_motion.rs` (easing, springs, tick-ups, `UiJuice` Off/Soft/Full),
`card_warp.rs` (every card's art wobbles from its own dream theme; frame and
text stay flat), `death_fx.rs` (dreamer dissolves, first-person hands dissolve),
`prompts.rs` (context chips, Required state for blocking choices), and a motion
pass over every screen (`screen_anim.rs`). R7/R8 are **not done**.

### Scraps — `plans/PLAN-scraps.md` (S1–S4 shipped)
`scraps.rs`: six one-shot cards (Mend, Lull, Lantern, Windfall, Safe Step,
Redraw) from packs, carried in `Booklet.satchel`, shuffled into the draw pile and
exhausted on use.

### Pick-a-pack — `plans/PLAN-pick-a-pack.md` (shipped)
Wake → pick a pack → open it (`packs.rs`, `reveal_ui.rs` `Mode::PackPick`). Packs
are built from the run's dream records, so a card's rarity is the dream's:
Fused/Resonant only appear when the source dream was fused or blended.

### Hard mode — `plans/hard-briefs/` (all six streams shipped)
`pressure.rs` (the dream collapses), `tactics.rs` (enemies cooperate),
`shard_rules.rs` (guarded/heavy shards), `theme_rules.rs` (per-theme hazards),
`pacts.rs` (8 pacts, +dust for a cost), ascension rules to 16.

### Deck, tools, sorting, hand scoring — `plans/deck-briefs/`, `deckgaps-briefs/`
A 3-card hand drawn from a 3–12 card deck (`deck.rs`, `deck_ui.rs`), passives and
edges follow the whole deck (`RunUpgrades::deck_passives`), tool cards wild for
attribute only, sorting on booklet/merge/loadout (`sorting.rs`), poker-style hand
scoring that multiplies earned dust (`handscore.rs`), and the deck survives save
and resume (`SavedRun.deck`).

### Retention and gambling — `plans/gamble-briefs/` (owner-approved guardrails)
Daily fortune (`fortune.rs`), tickets and scratch tab (`tickets.rs`), the moth
coin and wager portal (`gamble.rs`), weekly challenge and forgiving streaks
(`streaks.rs`), Lucid Store (`store.rs`, `shop_ui.rs`), moth merging
(`fusion.rs`, `merge_ui.rs`). Dust and cards only, odds shown, stakes opt-in and
capped, nothing punished for missing a day.

### Performance — `plans/perf-briefs/BASELINE.md`
Normal play 6.6 → 4.3 ms, merge 11.7 → 6.8 ms, dream generation 18.5 → 3.4 ms,
suite 8–9 min → ~46 s. Guards: `every_screen_stays_inside_its_ui_budget`,
`dream_generation_stays_inside_its_time_budget`, a golden dream hash, and a
prewarmed shader-variant cache.

### Co-op — Stanza 3 / M8 (shipped)
`coop.rs` protocol, `coop_game.rs` lobby and session tick, up to five dreamers
over TCP, host-authoritative dreams, ghosts and revive, no story in co-op.
Combat is off in co-op.

### Steam — M7 (partial)
Achievements bridge behind the `steam` feature and `steam_appid.txt` placeholder.
Lobbies wait on a real App ID.

---

## What's next (verified against source)

Ordered by "finish what is half-open" first, then features.

### 1. UI revamp R7 + R8 — the active plan's last two milestones
`PLAN-ui-revamp.md` decision 8 ships everything before the owner plays, and
R7/R8 are explicitly the parent session's. No artifact of either exists yet.

- R7: polish pass over sounds, `screen_smoke`, the settings audit, and the text-
  fit audit at both resolutions.
- R8: full suite + clippy, screenshots of **every** screen and every relic
  rarity, save compatibility against a pre-revamp `settings.ron` and booklet, and
  autopilot seeds 1–5.
- Unresolved design risks from that plan: the first-person death substitution
  (hands/weapon dissolve + ember fade) has never been seen by the owner, and
  warped-card readability at Full juice is only checked by the budget test.

### 2. Combat C8 + C9 leftovers
Verified gaps, not guesses:

- **No combat balance sweep.** `balance.rs` has no combat path and there is no
  `#[ignore]` autopilot-combat test (the plan's target: a fresh save reaches
  depth 5–8, and no single card trivialises depth 10).
- **The codex never lists the six new enemies.** `main.rs:4305` builds codex
  enemy entries from `dream::ALL_ENEMY_KINDS`, which by design excludes
  Moths/Mask/Leech and never contained Tooth/Weeper/Lullaby.
- **No strike tutorial.** `tutorial.rs` says nothing about STRIKE, so the first
  combat dream teaches nothing.
- **No "Combat hints" option.** `settings.rs` has `gentle_nerve` but not the
  hints toggle from plan §9.
- **No elite variants or `tactics.rs` shouts/packs** for the six new enemies;
  both enemy NOTES list this as undone.
- **No healing item in the store** (plan C9).
- Both enemy streams report no hand playtest of telegraphs, tears, the slam ring
  or latch feel.

### 4. Small meta-progression gaps (triage F1–F3)
- **F1:** breaking a record pays nothing today (`records.rs` is best depth only;
  `record_wake` just shows a banner).
- **F2:** no two-attunement combos; `attunement.rs` rewards one passive per
  attribute.
- **F3:** the daily has no shareable result string (seed codes in `seedcode.rs`
  cover runs, not daily results).

### 5. Housekeeping
- `plans/PLAN-relics.md` and `plans/PLAN-ui-revamp.md` still describe their
  systems as unbuilt.
- `plans/deckgaps-briefs/` has no `NOTES-decklife.md` even though the stream is
  in (`SavedRun.deck`, `RunUpgrades::deck_passives`); its decisions live only in
  the brief and the code.
- The repo-root `TASKS_3-6_GUIDE.md` describes editor tasks that are done
  (`engine/src/ui/dock_skeleton.rs`, `engine/tests/test_task4.rs`).

### 6. Bigger bets (need an owner decision)
- **Combat in co-op** (C11): off today — `coop_active` gates the strike, the
  nerve HUD reports 0/0, and the new enemies never spawn.
- **Steam lobbies** once an App ID exists.

## Known soft spots

Facts from the stream notes that still hold; useful before a polish or balance
pass.

- Hazards: enemies standing on a shrunk cell are not handled; White Dissolve's
  shrink rule is mostly cosmetic; no hand-tuned feel.
- Combat: tears are not consumed on contact (the 1.2 s i-frames cover repeat
  hits); Weeper and Lullaby phase through walls like pacers; only the mask can
  lose its shield.
- Relics: a pull requires the dearest price (120 Ash) in hand and then charges
  the rolled rarity's price; trail/clash have no on-screen labels; equipping
  mid-run changes the weapon at once but the buffs start next run.
- Deck: `loadout_ui::draw/toggle/slot_of` are kept for the title teaser and the
  loadout fallback, not used by the builder.
- Perf: not tested on weaker hardware (Steam Deck, older GPUs); the composite
  pass is now the biggest single cost (~2.1 ms).
- Death dissolve: a still screenshot cannot verify an animation, so the timings
  are carried by unit tests.

## Suggested order

1. **R7/R8 UI revamp** — closes the active plan and unblocks an owner playtest.
2. **Combat C8/C9 leftovers** — codex entries, strike tutorial, hints option,
   elite/tactics integration, then the balance sweep.
3. ~~Kaleidoscope and Breathing~~ — done.
4. **F1–F3** small meta gaps, plus the housekeeping in item 5.
5. **Co-op combat** and **Steam lobbies** once decided.

## Planning documents

| Plan | State |
|---|---|
| `plans/PLAN-ui-revamp.md` + `ui-revamp-briefs/` | merged except R7/R8 |
| `plans/2026-10-23-combat-system-plan.md` + `combat-briefs/` | C1–C7 merged; C8/C9 partial |
| `plans/PLAN-relics.md` + `relic-briefs/` | R1–R6 merged; R7/R8 open |
| `plans/PLAN-scraps.md` + `scrap-briefs/` | done |
| `plans/PLAN-pick-a-pack.md` + `pick-a-pack-briefs/` | done |
| `plans/hard-briefs/` | all six streams done |
| `plans/deck-briefs/`, `plans/deckgaps-briefs/` | done (DECKLIFE notes missing) |
| `plans/gamble-briefs/` | done |
| `plans/perf-briefs/` | done; BASELINE.md is the reference |
| `plans/2026-10-21-next-features-triage.md` | B4/B5 and F1–F3 still open |
| `plans/2026-09-28-dreamscape-rest-of-project-plan.md` | M0–M8 shipped; M9 bug round is R8 above |
| `plans/APPROVED-TEXT.md` | the approval ledger; any new text goes here first |

## References (dream-type research)

- [Visions and Imagery During Psilocybin Experiences (OmTerra)](https://www.omterra.org/articles-resources/visions-and-imagery-during-psilocybin-experiences-a-comprehensive-guide)
- [Understanding Mushroom Trip Visuals and Effects (The Healing Dose)](https://healingdose.com/blog/psychedelic-science/understanding-mushroom-trip-visuals-and-effects/)
- [Quantitative analysis of psychedelic visual experience reports (PMC)](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC11663017/)
- [An Encounter With the Other: DMT experience content analysis (Frontiers)](https://www.frontiersin.org/articles/10.3389/fpsyg.2021.720717/full)
- [Survey of entity encounter experiences (Davis et al., 2020)](https://journals.sagepub.com/doi/full/10.1177/0269881120916143)
- [DMT Hyperspace and its Liminal Aesthetics (St John)](https://chacruna.net/dmt-liminality-and-hyperspace/)
