# Jame Engine Guide

A lightweight Rust game engine (SDL2 + OpenGL) for building multiple games with
a PS2-era visual style: low internal resolution upscaled with nearest-neighbor
filtering, per-vertex lighting, fog, optional vertex snapping and affine
texture mapping, and posterize/dither post-processing — all controlled by
savable "shader profiles" you can tweak live in an in-game editor GUI.

## Workspace layout

```
ps2-engine/
  Cargo.toml              workspace root (members = engine, games/*)
  .cargo/config.toml       env vars needed to build vendored SDL2 on this machine
  engine/                  the reusable library — everything below lives here
    src/
      app/                 Game trait + App::run main loop
      platform/            SDL2 window + GL context setup
      input.rs             keyboard/mouse state
      time.rs              delta-time clock
      camera.rs            OrbitCamera
      ecs/                 hecs::World + Transform/MeshRenderer/Camera/Light/LevelObjectMeta/PlayerController components
      mesh/                OBJ loading (tobj), GPU mesh upload, primitives (cube/plane)
      texture.rs            image loading + GPU texture upload
      shader/               GLSL compile/link + shader-variant caching
      renderer/             low-res offscreen FBO + composite dither/posterize pass
      profile.rs            ShaderProfile/RenderParams + RON save/load/cycle
      level.rs               Level/LevelObject + RON save/load (the level editor's data model)
      physics.rs             hand-rolled RigidBody/Collider simulation (gravity + AABB/Sphere collision)
      ui/                   egui-based editor GUI (SDL2⇄egui bridge + panels)
  games/
    sandbox/                the demo game — also your template for new games
      src/main.rs
      assets/shaders/        mesh.vert, mesh.frag, post_composite.frag
      assets/models/, assets/textures/
      profiles/              *.ron shader profiles (auto-generated on first run)
      levels/                *.ron levels (auto-generated on first run)
```

## Quick start

```
cd ps2-engine
cargo run -p sandbox
```

The `.cargo/config.toml` already sets the env vars this machine needs to build
vendored SDL2 (an old CMake-version pin plus a C-standard flag for the GCC
toolchain) — you don't need to set anything yourself. The first build compiles
SDL2 from source and is slow (a minute or two); subsequent builds are fast.

**Controls** (Edit mode): left-drag orbits the camera, scroll wheel zooms,
**F1** toggles the shader-profile panel, **F2** toggles the level editor
panel, **Tab** cycles shader profiles, **F5** saves the current profile,
**F3** enters Play mode (see "Running the game" below), **Escape** quits.

## The `Game` trait and the main loop

Every game implements `engine::app::Game`:

```rust
trait Game {
    fn init(&mut self, ctx: &mut Context) -> anyhow::Result<()> { Ok(()) }
    fn handle_event(&mut self, ctx: &mut Context, event: &sdl2::event::Event) {}
    fn update(&mut self, ctx: &mut Context, dt: f32) -> anyhow::Result<()>;
    fn render(&mut self, ctx: &mut Context) -> anyhow::Result<()>;
}
```

`App::run("Title", width, height, YourGame::new())` owns the window, event
pump, and timing, and calls into your `Game` impl each frame in this order:
drain SDL events (`handle_event` per event) → `update(dt)` → `render()` →
swap buffers. `Context` gives you `ctx.gl()` (the `glow::Context`), `ctx.input`
(keyboard/mouse state), `ctx.time` (delta/elapsed), and `ctx.drawable_size()`.
Set `ctx.should_quit = true` to exit.

See `games/sandbox/src/main.rs`'s `Sandbox` struct for a complete example.

## Scene basics: ECS

The engine uses `hecs` (re-exported as `engine::ecs`). A `World` holds
entities; each entity is a bundle of components. The two you'll use constantly:

```rust
world.spawn((
    Transform::from_position(Vec3::new(0.0, 0.0, 0.0)),
    MeshRenderer { mesh: Arc::new(gpu_mesh), texture: Some(Arc::new(gpu_texture)) },
));
```

- `Transform { position, rotation, scale }` — `.matrix()` gives the model matrix.
- `MeshRenderer { mesh: Arc<GpuMesh>, texture: Option<Arc<GpuTexture>> }` — `Arc`
  so one loaded mesh/texture can back many entities without re-uploading.
- `Camera` exists as a data shape for future camera work driven off the ECS;
  the sandbox currently drives the camera via a standalone `OrbitCamera`
  (simpler for a single-camera game) rather than an ECS-driven one.
- `Light { color, intensity, kind: LightKind::{Directional{direction},
  Point{range}} }` — the directional sun/ambient light is still a hardcoded
  uniform, but `Point` lights are real, placeable ECS entities; see "Point
  lights" below.
- `LevelObjectMeta { name, mesh_source, texture_path, rotation_euler_deg }` —
  tags an entity as a level-editor-placed object and carries enough info to
  serialize it back into a `LevelObject` (see "Level editor" below). Only
  entities you want listed/editable/saved in the level editor need this;
  plain `Transform`/`MeshRenderer` entities work fine without it.
- `PlayerController { move_speed, jump_speed, interact_radius,
  interact_impulse }` — tags the player entity spawned in Play mode (see
  "Physics" and "Running the game" below). Plain tunable fields.

Load meshes/textures with `engine::mesh::load_obj(path)` (returns one
`MeshData` per OBJ sub-object) and `engine::mesh::GpuMesh::upload(gl, &data)`,
and `engine::texture::GpuTexture::load_from_file(gl, path, TextureFilter::Nearest)`.

Draw by querying: `world.query::<(&Transform, &MeshRenderer)>().iter()`.

## Camera

`engine::camera::OrbitCamera::new(target, distance)` — orbits a point via
`.orbit(delta_yaw, delta_pitch)` and `.zoom(delta)`; `.view_matrix()` and
`.projection_matrix(aspect)` feed straight into your shader uniforms.

## The render pipeline

Each frame: `Renderer::begin_scene` binds a **low-resolution offscreen
framebuffer** (sized `window_size * resolution_scale`) and draws your scene
into it — this is where the vertex shader applies lighting/fog/vertex-snapping
and where the compiled shader variant (affine vs. perspective-correct UVs) is
selected. `Renderer::present` then blits that small texture up to the full
window with `GL_NEAREST` filtering (the actual pixelation) through a composite
shader that layers on posterize + ordered-dither. The editor GUI is drawn
*after* `present`, directly at full window resolution, so the UI itself always
stays crisp regardless of how retro the 3D looks.

### `RenderParams` fields

| Field | Effect |
|---|---|
| `resolution_scale` | Internal render resolution as a fraction of the window (lower = blockier) |
| `fog_color` / `fog_start` / `fog_end` | Linear distance fog blended in per-vertex |
| `color_levels` | Posterize steps per color channel in the composite pass |
| `dither_strength` | Ordered-dither blended into the posterize step (breaks up banding) |
| `lighting_mode` | `Unlit` or `VertexLit` (Lambertian, computed per-vertex — flat "faceted" look) |
| `ambient_color` | Added to lit surfaces so shadowed faces aren't pure black |
| `vertex_snap_amount` | Clip-space grid size vertices are quantized to (0 = off); PS1-style geometric wobble |
| `affine_texture_mapping` | Toggles `noperspective` UV interpolation — the classic warped-texture look |
| `texture_filter` | `Nearest` (crisp/blocky) or `Bilinear` (smoothed) |
| `backface_culling` | Skips drawing a triangle's interior-facing side — see below |

