# jame-engine UI redesign — Tasks 3–6 Implementation Notes

**Status:** Tasks 0–2 ✅ complete and committed. This document guides Tasks 3–6.

**Repo state:** `/home/_l/Downloads/jame-engine-src/jame-engine-master/`, branch `master`, commit `a0f303c`.

---

## What's in place (Tasks 0–2)

1. **EditorTheme** (`engine/src/ui/theme.rs`)
   - Serializable color struct (8 RGB fields + font + font_size)
   - `builtin_default()` returns Godot-dark colors
   - Exported from `engine/src/ui/mod.rs`

2. **EditorDocks skeleton** (`engine/src/ui/dock_skeleton.rs`)
   - `EditorDocks` struct: owns theme, style, and `DockVisibility`
   - `draw_dock_shell()` method: renders placeholder panels (top toolbar, left Scene, right Inspector, bottom Output, central viewport)
   - Placeholder text in each panel; no content yet
   - Exported from `engine/src/ui/mod.rs`

3. **Dependencies**
   - `egui_dock = "0.15"` added to root Cargo.toml and engine workspace

**All code compiles clean.** No integration with sandbox yet.

---

## Tasks 3–6 (next implementation phase)

### Task 3: Scene dock + Inspector dock

**Files to create/modify:**
- `engine/src/ui/panels.rs` — extend with `draw_scene_hierarchy()` and `draw_inspector()`
- `engine/src/ui/mod.rs` — export the new functions
- `engine/tests/test_scene_inspector.rs` — write test first
- `games/sandbox/src/main.rs` — wire docks into the editor render pass

**What to implement:**
1. Scene hierarchy panel: iterate `hecs::World`, show entity IDs + component names, clickable selection
2. Inspector panel: when an entity is selected, display its components as editable property list
3. Component field editing: transform position/rotation, script name, mesh reference, etc.
4. Entity naming: either use a Name component (if one exists) or add one

**TDD flow:**
1. Write `test_scene_inspector.rs` — create a test world, verify hierarchy renders, selection works, component edit persists
2. Implement `draw_scene_hierarchy()` in `panels.rs`
3. Implement `draw_inspector()` in `panels.rs`
4. Wire both into `EditorDocks::draw_dock_shell()` or call from sandbox's render loop
5. Test: `cargo test -p engine test_scene_inspector -- --nocapture`

**Critical APIs:**
- `hecs::World::iter()` for entity iteration
- `games/sandbox/src/main.rs` line ~860: `selected_entity: Option<Entity>` field in `Sandbox` struct
- Component types: `Transform`, `MeshRenderer`, `Script`, etc. (check `engine/src/ecs/components.rs`)

---

### Task 4: FileSystem asset browser + Output/log dock + profiler tab

**Files to create/modify:**
- `engine/src/ui/asset_browser.rs` — extend existing code to draw file tree in dock
- `engine/src/ui/output_dock.rs` — new file, implement Output/log panel
- `engine/src/ui/mod.rs` — export new functions
- `engine/tests/test_asset_browser.rs` — write test first
- `games/sandbox/src/main.rs` — wire into dock layout

**What to implement:**
1. FileSystem dock: recursively list `games/sandbox/assets/` (textures/, meshes/, shaders/, scripts/)
   - Use egui `CollapsingHeader` or tree widget for folders
   - Show file icons/names
   - Optional: preview on hover (image thumbnail, shader code)
2. Output/Log dock: scrollable text area for log messages
   - Implement `struct OutputPanel { lines: Vec<String>, filter: LogLevel }`
   - Add buttons: Clear, Copy, Filter (Info/Warning/Error)
   - Wire the engine's logging (if it uses `log` crate) or use a push-based in-panel buffer
3. Profiler tab (bottom): reuse existing `engine/src/ui/profiler.rs`
   - Slot it as a tab alongside Output using egui TabBar or similar

**TDD flow:**
1. Write `test_asset_browser.rs` — verify asset tree iteration, file listing, log buffering
2. Implement asset tree rendering
3. Implement Output panel + log filtering
4. Wire profiler as a tab
5. Test: `cargo test -p engine test_asset_browser -- --nocapture`

**Critical APIs:**
- `std::fs::read_dir()` for directory iteration
- egui `CollapsingHeader`, `ScrollArea`, `TextEdit` for UI
- `engine/src/ui/profiler.rs` for profiler stats structure

---

### Task 5: Toolbar wiring + old window cleanup + theme persistence

