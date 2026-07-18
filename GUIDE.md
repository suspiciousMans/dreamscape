# PS2 Engine Guide

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
  `F3` again or **Escape** both stop; plain `Escape` only quits the app while
  already in Edit mode.
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

## Adding a new, separate game

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

No scaffolding CLI exists (yet) — the sandbox is the template; copy from it.

## Exporting a game (shipping a standalone `.exe`)

```
.\package.ps1 -Game sandbox
```

This builds `-p sandbox` in release mode and assembles `dist/sandbox/` with
the `.exe`, its `assets/`, and its `profiles/` sitting next to each other —
zip that folder (or hand it to an installer builder like Inno Setup/WiX if
you want a "Setup.exe") and it runs standalone on another machine. Substitute
your own game's crate name for `-Game`.

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

Caveat: the in-game editors save profiles/levels back into the `profiles/`/
`levels/` folders next to the `.exe`. If you install to a location that needs
admin rights (e.g. `Program Files`), those writes will fail for a normal
user — ship to a user-writable folder (Desktop, `Documents`, a portable zip)
or, for a real release where you don't want players editing shaders/levels at
all, gate the editor panels behind a debug-only flag before shipping.
