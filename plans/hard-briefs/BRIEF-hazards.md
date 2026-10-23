# Stream: HAZARDS (module `theme_rules.rs`, branch feat/hard-hazards)
Mechanic: each of the 20 dream themes has ONE rule that bites, so each dream is hard in its own way.
Design (decided):
- Data table in theme_rules.rs: `rule_for(theme) -> ThemeRule` for ALL 20 `ALL_THEMES` (test: every theme has a rule; rules reuse a
  small set of reusable kinds, e.g. RisingTide (slows more as the dream ages, floor resets when you reach a shard), ShrinkingFloor
  (outer cells crumble inward, never the route), GustWind (periodic push), Echoing/Silence (enemies hear dashes/blinks), DimmingLight,
  DriftingFog (existing fog pockets move), StickyFloor, BeatFloor (hazard tiles on the beat). Pick sensible per-theme pairings
  from theme names/flavour; write them in NOTES.
- Gate: depth >= 3, normal dreams only; strength ramps with depth (clamped). Existing hazards (crumbles, fog) must keep working.
- Pure, tested: `strength(depth)`, per-kind state machines (`tick(dt)`, `effect()` -> speed/push/visibility multipliers or cell sets)
  and a route-safety test: shrinking/crumbling never removes cells on the shard-portal route.
- HUD: a short rule line on the title card ("the tide is rising") using existing title-card text slots; must pass audit_text.
