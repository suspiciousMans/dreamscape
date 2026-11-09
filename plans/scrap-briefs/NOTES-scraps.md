# Scraps: notes

Built per `PLAN-scraps.md` (S1-S4). Branch `feat/scraps`.

## Behaviour summary
- Every pack opened from the pick screen adds one scrap to `Booklet.satchel` (saved at once; kind = `roll_scrap(run_seed, packs opened this run)`). Not given in autopilot, co-op or the prologue. At 9 of a kind nothing is added.
- Loadout (deck builder): top-left strip `SATCHEL n/2  [g] next  [h] carry`; `[g]` moves the cursor, `[h]` carries/leaves the scrap under it (max 2, a kind twice only if held twice).
- Deck runs shuffle the carried scraps into the draw pile (ids `SCRAP_BASE`.., below `TOOL_BASE`). Playing one exhausts it and spends it from the satchel (saved at once). Unplayed ones stay in the satchel. A scrap with nothing to act on (Mend at full nerve, Lantern with no shard, Safe Step already armed) is refused with the denied sound and kept.
- The hand draws a scrap at 86% size, in its own colour, with a short tag.

## for approval
Scrap names and one-line descriptions (`scraps.rs`):
- Mend: "Refill your nerve."  (tag MEND)
- Lull: "Calm every enemy for a few seconds."  (tag LULL)
- Lantern: "Light up the nearest shard."  (tag LIGHT)
- Windfall: "Bonus dust, paid when the run ends."  (tag DUST)
- Safe Step: "The next hit does nothing."  (tag SAFE)
- Redraw: "Throw back your hand and draw a fresh one."  (tag REDRAW)

Pack reveal line (under the cards, after the last one): `+ scrap: <Name> - <description>`

Loadout strip: `SATCHEL 1/2  [g] next  [h] carry`; selected line `<Name> x<held> (carry x<n> | left): <description>`.

## Numbers (scraps.rs)
Mend refills to full; Lull 6 s; Lantern pillar 10 s; Windfall +25 dust (plain, not scaled by the hand); carry limit 2; cap 9 per kind.

## Known limits
- Safe Step is not written into the run save: quitting with it armed loses the charge (the scrap was already spent).
- Lull stuns pacers and the newer enemies through `rattle_enemies` (whole dream); specials/hunter follow whatever `rattle_enemies` covers.
- The Lantern pillar cannot be tested without a GL context; the nearest-shard pick and the spend are tested.
