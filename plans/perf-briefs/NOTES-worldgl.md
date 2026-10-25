# NOTES: WORLDGL (branch perf/worldgl)
No GPU/display in the sandbox: numbers are call counts from the code; the parent should read the new profiler counters after merge (`ENGINE_PROF=1`, rows `world ...`).

## Changes (renderstate.rs + `// === perf ===` blocks in render())
- Uniform locations cached per program (`WorldLocs`): 27 `get_uniform_location` string lookups/frame -> 0 (built once).
- World queried once per frame (10-component query) instead of 4. Pass 0 draws non-water and collects hero/water entity lists (reused Vecs, query order kept); passes 1/2/3 walk those short lists with `query_one`. Entities visited: 4N -> N + 2*heroes + water (N = drawable entities, typically hundreds; heroes ~1-3).
- Draw state cache (lit/hero/melt/uv floats, bound texture, bound VAO): redundant uniform uploads, texture binds and `bind_vertex_array` skipped. Matrix upload stays per draw. State is fresh each frame (egui leaves GL state behind). Mesh without texture keeps the old "no bind" behaviour.
- Counters (profiler rows): world draws, uniform sets/skipped, tex binds/skipped, vao binds/skipped, visited p0..p3, queries.
- Draw order is unchanged in every pass, so output is identical. Not done on purpose: sorting pass 0 by texture (depth ties/z-fighting could change pixels), instancing.
- present(): trail pass is already skipped when trail_amount <= 0 and the trail targets are only (re)created on size change; nothing to remove.
