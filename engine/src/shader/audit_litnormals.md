# Shader / Material Infrastructure — LitNormals Path Audit

## Scope
Trace from shader module definitions → program construction → sandbox render path.
Locate where material properties (normal map, roughness, metallic) are uploaded and
how variants such as LitNormals are handled. Confirm exists vs missing.

---

## 1. Material data layer — EXISTS (engine/src/material.rs)

- `MaterialVariant` enum: `Lit`, `LitNormals` (`rename = "lit_normals"`), `Unlit`.
- `Material` struct with all properties:
  - `base_color: Option<String>`
  - `normal_map: Option<String>`
  - `roughness: f32` (default 0.5)
  - `metallic: f32` (default 0.0)
  - `variant: MaterialVariant`
- `MaterialFile { name, material }` with RON round-trip.
- `MaterialCycler` for panel cycling.
- `load_from_file` / `save_to_file` / `load_dir` scaffolding present.
- `Default` impls: `MaterialVariant::Lit`, `Material { roughness: 0.5, metallic: 0.0, ... }`.

**Verdict: data layer is complete and ready for render consumption.**

---

## 2. MeshRenderer component — EXISTS (engine/src/ecs/components.rs:58)

```rust
pub struct MeshRenderer {
    pub mesh: Option<Mesh>,
    pub material: Option<Material>,
}
```

`MeshRenderer.material` is on the ECS component. It is **not** consumed anywhere in the sandbox render loop yet.

---

## 3. Test — EXISTS (engine/tests/test_material.rs)

`material_variant_tag_selects_shader_branch` asserts:

| Variant | vertex | fragment |
|---|---|---|
| `LitNormals` | `shaders/mesh_normals.vert` | `shaders/mesh_normals.frag` |
| `Lit` | `shaders/mesh.vert` | `shaders/mesh.frag` |
| `Unlit` | `shaders/mesh.vert` | `shaders/mesh.frag` |

Uses `jame_engine::shader::shader_files_for`. **This function does not exist on disk** (see §5).

---

## 4. Shader variant cache — EXISTS (engine/src/shader/variant.rs)

```rust
pub struct ShaderVariantCache {
    vertex_body: String,
    fragment_body: String,
    variants: HashMap<u32, glow::Program>,
}
```

- `get_or_compile(gl, flags)` — compile-once, cache-by-bitmask, swaps programs.
- `defines_for(flags)` currently emits only `#define AFFINE_UV` when `AFFINE_UV_BIT` is set.
- `variant.rs` imports `super::link_program`, which expects `engine/src/shader/mod.rs` or `program.rs` to define it — **those files do not exist on disk** (see §5).

**Verdict: the cache mechanism is real but only wires `AFFINE_UV_BIT`. There is no
variant-bit dispatch for `MaterialVariant`.**

---

## 5. Broken build — BLOCKER (engine/src/)

On disk, `engine/src/shader/` contains **only** `variant.rs`. There is no
`mod.rs`, no `program.rs`, no `shader.rs`.

`engine/src/lib.rs:25` declares `pub mod shader;` with no backing file.
`cargo build` (and `cargo test`) fails:

```
error[E0596]: ...  --> engine/src/lib.rs:25:1
25 | pub mod shader;
   | ^^^^^^^^^^^^^^^
   = help: to create the module `shader`, create file "engine/src/shader.rs"
     or "engine/src/shader/mod.rs"
```

Additionally, `variant.rs:3` does `use super::link_program;` — that function
would need to live in the missing module root. The test's
`jame_engine::shader::shader_files_for` likewise has nowhere to live.

**Verdict: the crate does not compile. The shader module entrypoint +
program-linking + `shader_files_for` implementation are all absent from disk.**

---

## 6. Sandbox shader files