**`backface_culling`**, when on, wraps just the opaque mesh loop in
`Sandbox::render` with `gl.enable(CULL_FACE)`/`gl.cull_face(BACK)` (off
again before particles draw — skybox and particles are unaffected either
way). For a closed mesh this changes nothing visible from a normal
distance — the front face already occludes the back one via the depth
buffer — but it's what keeps the camera from rendering a wall's or
object's *interior* surface when it clips very close to or slightly
inside solid geometry (the simple collision resolver in `engine::physics`
doesn't guarantee zero interpenetration, so this happens in practice).
`#[serde(default)]` (`false`) so profiles saved before this field existed
keep their exact old look; the checkbox lives in the F1 panel's
**Geometry / Texturing** section, right under "Affine texture mapping".

## Shader profiles

A `ShaderProfile` (in `engine::profile`) bundles shader file paths + a
`RenderParams`. They're saved as human-editable `.ron` files under a game's
`profiles/` folder — open one in a text editor and change numbers directly if
you don't want to use the GUI. On first run, if a game's `profiles/` folder is
empty, `games/sandbox/src/main.rs`'s `default_demo_profiles()` writes out three
starter looks (`ps2_classic`, `ps1_wobble`, `clean_lowpoly`) so there's always
something to load and cycle through — copy that pattern (or just copy the
generated `.ron` files) into a new game.

`engine::profile::ProfileCycler` manages a loaded set: `.next()`/`.prev()` for
hotkey cycling, `.add()` for a runtime "save as new profile", `.select(i)` for
GUI list clicks. `load_dir`/`load_from_file`/`save_to_file` handle the RON I/O.

## The editor GUI

`engine::ui::EguiState` bridges SDL2 input to `egui` and paints it via
`egui_glow` — any game gets this by owning one `EguiState`, forwarding every
SDL event to `handle_event`, and wrapping its panel code in `.run(...)` /
`.paint(...)` once per frame (see `Sandbox::render` for the exact pattern,
including gating camera-drag and hotkeys on `ui.wants_pointer_input()` /
`ui.wants_keyboard_input()` so interacting with either panel doesn't also spin
the camera or trigger Tab/F5). `engine::ui::render_params_editor(ui, &mut
params)` draws sliders/checkboxes for every `RenderParams` field and is
reusable as-is in any game.

The sandbox shows two independent panels, each toggled by its own hotkey:
a right-hand **shader profile panel** (F1 — profile list, Save/Save-As-New)
and a left-hand **level editor panel** (F2 — described next).

## Level editor

A `Level` (in `engine::level`) is a named list of `LevelObject`s — name, mesh
source, optional texture, and a position/rotation/scale — saved as a
human-editable `.ron` file under a game's `levels/` folder, the same
auto-bootstrap-if-empty convention as shader profiles: on first run
`games/sandbox/src/main.rs`'s `default_level()` writes the classic 3-cube demo
scene to `levels/default.ron` so there's always something to load.

A `LevelObject`'s mesh is either a built-in **primitive** (`Cube`/`Plane`,
generated procedurally by `engine::mesh::primitives` and uploaded once, then
shared via `Arc<GpuMesh>` across every instance — scale is applied through the
`Transform`, not by regenerating geometry) or an **`ObjFile`** path (only the
first sub-mesh of a multi-object OBJ is used per level object — author
multi-part models as separate level objects instead). Texture/OBJ paths are
stored relative to the game's asset root when possible
(`engine::level::relativize`), the same portability fix applied to shader
profile paths, so saved levels don't break when the game is shipped to another
machine.

The **F2 panel** (`Sandbox::draw_level_editor_ui`/`draw_selected_object_ui` in
`games/sandbox/src/main.rs`) gives you:

- **Save** / **Save As New** / a clickable list of saved levels to load —
  same pattern as the shader-profile panel.
- **Add Cube** / **Add Plane** — spawns a primitive at a simple
  non-overlapping default position, auto-selected.
- **Import Model as Object...** — opens a native file picker (`rfd`) for an
  `.obj`, then a second picker for an optional texture (a solid white 1×1
  fallback texture is used if you cancel), and adds it as a **new** object
  — unlike early versions of this button, it no longer clears the scene.
- A **scene outliner** (every entity with a `LevelObjectMeta`) — click a name
  to select it.
