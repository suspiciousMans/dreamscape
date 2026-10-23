# Dreamscape: combat system plan

Status: draft for sign-off. Nothing here is built. Section 10 lists the decisions I need from you before code starts.

## 1. What this adds, and what it must not break

Today you can't fight. An enemy that touches you "catches" you: you drop this dream's shard, respawn, and the run goes on. Abilities (Dash, Blink, Stillness...) exist to *avoid* that. Cards give one active and one passive each (`powers.rs`).

This plan adds a second way to play the same dreams: **fight back**. It adds a health stat, a basic attack, combat uses for all 16 card abilities, combat hooks for the existing passives plus a few new ones, six new enemies, and health bars on enemies and nightmares.

It must not break:
- **Classic play stays.** A setting (`Dream rules: CLASSIC / COMBAT`) keeps the current "touch = caught" rules untouched. Autopilot, daily runs, seed codes and existing saves replay as before in CLASSIC.
- **Routes stay valid.** Patient-walker and route tests must stay green: enemies never wall the route in.
- **Dread over gore.** Nothing dies. Enemies *unravel*: they dissolve using `dissolve.rs`. No new story text without your approval.
- **Hard mode keeps meaning.** Pressure, tactics, pacts, hazards and ascension keep working and get combat hooks (milestone C8).

## 2. What exists today (verified in source)

| Area | Where | Fact that matters |
|---|---|---|
| Catch / respawn | `main.rs` `// 4. Fail states`, `threat_touching()` | One touch = caught. Dash and Phase make you untouchable. Ward absorbs one catch. `grace` timer gives post-respawn safety. |
| Pacers | `enemy_ai.rs`, `Pacer` in `main.rs` | Patrol, then lunge for 1.2s. Four elites: Fast, Big, Splitter, Shade (`specials.rs`). |
| Specials | `specials.rs`, `SpecialAI` | Stalker, Mimic, Sentry, Drifter, Jester. |
| Bosses | `hunter.rs`, `boss.rs` | Nightmares: collect sigils. Tier 2+ add Shockwave, Summon (wisps), Eclipse, all telegraphed 0.8s. |
| Enemy stun | `main.rs` `stunned`, `rattle_enemies()` | SLAM and GUST already knock back and stun. This is the seed of staggering. |
| Abilities | `upgrades.rs` | 16 abilities with cooldown and duration tables. Passives in `Upgrade` (12 + 2 mythic). |
| Cards to powers | `powers.rs` | 20 attributes map to one active and one passive. Approved table. Do not change it. |
| Hand | `hand.rs` | Cards pop on use, show passive pulses and synergy links. Combat reuses this. |
| Hard mode | `pressure.rs`, `tactics.rs`, `shard_rules.rs`, `theme_rules.rs`, `pacts.rs` | Marker blocks (`// === name ===`) in `main.rs`. Combat follows the same convention. |
| Co-op | `coop_game.rs`, `coop.rs` | Caught means ghost, revive by teammate. Host-synced pacers only. |

## 3. Design pillars

1. **Fight is a choice, not a gate.** You can still sneak, dodge and leave. Fighting pays (dust, healing motes, safer routes) but is never required to reach a portal.
2. **Telegraphed and fair.** Every enemy attack is announced (0.4–0.8s), uses the existing tells (eye colour, tint), and can be dodged. Same rule as `boss.rs`.
3. **Cards are the build.** What you equip decides how you fight. Every card's active has a combat use, and its passive gets a combat hook.
4. **Short fights.** Basic enemies take 2–3 hits. A fight lasts seconds, not minutes.
5. **Readable on screen.** Nerve, enemy health and hits are visible without text.

## 4. Health: NERVE

Name: **Nerve** (not "health", to keep the tone). Shown as pixel hearts (`Icon::Heart` already exists) under the depth counter, left side.

| Rule | Value (to tune in C8) |
|---|---|
| Start | 3 nerve; max 5 (raised by picks) |
| Contact hit | 1 (heavy attacks 2) |
| Invulnerability after a hit | 1.2s, player flickers (reduced motion: steady tint) |
| Knockback on hit | 1 cell away from the source, blocked by walls |
| Nerve reaches 0 | The existing *caught* flow runs unchanged (shard drop, respawn, grace). Nerve then refills to 2. |
| Between dreams | +1 nerve on taking a portal (to max). No full heal. |
| Healing | Calm mote (rare enemy drop, +1), REWIND (restores what you lost in the last 5s), shop item (C9), CALM MIND passive (slow regen out of combat) |
| Autopilot | Stays immune, as today |
| CLASSIC rules | Nerve is not used at all |

