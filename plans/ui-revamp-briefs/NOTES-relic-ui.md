# NOTES-relic-ui (R6)

## Dev
- `DREAMSCAPE_SCREEN=relics` (case, 600 ash), `relic_pull` (one pull already made), `relic_pouch` (7 relics, pouch page). Also `store` then [tab] x3.

## Deviations from the brief
- Store tabs are Perks / Lottery / Tickets / Relics (tab index 3); there is no "Abilities" tab, so Relics is the fourth.
- `relics::pull` charges `cost_for(rolled rarity)` (40/60/80/120), known only after the roll. So a pull requires the dearest price (120) in hand, and the rolled rarity's price is then charged. Both are on screen (button says "UP TO 120 ASH", odds table lists each rarity's price). If the owner wants one flat price, change `relics::pull`/`cost_for`.
- Pull seed = mix(save_seed, total_pulls): an unsaved pull replays to the same relic (no re-roll by quitting); a saved one advances the counter.
- Equipping mid-run (store opens from Pause) changes the held weapon at once; the buff effects (`card_fx`) still apply from the next run start.
- Weapon trail/clash have no labels on screen; only the icon and "SIZE n%".

## For approval (all new on-screen copy)
Titles: THE RELIC CASE, THE RELIC POUCH, WHAT THE CASE HOLDS, YOUR RELICS
Header: "{n} ash · {n} relics · {n} pulls so far"
Rarity labels: CRACKED, WORN, PRISTINE, MYTHIC; tag: "· FOIL"
Odds panel: "{r} {pct}% {cost} ash" per rarity; "FOIL: {pct}% OF ANY RELIC"; "FEATURED: {dream} ({pct}% OF RELICS)"; "MYTHIC GUARANTEED IN {n} PULLS"; "MYTHIC GUARANTEED ON THE NEXT PULL"; "{n} PULLS SINCE YOUR LAST MYTHIC"
Button: "OPEN THE CASE · UP TO {n} ASH"; pouch: "EQUIP", "EQUIPPED"
Pouch: "THE POUCH IS EMPTY", "OPEN THE CASE TO FIND A RELIC", "SIZE {n}%"
Buffs: "+{n}% DUST", "+{x.xx} MEMORY", "+{n}% SPEED", "+{n}% CALM", "+{n} GRACE", "+{n} FREE REROLL(S)", "+{n} START SHARD(S)", "+{n} MAX NERVE"
Status: "you need {n} more ash", "equipped", "the case couldn't be written down (see log)", "the pouch couldn't be written down (see log)"
Hint bars: "[enter] open   [p] pouch   [tab] next tab   [esc] leave"; "[w][s] choose   [enter] equip   [p] the case   [tab] next tab   [esc] leave"
Existing store hint "[tab] the moth's lottery" etc. unchanged.
- tickets hint now "[tab] the relic case" (tab order changed: Tickets -> Relics -> Perks).