- For the selected object: an editable name, **Position**/**Rotation**/
  **Scale** as drag-value fields (click and drag left/right to change, or
  click once to type an exact number), **Assign Texture...**, and **Delete**.
  Rotation is edited as Euler degrees and only converted to the `Transform`'s
  quaternion on change (`engine::glam`'s `Quat::from_euler`), so repeated
  edits don't drift the value the way a naive Euler↔quaternion round-trip
  would.

There's no 3D transform gizmo or click-to-select-in-viewport — this is
deliberately a *simple* editor. Vertex/face-level mesh editing is also out of
scope; "editing geometry" here means placing/transforming whole objects built
from primitives or imported meshes.

Each object also has a **Dynamic Rigid Body** checkbox (see "Physics" below)
— checking it inserts a `RigidBody` on the live entity so it falls/collides
in Play mode; unchecking it removes the component, making the object static
again. New primitives default sensibly (`Cube` → dynamic, `Plane`/imported →
static) but every object is overridable.

## Point lights

Beyond the single hardcoded directional light (sun + ambient), the engine
supports up to **4 point lights** per scene, placed and edited right in the
level editor. A `LevelLight` (in `engine::level`) is a name, position, color,
intensity, and range, saved as part of a `Level`'s `lights` list — same RON
file, same portability rules as everything else in the level.

A placed light is an entity like any other: `Transform` + `Light` +
`LevelObjectMeta` (`Sandbox::spawn_level_light`), but with no `MeshRenderer` —
lights are invisible in both Edit and Play mode, there's no gizmo/billboard
marking their position (a known simplification; move the object and watch the
lighting change if you need to find it). Because it carries `LevelObjectMeta`
like a mesh object does, a light shows up in the **same F2 scene outliner**
as everything else and can be renamed/repositioned/deleted the same way;
`draw_selected_object_ui` checks for a `Light` component and, if present,
dispatches to `draw_selected_light_ui` instead of the mesh-object fields.

**F2 panel**: **Add Light** (next to Add Cube/Add Plane) drops a white,
intensity-1.0, range-5.0 light at the origin, auto-selected. The selected-light
panel gives you Position, a color picker, an Intensity slider (0–5), and a
Range slider (0.5–30).

Rendering: `mesh.vert`/`mesh.frag` declare fixed-size arrays —
`uPointLightPos[4]`, `uPointLightColor[4]`, `uPointLightIntensity[4]`,
`uPointLightRange[4]`, `uPointLightCount` — a plain uniform array rather than
a UBO/SSBO, which is all a handful of level lights need. Each frame,
`Sandbox::render` collects up to 4 `(Transform, Light)` entities (extra
lights past 4 are silently ignored — keep scenes within the budget) and
uploads them; the vertex shader sums each light's contribution into the
existing per-vertex Lambertian term using simple linear-falloff attenuation
(`clamp(1 - distance/range, 0, 1)`), so point lights only affect
`VertexLit`-mode surfaces (same as the directional light — `Unlit` mode
ignores all lighting).

`build_level_from_ecs` gathers lights back out with a
`.without::<&Light>()` filter on the object query (so a light entity, which
also carries `LevelObjectMeta`, doesn't get double-counted as a mesh object)
and a separate `(&Transform, &LevelObjectMeta, &Light)` query for the
`lights` list.

## Physics

`engine::physics` is a small (~150 line), deliberately simple/readable
simulation — the goal is an *easily editable* physics engine, not a
feature-complete one:

```rust
pub struct PhysicsParams { pub gravity: f32, pub linear_damping: f32 }
pub struct RigidBody { pub velocity: Vec3, pub grounded: bool }
pub enum ColliderShape { Sphere { radius: f32 }, Aabb { half_extents: Vec3 } }
pub struct Collider { pub shape: ColliderShape, pub is_trigger: bool }
pub fn step(world: &mut hecs::World, dt: f32, params: &PhysicsParams) -> Vec<(Entity, Entity)>;
```

An entity with a `Collider` but **no `RigidBody` is static** — collided
against, but never moved or integrated. That's a deliberate choice: whether
an object is dynamic is just "does it have a `RigidBody`," one thing to keep
in sync, not a separate flag that could disagree with it. `step` (called
once per frame from `Sandbox::update`, only in Play mode) integrates gravity
+ damping + position for every dynamic body, then resolves overlaps by
pushing dynamic bodies out along the axis of least penetration (simple
AABB-AABB / AABB-Sphere / Sphere-Sphere tests).

Known simplifications, worth knowing before you lean on this for something
demanding: axis-aligned boxes only (no rotation/torque on colliders), and one
resolution pass per frame (an object resting against two overlapping
surfaces can jitter slightly rather than settle perfectly). Fine for a level
built from boxes/planes and a player sphere; not a general-purpose solver —
swap in `rapier3d` or similar if you outgrow it.

`spawn_level_object` in `games/sandbox/src/main.rs` derives each object's
`Collider` from its `Transform::scale` (an approximate box, not an exact
mesh-fitted bound — fine for boxy low-poly levels). `PhysicsParams` lives on
the currently-loaded `Level` (`Level::physics`), so gravity/damping save,
load, and revert together with everything else — edit them live under
**Physics** in the F1 panel.

## Audio

`engine::audio::AudioContext` wraps `rodio` (with its `symphonia-all`
feature, which already decodes WAV/OGG/MP3/FLAC — no extra dependency
needed) behind a small, purpose-specific API:

```rust
pub fn new() -> anyhow::Result<Self>;                       // fails if no output device
pub fn play_sfx_file(&self, path: &Path) -> anyhow::Result<()>;   // one-shot, fire-and-forget
pub fn play_tone(&self, frequency_hz: f32, duration_secs: f32);   // procedural sine blip, no file
pub fn play_music_file(&mut self, path: &Path, looped: bool) -> anyhow::Result<()>;
pub fn stop_music(&mut self);
pub fn set_music_volume(&mut self, volume: f32);
```

`play_tone` needs no asset at all (a `SineWave` source built on the fly) —
handy while prototyping a jump/interact/UI sound before a real asset
exists, and what the jump/push demos still use. `play_sfx_file`/
`play_music_file` decode a real file via `rodio::Decoder`; looped music is
fully decoded into an in-memory `SamplesBuffer` and repeated (fine for
typical track lengths, not streamed). `Sandbox.audio: Option<AudioContext>`
degrades silently if `AudioContext::new()` fails (no output device) —
every call site checks `Some` first, so a game runs fine, just silent, on
a machine with no audio hardware.

**Scripts and native `Behavior`s** reach SFX through the same `ScriptApi`
trait as everything else: `play_sfx(path)` (a script call) /
`ScriptApi::play_sfx(&mut self, path: &str)` (native), resolved relative to
the asset root the same way script/texture paths are. See
`bob_demo.pss`'s `interact()`, which calls both `play_tone` and
`play_sfx("sfx/demo_blip.wav")` side by side — procedural and
sample-based, from the same script.

**Background music is level-wide**, not per-object: `Level.music_path:
Option<PathBuf>` (`#[serde(default)]`). `apply_level` starts it looping
whenever a level loads (or calls `stop_music()` if `None`, so switching to
a level without music doesn't leak the previous track), and
`Sandbox.current_music_path` mirrors it so `build_level_from_ecs` writes
it back out. **F2 panel**: the **Level** section gains "Assign Music..."
(same `rfd` file-picker pattern as "Assign Texture..."), "Stop Music", and
a volume slider — `Sandbox.music_volume` persists across level loads since
`play_music_file` itself has no volume parameter.

**Demo content**: no audio assets shipped in the repo, and pulling in an
encoder crate just for placeholder sounds isn't worth a new dependency —
`games/sandbox/src/main.rs`'s `write_wav`/`generate_blip_samples`/
`generate_ambient_samples` hand-roll a minimal mono 16-bit PCM WAV file
instead (a short faded sine blip, and a three-note chord sized to loop
with no click — every frequency completes a whole number of cycles within
the loop duration). Generated once into `sfx/demo_blip.wav`/
`music/demo_ambient.wav` if missing, same bootstrap-if-empty convention as
profiles/rigs/classes/levels.

## Trigger volumes

A `Collider` can be marked `is_trigger: true` — it's never physically solid:
overlapping it never pushes anything out or zeroes velocity. Instead,
`physics::step` returns every `(dynamic_entity, other_entity)` pair that
overlapped that frame, trigger or not, so it's up to the caller to act on the
ones that matter.

`Sandbox::update` (Play mode only) filters that list down to pairs involving
`player_entity`, diffs the current frame's set against a stored
`trigger_overlaps: HashSet<(Entity, Entity)>` from the previous frame, and
calls `on_trigger_entered`/`on_trigger_exited` for whatever newly
appeared/disappeared. Those two methods are the extension point — the demo
behavior logs the trigger's name and plays a tone (same `AudioContext` used
for jump/interact), showing triggers + physics + audio working together.
Replace them with your own game's trigger logic (checkpoints, level
transitions, damage zones, ...).

`LevelObject.is_trigger` (`#[serde(default)]`, so old levels still load as
non-triggers) saves/loads the flag with the level. **F2 panel**: an
**"Is Trigger (non-solid)"** checkbox next to "Dynamic Rigid Body" on any
selected object, and an **"Add Trigger Zone"** button (a `Cube`,
`is_trigger: true`, static by default) alongside Add Cube/Add Plane/Add
Light. A trigger renders as a solid bright cyan box
(`solid_color_texture`/`TRIGGER_COLOR` in `games/sandbox/src/main.rs`)
instead of its assigned/white texture, in both Edit and Play mode, so it
stays visually identifiable — true see-through would need alpha blending,
which the render pipeline doesn't have yet (a known simplification).

## Animation

`engine::animation` gives level objects procedural motion — two kinds, not a
full keyframe/timeline system:

```rust
pub enum AnimationKind {
    Orbit { axis: Vec3, speed_deg_per_sec: f32 },          // continuous rotation
    Bob { axis: Vec3, amplitude: f32, period_secs: f32 },  // sinusoidal position offset
}
pub struct Animator { pub kind: AnimationKind, pub base_position: Vec3, pub base_rotation: Quat, pub elapsed: f32 }
pub fn step(world: &mut hecs::World, dt: f32);
```

`Animator` recomputes `Transform` from a **fixed base pose** (`base_position`/
`base_rotation`) plus `kind` and `elapsed` every call — it never mutates the
transform incrementally, so the motion can't drift. `Sandbox::update` calls
`engine::animation::step` unconditionally, in both Edit and Play mode — a
live preview while editing costs nothing extra and is a nice default (try
it: the shipped default level's "Orbiting Cube" spins continuously, visible
right away without entering Play mode).

An animated object with **no `RigidBody`** (the common case — decorative
motion, moving platforms) still pushes anything that touches it in Play
mode, with no special-casing needed: the physics collision loop already
checks every `Collider` regardless of `RigidBody` presence, so a static
animated object is a free kinematic mover.

`engine::level::AnimationSpec` mirrors `AnimationKind` (serializable) and
`LevelObject.animation: Option<AnimationSpec>` (`#[serde(default)]`) saves it
with the level. `spawn_level_object` attaches an `Animator` when present,
using the object's authored transform as the base pose. **Important:**
`build_level_from_ecs` reads `Animator::base_position` (not the live,
currently-oscillating `Transform::position`) for animated objects, so saving
mid-animation doesn't capture a random instant — rotation is unaffected
since it's already saved from `LevelObjectMeta::rotation_euler_deg`, which
animation doesn't touch.

**F2 panel**: an **Animation** dropdown (None/Orbit/Bob) on the selected
object, plus an Axis and the kind's parameters (Speed for Orbit; Amplitude/
Period for Bob). Switching **None → Orbit/Bob** captures the object's
*current* transform as the new base pose; switching **back to None** just
removes the `Animator`, leaving the object wherever it last was. Tweaking
axis/parameters *while already animating*, or switching between Orbit and
Bob, preserves the existing base pose rather than re-capturing the
(possibly mid-motion) live transform — otherwise every parameter edit would
make the object jump.