The eye reacts: its lid flinches on a hit and the pupil shakes at 1 nerve (reuse `eye_motion.rs`).

## 5. Basic combat

**STRIKE** — a short arc attack with its own key (proposed `F`, controller X; see decisions).

| Stat | Value |
|---|---|
| Reach / arc | 2.4 units (about 0.8 cell), 120° in front |
| Damage | 1 |
| Windup / recovery | 0.1s / 0.25s; you can walk during recovery at 60% speed |
| Cooldown | 0.45s |
| Hit effects | Brief stagger on the target, small knockback, hit-stop via `juice.rs` (honours reduced motion), spark particles in the card's tint |
| Bonuses | Backstab ×2 (from behind), airborne ×1.5, versus a staggered or frozen enemy ×2 |

**Enemy health and stagger.** Each enemy gets `hp` and a `poise` bar. Hits fill poise; a full bar staggers (reuses the `stunned` map). An enemy at 0 hp unravels (`dissolve.rs`), drops dust (1–3), and rarely a calm mote. Existing enemies get hp:

| Enemy | hp | Notes |
|---|---|---|
| Pacer | 2 | Chases less after being hit (resets its lunge) |
| Elite Fast / Shade / Big | 2 / 2 / 4 | Shade is hittable only while visible |
| Splitter | 2 | Children have 1 each |
| Stalker | 3 | Only vulnerable while watched (it is frozen then) |
| Mimic | 3 | Vulnerable only in its replay window |
| Sentry | 4 | Hit its back (it turns slowly) to break the beam |
| Drifter | 2 | |
| Jester | 1 | Flees when hit |

## 6. Abilities in combat (all 16)

Every ability keeps its present job. Combat adds an effect when the run is in COMBAT rules. Nothing here changes `powers.rs`.

| Ability | Combat effect | Notes |
|---|---|---|
| Dash | Cuts through enemies on its path for 1 damage each; i-frames as today | The aggressive dodge |
| Blink | Lands behind the nearest enemy within reach: next STRIKE is a guaranteed backstab for 2s | Counter to Mask |
| Stillness | Freezes enemies in a wide area; frozen enemies take ×2 and cannot be knocked back | |
| Phase | Pass through enemies; your next STRIKE within 3s hits for ×2 as a "ghost strike" | |
| ShardCall | Utility: also pulls nearby calm motes to you | Stays mostly non-combat |
| Decoy | Enemies attack the double; it bursts when it expires: 1 damage and stagger in a small radius | |
| Float | Slow fall; a STRIKE while floating is an aerial ×1.5 | |
| Flare | Light cone blinds and staggers enemies, reveals Shades, damages Shades 1 | Counter to Shade |
| Rewind | Snap back and heal the nerve lost in the last 5s (max 2); long cooldown unchanged | The only real heal |
| Leap | Landing makes a small shock: 1 damage and stagger to what it lands on | |
| Slam | Landing damages 1 and staggers every enemy near (already stuns) | The crowd tool |
| Anchor | Second use snaps you back and leaves a shockwave at the marker's origin: stagger | |
| Ward | Absorbs one hit as today, and reflects 1 damage back at the source | |
| Step | A stair of air; high ground gives ×1.5 damage to enemies below | |
| Hush | Silence plus a stealth takedown: STRIKE on an unalerted enemy deals ×3 | The assassin |
| Gust | Knockback, plus 1 damage if it pushes an enemy into a wall or hazard | |

**On the hand.** A card pops when its ability fires (done). New in combat: the card flashes a damage number-free pulse when the effect hits something, and a card whose ability is the *right counter* to a nearby enemy glows softly (a hint, off in `Settings`).

## 7. Passives in combat

Existing passives get a combat hook, so card choice matters (all values in `combat_tuning`):

| Passive | Combat hook |
|---|---|
| Swift Feet | Faster STRIKE recovery movement (80% instead of 60%) |
| Spring | Higher jumps; aerial STRIKE gets the airborne bonus longer |
| Wide Eyes | Shows enemy hp pips and the weak spot highlight; backstab arc widens |
| Heavy Air | STRIKE and landings knock back further; Brutes stagger easier |
| Long Breath | Contact i-frames last 0.4s longer |
| Shard Sense | Marks the nearest enemy that is about to attack |
| Dream Anchor | Last stand: the first lethal hit leaves you at 1 nerve (once per dream) instead of being caught |
| Dust Hoarder | Enemies drop more dust |
| Lucky Memory | Enemies can drop a card fragment toward remembering the dream |
| Calm Mind | Nerve regenerates 1 per 25s while no enemy is alerted |
| Double Jump | One extra air STRIKE; dodge over low attacks |
| Quicken | Abilities recharge faster (as today) and STRIKE cooldown −10% per stack |

