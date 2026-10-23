# Stream: PACTS (module `pacts.rs`, branch feat/hard-pacts)
Mechanic: opt-in difficulty for dust. A harder tier than the existing Cursed cards (see upgrades.rs: Upgrade::Cursed, Tier::Cursed).
Design (decided):
- At each upgrade choice from depth >= 3 there is a small chance (PACT_CHANCE, seeded own stream) that the choice screen also offers
  a PACT slot: "take a pact" (separate card, visually distinct, uses upgrade_ui) instead of one upgrade. A pact = a run-long rule
  with a drawback plus a dust multiplier (+0.15..0.35 each, stacking, additive, cap +1.0 total) paid at wake.
- 8 pacts (data table, tested): e.g. "no regrets" (no reroll/skip for the rest of the run), "thin air" (-1 card at every later pick),
  "loud steps" (alert radius x1.25), "short fuse" (dreams' shard needs +1), "the long way" (enemies 8% faster), "heavy dreams"
  (player 5% slower), "bright eyes" (less tunnel-vision darkness is NOT allowed: pacts only make it harder), "owed" (pay 10 dust now or lose a card).
- Pure API: `roll_pact(seed, depth, taken) -> Option<Pact>`, `Pact::effects()` (multipliers/flags), `dust_bonus(&taken)`, tested
  (determinism, stacking cap, no duplicates, gating by depth).
- Wiring: choice screen, RunUpgrades/run state keeps taken pacts, effects applied through existing multiplier call sites, the wake summary
  lists pacts and the bonus (summary_ui). Must pass audit_text with the new card/summary lines.