## Running the game (Play mode)

**F3** toggles between **Edit** mode (everything above — orbit camera, both
GUI panels, nothing simulated) and **Play** mode, which is meant to feel like
hitting Play in a normal game engine:

- The level is **snapshotted** before entering Play (`build_level_from_ecs`)
  and **restored exactly** on Stop (`apply_level` with that snapshot) — physics
  knocking objects around, or you pushing them, never corrupts your edits.
  **F3** always stops immediately, back to the editor. Plain **Escape**
  pauses instead of stopping (see "Pausing" below) while in Play, and quits
  the app while already in Edit mode.
- Both GUI panels hide automatically (F1/F2/Tab/F5/orbit-drag are gated to
  Edit mode) and a small "Play Mode" banner appears instead. The mouse is
  captured (SDL2 relative mouse mode) for unbounded FPS look, released again
  on Stop.
- An invisible player (a sphere `Collider` + `RigidBody` + `PlayerController`,
  no mesh) spawns above the level and falls onto whatever's there. The camera
  becomes `engine::camera::FirstPersonCamera`, positioned at the player's
  `Transform::position` plus a fixed eye-height offset each frame.
- **WASD** moves relative to look direction, **mouse** looks freely, **Space**
  jumps when `RigidBody::grounded` is true, **E** interacts. A connected
  **gamepad** blends in automatically — see "Gamepad support" below.

### Pausing

**Escape** while playing pauses instead of stopping — `Sandbox::pause`/
`resume` flip a `paused: bool` (only meaningful while `mode ==
EditorMode::Play`) and release/re-capture the mouse so a real cursor can
click the pause menu's buttons (relative mouse mode otherwise swallows
clicks as look-deltas). `engine::ui::draw_pause_menu` — a small, stateless
function in the same spirit as `draw_hud`, just returning whichever button
was clicked — draws a centered **Resume** / **Exit to Editor** / **Quit
Game** window.

Pausing is a real freeze, not just an input lock: `Game::update`'s
"always on, live preview" block (`animation::step`, rig clip stepping,
`particles::step`, `hud.tick`) — which normally *also* runs in Edit mode
so editing has a live preview — skips entirely while `mode == Play &&
paused`, so particles/rig poses/HUD toast timers actually stop rather
than keep animating behind the menu. The frozen HUD and world are still
drawn, though, so pausing looks exactly like hitting pause, not like the
screen going blank. F9 checkpoint-save, **E** interact, and the gamepad
interact button are all disabled while paused, alongside the existing
Play-mode-only gating.

**Exit to Editor** calls the same `exit_play_mode` F3 already uses (player
despawned, pre-Play snapshot restored). **Quit Game** sets
`ctx.should_quit`. Both are here so a pause menu built this way behaves
like a real shipped game's, not just a debug convenience. In a release
build there's no editor to exit to, so that button becomes **"Restart
Level"** instead (`Sandbox::restart_level`) — see "Exporting a game"
below for exactly when that switch happens.

A forced title/start screen before Edit mode was considered and
deliberately left out: a debug build boots straight into the dev-time
level editor by design, and a menu you have to click through on every
launch would fight that workflow (a release build already skips it,
straight to Play — see "Exporting a game" below). The pause menu
demonstrates the same reusable `engine::ui` modal-overlay pattern a title
or game-over screen would reuse — add one the same way once a game built
on this engine actually needs it.

**This is the place to add your own game's input** — two methods on
`Sandbox` in `games/sandbox/src/main.rs`, both heavily commented as the
extension points:

- `update_player_input` (called from `Game::update`, Play mode only) — reads
  `ctx.input`/mouse-delta every frame and drives continuous movement/look.
  Swap the movement scheme, add sprint/crouch, bind new keys, etc. here.
- `interact` (bound to `KeyDown(E)` in `handle_event`) — the example
  "interact with the environment" hook: finds the nearest entity with a
  `RigidBody` within `PlayerController::interact_radius` and pushes it with
  an outward impulse (`PlayerController::interact_impulse`), tying the
  physics engine and the input hook together in one visible action. Replace
  this with your own game's interaction (pickup, dialogue, opening a door,
  ...) — it's a one-shot action, not a per-frame poll, so it's the natural
  place for anything triggered by a single keypress.

## Gamepad support

`engine::input::Input` mirrors its keyboard down/just-pressed/just-released
pattern for a controller: `left_stick()`/`right_stick() -> (f32, f32)` (each
axis `-1.0..=1.0`, deadzoned), `controller_button_down`/
`controller_just_pressed(sdl2::controller::Button) -> bool`. Only **one**
controller is tracked — the first one connected — fine for a single local
player; a second one plugged in while the first is still connected is
ignored.

`Platform` owns the `GameControllerSubsystem` (must stay alive for opened
controllers to remain valid); `Input::handle_event` takes it as a second
argument so it can open a `GameController` on `Event::ControllerDeviceAdded`
and drop it on `..Removed`, tracking axes/buttons the same way keyboard
state is tracked. No extra `sdl2` Cargo feature is needed — controller/
joystick support is already in the base crate, with a built-in mapping
database that recognizes common Xbox/PlayStation-layout pads via XInput on
Windows out of the box.

In the sandbox, `update_player_input` blends the **left stick** additively
with WASD (whichever is actually being pushed drives movement — note this
means the stick's analog magnitude is *not* preserved, since keyboard input
is a normalized unit vector; a known simplification) and the **right
stick** additively with mouse-look. **A** jumps (same as Space), **X**
interacts (same as E).

