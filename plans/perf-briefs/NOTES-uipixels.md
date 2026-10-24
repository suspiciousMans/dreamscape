# NOTES: UIPIXELS
Measured by `cargo test -p dreamscape ui_cost_table -- --nocapture` (1280x720, 240-card booklet, DEBUG build: tess_us is only comparable between rows/runs of the same build).

## Baseline
```
screen      shapes  vertices  triangles    tess_us
Warning         87      3040       2022        463
Title          586      5944       6208       1300
Summary        184      1236       1466        332
Paused         411      2552       2254        574
Playing        329      1964       1954        398
Settings       213      2732       2226        426
Codex         9208     66820      83034      21906
Booklet        181      3976       2854        664
Loadout       1734     16212      18070       5864
Merge        10213     43668      52322      12622
Store         4540     37156      45216       9540
Lottery       4594     17928      21062       4809
Tickets        206      2608       2202        412
```

## After (same test, same 240-card booklet, 1280x720)
```
screen      shapes  vertices  triangles    tess_us (debug, noisy)
Warning         81      3012       1966        346
Title          389      3692       2742        883
Summary         53       700        394        226
Paused         190      1604        808        462
Playing        108      1016        508        252
Settings        82      2196       1154        243
Codex           91      1616        926        249   (was 9208 / 83034 tri / 21906 us)
Booklet        169      3920       2742        514
Loadout        310      7868       4880        958   (was 1734 / 18070 / 5864)
Merge          354      8200       5086       1263   (was 10213 / 52322 / 12622)
Store          240      7108       4110        760   (was 4540 / 45216 / 9540)
Lottery        352      4540       3880        768   (was 4594 / 21062 / 4809)
Tickets         69      2044       1074        216
```
Tessellation time is from an unoptimised test build: read it as a ratio (Merge -90%, Loadout -84%, Store -92%).

## What changed (all in `pixel_ui.rs` unless noted; `// === perf === uipixels` blocks)
- `hatch`: one quad sampling a 4x4 repeating NEAREST texture (was one rect per cell: ~6000 per dimmed card).
- `Quads` / `merge_cells` / `cell_mesh`: many solid quads in ONE mesh (4 vertices, 2 triangles each instead of a feathered rect), same-colour cells merged into runs. Used by `icon`, `scanlines`, `panel` (7 bars), shop `draw_icon` (shadow + fill), HUD `px_cells`, moth body, card-back pattern, pack art cells, reveal eye, upgrade icons.
- Equality tests: `pixel_ui::tests::raster` is a CPU rasteriser (pixel centres, premultiplied source-over); each change is compared with the original painter kept as `#[cfg(test)]` reference (`hatch_rects`, `icon_rects`, `panel_rects`, `shadowed_icon_rects`), tolerance 0.

## Caveat (for the parent's screenshot check)
egui feathers `rect_filled` edges by 1 px. On whole-pixel geometry (all cell art at 1280x720) that is invisible, so the pixels match. Where the old rect had a fractional edge (scanlines are 1.05 px tall) the mesh version has a hard edge instead of a faint soft one (alpha 8-10/255). Translucent icons lose the faint seams the old per-cell rects left where neighbours abutted.
Not done: text glyph vertices (the rest of the cost), booklet card grid culling (booklet page shows 1-3 cards already), per-frame String allocations.
