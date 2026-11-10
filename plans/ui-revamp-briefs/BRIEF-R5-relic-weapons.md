# Brief R5: relic weapons (first person) + combat hooks

Goal: the equipped relic is a visible held weapon, lower-right, swaying; it changes strikes; it dissolves with the dreamer.

## Scope
- `relic_types::Weapon` (parent adds `tint`, `trail`, `clash` with `#[serde(default)]`). New pure `src/relic_weapon.rs`: `weapon_for(rarity, theme, seed) -> Weapon` (shape index into `dream::ALL_SHAPES`, `scale_pct` 15-35 in 0.15-0.35 world units, tint = theme accent `theme.spec()` accent 0), `strike_mods(&Weapon, rarity) -> StrikeMods { damage_bonus: u32, reach_mult: f32, arc_relax: f32, knock_add: f32, hit_stop_secs: f32 }` (caps: damage +1 at most for Mythic; reach <= x1.25; arc relax <= 0.15; hit stop <= 0.05 s), `held_pose(t, turn_rate, juice, reduced) -> HeldPose { offset, rot, bob }`.
- Pose: lower-right of view, tilted ~15 degrees toward screen centre, idle bob 0.6 Hz / small amplitude, slight lag behind camera turn (spring from `ui_motion::Spring`); `reduced_motion` = static pose.
- Held mesh: build with `dream::build_mesh(shape)` (grep the signature), spawn as a `Lit` entity rendered after the world pass, parented each frame to the camera (not a `Hero`: pass 1 draws a black shell around `Hero`). Mythic: a tiny orbiting `Orb`/`Crescent` (cosmetic). Third person: same mesh at the dreamer's right hand.
- `combat.rs`: apply `StrikeMods` to reach/arc/damage/knockback where the consts are read (`REACH`, `ARC_COS`, `DAMAGE`, `KNOCKBACK`; grep). No behaviour change when no relic is equipped. `slash_ray`/`slash_spark` take an optional trail style. `juice.rs`: hit-stop override via `Weapon.clash` only; still obeys the soft ceilings (40-50 ms).
- `sounds.rs`: add `Sound::RelicStrike` (one per tier = 4 variants `RelicStrikeCracked..Mythic`); bump `ALL_SOUNDS`; low audible f0 with `.noise(1.0)` (f0 = 0 gives NaN).
- Uses `uDissolve` from M2 on the held mesh (`Death::body_dissolve`) once M2 is merged.
- Marker `// === relics-weapon ===` in main.rs.

## Tests
- `weapon_for` deterministic, shape index valid for all seeds, scale in range, tint equals theme accent; `strike_mods` caps; no-relic = identity mods in combat tests; pose bounded and static when reduced; new sounds finite and non-NaN; `Weapon` old save loads.
- Parent screenshots first-person with each of the 4 rarities: `DREAMSCAPE_GIVE_RELIC=<id>`-style dev env var you add (document it).

## Approval list
Weapon trail/clash labels if shown in UI.