**Caveat, stated plainly**: unlike every other feature in this guide, this
one could not be end-to-end tested with real hardware in the environment
this was built in (no physical controller, no virtual joystick driver) — it
was built carefully against SDL2's documented event/API surface and the app
was confirmed to start up cleanly with the subsystem initialized, but
**please confirm on real hardware** before relying on it.

## Scripting

A small custom scripting language — `.pss` files ("PS2 Script") — lets you
attach programmable behavior to a level object without recompiling the
engine. It's deliberately minimal: a hand-written lexer/parser/tree-walking
interpreter (`engine::script`), not an embedded crate like Lua/Rhai, and
not a general-purpose language — just enough to write per-object game
logic.

### Language

```
let phase = 0.0;          // top-level `let` = the script's persistent state
let amplitude = 0.4;       // ("fields" that survive across separate calls)

fn ready() { }             // called once when the script is attached/loaded
fn update(dt) { }          // called every frame in Play mode
fn interact() { }          // called when the player interacts (E / gamepad X)
fn trigger_enter(other) { }// called when something enters this trigger
fn trigger_exit(other) { } // called when something exits this trigger

fn update(dt) {
    phase = phase + dt;                  // assignment (no `let`) updates an
    if phase > 10.0 { phase = 0.0; }     // existing variable, local or global
    let wave = sin(phase);               // function-local `let`
    move_by(0.0, wave * amplitude, 0.0);
}
```

Keywords: `let fn if else while return true false nil and or not`. Operators:
`+ - * / % == != < <= > >=`, `//` line comments. Built-in math (no host
needed): `sin cos abs sqrt min max floor`. There are no closures, classes,
or arrays — a script is one flat set of globals plus top-level functions,
which is all the hooks above need. All five hook functions are optional; a
script that doesn't define e.g. `interact` just does nothing when
interacted with.

**Standard library** (everything a script can call besides built-in math):
`log(msg, ...)`, `get_x()`/`get_y()`/`get_z()`, `set_position(x,y,z)`,
`move_by(dx,dy,dz)`, `play_tone(freq,duration)`, `time()`,
`hud_bar(name, fraction)`, `toast(message, seconds)`. Position is three
scalar calls rather than one call returning a triple — the language has no
vector/tuple type. `hud_bar`/`toast` drive the in-game HUD (see "In-game
HUD" below) — a script can update a named progress bar or pop a fading
toast the same way a native `Behavior` can.

### `Behavior`/`ScriptApi` — using Rust the same way a script would

```rust
// engine::behavior
pub trait ScriptApi {
    fn log(&mut self, message: &str);
    fn position(&self) -> (f32, f32, f32);
    fn set_position(&mut self, x: f32, y: f32, z: f32);
    fn move_by(&mut self, dx: f32, dy: f32, dz: f32);
    fn play_tone(&mut self, frequency_hz: f32, duration_secs: f32);
    fn elapsed(&self) -> f32;
    fn set_hud_bar(&mut self, name: &str, fraction: f32);
    fn show_toast(&mut self, message: &str, seconds: f32);
}
pub trait Behavior: Send + Sync {
    fn on_ready(&mut self, api: &mut dyn ScriptApi) { }
    fn on_update(&mut self, api: &mut dyn ScriptApi, dt: f32) { }
    fn on_interact(&mut self, api: &mut dyn ScriptApi) { }
    fn on_trigger_enter(&mut self, api: &mut dyn ScriptApi, other_name: &str) { }
    fn on_trigger_exit(&mut self, api: &mut dyn ScriptApi, other_name: &str) { }
}
pub struct BehaviorSlot(pub Box<dyn Behavior>);  // the ECS component
```

`ScriptBehavior` implements `Behavior` by running a compiled `.pss` script
against these same five hooks. Crucially, **a plain Rust type can implement
`Behavior` directly** — see `engine::behavior::NativeBobBehavior` — and gets
attached to an entity via the exact same `BehaviorSlot` component. The game
dispatches through the `Behavior` trait uniformly and never needs to know
whether a given entity's logic came from a script or from compiled Rust.
That's the whole point: **prototype as a script, port to native Rust later
for performance or complex logic, and nothing about how it's wired into the
game has to change** — same trait, same hooks, same attach point.

The default level's **"Script Demo Cube"** (running `scripts/bob_demo.pss`)
and **"Native Behavior Cube"** (running `NativeBobBehavior`, spawned in
`Sandbox::spawn_native_behavior_demo`) stand side by side and do the
identical sine-bob motion + interact log/tone — one from a script, one from
compiled Rust — to make the parity concrete. Note the native one is a
compiled-in demo, not saved level data (`NativeBehaviorDemoMarker` tells
`build_level_from_ecs` to skip it) — that's the actual tradeoff: a script
can be authored, saved, and reloaded as level data; a native `Behavior` has
to be wired up in code. Use a script when you want data-driven, editable
logic; port to native Rust when a script becomes a performance bottleneck
or needs something the language can't express.

### Runtime wiring (`games/sandbox/src/main.rs`)

- `LevelObject.script: Option<PathBuf>` points at a `.pss` file;
  `spawn_level_object` compiles it and attaches a `BehaviorSlot` if present
  (a parse/read failure is logged and the object still spawns, just without
  behavior — same tolerance as a missing texture falling back to white).
- `Sandbox::with_behavior(entity, |behavior, api| ...)` is the one place
  that bridges `Behavior`/`ScriptApi` to the ECS: it fetches
  `(&mut Transform, &mut BehaviorSlot)` for one entity via the same
  "combined tuple `query_one`" idiom used everywhere else in this file, and
  builds a `ScriptApi` scoped to just that entity.
- `on_update` is dispatched for every `BehaviorSlot` entity once per frame,
  **Play mode only** (like physics and player input — a script's `update`
  is generally written assuming the game is actually running).
  `on_interact` extends the existing `interact()` hook: a nearby entity
  with a `BehaviorSlot` gets its own `on_interact` called *instead of* the
  generic "push it" demo. `on_trigger_enter`/`on_trigger_exit` extend the
  existing trigger log+tone the same way, additively.

### F2 panel

The selected object's panel gains a **Script** section: the current path
(or "None"), **Attach/Change Script...** (a `.pss` file picker),
**Reload** (re-reads and re-compiles from disk, re-firing `ready`), an
**Edit Inline** checkbox that reveals a monospace text box + **Save**
(writes the file and reloads), and **Open in Script Editor**, which
launches the standalone editor below pointed at the file.

### The standalone editor (`tools/script_editor`)

A separate app — build/run it with `cargo run -p script_editor [path]`.
It reuses `engine::app`/`engine::ui` (the same `Platform`/`EguiState` as the
game) rather than a different windowing stack, and draws no 3D content at
all — proof those pieces of `engine` are reusable outside a "real" game.
New / Open / Save / Save As, a full-window monospace text box, and
**Check Syntax**, which calls `engine::script::Interpreter::compile`
directly (no `Host`/`ScriptApi` needed — it only type-checks, it can't run
a script against a game) and reports every parse error with a line number.
No token syntax-highlighting yet — plain text editing, noted here as a
known simplification, easy to add later via a custom egui `LayoutJob`.

The F2 panel's "Open in Script Editor" button launches this as a sibling
process (`std::process::Command`), assuming `script_editor(.exe)` sits next
to the game's own executable — true when both are built from this
workspace's `target/{profile}/` folder, which is a dev-time convenience
only; it's not something a shipped game needs to carry.

## Character rigs

`engine::rig` gives level objects skeletal-*ish* animation using the classic
PS1/PS2-era technique: a character is several separate rigid meshes (torso,
head, arms, legs) parented in a hierarchy, each moving as a whole — **not**
per-vertex skin deformation. Joints don't bend smoothly (a raised arm is a
rigid box swinging from its shoulder, not a soft-deforming sleeve); that's
an inherent limitation of the technique, not a bug, and it's why this is
the right-sized approach for a lightweight PS2-style engine rather than
importing pre-skinned glTF characters or building an in-engine
weight-painting tool.