**New pick-only upgrades** (offered in upgrade picks, not on cards, so the approved card table stays):

| Upgrade | Effect | Max |
|---|---|---|
| Second Heartbeat | +1 max nerve and heal 1 | 2 |
| Sharp Edge | STRIKE +1 damage against staggered enemies | 3 |
| Long Reach | STRIKE reach +25% | 2 |
| Counter | A perfectly timed dodge (within 0.2s of a hit landing) staggers the attacker | 1 |
| Echo Strike | Every 4th STRIKE sends a second wave through the enemy | 1 |
| Thorns | Enemies that hit you take 1 | 1 |
| Calm Spore | Calm motes heal 2 and spread to nearby enemies as a short stagger | 1 |

Synergies get 3 new entries (for example: GHOST STEP + Sharp Edge gives phased strikes extra damage). They plug into `ALL_SYNERGIES` and the hand's links.

## 8. New enemies

Each is a pure module (decisions only, like `tactics.rs`) plus a builder, texture and spawn rule. Telegraphs follow `boss::TELEGRAPH`. Chasers stay slower than the player. Spawn gating by depth and theme avoids overwhelming the first dreams.

| Enemy | Depth | hp | Behaviour | Counter |
|---|---|---|---|---|
| **Tooth** | 3 | 2 | Fast short lunge from the dark; low reach | Dash through it, or STRIKE the lunge recovery |
| **Weeper** | 4 | 2 | Keeps its distance and spits slow tears (dodgeable, 0.8s tell) | Close in with Blink or Dash; Decoy draws its fire |
| **Lullaby** (brute) | 5 | 6 | Slow, big; ground-slam with a ring telegraph; front is armoured (STRIKE staggers only) | Hit from behind (Blink, Hush), or Slam |
| **Moths** | 6 | 1 each, groups of 4 | Orbit, then dive one at a time | Stillness, Gust, or one STRIKE each |
| **Mask** | 7 | 3 | Shield blocks the front; slow turns | Blink behind it, Gust to break the shield, Slam |
| **Leech** | 8 | 2 | Latches on and drains 1 nerve every 2s until shaken (jump three times) or struck | Strike it; Ward blocks the latch |

Elite versions of the new enemies reuse the elite tints and plug into `tactics.rs` shouts and packs. Nightmares gain hp bars (C7): the hunter is vulnerable while resting, Summon wisps die in one STRIKE, Eclipse keeps its sight rule. The sigil objective is unchanged: killing is optional.

## 9. Screens, feel and options

- **HUD:** Nerve hearts left under "deepest". Enemy hp pips float over targeted enemies (only when hit or with Wide Eyes). The hand shows cooldown and counter hints. All fit `audit_text_fits_every_screen`.
- **Juice:** hit-stop and shake through `juice.rs`, red flash through the existing `flash` (reduced motion tones it down). Sounds: strike, hit, stagger, unravel, nerve loss (new `Sound` variants).
- **Options:** `Dream rules` (CLASSIC / COMBAT), `Combat hints`, and a *Gentle* nerve setting (5 start, 0.5s longer i-frames). Reduced motion removes hit-stop, shake and the flicker.
- **Tutorial:** first COMBAT dream shows a controls line and one slow Tooth.
- **Saves:** new fields use `#[serde(default)]`. Old booklets load. Records and stats add kills and hits taken.

## 10. Decisions I need from you

1. **Tone.** Enemies "unravel" (my default), or die more plainly?
2. **Mode.** COMBAT as an option beside CLASSIC (my default), or replace the old rules?
3. **Strike key.** `F` / controller X, or left mouse / controller right trigger?
4. **Nerve.** 3 start, 5 max, +1 per portal. OK, or harsher?
5. **Rewards.** Dust and rare calm motes only, or also card fragments / new card drops from kills?
6. **Hard mode.** Should pacts and ascension rules touch combat in the first release, or wait for C8?
7. **Co-op.** Combat off in co-op for now (my default), or host-synced from the start (adds about two milestones)?
8. **New achievements.** The earlier plan said the 29 in `achievements.rs` were exact and no new ones without your say; combat ones need your approval.
9. **New text.** Enemy names (Tooth, Weeper, Lullaby, Moths, Mask, Leech) and codex lines need your approval.

