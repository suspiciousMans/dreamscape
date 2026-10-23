# Next game features: triage and plan

Written after pulling `bfaef65` (PR #36, "roadmap gaps"). Statuses below come
from reading the source, not from `ROADMAP.md`, whose phase headers are stale.

## 1. Verified state

**Done and wired** (checked in `main.rs`, not just defined):
forms (4.4), attunement (4.2), records (4.5, `record_wake` called, banner
shown on summary), adaptive music (5.5), ability FX (5.1), silhouettes (5.3),
colour-blind markers (5.6, used when building shard/portal textures), balance
log (6.4, called at dream end). Also done and not marked in the roadmap:
cursed cards (1.3), six synergies (1.5), four elite kinds (2.6), the weekly
challenge (`streaks.rs`), merging at the moth (`fusion.rs`), the ending.

**Correction to my earlier note:** screen shake and hit-stop (5.4) already
exist. `juice.rs` has a time dip that bottoms out at 0.2x speed, a decaying
shake, hit kinds for caught/shard/pickup, and honours reduced motion. It is
not a gap. I said it was because I grepped for "hitstop" and missed the dip.

**Not built** (no matching code):
- Per-dream variants. `dream/variant.rs` has eight: LowGravity, Flooded,
  Blackout, Inverted, Swarm, Frozen, Hasty, Gilded. None of roadmap 3.4's six
  exist (Mirrored, Tiny, Echo, Night Shift, Kaleidoscope, Breathing).
- Between-dream events (3.5). Nothing between dreams except upgrade picks and
  the pack.
- Secret rooms (3.6). The moth only does merging, from the store or after a
  nightmare reward. There is no shop dream.

Note: `streaks.rs` also has an enum called `Twist`. That is the *weekly
challenge* modifier, unrelated to per-dream `Variant`s. Easy to confuse.

## 2. Triage

Effort: S under a day, M a few days, L a week or more.
Value is my judgement of how much it changes how a run feels.

| ID | Item | Effort | Value | Risk | Depends on |
|---|---|---|---|---|---|
| A | Night Shift variant | S | High | Low | none |
| B1 | Tiny variant | S | Med | Low (camera only) | none |
| B2 | Mirrored variant | S | Med | Med (disorienting; clashes with Inverted) | none |
| B3 | Echo variant | M | Med | Low (reuses `update_echoes`) | none |
| B4 | Kaleidoscope variant | M | Med | Med (post shader; flashing) | none |
| B5 | Breathing variant | M | Med | High (must keep routes valid) | none |
| C | Between-dream events | M | High | Med (needs new text) | none |
| D | Secret rooms and moth shop | L | High | High | C optional |
| E | More forms | S each | Med | Low | none |
| F1 | Rewards for records | S | Med | Low | none |
| F2 | Two-attunement combos | M | Med | Med (balance) | none |
| F3 | Daily share string | S | Low-Med | Low | none |
| G | Housekeeping | XS | n/a | none | none |

Recommended order: **G, A, C, then the B variants, then D.** Rationale:
- G is minutes and stops the roadmap misleading the next planner.
- A is the cheapest high-value item. Blackout and the silhouette work already
  taught the renderer to dim everything but `Lit` things; Night Shift is a
  timer on top.
- C changes the rhythm of a run more than any other item and touches little
  existing code, so it is the best mid-size bet.
- D is the biggest and riskiest. Do it after C so the event system can offer
  "a cracked wall" as one more choice.

## 3. Plan

### G. Housekeeping (do first)
1. Update `ROADMAP.md` headers and tables to match section 1.
2. Move the two displaced doc comments: "Takes `u`..." in `upgrades.rs` back
   above `take`, and "Leaving a dream through the portal" in `main.rs` back
   above `dream_bonus`.
3. Add this plan to the roadmap's "Suggested order".

Accept: roadmap statuses match section 1; `cargo test` green; no new warnings.

### A. Night Shift
- Add `Variant::NightShift`. Every 8s the lights go fully out for 2s.
- Reuse `gameplay::lit_level`: during the dark, only `Lit` things draw at full
  strength; silhouettes stay at `SILHOUETTE_LIT`; everything else is black.
- Pure timing function (`night_shift::dark(age) -> f32` with a short fade in
  and out) so it is unit-tested like `juice.rs`.
- `reduce_flashing` must turn the cut into a slower fade. Reduced motion
  should not change it.
- Must not stack in a way that makes a dream unwinnable: forbid it together
  with Blackout, and check shard and portal beacons stay visible in the dark.

Tests: dark window timing; never dark at age 0; excluded with Blackout; all
existing generation tests still pass with the variant forced on.
Accept: playable; shard and portal still findable during the dark.

### C. Between-dream events
- New mode shown after a dream, about one time in six, in place of the upgrade
  pick. Add a `Pending`-style step in `transition.rs`'s flow rather than a
  new top-level mode if it fits.
- `events.rs`: `Event { prompt, choices: [Choice { text, outcome }] }`, with
  `Outcome` covering the existing currencies: upgrade, curse, shard, dust,
  gilded next dream, lose an upgrade. Pure data plus a resolver, seeded from
  the run seed on its own stream so existing seeds replay.
- UI in the existing pixel style (`pixel_ui.rs`), pad and keyboard.
- Start with five events. **Needs your approval of the text before it goes in**,
  matching the rule from the earlier plan that no new story text is added
  unreviewed.
- Records and balance log should note when an event was taken.

Tests: outcomes apply exactly once; same seed gives same event; every outcome
leaves the run valid (no negative dust, no overfull kit).
Accept: events appear at the expected rate in a 100-dream balance run.

### B. Remaining variants (any order)
Each is a `Variant`, a label, a vocabulary entry, and tests that the
traversal guarantees still hold with it forced on.
- **B1 Tiny:** camera pulls back 2x. Check the pause menu and HUD scale.
- **B2 Mirrored:** swap left and right. Do not allow with Inverted. Respect key
  remapping (swap the actions, not the keys).
- **B3 Echo:** your ghost from 5s ago collects shards and enemies chase it.
  Reuse `update_echoes` from Afterimage Fields; do not allow in that dream.
- **B4 Kaleidoscope:** 4-way mirror post effect, gameplay unchanged. Off under
  `reduce_flashing` and reduced motion.
- **B5 Breathing:** non-route walls shift by up to half a cell. Do last: it is
  the only one that can break reachability. The patient-walker test must pass
  at every phase of the motion, not just at spawn.

### D. Secret rooms and the moth shop
Stage it so each step ships alone.
1. Layout support: a cracked-wall cell type the generator can place off the
   main route, with tests that it never blocks the route or the shard.
2. Breaking it: Blink or Dash into it opens the door; other abilities do not.
   Add feedback so players can tell a cracked wall from decoration.
3. The shop dream: a small `DreamTheme`-style room run by the moth, spending
   shards on upgrades. Reuse the store's pixel UI and `fusion.rs` entry points
   rather than a second shop.
4. Rarity: roughly one secret per few dreams, tuned with the balance log.

Biggest risk is step 1. Open question for you: should the shop also offer
merging, or stay separate from the store's `[m]` merge screen?

### E, F: small follow-ups
- **E:** new forms are data (gift, cost, unlock condition in `form.rs`). The
  existing tests already assert each non-default form trades something.
- **F1:** a dust payout or card frame the first time each record is broken.
- **F2:** only after playtesting single attunements, to avoid piling free
  power on top of forms and loadouts. Watch the loadout difficulty scaling.
- **F3:** a copyable `seed + depth` string for the daily.

## 4. Decisions (2026-10-21)
1. Night Shift first, then events. Night Shift is built (shown in game as
   FLICKERING): branch `feat/night-shift`.
2. Event text is drafted in `2026-10-21-event-drafts.md` for review. Nothing is
   wired into the game until the text is approved.
3. Secret rooms (D) are in scope. They stay last, after events.

## 5. Not planned here
Engine work (see `jame-engine/docs/ROADMAP.md`) and balance retuning. Both are
separate efforts, though the balance log should be used to check A and C.
