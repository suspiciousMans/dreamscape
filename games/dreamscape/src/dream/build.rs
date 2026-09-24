//! Turns a layout + theme into concrete blocks (every block is a coloured cube).

use super::grid::{Cell, Grid, P};
use super::layout;
use super::meshes::Shape;
use super::texture::{self as tex, Pattern};
use super::theme::{DreamTheme, PropKind, ThemeSpec};
use crate::gameplay::{CELL, SLAB};
use engine::glam::{EulerRot, Quat, Vec3};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use std::collections::HashSet;
use std::f32::consts::TAU;

pub const PORTAL_COLOR: [u8; 4] = [40, 220, 220, 255];
pub const PORTAL_SIZE: Vec3 = Vec3::new(1.5, 2.0, 1.5);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Floor,
    Wall,
    Prop,
    Portal,
    /// Floating, non-solid set dressing.
    Decor,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub kind: BlockKind,
    pub shape: Shape,
    pub pos: Vec3,
    pub size: Vec3,
    pub rotation: Quat,
    /// Flat palette colour. Rendering now uses the dream's procedural
    /// textures; kept as the per-block palette record the cohesion tests check.
    #[allow(dead_code)]
    pub color: [u8; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct Waypoint {
    pub pos: Vec3,
    /// This waypoint is reached by jumping over a void cell.
    pub jump: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Atmosphere {
    pub fog_color: [f32; 3],
    pub ambient: [f32; 3],
    pub fog_start: f32,
    pub fog_end: f32,
}

pub struct Dream {
    pub theme: DreamTheme,
    pub seed: u64,
    pub depth: u32,
    pub blocks: Vec<Block>,
    pub spawn: Vec3,
    pub portal: Vec3,
    /// Shortest walkable route spawn → portal (cell centres, y = 0).
    pub route: Vec<Waypoint>,
    /// Enemy patrol endpoints (y = 0.5).
    pub patrols: Vec<(Vec3, Vec3)>,
    pub atmosphere: Atmosphere,
    /// Where the previous dream's motif prop was placed, if any.
    pub motif_at: Option<Vec3>,
    /// Depth-scaled weirdness, 0..=1. Drives colour spread, debris, texture
    /// accents, and the shader's warp/hue-cycling intensity.
    pub strangeness: f32,
    pub surfaces: Surfaces,
    /// Lucidity shard location (floor level), if this dream has one.
    pub shard: Option<Vec3>,
    /// Route spawn → shard → portal (equals `route` when there is no shard).
    pub lucid_route: Vec<Waypoint>,
}

/// Everything needed to (re)build one procedural texture.
#[derive(Clone, Debug, PartialEq)]
pub struct TexSpec {
    pub pattern: Pattern,
    pub palette: Vec<[u8; 3]>,
    pub bands: f32,
    pub seed: u64,
}

impl TexSpec {
    pub fn rgba(&self) -> Vec<u8> {
        tex::generate(self.pattern, &self.palette, self.bands, self.seed)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Surfaces {
    pub floor: TexSpec,
    pub wall: TexSpec,
    pub prop: TexSpec,
    pub shard: TexSpec,
    pub enemy: TexSpec,
}

/// Dreams get stranger the deeper you go. Waking is always calm.
pub fn strangeness(theme: DreamTheme, depth: u32) -> f32 {
    if theme == DreamTheme::Awakening {
        return 0.0;
    }
    (theme.spec().strangeness + depth as f32 * 0.07).min(1.0)
}

fn surface(base: &[[u8; 3]], spec: &ThemeSpec, rng: &mut StdRng) -> TexSpec {
    let s = spec.strangeness;
    let mut palette = base.to_vec();
    if s > 0.3 {
        let n = ((s * spec.accents.len() as f32).ceil() as usize).min(spec.accents.len());
        palette.extend_from_slice(&spec.accents[..n]);
    }
    TexSpec {
        pattern: *spec
            .patterns
            .choose(rng)
            .expect("theme patterns are non-empty (theme tests)"),
        palette,
        bands: 1.0 + s * 3.0,
        seed: rng.gen(),
    }
}

/// The portal is a window into the NEXT dream: its floor pattern at full
/// psychedelic intensity, so you see where you're going before you step in.
pub fn portal_surface(next: DreamTheme, seed: u64) -> TexSpec {
    let spec = next.spec();
    let mut palette = spec.floor_colors.to_vec();
    palette.extend_from_slice(spec.accents);
    TexSpec {
        pattern: spec.patterns[0],
        palette,
        bands: 3.0,
        seed,
    }
}

pub fn generate(
    theme: DreamTheme,
    seed: u64,
    depth: u32,
    motif: Option<PropKind>,
    with_shard: bool,
) -> Dream {
    let mut spec = theme.spec();
    spec.strangeness = strangeness(theme, depth);
    let mut rng = StdRng::seed_from_u64(seed);
    let layout = layout::generate(spec.layout, spec.grid_size, &mut rng);
    if layout.fallback {
        log::warn!("{theme:?} seed {seed}: layout generator gave up, using fallback hall");
    }
    let grid = &layout.grid;
    let path = grid
        .path(layout.spawn, layout.portal)
        .expect("layout::generate only returns connected layouts");
    let mut on_path: HashSet<P> = path.iter().map(|&(p, _)| p).collect();

    // Lucidity shard: an off-route detour, reachable, never blocked by props.
    let shard_cell = if with_shard {
        pick_shard(grid, &on_path, layout.spawn, &mut rng)
    } else {
        None
    };
    let lucid_path: Vec<(P, bool)> = match shard_cell {
        Some(c) => {
            let there = grid.path(layout.spawn, c).expect("shard chosen reachable");
            let back = grid.path(c, layout.portal).expect("moves are symmetric");
            there.into_iter().chain(back.into_iter().skip(1)).collect()
        }
        None => path.clone(),
    };
    on_path.extend(lucid_path.iter().map(|&(p, _)| p));

    let mut blocks = Vec::new();
    floor_slabs(grid, &spec, &mut rng, &mut blocks);
    walls(grid, &spec, &mut rng, &mut blocks);
    let motif_at = props(
        grid,
        &spec,
        &on_path,
        layout.spawn,
        motif,
        &mut rng,
        &mut blocks,
    );
    decor(grid, &spec, &mut rng, &mut blocks);
    let surfaces = Surfaces {
        floor: surface(spec.floor_colors, &spec, &mut rng),
        wall: surface(spec.wall_colors, &spec, &mut rng),
        prop: surface(spec.prop_colors, &spec, &mut rng),
        shard: TexSpec {
            pattern: Pattern::Swirl,
            palette: spec.accents.to_vec(),
            bands: 2.0,
            seed: rng.gen(),
        },
        // Enemies are always bloodshot eyes: skin, white, iris, pupil.
        enemy: TexSpec {
            pattern: Pattern::Eyes,
            palette: vec![[90, 10, 20], [255, 235, 225], [220, 30, 50], [10, 0, 5]],
            bands: 1.0,
            seed: rng.gen(),
        },
    };
    let portal = grid.world(layout.portal);
    let waypoints = |p: &[(P, bool)]| -> Vec<Waypoint> {
        p.iter()
            .map(|&(c, jump)| Waypoint {
                pos: grid.world(c),
                jump,
            })
            .collect()
    };
    blocks.push(Block {
        kind: BlockKind::Portal,
        shape: Shape::Orb,
        pos: portal + Vec3::Y * PORTAL_SIZE.y * 0.5,
        size: PORTAL_SIZE,
        rotation: Quat::IDENTITY,
        color: PORTAL_COLOR,
    });

    Dream {
        theme,
        seed,
        depth,
        blocks,
        spawn: grid.world(layout.spawn),
        portal,
        route: waypoints(&path),
        lucid_route: waypoints(&lucid_path),
        shard: shard_cell.map(|c| grid.world(c)),
        patrols: patrols(grid, &path, &spec, depth, &mut rng),
        atmosphere: atmosphere(&spec, &mut rng),
        motif_at,
        strangeness: spec.strangeness,
        surfaces,
    }
}

/// A reachable floor cell off the direct route, biased toward far-away ones
/// so fetching it is a real detour.
fn pick_shard(grid: &Grid, on_path: &HashSet<P>, spawn: P, rng: &mut StdRng) -> Option<P> {
    let mut candidates: Vec<(usize, P)> = grid
        .cells_of(Cell::Floor)
        .into_iter()
        .filter(|c| !on_path.contains(c))
        .filter_map(|c| grid.path(spawn, c).map(|p| (p.len(), c)))
        .collect();
    if candidates.is_empty() {
        return None;
    }
    candidates.sort();
    let far_half = &candidates[candidates.len() / 2..];
    far_half.choose(rng).map(|&(_, c)| c)
}

fn pick(palette: &[[u8; 3]], rng: &mut StdRng) -> [u8; 3] {
    *palette
        .choose(rng)
        .expect("theme palettes are non-empty (theme tests)")
}

/// Palette colour ± up to 40 per channel, more spread the stranger the dream.
fn tint(c: [u8; 3], strangeness: f32, rng: &mut StdRng) -> [u8; 4] {
    let spread = 12.0 + 28.0 * strangeness;
    let mut out = [0, 0, 0, 255];
    for i in 0..3 {
        out[i] = (c[i] as f32 + rng.gen_range(-spread..spread))
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    out
}

/// One slab per horizontal run of floor cells (fewer entities than one per cell).
fn floor_slabs(grid: &Grid, spec: &ThemeSpec, rng: &mut StdRng, out: &mut Vec<Block>) {
    for y in 0..grid.h {
        let mut x = 0;
        while x < grid.w {
            if grid.get((x, y)) != Cell::Floor {
                x += 1;
                continue;
            }
            let start = x;
            while x < grid.w && grid.get((x, y)) == Cell::Floor {
                x += 1;
            }
            let centre = (grid.world((start, y)) + grid.world((x - 1, y))) * 0.5;
            out.push(Block {
                kind: BlockKind::Floor,
                shape: Shape::Cube,
                pos: centre - Vec3::Y * SLAB * 0.5,
                size: Vec3::new((x - start) as f32 * CELL, SLAB, CELL),
                rotation: Quat::IDENTITY,
                color: tint(pick(spec.floor_colors, rng), spec.strangeness, rng),
            });
        }
    }
}

fn walls(grid: &Grid, spec: &ThemeSpec, rng: &mut StdRng, out: &mut Vec<Block>) {
    if spec.wall_height <= 0.0 {
        return;
    }
    for cell in grid.cells_of(Cell::Wall) {
        out.push(Block {
            kind: BlockKind::Wall,
            shape: Shape::Cube,
            pos: grid.world(cell) + Vec3::Y * spec.wall_height * 0.5,
            size: Vec3::new(CELL, spec.wall_height, CELL),
            rotation: Quat::IDENTITY,
            color: tint(pick(spec.wall_colors, rng), spec.strangeness, rng),
        });
    }
}

fn place(kind: PropKind, base: Vec3, scale: f32, color: [u8; 4], out: &mut Vec<Block>) {
    for &(offset, size, shape) in kind.parts() {
        out.push(Block {
            kind: BlockKind::Prop,
            shape,
            pos: base + Vec3::from(offset) * scale,
            size: Vec3::from(size) * scale,
            rotation: Quat::IDENTITY, // solid => axis-aligned, so the AABB collider matches
            color,
        });
    }
}

/// Props only go on floor cells that are NOT on the route, so they can never
/// block the way to the portal. Returns where the motif was placed.
#[allow(clippy::too_many_arguments)]
fn props(
    grid: &Grid,
    spec: &ThemeSpec,
    on_path: &HashSet<P>,
    spawn: P,
    motif: Option<PropKind>,
    rng: &mut StdRng,
    out: &mut Vec<Block>,
) -> Option<Vec3> {
    let free: Vec<P> = grid
        .cells_of(Cell::Floor)
        .into_iter()
        .filter(|p| !on_path.contains(p))
        .collect();
    let motif_cell = motif.and_then(|kind| {
        let cell = *free
            .iter()
            .min_by_key(|p| (p.0 - spawn.0).abs() + (p.1 - spawn.1).abs())?;
        let color = tint(pick(spec.prop_colors, rng), spec.strangeness, rng);
        place(kind, grid.world(cell), 1.0, color, out);
        Some(cell)
    });
    for &cell in &free {
        if Some(cell) == motif_cell || !rng.gen_bool(spec.prop_density as f64) {
            continue;
        }
        let kind = *spec
            .props
            .choose(rng)
            .expect("theme props are non-empty (theme tests)");
        let scale = 1.0 + rng.gen::<f32>() * spec.strangeness * 0.3;
        let color = tint(pick(spec.prop_colors, rng), spec.strangeness, rng);
        place(kind, grid.world(cell), scale, color, out);
    }
    motif_cell.map(|c| grid.world(c))
}

/// The dream coming apart underneath: tumbling cubes below the floor plane,
/// visible past the edges and through void gaps. Count scales with strangeness.
fn decor(grid: &Grid, spec: &ThemeSpec, rng: &mut StdRng, out: &mut Vec<Block>) {
    let half_w = grid.w as f32 * CELL * 0.75;
    let half_h = grid.h as f32 * CELL * 0.75;
    for _ in 0..(spec.strangeness * 16.0) as usize {
        out.push(Block {
            kind: BlockKind::Decor,
            shape: *[Shape::Cube, Shape::Octahedron, Shape::Orb, Shape::Cone]
                .choose(rng)
                .expect("non-empty"),
            pos: Vec3::new(
                rng.gen_range(-half_w..half_w),
                rng.gen_range(-10.0..-2.0),
                rng.gen_range(-half_h..half_h),
            ),
            size: Vec3::splat(rng.gen_range(0.3..1.5)),
            rotation: Quat::from_euler(
                EulerRot::XYZ,
                rng.gen_range(0.0..TAU),
                rng.gen_range(0.0..TAU),
                rng.gen_range(0.0..TAU),
            ),
            color: tint(pick(spec.prop_colors, rng), spec.strangeness, rng),
        });
    }
}

/// Enemies patrol ON the route (so you have to dodge them), never within the
/// first 3 route cells. Count grows by one every 3 dreams deep.
fn patrols(
    grid: &Grid,
    path: &[(P, bool)],
    spec: &ThemeSpec,
    depth: u32,
    rng: &mut StdRng,
) -> Vec<(Vec3, Vec3)> {
    if spec.enemies.1 == 0 || path.len() < layout::MIN_PATH_CELLS {
        return Vec::new();
    }
    let count = (rng.gen_range(spec.enemies.0..=spec.enemies.1) + depth / 3).min(spec.enemies.1 + 2)
        as usize;
    let mut candidates: Vec<P> = path[3..path.len() - 1]
        .iter()
        .filter(|(_, jump)| !jump)
        .map(|&(p, _)| p)
        .collect();
    candidates.shuffle(rng);
    candidates
        .into_iter()
        .take(count)
        .filter_map(|a| {
            let (b, _) = grid.moves(a).into_iter().find(|&(_, jump)| !jump)?;
            Some((grid.world(a) + Vec3::Y * 0.5, grid.world(b) + Vec3::Y * 0.5))
        })
        .collect()
}

fn jitter3(c: [f32; 3], rng: &mut StdRng) -> [f32; 3] {
    let mut out = c;
    for v in &mut out {
        *v = (*v + rng.gen_range(-0.04..0.04)).clamp(0.0, 1.0);
    }
    out
}

fn atmosphere(spec: &ThemeSpec, rng: &mut StdRng) -> Atmosphere {
    Atmosphere {
        fog_color: jitter3(spec.fog_color, rng),
        ambient: jitter3(spec.ambient, rng),
        fog_start: spec.fog_start,
        fog_end: spec.fog_end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::theme::ALL_THEMES;

    fn near_palette(c: [u8; 4], palette: &[[u8; 3]]) -> bool {
        palette
            .iter()
            .any(|p| (0..3).all(|i| (c[i] as i32 - p[i] as i32).abs() <= 40))
    }

    #[test]
    fn same_seed_same_dream() {
        for theme in ALL_THEMES {
            let (a, b) = (
                generate(theme, 7, 2, None, true),
                generate(theme, 7, 2, None, true),
            );
            assert_eq!(a.blocks.len(), b.blocks.len());
            for (x, y) in a.blocks.iter().zip(&b.blocks) {
                assert_eq!((x.pos, x.color), (y.pos, y.color));
            }
        }
    }

    #[test]
    fn different_seeds_give_different_dreams() {
        for theme in ALL_THEMES {
            let distinct: HashSet<String> = (0..20)
                .map(|s| {
                    let d = generate(theme, s, 0, None, true);
                    format!(
                        "{}:{:?}",
                        d.blocks.len(),
                        d.route
                            .iter()
                            .map(|w| (w.pos.x as i32, w.pos.z as i32))
                            .collect::<Vec<_>>()
                    )
                })
                .collect();
            assert!(
                distinct.len() >= 5,
                "{theme:?}: only {} distinct dreams in 20 seeds",
                distinct.len()
            );
        }
    }

    #[test]
    fn route_runs_spawn_to_portal_in_legal_hops() {
        for theme in ALL_THEMES {
            for seed in 0..100 {
                let d = generate(theme, seed, 0, None, true);
                assert_eq!(d.route.first().unwrap().pos, d.spawn);
                assert_eq!(d.route.last().unwrap().pos, d.portal);
                for hop in d.route.windows(2) {
                    let dist = hop[0].pos.distance(hop[1].pos);
                    let want = if hop[1].jump { 2.0 * CELL } else { CELL };
                    assert!(
                        (dist - want).abs() < 1e-3,
                        "{theme:?}/{seed}: hop of {dist}"
                    );
                }
            }
        }
    }

    #[test]
    fn props_never_block_the_route() {
        for theme in ALL_THEMES {
            for seed in 0..100 {
                let d = generate(theme, seed, 0, Some(PropKind::Machine), true);
                for prop in d.blocks.iter().filter(|b| b.kind == BlockKind::Prop) {
                    for wp in &d.route {
                        let gap = (prop.pos - wp.pos).abs();
                        let reach = prop.size * 0.5 + Vec3::splat(CELL * 0.5);
                        assert!(
                            gap.x >= reach.x - 1e-3 || gap.z >= reach.z - 1e-3,
                            "{theme:?}/{seed}: prop at {:?} intrudes on route cell {:?}",
                            prop.pos,
                            wp.pos
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn solid_blocks_are_axis_aligned() {
        for theme in ALL_THEMES {
            let d = generate(theme, 3, 0, None, true);
            for b in d.blocks.iter().filter(|b| b.kind != BlockKind::Decor) {
                assert_eq!(
                    b.rotation,
                    Quat::IDENTITY,
                    "{theme:?}: rotated {:?}",
                    b.kind
                );
            }
        }
    }

    #[test]
    fn every_colour_comes_from_the_theme_palette() {
        for theme in ALL_THEMES {
            let s = theme.spec();
            for seed in 0..20 {
                for b in &generate(theme, seed, 0, None, true).blocks {
                    let ok = match b.kind {
                        BlockKind::Floor => near_palette(b.color, s.floor_colors),
                        BlockKind::Wall => near_palette(b.color, s.wall_colors),
                        BlockKind::Prop | BlockKind::Decor => near_palette(b.color, s.prop_colors),
                        BlockKind::Portal => b.color == PORTAL_COLOR,
                    };
                    assert!(
                        ok,
                        "{theme:?}/{seed}: off-palette {:?} {:?}",
                        b.kind, b.color
                    );
                }
            }
        }
    }

    #[test]
    fn enemies_only_where_allowed_and_never_near_spawn() {
        for seed in 0..100 {
            assert!(generate(DreamTheme::Lobby, seed, 5, None, true)
                .patrols
                .is_empty());
            let d = generate(DreamTheme::NightmareFactory, seed, 0, None, true);
            assert!(!d.patrols.is_empty(), "factory seed {seed} has no enemies");
            for (a, _) in &d.patrols {
                let i = d
                    .route
                    .iter()
                    .position(|w| (w.pos + Vec3::Y * 0.5).distance(*a) < 1e-3)
                    .expect("patrols start on the route");
                assert!(i >= 3, "enemy only {i} steps from spawn");
            }
        }
    }

    #[test]
    fn atmosphere_stays_near_the_theme() {
        for theme in ALL_THEMES {
            let s = theme.spec();
            let a = generate(theme, 11, 0, None, true).atmosphere;
            for i in 0..3 {
                assert!((a.fog_color[i] - s.fog_color[i]).abs() <= 0.041);
                assert!((a.ambient[i] - s.ambient[i]).abs() <= 0.041);
            }
        }
    }

    /// Drive the autopilot through the REAL physics engine on a generated
    /// dream. Returns Err(description) if the player falls or times out.
    fn traverse(d: &Dream) -> Result<(), String> {
        use crate::gameplay::{autopilot_velocity, JUMP_SPEED, KILL_Y, PLAYER_RADIUS};
        use engine::ecs::Transform;
        use engine::physics::{step, Collider, ColliderShape, PhysicsParams, RigidBody};
        let mut world = hecs::World::new();
        for b in d
            .blocks
            .iter()
            .filter(|b| matches!(b.kind, BlockKind::Floor | BlockKind::Wall | BlockKind::Prop))
        {
            world.spawn((
                Transform {
                    position: b.pos,
                    rotation: Quat::IDENTITY,
                    scale: b.size,
                },
                Collider {
                    shape: ColliderShape::Aabb {
                        half_extents: b.size * 0.5,
                    },
                    is_trigger: false,
                },
            ));
        }
        let p = world.spawn((
            Transform::from_position(d.spawn + Vec3::Y),
            RigidBody::default(),
            Collider {
                shape: ColliderShape::Sphere {
                    radius: PLAYER_RADIUS,
                },
                is_trigger: false,
            },
        ));
        let params = PhysicsParams::default();
        let route = &d.lucid_route;
        let mut i = 0;
        for frame in 0..60 * 120 {
            let pos = world.get::<&Transform>(p).unwrap().position;
            let grounded = world.get::<&RigidBody>(p).unwrap().grounded;
            i = crate::gameplay::advance_waypoint(route.iter().map(|w| w.pos), i, pos, grounded);
            let Some(wp) = route.get(i) else {
                return Ok(());
            };
            {
                let mut body = world.get::<&mut RigidBody>(p).unwrap();
                let v = autopilot_velocity(pos, wp.pos);
                body.velocity.x = v.x;
                body.velocity.z = v.z;
                if wp.jump && body.grounded {
                    body.velocity.y = JUMP_SPEED;
                }
            }
            step(&mut world, 1.0 / 60.0, &params);
            let pos = world.get::<&Transform>(p).unwrap().position;
            if pos.y < KILL_Y {
                let prev = route[i.saturating_sub(1)];
                return Err(format!(
                    "fell at frame {frame} heading to waypoint {i}/{} {:?} (jump={}) from {:?}",
                    route.len(),
                    wp.pos,
                    wp.jump,
                    prev.pos
                ));
            }
        }
        Err(format!("timed out at waypoint {i}/{}", route.len()))
    }

    #[test]
    fn every_route_is_physically_traversable() {
        let mut failures = Vec::new();
        for theme in ALL_THEMES {
            for seed in 0..25 {
                for depth in [0, 3, 10] {
                    let d = generate(theme, seed, depth, Some(PropKind::Pillar), true);
                    if let Err(e) = traverse(&d) {
                        failures.push(format!("{theme:?} seed={seed} depth={depth}: {e}"));
                    }
                }
            }
        }
        assert!(
            failures.is_empty(),
            "{} failures:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn strangeness_grows_with_depth_and_caps() {
        let a = strangeness(DreamTheme::LiminalOffice, 0);
        let b = strangeness(DreamTheme::LiminalOffice, 5);
        assert!(b > a);
        assert_eq!(strangeness(DreamTheme::LiminalOffice, 1000), 1.0);
        assert_eq!(strangeness(DreamTheme::Awakening, 50), 0.0);
    }

    #[test]
    fn deep_dreams_bleed_accent_colours_into_textures() {
        let shallow = generate(DreamTheme::LiminalOffice, 4, 0, None, true);
        let deep = generate(DreamTheme::LiminalOffice, 4, 12, None, true);
        let acc = DreamTheme::LiminalOffice.spec().accents;
        assert!(!shallow
            .surfaces
            .floor
            .palette
            .iter()
            .any(|c| acc.contains(c)));
        assert!(deep.surfaces.floor.palette.iter().any(|c| acc.contains(c)));
        assert!(deep.surfaces.floor.bands > shallow.surfaces.floor.bands);
    }

    #[test]
    fn surfaces_use_the_theme_patterns() {
        for theme in ALL_THEMES {
            let d = generate(theme, 21, 3, None, true);
            let pats = theme.spec().patterns;
            for s in [&d.surfaces.floor, &d.surfaces.wall, &d.surfaces.prop] {
                assert!(pats.contains(&s.pattern), "{theme:?}: {:?}", s.pattern);
            }
        }
    }

    #[test]
    fn shard_is_off_route_reachable_and_unblocked() {
        let mut with_shard = 0;
        for theme in ALL_THEMES {
            for seed in 0..60 {
                let d = generate(theme, seed, 2, Some(PropKind::Pillar), true);
                let Some(shard) = d.shard else { continue };
                with_shard += 1;
                assert!(
                    !d.route.iter().any(|w| w.pos == shard),
                    "{theme:?}/{seed}: shard on route"
                );
                assert!(d.lucid_route.iter().any(|w| w.pos == shard));
                assert_eq!(d.lucid_route.first().unwrap().pos, d.spawn);
                assert_eq!(d.lucid_route.last().unwrap().pos, d.portal);
                for hop in d.lucid_route.windows(2) {
                    let dist = hop[0].pos.distance(hop[1].pos);
                    let want = if hop[1].jump { 2.0 * CELL } else { CELL };
                    assert!((dist - want).abs() < 1e-3, "{theme:?}/{seed}: hop {dist}");
                }
                for prop in d.blocks.iter().filter(|b| b.kind == BlockKind::Prop) {
                    for wp in &d.lucid_route {
                        let gap = (prop.pos - wp.pos).abs();
                        let reach = prop.size * 0.5 + Vec3::splat(CELL * 0.5);
                        assert!(
                            gap.x >= reach.x - 1e-3 || gap.z >= reach.z - 1e-3,
                            "{theme:?}/{seed}: prop blocks lucid route"
                        );
                    }
                }
            }
        }
        assert!(with_shard > 250, "only {with_shard}/360 dreams got a shard");
    }

    #[test]
    fn no_shard_when_not_asked() {
        let d = generate(DreamTheme::Garden, 1, 1, None, false);
        assert!(d.shard.is_none());
        assert_eq!(d.lucid_route.len(), d.route.len());
    }

    #[test]
    fn previous_dreams_motif_appears_next_to_spawn() {
        for seed in 0..50 {
            let d = generate(DreamTheme::Lobby, seed, 1, Some(PropKind::Tree), true);
            let at = d.motif_at.expect("the lobby always has free floor");
            assert!(
                at.distance(d.spawn) <= 2.0 * CELL + 1e-3,
                "motif {at:?} far from spawn {:?}",
                d.spawn
            );
            assert!(d.blocks.iter().any(|b| b.kind == BlockKind::Prop
                && (b.pos.x - at.x).abs() < 1e-3
                && (b.pos.z - at.z).abs() < 1e-3));
        }
    }

    #[test]
    fn shapes_are_assigned_sensibly() {
        use crate::dream::meshes::Shape;
        let mut decor = HashSet::new();
        for seed in 0..10 {
            let d = generate(DreamTheme::VoidPlatforms, seed, 8, None, true);
            for b in &d.blocks {
                match b.kind {
                    BlockKind::Floor | BlockKind::Wall => assert_eq!(b.shape, Shape::Cube),
                    BlockKind::Portal => assert_eq!(b.shape, Shape::Orb),
                    BlockKind::Decor => {
                        decor.insert(b.shape);
                    }
                    BlockKind::Prop => {}
                }
            }
        }
        assert!(decor.len() >= 3, "decor shapes: {decor:?}");
    }
}
