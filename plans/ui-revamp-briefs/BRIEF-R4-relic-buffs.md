# Brief R4: relic buffs

Goal: an equipped relic's `RelicEffect` changes the run through the existing quirk levers, plus a nerve-cap lever.

## Scope
- `relic_types.rs`: `RelicEffect.max_nerve: u32` (`#[serde(default)]`) is landed by the parent in wave 0; extend `to_quirk` only if `QuirkEffect` gains the lever (add `max_nerve: u32` to `QuirkEffect` in `worth.rs` with `Default`; fix literals with `..Default::default()`).
- `retention.rs` `card_effects(b, cards)` (line 13): add the equipped relic's `to_quirk()` via the existing `.plus()` (grep `fn plus`). Equipped relic = `Stash`/`Booklet` equip field. Add `Booklet.equipped_relic: Option<u32>` (`#[serde(default)]`) in a `// === relic-equip ===` marker block, plus `retention::equip_relic(&mut Booklet, id) -> bool` (id must be owned) and `equipped(&Booklet) -> Option<&Relic>` (fallback: highest rarity owned, per the plan).
- `nerve.rs`: `MAX` stays; add `Nerve::max_with(extra)` or apply `max_nerve` where the cap is read (grep `nerve::MAX`), never above 8.
- Balance table in `relics.rs`? No: R4 owns `effect_for(rarity, theme, seed) -> RelicEffect` in a new `relic_effects.rs` (pure): Cracked one small lever, Worn one medium, Pristine two, Mythic three + a nerve or shard bonus; magnitudes capped (dust <= 15%, speed <= 8%, calm <= 10%, memory <= 0.06, rerolls <= 2, shards <= 2, grace <= 2, max_nerve <= 1). The theme biases which levers (Garden: memory/calm; Clockworks: speed/grace...).

## Tests
- `effect_for` deterministic; caps hold over 10k seeds; rarity -> number of non-zero levers; `card_effects` with/without relic; equip rejects unowned id; old booklet loads with `equipped_relic = None`; `max_nerve` raises cap and respects 8.

## Approval list
None (numbers only).
