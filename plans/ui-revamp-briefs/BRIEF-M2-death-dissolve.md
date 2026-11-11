# Brief M2: death dissolve

Goal: the dreamer dissolves into embers on death, then the Summary fades in; a catch+respawn gets a short version.

## Scope
- New `src/death_fx.rs` (pure) + `mod death_fx;`.
- `assets/shaders/mesh.frag`/`mesh.vert`: add `uniform float uDissolve;` (0 = solid, 1 = gone) and `uniform vec3 uDissolveColor;`. Fragment: hash/value-noise from world position (3D) `n`; `if (n < uDissolve) discard;` plus a band `n < uDissolve + 0.06` tinted `uDissolveColor` and emissive. Uniform only: **no new variant flag bit** (so no `ALL_FLAGS` change). Default 0.0 so every other draw is unchanged. Find where `uHero`/`uOutline` are set in the render path (grep `uHero`) and set the new uniforms the same way.
- `main.rs`: marker `// === deathfx ===` in the caught block (~12940), the run-ending path that sets `Mode::Summary` (~9533), and the per-frame update. Delay the mode switch until the dissolve ends; lock input during it; any key skips.
- Leave alone: M1/M3 files, hand.rs, relic files.

## Types / functions
```rust
pub enum Kind { Full, Short }                       // Full ~1.8 s run-ending, Short ~0.4 s mid-run catch
pub struct Death { kind: Kind, age: f32, first_person: bool, color: [f32;3] }
impl Death { pub fn new(kind, first_person, theme_color) -> Self; pub fn update(&mut self, dt: f32); pub fn progress(&self) -> f32 /*0..1 eased*/; pub fn finished(&self) -> bool; pub fn skip(&mut self);
  pub fn body_dissolve(&self) -> f32;          // 0..1 for Hero
  pub fn view_fade(&self) -> f32;              // first person: ember/desaturate overlay 0..1
  pub fn particles(&self, n: usize) -> Vec<Ember>; // seeded drifting up, fading
}
pub fn short_for(reduced_motion: bool, juice: UiJuice) -> bool;
```
- Third person: `Hero` (and the held weapon entity later, R5 reads `body_dissolve`) gets `uDissolve = progress`. Particle embers: reuse the existing spark/dust particle path in `juice.rs`/`main.rs` (grep `spark`); do not invent a new renderer.
- First person (`first_person_active()`): there is no body; dissolve the hands/weapon if drawn, draw an ember fade as a full-screen overlay through the existing `flash` overlay path with theme colour, ramped with `view_fade`.
- Reduced motion: no particles, 0.5 s plain fade. `reduce_flashing`: overlay alpha <= 0.5, no pulse.
- Run-ending death = the existing path to `Mode::Summary`; mid-run catch = the existing `caught` respawn flow, play `Short` and do not delay respawn logic.

## Tests
- progress monotone, 0 at start, 1 at end; `finished` only at end; `skip` jumps to finished; Short < Full; reduced path has zero particles; particle positions deterministic per seed.
- Unit test (no GL) that a dying run stays in its previous Mode until `finished()` then goes to Summary (extract the decision to a pure fn `should_enter_summary(&Death)`).
- `screen_smoke` unaffected. The parent screenshots mid-dissolve (`SHOT_AT` just after the trigger).

## Approval list
None.