## 11. Milestones

Each is one PR against `main`, TDD per task, commit per green task (`combat: <what>`). Marker blocks `// === combat ===` keep `main.rs` merges clean, as with hard mode.

| # | Milestone | Size | Depends on |
|---|---|---|---|
| C0 | Sign-off on section 10; briefs written in `plans/combat-briefs/` (CONTEXT.md plus one BRIEF per stream) | S | you |
| C1 | **Nerve**: `nerve.rs` (pure), contact damage and i-frames replace instant catch in COMBAT only, hearts HUD, setting, refill rules, eye flinch | M | C0 |
| C2 | **Strike and enemy hp**: `combat.rs` (pure hit resolution), `hp`/`poise` on `Pacer` and `SpecialEnemy`, unravel, drops, hit-stop, sounds | M | C1 |
| C3 | **Ability combat effects** (table 6), hand glow hints | M | C2 |
| C4 | **Passive hooks and 7 new upgrades** plus 3 synergies | M | C2 |
| C5 | **Enemies A**: Tooth, Weeper, Lullaby (pure AI, builders, textures, spawn tables, tests) | L | C2 |
| C6 | **Enemies B**: Moths, Mask, Leech; tactics and elite integration | L | C2 |
| C7 | **Nightmare combat**: hp bars, vulnerability windows, wisps killable, reward tuning | M | C2 |
| C8 | **Balance and hard mode**: headless autopilot combat sweep (`#[ignore]`), curves, pacts/ascension/pressure hooks | M | C3–C7 |
| C9 | **Polish**: tutorial, options, codex, achievements (if approved), shop healing item, accessibility pass, screenshots | M | C8 |
| C10 | **Bug-testing round**: full tests, clippy, screenshots of every enemy and ability, autopilot seeds 1–5, route tests | S | C9 |
| C11 | **Co-op** (only if approved) | L | C10 |

**Dispatch.** Per your usual preference: C1 and C2 go first, one stream each, since everything else sits on them. After C2 lands, C3, C4, C5, C6 and C7 run **in parallel** as subagents with their own worktrees, each writing a `NOTES-<name>.md`. Overlap in `main.rs` and `hud.rs` is expected and handled by the marker blocks; I merge them in order C3, C4, C7, C5, C6 and fix conflicts.

## 12. Testing and verification

- **Pure modules** (`nerve.rs`, `combat.rs`, one per enemy): unit tests for every number in sections 4–8, including edge cases (hit during i-frames, simultaneous hits, lethal hit with Dream Anchor, stagger on a frozen enemy).
- **Route safety:** existing patient-walker and `every_route_is_physically_traversable` tests run with each new enemy forced into every layout.
- **Headless frame tests:** `screen_smoke` frames for nerve HUD, enemy pips and each new enemy's telegraph, plus `audit_text_fits_every_screen`.
- **Determinism:** existing seeds replay in CLASSIC; new RNG draws use their own seeded streams.
- **Dev switches:** `DREAMSCAPE_RULES=combat`, `DREAMSCAPE_ENEMY=Tooth`, `DREAMSCAPE_NERVE=1` for screenshots, matching the existing `DREAMSCAPE_*` style.
- **Balance sweep (C8):** `#[ignore]` long test: autopilot with each single-card loadout and 5-card loadouts, seeds 1–50, reporting depth reached, nerve lost and kills per dream. Target: a fresh save reaches depth 5–8 in COMBAT, and no single card trivialises depth 10.
- **Real play:** you hand-test each milestone via the game window, like the hand.

## 13. Risks

| Risk | Mitigation |
|---|---|
| Combat dilutes the stealth identity | COMBAT is optional; fights stay short; sneaking always still works |
| `main.rs` is about 10k lines and conflicts are likely | Marker blocks, pure modules, merge order, small PRs |
| Enemies walling the route | Spawn rules reuse the pacer pocket system; route tests forced per enemy |
| Balance spiral with 20 cards, 16 abilities and hard mode | Autopilot sweep and a single `combat_tuning` constants module |
| More flashing: there is a launch photosensitivity warning but no flash limiter I could find | Keep hit flashes short and at most 3 per second; add that cap in C1; test the reduced-motion path |
| Full test suite takes about 8 minutes | Run focused tests while building; full suite per PR |
| Co-op desync | Combat off in co-op until C11 |
