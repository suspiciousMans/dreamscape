# C9: combat hints, new-enemy elites, tactics shouts

FOR THE OWNER — nothing here is approved. This lists what was built, which new
player-facing words it adds, and what could not be checked.

## Combat hints (setting)

- `Settings.combat_hints`, default **on**, new Settings row **"combat hints"**.
  It sits after "gentle nerve" and before "colour-blind markers".
- While it is on, a card in hand whose ability answers a foe within
  `combat::REACH + 5` units glows with a pale halo (two quads, still and quiet
  when reduced motion or menu motion Off is set). It never says anything: the
  hint is the glow alone.
- The mapping is pure: `combat::counter(Kind) -> Option<Ability>`.
  Pacer and Tooth → DASH; Weeper → BLINK; Lullaby → SLAM; Stalker → FLARE;
  Moths → STILLNESS; Mask → GUST; Leech → WARD.
  Sentry, Drifter and Jester answer to where you stand, not to a card, so they
  get no hint.

New strings for approval: the row label **"combat hints"** and its setting
description comment ("cards that answer a foe in reach glow softly"). The row's
value is the existing "on"/"off".

## Elite variants for the six new enemies

- Tooth, Weeper, Lullaby: `specials::roll_foe_elite` may give FAST, BIG or
  SHADE. FAST runs that foe's clock 1.25× (its tell, lunge and recovery all
  speed up). BIG adds 2 hp, stands 1.3× larger and wears the elite tint on its
  tell texture. SHADE is invisible beyond `SHADE_SHOWS_WITHIN`, as a pacer is.
- Moths, Mask, Leech: `specials::roll_group_elite` may give BIG or SHADE. FAST
  is not offered because their pace is authored inside each module (moth swarm,
  mask sweep, leech drift), where a per-foe speed multiplier would fight the
  tuning. This is a deliberate gap, not an oversight.
- The roll has its own, well-mixed seed stream, so one roll never turns another
  enemy elite. It runs at **parity with the pacer curve** (share 1.0, chosen by
  the spawn sweep in `specials::tests`): across 1,800 sampled deep dreams that
  is 26% of new foes elite — the same 26% the pacer curve gives — 39% of deep
  dreams holding one, 0.53 each, and 26% holding a group elite. No elites at
  gentle difficulty (same `elite_chance` curve as pacers).
- Elites carry their hp from the first frame (`combat::foe_hp`), so the strike
  and the budget rules see the armoured total immediately; a BIG costs one more
  strike.

## Tactics shouts (hard mode, depth 3+)

- `tactics::stage`, `tactics::earshot`, `tactics::tempo`, `tactics::ALERT_SECS`
  (6 s) and `tactics::ALERT_SPEED` (1.25×) are pure and tested. The hurry is
  picked by a sweep over the real tells: 1.25 is the quickest that still leaves
  0.40 s to answer a Tooth's 0.5 s tell, and it shortens a foe's cycle by 20%.
  Tempo combines FAST and alert with `max`, never a product, so the quickest
  possible foe (a FAST Weeper) only keeps pace with your walk at worst; at 1.30
  it would be 4% faster than you and the tell 0.385 s.
- A pacer that notices you now raises the alarm through one path
  (`DreamscapeGame::raise_alarm`): pacers in earshot are marked and alerted, an
  elite leader still pulls its pack, and new enemies and ENEMIES-B specials in
  earshot get a 6 s hurry.
- A new enemy's **first frame of a tell** is its shout. During the hurry its
  clock runs 1.2× faster (moths: the whole swarm's, mask and leech: their own).
  The hurry ebbs away on its own and never stacks into a freeze.
- Alarms are switched off in co-op and while a hunter is present, as before; the
  gentle first dreams still never shout.

## What the sweep caught

- The elite roll's first draft read its top bits from an XOR with the dream
  seed. A small seed only moves the low bits, so every dream drew the same
  elite index and the sweep showed 0% or 100%. It now goes through
  `worth::mix`, and the sweep's per-foe rate tracks the pacer curve (26% vs 26%).
- The first hurry draft multiplied FAST and alert (1.25 × 1.2), which made an
  alerted FAST Weeper 15% faster than your walk: it could never be escaped. The
  sweep now requires the quickest possible foe to stay at or under your walk,
  which is why the combination is a `max`.

## Not verified

- No display in this environment: the halo, the BIG/SHADE reads and the hurry
  have not been seen in motion. Only the compile, the pure tests and the UI
  budget/text guards were checked.
- No balance sweep: elite share 0.6× and hurry 1.2× are guesses to be tuned by
  play.
