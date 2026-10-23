# Stream: ASCENSION RULES (file `progress.rs`, module-less; branch feat/hard-ascension)
Existing: Ascension levels 0..=10 in `progress.rs` (`ascension_rule`, `Ascension` impl), all numeric. Add levels 11..=16 (raise MAX_ASCENSION
to 16) that change RULES instead of numbers; unlock exactly like the others (beat a long run at the highest level).
Design (decided, cumulative like existing):
- 11 "no safe respawn": caught = back to the start of the dream with half your lucidity (do not end the run), no grace.
- 12 "the dream remembers": enemies spawn near where you were caught last dream (seeded, bounded; never on the start cell).
- 13 "one ability only": you start the run with one fewer loadout ability slot (min 1).
- 14 "no pickups twice": shard beacon/Shard Sense effects halved.
- 15 "the eye is closing": Pressure-like timer: if `pressure.rs` exists on the branch it will be merged later; DO NOT depend on it. Instead
  implement: every dream is HASTY-style with a generous timer (90s, shrinks 3s per ascension above 14), failing costs a card of that run.
- 16 "everything at once": a random pact-free twist is always active (use existing variants, seeded).
- Pure `Ascension` methods for each rule with unit tests (extend `ascension_rules_stack`), titles in `ascension_rule` text (short, lowercase),
  wiring in main.rs at the existing `active_asc` call sites; the title-screen ascension picker must list the new levels and pass audit_text.
