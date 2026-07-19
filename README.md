# Jame Engine

A lightweight Rust game engine (SDL2 + OpenGL) built around a PS2-era visual
style: low internal resolution upscaled with nearest-neighbor filtering,
per-vertex lighting, fog, optional vertex snapping and affine texture mapping,
and posterize/dither post-processing — all controlled by savable "shader
profiles" you can tweak live in an in-game editor GUI.

The engine and its demo game (`sandbox`) live side by side in one workspace,
so the sandbox doubles as both a working example and a dev-time level/shader
editor. Everything is documented in depth in **[GUIDE.md](GUIDE.md)** — this
README is just the quick tour.

## Quick start

```
cd ps2-engine
cargo run -p sandbox
```

The first build compiles SDL2 from source and is slow (a minute or two);
subsequent builds are fast.

**Controls** (Edit mode): left-drag orbits the camera, scroll wheel zooms,
**F1** toggles the shader-profile panel, **F2** toggles the level editor
panel, **Tab** cycles shader profiles, **F5** saves the current profile,
**F3** enters Play mode, **Escape** quits.

## What's in it

- **Render pipeline** — low-res offscreen framebuffer, per-vertex lighting/fog,
  vertex snapping, affine texture mapping, posterize/dither post-process,
  skybox, backface culling — all as savable, hot-swappable shader profiles
- **ECS** (`hecs`) with a level editor GUI (F2) for placing/editing objects,
  lights, triggers, and particle emitters, all saved as human-readable RON
- **Physics** — a small, readable RigidBody/Collider simulation (gravity,
  AABB/Sphere collision)
- **Object classes** — share mesh/texture/physics/behavior across many
  placed instances from one editable "class," like a lightweight prefab
- **Scripting** — a small custom language (`.pss` files) for per-object
  behavior, plus a standalone script editor tool, with native Rust as a
  drop-in alternative to a script for the same hooks
- **Character rigs** — segmented (non-skinned) rigid-part rigs with
  keyframed clips and smooth clip-to-clip blending
- **Particles**, **point lights**, **trigger volumes**, and simple
  **animation** (orbit/bob) as placeable level content
- **Audio** — procedural tones and real sample/OGG/MP3/WAV playback for SFX
  and looping level music
- **In-game HUD**, a **pause menu**, and **checkpoint save/load** (F9/F10)
  for Play mode
- **Gamepad support** alongside keyboard/mouse
- Release builds automatically strip out the dev-time editor entirely and
  boot straight into the game — see `package.ps1` for shipping a standalone
  `.exe`

## Workspace layout

```
ps2-engine/
  engine/            the reusable library — rendering, ECS, physics, scripting, etc.
  games/sandbox/      the demo game — also the template for starting a new one
  tools/script_editor/ standalone editor for .pss script files
  package.ps1         builds a game in release mode and assembles a dist/ folder
```

See **[GUIDE.md](GUIDE.md)** for the full breakdown of every module, how to
add a new game, and how to export a standalone build.