**Files to modify:**
- `engine/src/ui/dock_skeleton.rs` — wire Play/Stop/editor-toggle buttons
- `engine/src/ui/backend.rs` — remove old ad-hoc window code (after Task 3–4 are done)
- `engine/src/ui/theme.rs` — add persistence stubs (load_from_file, save_to_file using serde RON)
- `games/sandbox/src/main.rs` — call `draw_dock_shell()` in the editor render pass

**What to implement:**
1. Wire Play/Stop buttons to existing `Sandbox::mode` (EditorMode::Edit vs EditorMode::Play) and `Sandbox::paused`
2. Wire editor-toggle button to `Sandbox::editor_mode` (or equivalent)
3. Remove old free-floating windows from `backend.rs` (only after Task 3–4 confirm the new docks work)
4. Add `EditorTheme::load_from_file(path)` and `.save_to_file(path)` using `ron` serialization
5. Add test for theme roundtrip (serialize → deserialize → compare)

**No new files; mostly wiring + cleanup.**

---

### Task 6: Prepare shell for Phase 1 (material editor, console, tilemap)

**Files to modify:**
- `engine/src/ui/dock_skeleton.rs` — add placeholder tab slots for material editor, console, tilemap
- Plan file(s) reference: Phase 1 features land in these tabs

**What to implement:**
1. Add bottom tab set: Output, Profiler, Material Editor, Debug Console, Tilemap
2. Add visibility toggles for new tabs
3. Placeholder rendering (empty panels with titles)

**This is mostly placeholders.** Phase 1 features (material system, tilemap rendering, console) are separate work.

---

## Build/test workflow for next session

```bash
cd /home/_l/Downloads/jame-engine-src/jame-engine-master

# After implementing Task 3:
cargo test -p engine test_scene_inspector -- --nocapture
cargo check -p engine

# After implementing Task 4:
cargo test -p engine test_asset_browser -- --nocapture
cargo check -p engine

# After wiring into sandbox (Task 5):
# (sandbox binary test — currently can't run due to SDL2 bundled build issue;
#  can be run on a machine with native SDL2 headers or after fixing bundled build)

# Commit after each task:
git add -A && git commit -m "feat(ui): <task description>"
```

---

## Known blockers & workarounds

**SDL2 bundled build fails on some systems.** 
- Root cause: CMake configuration for bundled SDL2 requires native build tools.
- Workaround: `cargo check -p engine` works (checks without linking). Full `cargo build` will fail.
- Solution: Either install SDL2 dev headers on the system, or skip the bundled feature and link to system SDL2.
- **Impact for this work:** We can develop and test the UI code (which is egui + panels, not graphics), but can't run the full sandbox binary to verify the editor visually. Integration test via `cargo test` is sufficient for now.

---

## Repo structure reference

```
/home/_l/Downloads/jame-engine-src/jame-engine-master/
├── Cargo.toml                          (root workspace)
├── engine/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                      (re-exports all modules)
│   │   ├── ecs/
│   │   │   ├── components.rs           (Transform, MeshRenderer, etc.)
│   │   │   └── mod.rs
│   │   └── ui/
│   │       ├── mod.rs                  (exports all UI modules)
│   │       ├── theme.rs                (✅ EditorTheme — DONE)
│   │       ├── dock_skeleton.rs        (✅ EditorDocks — DONE)
│   │       ├── panels.rs               (render_params_editor; extend with scene/inspector)
│   │       ├── asset_browser.rs        (existing; extend for dock)
│   │       ├── backend.rs              (EguiState; remove old windows in Task 5)
│   │       ├── profiler.rs             (existing; reuse in Task 4)
│   │       └── ...
│   └── tests/
│       ├── test_theme.rs               (✅ DONE)
│       ├── test_scene_inspector.rs     (TODO: write in Task 3)
│       └── test_asset_browser.rs       (TODO: write in Task 4)
├── games/sandbox/
│   └── src/
│       └── main.rs                     (Sandbox struct; wire docks in Task 5)
└── docs/
    └── editor_theme_default.md         (✅ theme capture — DONE)
```

---

## Next session entry point

1. Read this file.
2. Verify repo state: `git log --oneline -3` should show theme + dock_skeleton commits.
3. Run `cargo check -p engine` — should succeed.
4. Start with Task 3: write `test_scene_inspector.rs`, then implement `draw_scene_hierarchy()` + `draw_inspector()`.
5. Test and commit after each task.

---

**Last updated:** 2026-09-16, commit `a0f303c`
