# Between-dream events: second batch (draft, not wired in)

Same voice and limits as the first five (title <= 28, prompt <= 64, choice <= 30,
detail <= 60 characters). Nothing here is in the game until you approve it.
Every event ends in a free "nothing" choice. Bracketed text is the outcome.

Mechanics marked NEW need a small addition to `events.rs`; everything else
reuses what the first five already do.

### 6. THE LOUD ROOM
> Every wall is talking at once. Nobody is listening.

- **Listen** — [next dream gets a random extra twist; one free reroll]
- **Shout back** — [a random upgrade AND a random curse]
- **Hum along** — [nothing]

### 7. THE TOLL BOOTH
> A booth in the middle of nothing. The sign says EXACT CHANGE.

- **Pay 12 dust** — [a safe upgrade (no curse)]
- **Pay with a card** — [NEW: lose an upgrade, gain 20 dust]
- **Drive through** — [nothing]

### 8. THE SLEEPING GUARD
> A guard asleep at a gate that was never closed.

- **Tiptoe past** — [next dream gilded; next pick has one fewer card]
- **Wake him** — [a random curse, but a random rare-or-better upgrade]
- **Let him sleep** — [nothing]

### 9. THE SPARE KEY
> A key on a string. It fits something you haven't dreamed yet.

- **Use it now** — [a random upgrade; next dream gets a random extra twist]
- **Keep it** — [+8 dust]
- **Put it back** — [nothing]

### 10. THE COLD SPOT
> One square of floor is colder than the rest.

- **Stand in it** — [NEW: swap one held upgrade for a different random one]
- **Warm it with dust** — [costs 6 dust; one free reroll]
- **Step around** — [nothing]

### 11. THE LATE GUEST
> Someone has set a place for you. You're not sure you're invited.

- **Eat** — [a random upgrade, and the next dream is gilded; a curse comes with it]
- **Pour the tea** — [+6 dust]
- **Stay standing** — [nothing]

### 12. THE LIGHTHOUSE
> A lighthouse in the corridor. Its beam sweeps the floor.

- **Put out the lamp** — [next dream is BLACKOUT and gilded]
- **Follow the beam** — [next dream's shard is easier to find: SHARD SENSE for one pick? (needs a design call)]
- **Walk on** — [nothing]

---

## Notes for you

- **Strongest** (I'd keep as-is): 6, 7, 9. They each give a real trade with a clear price.
- **Needs a call:** 12's second choice. I don't have a clean "one-dream boost" in the
  game, so either cut it for something already supported (e.g. +1 free reroll) or
  you tell me what it should do.
- **Weakest:** 11. "Eat" stacks an upgrade, a gilded dream, and a curse, which is
  hard to read at a glance. I'd drop the gilding.
- **Risk:** 8's "wake him" asks for rare-or-better upgrades, which the existing
  roller can't guarantee at low depth; at depth < 6 it would fall back to any upgrade.
- With all 12 the chance of a repeat inside a long run drops a lot. The existing
  "never twice running" rule stays.