### Data shapes

```rust
// engine::rig — a reusable character definition, its own `rigs/*.ron` files
pub struct RigPartDef { pub name: String, pub parent: Option<String>, pub mesh: MeshSource, pub texture_path: Option<PathBuf>, pub local_position: [f32; 3], pub local_rotation_euler_deg: [f32; 3], pub scale: [f32; 3] }
pub struct Keyframe { pub time: f32, pub rotation_euler_deg: [f32; 3] }
pub struct JointTrack { pub joint_name: String, pub keyframes: Vec<Keyframe> }
pub struct RigClip { pub name: String, pub duration: f32, pub looping: bool, pub tracks: Vec<JointTrack> }
pub struct RigAsset { pub name: String, pub parts: Vec<RigPartDef>, pub clips: Vec<RigClip> }

// runtime ECS components
pub struct RigPart { pub parent: Option<Entity>, pub local_position: Vec3, pub local_rotation_euler_deg: Vec3, pub local_rotation: Quat }
pub struct Rig { pub parts_by_name: HashMap<String, Entity> }       // lives on the root entity
pub struct RigAnimator {
    pub clips: Vec<RigClip>, pub current_clip: Option<usize>, pub time: f32, pub playing: bool, pub speed: f32,
    // blend-out state for whatever clip `current_clip` just replaced — see "Clip blending" below
    pub previous_clip: Option<usize>, pub previous_time: f32, pub blend_elapsed: f32, pub blend_duration: f32,
}
```

A rig part is just an entity with a `Transform` + `MeshRenderer`, exactly
like any other level object — the render loop needed **zero changes** for
this feature. The only new problem was computing each part's *world*
transform from its parent chain: `RigPart::local_position`/
`local_rotation` are the authoritative local pose (set at spawn, hand-posed
in the F2 panel, or driven by clip playback); `Transform` is treated as
pure output for a rigged entity, overwritten every frame — the same
pattern `Animator` already uses for `base_position`/`base_rotation` vs. the
live `Transform`.

Two systems, both run unconditionally every frame from `Game::update`
(Edit and Play mode, same "live preview" philosophy as
`engine::animation::step`):

- `step_rig_animation` advances each playing `RigAnimator`'s clock
  (wrapping or clamping against the clip's `duration`) and **slerps**
  between a track's two bracketing keyframes (not raw Euler lerp — avoids
  gimbal artifacts), writing the sampled rotation into the named part's
  `RigPart::local_rotation`. It's gated on `playing` specifically:
  pausing lets you hand-pose a part via the F2 panel without the sampler
  immediately overwriting it — that hand-pose *is* the keyframe-authoring
  workflow.
- `update_world_transforms` resolves every `RigPart`'s world
  position/rotation via a memoized parent-first recursive walk
  (`parent_world_pos + parent_world_rot * local_pos`, `parent_world_rot *
  local_rot`; a root's world pose *is* its local pose) and writes it into
  `Transform`.

### The F2 workflow

The level editor panel gains a **Rigs** section, below Scene Objects:

- **New Rig** (name + button) creates a fresh `RigAsset` with one root
  part named "Root", saves it to `rigs/<name>.ron`, and places an instance
  immediately so there's something to build on.
- **Add instance of: \<rig name\>** places another instance of an
  already-loaded rig asset.
- Clicking a placed instance's name opens its panel: **Root Local
  Position/Rotation** (editing `RigPart` directly, *not* `Transform` —
  same reasoning as animated objects, since `update_world_transforms`
  overwrites `Transform` every frame), a **Clips** list with **New
  Clip...**, **Play/Pause**, a **Loop** checkbox, a **Speed** slider, and a
  **Time (scrub)** slider (dragging it pauses playback, since scrubbing
  against a running clip is useless), a **Parts** list, **Add Part...**
  (always a `Cube` primitive — resize/retexture afterward like any other
  primitive; picks a parent from the rig's existing part names, falling
  back to the root if the name doesn't match one), **Save Rig**, and
  **Delete Rig Instance**.
- Selecting a part shows *its* Local Position/Rotation and a **Set
  Keyframe Here** button that upserts `(current scrub time, current local
  rotation)` into the selected clip's track for that part. The whole
  authoring loop is: pause the clip, scrub to a time, pose a part by hand
  with the drag-values, click **Set Keyframe Here**, scrub to the next
  time, repeat, **Save Rig**.

`engine::level::RigInstance` (a *placement*: name, `rig_path`, position,
rotation, which clip is playing) is a sibling list on `Level` —
`Level.rig_instances`, the same pattern as `Level.lights` — since a rig is
structurally a multi-entity hierarchy, not a single mesh like
`LevelObject`. No per-instance scale: scale is authored per-part inside
the rig asset itself, since scaling only the root's own mesh (not the
whole hierarchy's offsets) would be more confusing than useful.

### Clip blending

Switching clips instantly (cutting `current_clip` and `time` straight to
the new values) makes every part's pose jump on the very next frame — a
visible pop, worst on a part mid-swing. `RigAnimator::play_clip(index)`
avoids that by snapshotting whatever was playing (`previous_clip`,
`previous_time`) before switching, then blending: for `blend_duration`
seconds (default 0.2), `step_rig_animation` samples **both** the old
clip (frozen at `previous_time` — it isn't advancing anymore, just
providing a fixed starting pose) and the new clip (advancing normally)
per track and `Quat::slerp`s between them by `blend_elapsed /
blend_duration`. Once the blend window elapses, `previous_clip` is
cleared and the part reads purely from the new clip. Calling `play_clip`
with the already-current clip index is a no-op — nothing to blend from
itself.

The F2 panel's clip list routes through `play_clip` when you click a
different clip, so hand-testing an authored transition always gets the
blend; nothing else needed it (the "Time (scrub)" slider still snaps
`time` directly within one clip — scrubbing is meant to jump exactly
where you drag it, not ease into it). Since a full blend finishes inside
0.2 seconds, verifying the interpolation visually frame-by-frame isn't
practical — see `engine::rig`'s `play_clip_blends_smoothly_instead_of_popping`
unit test for the actual verification: it steps the simulation at fixed
`dt`s and asserts the sampled rotation sits at the old pose right at the
switch, at the slerp midpoint halfway through the blend window, and
exactly at the new pose once it's elapsed.

## Object classes

`engine::class` lets many placed objects share one definition — a
**class** — instead of duplicating mesh/texture/scale/physics/animation/
script settings on every instance by hand. It mirrors `engine::profile`'s
save/load pattern exactly, its own `classes/*.ron` folder:

```rust
pub struct ObjectClass {
    pub name: String,
    pub mesh: MeshSource,
    pub texture_path: Option<PathBuf>,
    pub scale: [f32; 3],
    pub is_dynamic: bool,
    pub is_trigger: bool,
    pub animation: Option<AnimationSpec>,
    pub script: Option<PathBuf>,
}
```

`LevelObject` gains `class: Option<PathBuf>`. When set, `spawn_level_object`
resolves mesh/texture/scale/dynamic/trigger/animation/script **from the
class** instead of the object's own copies of those fields — only
position/rotation/name stay per-instance. A `ClassMember` marker component
tags every class-spawned entity so the game can find them again later.

Editing a class does **not** live-update its instances automatically —
that would make "why did this object just change?" hard to reason about.
Instead, **Apply to All Instances** (F2 panel) is an explicit action: click
it after editing a class and every live `ClassMember` entity referencing
that class path is re-resolved from the new values. Until you click it,
existing instances keep whatever they were spawned with.

### F2 workflow

A **Classes** section (below Rigs) gives you: **New Class** (name +
button), a list of loaded classes with a **+ Instance** button per row
(spawns a new object referencing that class at a default position), and a
per-class editor (mesh/texture/scale/dynamic/trigger/animation/script —
the same fields the plain object editor exposes, plus **Save Class** and
**Apply to All Instances**).

Selecting a class-spawned object in Scene Objects shows a reduced panel —
Name/Position/Rotation plus "Class: `<name>`", **Edit Class** (jumps to it
in the Classes section), and **Unlink from Class** (clears `class`,
freezing the object's current resolved values as its own independent
copy — from then on it's a plain object again).

## Skybox

A simple vertical-gradient sky, drawn as `engine::renderer::SkyboxPass` —
no cubemap or dome mesh. It reuses the fullscreen-triangle trick already
in `post.rs`'s `CompositePass`: a single triangle covering the whole
screen via `gl_VertexID`, no vertex buffer needed. The fragment shader
reconstructs each pixel's world-space view ray from the inverse
view-projection matrix and mixes a horizon color and a zenith color by the
ray's Y component.

Drawn **first**, before any mesh, with `gl.depth_mask(false)` so it never
writes depth — normal opaque geometry drawn afterward occludes it exactly
like it would occlude a real sky. It renders into the same low-res
offscreen target as everything else, so it's pixelated/dithered along
with the rest of the scene rather than looking like a crisp UI overlay.

`RenderParams` gains `sky_horizon_color`/`sky_zenith_color: [f32; 3]`,
editable in the Render Params panel's new **Sky** section. Both are
`#[serde(default)]` (falling back to black) so profiles saved before this
feature still load — re-save them once to pick up the default blue
gradient.

## Particle effects

`engine::particles` is a small, non-instanced particle system — each live
particle is one draw call, deliberately simple and PS2-chunky rather than
built for thousands of particles at once (easy to swap for instancing
later if a scene needs that).

```rust
pub struct ParticleEmitterDef {
    pub rate_per_sec: f32,                      // 0 = burst-only, never auto-emits
    pub lifetime_min: f32, pub lifetime_max: f32,
    pub speed_min: f32, pub speed_max: f32,
    pub spread_deg: f32,                         // cone half-angle around +Y
    pub gravity_scale: f32,
    pub start_size: f32, pub end_size: f32,
    pub start_color: [f32; 4], pub end_color: [f32; 4],  // lerped by age
    pub max_particles: u32,
}
```

A `ParticleEmitter` runtime component owns a `Vec<Particle>` pool and a
tiny hand-rolled xorshift32 PRNG (seeded from the OS via
`std::hash::RandomState` — no `rand` dependency for something this small).
`engine::particles::step(world, dt) -> Vec<Entity>` ages/culls particles,
auto-emits for continuous emitters (`rate_per_sec > 0`), and returns
one-shot burst emitters (`rate_per_sec <= 0`) that have gone empty, so
`Game::update` can despawn them — this is how a triggered burst cleans
itself up without becoming permanent level clutter.

Rendering (`engine::renderer::ParticlePass`) draws each particle as a
camera-facing billboarded quad (`mesh::primitives::quad()`), rotation
taken from the camera's world-space orientation (extracted from the
inverse view matrix), with GL blending enabled around just this pass — the
**first thing in the engine to use blending** — so particles fade instead
of punching hard-edged holes in whatever's behind them.

Placed emitters live in `Level.particle_emitters`
(`LevelParticleEmitter { name, position, def }`), the same
sibling-list-on-`Level` pattern as lights/rig instances. The F2 panel gets
**Add Particle Emitter**; selecting one shows every `ParticleEmitterDef`
field as sliders/drag-values, a live **"Live particles: N"** count, a
**Test Burst (12)** button, and **Delete**. `Sandbox::spawn_burst_at` is
the transient-burst path used by `interact()`'s push demo — no
`LevelObjectMeta`, so it never shows up as level data, just particles that
age out and clean themselves up.

## In-game HUD

`engine::hud::HudState` is deliberately tiny — a title, named progress
bars, and fading toast messages:

```rust
pub struct HudState {
    pub title: Option<String>,
    pub bars: Vec<(String, f32)>,   // (name, fraction 0..=1)
    // toasts: private — go through set_bar/show_toast/tick
}
impl HudState {
    pub fn set_bar(&mut self, name: &str, fraction: f32);   // clamped 0..=1
    pub fn show_toast(&mut self, message: &str, seconds: f32);
    pub fn tick(&mut self, dt: f32);                        // ages/removes expired toasts
}
```

`engine::ui::draw_hud(ctx, hud)` renders it as a fixed top-left
`egui::Area` — title, each bar as an `egui::ProgressBar`, toasts stacked
below and fading out as their remaining time runs low. The sandbox calls
it from `Game::render`'s egui closure **gated to Play mode only** — the
one deliberate gap in the "editor panels only show in Edit mode" rule,
since a HUD is a gameplay thing with nothing to show while you're editing.
`Sandbox::update` only calls `self.hud.tick(dt)` inside the Play-mode
branch too, so a toast's timer doesn't run out while the game is paused in
Edit mode.

Both scripts and native `Behavior`s reach the HUD through the same
`ScriptApi` trait everything else in the scripting story uses —
`set_hud_bar`/`show_toast` are two more required methods alongside
`log`/`move_by`/etc., and `HostAdapter::call_native` forwards a script's
`hud_bar(name, value)`/`toast(message, seconds)` calls to them. See the
Script Demo Cube's `update` (sets a "Wobble" bar from its bob phase) and
`interact()`'s push demo (`show_toast("Pushed!", 1.5)`) for one example
from each call site.

## Save/load (checkpoints)

`engine::save` is intentionally small — a single gameplay checkpoint, not a
full save-file system:

```rust
pub struct SaveData {
    pub level_name: String,
    pub player_position: [f32; 3],
    pub player_yaw: f32,
    pub player_pitch: f32,
    pub saved_at_elapsed: f32,
}
```

Same `load_from_file`/`save_to_file` RON pattern as every other asset type
in the engine, written to a game's `saves/` folder — but unlike
profiles/rigs/classes/levels, there's exactly one file
(`saves/checkpoint.ron`), overwritten each time, since this is "get back
to roughly where I was," not a multi-slot save system. **Per-object or
per-script custom state (inventory, world flags, quest progress, ...) is
explicitly out of scope** — add it once a concrete game actually needs
that, rather than guessing at a shape now that would likely be wrong for
whatever that game turns out to be.

**F9** saves — Play mode only, since there's no player entity to snapshot
in Edit mode. **F10** loads, entering Play mode first if you're currently
in Edit mode (re-using the exact same `enter_play_mode` the F3 hotkey
calls, so a loaded checkpoint gets a freshly spawned player/physics state
under it, not a stale one). Both show a HUD toast ("Checkpoint saved"/
"Checkpoint loaded") so the otherwise-silent snapshot is visible — the
same `hud.show_toast` call site as the interact-push demo. If the
checkpoint's `level_name` doesn't match whatever level is currently
loaded, F10 logs a warning and repositions the player anyway rather than
switching levels for you — a checkpoint this simple isn't trying to
reconstruct "which level should be active," just where you were standing.

## Adding a new, separate game

**Fastest path:** `.\new_game.ps1 -Name mygame` copies `games/sandbox` as a
template, renames its crate to `mygame`, registers it in the workspace
root `Cargo.toml`, retitles its window/error-dialog strings, and builds it
to confirm the scaffold actually compiles. Everything comes along except
the bootstrap-generated `profiles/`/`levels/`/`rigs/`/`classes/`/`sfx/`/
`music/`/`saves/` folders (those regenerate fresh for the new game via the
same `default_level()`/`default_demo_profiles()`/etc. functions it
inherits in `src/main.rs`) — so `cargo run -p mygame` gives you the
sandbox's full working demo scene as a starting point to replace piece by
piece, rather than a truly blank slate.

**From scratch instead**, if you don't want any of the demo content:

1. Create the folder and manifest:
   ```
   games/mygame/
     Cargo.toml
     src/main.rs
   ```
   ```toml
   # games/mygame/Cargo.toml
   [package]
   name = "mygame"
   version = "0.1.0"
   edition = "2021"

   [dependencies]
   engine = { path = "../../engine" }
   sdl2.workspace = true
   glow.workspace = true
   glam.workspace = true
   egui.workspace = true
   rfd.workspace = true
   anyhow.workspace = true
   log.workspace = true
   env_logger.workspace = true
   ```
2. Add it to the workspace root `Cargo.toml`: `members = ["engine", "games/sandbox", "games/mygame"]`.
3. Copy `games/sandbox/assets/shaders/{mesh.vert,mesh.frag,post_composite.frag}`
   into `games/mygame/assets/shaders/` as your starting shader set (they're
   plain GLSL — edit freely; `mesh.vert`/`mesh.frag` must stay `#version`-line-free
   since `ShaderVariantCache` prepends it).
4. Implement `Game` for a struct in `src/main.rs` (start from `Sandbox` in
   `games/sandbox/src/main.rs` and delete what you don't need — the profile
   and level bootstrap/apply logic, ECS setup, render loop, and Play-mode
   wiring (physics step, first-person camera, `update_player_input`/
   `interact`) are all reusable as-is — a new game mostly means rewriting
   what's *inside* those last two methods for its own input/interactions).
5. `fn main() { App::run("My Game", 1280, 720, MyGame::new()) }`
6. `cargo run -p mygame` — empty `profiles/`/`levels/` folders mean you'll
   want your own `default_demo_profiles()`/`default_level()` (or just
   hand-write one `.ron` file matching the `ShaderProfile`/`RenderParams`
   shape in `engine::profile`, or the `Level`/`LevelObject` shape in
   `engine::level`).

## Exporting a game (shipping a standalone `.exe`)

```
.\package.ps1 -Game sandbox
```

This builds `-p sandbox` in release mode and assembles `dist/sandbox/` with
the `.exe` and every asset folder that exists next to it — `assets/`,
`profiles/`, `levels/`, `rigs/`, `classes/`, `scripts/`, `sfx/`, `music/`
(not `saves/`: checkpoints are pure runtime state, re-created empty on
first launch, so shipping the dev's own `checkpoint.ron` would just hand
players a stale position instead of a fresh game) — zip that folder (or
hand it to an installer builder like Inno Setup/WiX if you want a
"Setup.exe") and it runs standalone on another machine. Substitute your
own game's crate name for `-Game`; if your game adds its own asset
subfolder beyond this list, add it to `package.ps1`'s copy loop too.

A few things make this work, already wired up for every game in the workspace:

- **Assets resolve relative to the `.exe`, not the dev machine.** Early on,
  asset/profile paths were resolved via `env!("CARGO_MANIFEST_DIR")` — a
  compile-time constant baking in the *source folder on the machine that
  built it*. That's fine for `cargo run` but breaks the instant you copy the
  binary anywhere else. Each game should resolve its asset root at runtime via
  something like `Sandbox`'s `resolve_asset_root()` in
  `games/sandbox/src/main.rs`: look next to `std::env::current_exe()` first
  (where a shipped build's assets live), and only fall back to
  `CARGO_MANIFEST_DIR` in debug builds (`#[cfg(debug_assertions)]`) so
  `cargo run` keeps working without copying assets into `target/debug` on
  every change — and so that fallback, and the dev machine's path, don't even
  exist in a release binary.
- **No console window in release.** `#![cfg_attr(not(debug_assertions),
  windows_subsystem = "windows")]` at the top of `main.rs` hides the console
  for players while keeping it for `cargo run`. Since that also means no
  visible `log::info!`/panic output, release builds instead log to a
  `game.log` file next to the `.exe` and show a native error dialog (via
  `rfd::MessageDialog`, already a dependency) on panic or a fatal `Err`
  returned from `main` — copy the `init_logging`/`show_fatal_error_dialog`
  pattern from the sandbox into a new game.
- **`[profile.release]`** in the root `Cargo.toml` (`lto = true`,
  `codegen-units = 1`, `strip = true`, `panic = "abort"`) applies to every
  game automatically — smaller binary, no unwinding tables, better codegen.
- **Custom icon (optional):** drop an `icon.ico` next to a game's `Cargo.toml`
  and add the same `build.rs` + `winres` build-dependency pattern from
  `games/sandbox/build.rs`/`Cargo.toml` — it embeds the icon if present and
  silently falls back to the default icon (with a build warning) if not.
- **The level/shader editor is automatically gone in release.** `main.rs`'s
  `editor_available()` returns `cfg!(debug_assertions)`, and every editor
  affordance (both `SidePanel`s, F1/F2/Tab/F5/F3, orbit-camera drag/zoom)
  was already gated on "currently in `EditorMode::Edit`" — so a release
  build, which never enters `Edit` mode (`Sandbox::init` calls
  `enter_play_mode` immediately when `!editor_available()`), simply never
  reaches any of it. `package.ps1` already builds `--release`, so this
  needs no packaging changes: what a player gets is exactly the game,
  booting straight into Play. The pause menu's middle button becomes
  **"Restart Level"** instead of "Exit to Editor" (see "Pausing" above) —
  there being no editor to exit to.

Caveat: the in-game editors save profiles/levels back into the `profiles/`/
`levels/` folders next to the `.exe`. If you install to a location that needs
admin rights (e.g. `Program Files`), those writes will fail for a normal
user in a *debug* build — ship to a user-writable folder (Desktop,
`Documents`, a portable zip), which is moot for a real `--release` shipment
anyway since the editor (and its writes) aren't reachable there at all.