| File | Status |
|---|---|
| `games/sandbox/assets/shaders/mesh.vert` | exists (GLSL 330 core, per-vertex lighting, fog) |
| `games/sandbox/assets/shaders/mesh.frag` | exists (simple textured fragment, fog blend, no PBR/no normal map) |
| `games/sandbox/assets/shaders/mesh_normals.vert` | **MISSING** |
| `games/sandbox/assets/shaders/mesh_normals.frag` | **MISSING** |

---

## 7. Sandbox render path — MISSING material upload (games/sandbox/src/main.rs)

### MeshUniforms (around line 680)
```rust
struct MeshUniforms {
    model: glam::Mat4,
    view: glam::Mat4,
    proj: glam::Mat4,
    fog_color: glam::Vec3,
    fog_factor: f32,
    ambient: glam::Vec3,
    light_dir: glam::Vec3,
}
```
No `normal_map_id`, no `roughness`, no `metallic`.

### Shader cache (around line 720)
```rust
shader_cache: HashMap<String, glow::Program>,
```
Plain name→program cache. Not `ShaderVariantCache`. No variant dispatch.

### Render loop (around line 5600)
Per-entity uniform upload sequence:
1. `uModel`, `uView`, `uProj`
2. `uFogColor`, `uFogFactor`
3. `uAmbient`, `uLightDir`
4. Bind base texture → `uTex` (texture unit 0)

No normal map texture bind, no roughness/metallic uniform upload, no
variant-aware program selection.

**Verdict: the sandbox render path is a single unweighted textured shader.
Material properties on `MeshRenderer.material` are ignored.**

---

## 8. Summary — exists vs missing for LitNormals

### Exists
- `MaterialVariant::LitNormals` enum variant
- `Material.normal_map`, `roughness`, `metallic` fields + serialization
- `MeshRenderer.material: Option<Material>`
- `ShaderVariantCache` with bitmask dispatch (only `AFFINE_UV_BIT` today)
- Test asserting `shader_files_for(LitNormals)` → `mesh_normals.{vert,frag}`

### Missing
1. **Shader module root.** `engine/src/shader/mod.rs` (or `shader.rs`) + `program.rs`
   do not exist on disk. The crate does not compile; `link_program` and
   `shader_files_for` have no home.
2. **`mesh_normals.vert` / `mesh_normals.frag`** — the shader files the test
   expects are not on disk.
3. **`MeshUniforms`** lacks normal-map-id, roughness, metallic fields.
4. **Sandbox render loop** does not upload normal map, roughness, or metallic,
   and does not switch programs based on `MaterialVariant`.
5. **`ShaderVariantCache` is not integrated** into the sandbox; the sandbox uses
   a plain `HashMap<String, Program>` with no variant dispatch.

### LitNormals readiness
Data model: **ready.** Test: **ready (but fails to compile because shader module
is missing).** Runtime path: **not ready** — shader files, uniform struct,
upload calls, and variant dispatch all absent.

---

## Update 2026-10-05 — resolved

Everything listed under "Missing" above is now in place; this audit is kept
for history only.

- `shader::shader_files_for` lives in `engine/src/shader/mod.rs`
  (`test_material.rs` now imports it via `engine::`, the real crate name).
- `mesh_normals.{vert,frag}` rewritten: no tangent attribute (the vertex
  format has none) — the TBN is rebuilt per pixel from screen-space
  derivatives; per-pixel directional + point lights match `mesh.vert`;
  Blinn-Phong spec from roughness/metallic. Normal maps are OpenGL
  convention; green is negated because `GpuTexture` uploads unflipped.
- Sandbox draws in two passes: profile program, then a `LitNormals` program
  (`normals_shader_cache`, rebuilt in `apply_profile`, hot-reloaded).
  Missing/broken normals shaders or normal-map files log and fall back
  instead of panicking.
- Verified visually with a throwaway dome normal map: bumps light from the
  sun's side on both a +Z cube face and the +Y floor.

Still open: nothing assigns `MeshRenderer.material` yet (all spawn sites
pass `None`), so the path is only reachable from code — needs a
`LevelObject.material` field + F2 inspector control.
