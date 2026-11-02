# Brief: Ash + Booklet (R1)

Goal: add Ash to `Stash`, add `relics` and `relic_lottery` to `Booklet`, and add `press_relic` to `retention.rs`.

## Scope
- `store.rs`: add `ash` field to `Stash` with `#[serde(default)]`, add `earn_ash` and `spend_ash` helpers.
- `cards.rs`: add `relics: Vec<Relic>` and `relic_lottery: RelicState` to `Booklet`, both `#[serde(default)]`.
- `retention.rs`: add `press_relic` that appends a `Relic` to `Booklet.relics` and returns it.
- `progress.rs`: if it reads `Stash` for UI, add `ash` to the displayed counts.

## Ash earn rules
- +1 Ash per completed dream.
- +1 bonus Ash per 3 dream depth reached, max +2 bonus (so depth 1–2 = +1, 3–5 = +2, 6–8 = +3, 9+ = +3).
- +3–5 Ash per beaten nightmare (roll `percent_roll(nightmare_seed, ASH_NIGHTMARE_SALT)` in 3..=5).
- No other sources.

## Relic types (data only, no logic yet)
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelicRarity { Cracked, Worn, Pristine, Mythic }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Relic {
    pub id: u32,
    pub theme: DreamTheme,
    pub seed: u64,
    pub rarity: RelicRarity,
    pub foil: bool,
    pub name: String,
    pub flavour: String,
    pub effect: RelicEffect,
    pub art: TexSpec,
    pub weapon: Option<Weapon>,
}
```

## Tests
- `Stash` defaults to 0 Ash; `earn_ash` adds correctly; `spend_ash` refuses when poor and deducts when possible.
- `Booklet` loads old saves without `relics`/`relic_lottery` as empty.
- `press_relic` appends and assigns the next `id`.

## Approval list
- No new player-facing text in this brief.
