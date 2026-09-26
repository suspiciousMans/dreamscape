//! Set dressing that makes a dream read as a *place*: wall caps, skirting,
//! corner posts, floor inlays, ceilings (first-person dreams) and far-off
//! vistas. Everything is `BlockKind::Trim`: static, never solid, never in the
//! physics world, so it can't block a route. Uses its own RNG stream so adding
//! it doesn't change any existing dream.

use super::build::{pick, tint, Block, BlockKind};
use super::grid::{Cell, Grid, P};
use super::meshes::Shape;
use super::theme::ThemeSpec;
use crate::gameplay::CELL;
use engine::glam::{Quat, Vec3};
use rand::{rngs::StdRng, seq::SliceRandom, Rng};
use std::collections::HashSet;

pub const CAP_H: f32 = 0.18;
pub const SKIRT_H: f32 = 0.22;
pub const SKIRT_T: f32 = 0.08;
pub const INLAY_Y: f32 = 0.015;
pub const CEILING_T: f32 = 0.3;
/// Each block is an entity and a draw call; keep dressing bounded.
pub const MAX_DETAIL: usize = 700;

const DIRS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

fn shade(c: [u8; 4], k: f32) -> [u8; 4] {
    let f = |v: u8| (v as f32 * k).clamp(0.0, 255.0) as u8;
    [f(c[0]), f(c[1]), f(c[2]), c[3]]
}

fn trim(shape: Shape, pos: Vec3, size: Vec3, rotation: Quat, color: [u8; 4]) -> Block {
    Block {
        kind: BlockKind::Trim,
        shape,
        pos,
        size,
        rotation,
        color,
    }
}

/// Maximal runs of consecutive cells along one line that satisfy `want`:
/// (first cell, last cell, length).
fn runs(len: i32, at: impl Fn(i32) -> P, want: impl Fn(P) -> bool) -> Vec<(P, P, i32)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < len {
        if !want(at(i)) {
            i += 1;
            continue;
        }
        let s = i;
        while i < len && want(at(i)) {
            i += 1;
        }
        out.push((at(s), at(i - 1), i - s));
    }
    out
}

/// `skip`: floor cells that move (crumbling, shifting, dissolving): no inlays on them.
pub fn details(
    grid: &Grid,
    spec: &ThemeSpec,
    skip: &HashSet<P>,
    first_person: bool,
    rng: &mut StdRng,
    out: &mut Vec<Block>,
) {
    let mut d = Vec::new();
    if spec.wall_height > 0.0 {
        if first_person {
            ceiling(grid, spec, rng, &mut d);
        } else {
            wall_caps(grid, spec, rng, &mut d);
        }
        corner_posts(grid, spec, rng, &mut d);
        skirting(grid, spec, rng, &mut d);
    }
    if !first_person {
        vistas(grid, spec, rng, &mut d);
    }
    inlays(grid, spec, skip, rng, &mut d);
    d.truncate(MAX_DETAIL); // inlays (least important) go first
    out.extend(d);
}

fn wall_caps(grid: &Grid, spec: &ThemeSpec, rng: &mut StdRng, out: &mut Vec<Block>) {
    for y in 0..grid.h {
        for (a, b, n) in runs(grid.w, |x| (x, y), |p| grid.get(p) == Cell::Wall) {
            let c = (grid.world(a) + grid.world(b)) * 0.5;
            let col = shade(
                tint(pick(spec.wall_colors, rng), spec.strangeness, rng),
                1.3,
            );
            out.push(trim(
                Shape::Cube,
                c + Vec3::Y * (spec.wall_height + CAP_H * 0.5),
                Vec3::new(n as f32 * CELL + 0.12, CAP_H, CELL + 0.12),
                Quat::IDENTITY,
                col,
            ));
        }
    }
}

/// First-person dreams: a lid over every floor run at the wall tops.
fn ceiling(grid: &Grid, spec: &ThemeSpec, rng: &mut StdRng, out: &mut Vec<Block>) {
    for y in 0..grid.h {
        for (a, b, n) in runs(grid.w, |x| (x, y), |p| grid.get(p) == Cell::Floor) {
            let c = (grid.world(a) + grid.world(b)) * 0.5;
            let col = shade(
                tint(pick(spec.wall_colors, rng), spec.strangeness, rng),
                0.7,
            );
            out.push(trim(
                Shape::Cube,
                c + Vec3::Y * (spec.wall_height + CEILING_T * 0.5),
                Vec3::new(n as f32 * CELL, CEILING_T, CELL),
                Quat::IDENTITY,
                col,
            ));
        }
    }
}

fn skirting(grid: &Grid, spec: &ThemeSpec, rng: &mut StdRng, out: &mut Vec<Block>) {
    let col = shade(
        tint(pick(spec.wall_colors, rng), spec.strangeness, rng),
        0.55,
    );
    for (dx, dy) in DIRS {
        let n = Vec3::new(dx as f32, 0.0, dy as f32);
        let faces =
            |p: P| grid.get(p) == Cell::Floor && grid.get((p.0 + dx, p.1 + dy)) == Cell::Wall;
        let lift = n * (CELL * 0.5 - SKIRT_T * 0.5) + Vec3::Y * SKIRT_H * 0.5;
        if dy != 0 {
            for y in 0..grid.h {
                for (a, b, len) in runs(grid.w, |x| (x, y), &faces) {
                    let c = (grid.world(a) + grid.world(b)) * 0.5 + lift;
                    out.push(trim(
                        Shape::Cube,
                        c,
                        Vec3::new(len as f32 * CELL, SKIRT_H, SKIRT_T),
                        Quat::IDENTITY,
                        col,
                    ));
                }
            }
        } else {
            for x in 0..grid.w {
                for (a, b, len) in runs(grid.h, |y| (x, y), &faces) {
                    let c = (grid.world(a) + grid.world(b)) * 0.5 + lift;
                    out.push(trim(
                        Shape::Cube,
                        c,
                        Vec3::new(SKIRT_T, SKIRT_H, len as f32 * CELL),
                        Quat::IDENTITY,
                        col,
                    ));
                }
            }
        }
    }
}

/// A post on every outside wall corner (both sides and the diagonal are floor).
fn corner_posts(grid: &Grid, spec: &ThemeSpec, rng: &mut StdRng, out: &mut Vec<Block>) {
    let col = shade(
        tint(pick(spec.wall_colors, rng), spec.strangeness, rng),
        1.15,
    );
    let floor = |p: P| grid.get(p) == Cell::Floor;
    let h = spec.wall_height + 0.3;
    for w in grid.cells_of(Cell::Wall) {
        for (dx, dy) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
            if floor((w.0 + dx, w.1)) && floor((w.0, w.1 + dy)) && floor((w.0 + dx, w.1 + dy)) {
                let at = grid.world(w) + Vec3::new(dx as f32, 0.0, dy as f32) * (CELL * 0.5);
                out.push(trim(
                    Shape::Cylinder,
                    at + Vec3::Y * h * 0.5,
                    Vec3::new(0.4, h, 0.4),
                    Quat::IDENTITY,
                    col,
                ));
            }
        }
    }
}

fn inlays(
    grid: &Grid,
    spec: &ThemeSpec,
    skip: &HashSet<P>,
    rng: &mut StdRng,
    out: &mut Vec<Block>,
) {
    let chance = (0.08 + 0.3 * spec.strangeness as f64).min(0.45);
    let diamond = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
    for c in grid.cells_of(Cell::Floor) {
        if skip.contains(&c) || !rng.gen_bool(chance) {
            continue;
        }
        let col = tint(pick(spec.accents, rng), spec.strangeness, rng);
        let (rot, s) = if rng.gen_bool(0.5) {
            (diamond, CELL * 0.5)
        } else {
            (Quat::IDENTITY, CELL * 0.7)
        };
        out.push(trim(
            Shape::Cube,
            grid.world(c) + Vec3::Y * INLAY_Y,
            Vec3::new(s, 0.02, s),
            rot,
            col,
        ));
    }
}

/// Three colossal copies of the dream's own props, half-sunk beyond the edges.
pub(super) fn vistas(grid: &Grid, spec: &ThemeSpec, rng: &mut StdRng, out: &mut Vec<Block>) {
    let (hw, hh) = (grid.w as f32 * CELL * 0.5, grid.h as f32 * CELL * 0.5);
    for base in [
        Vec3::new(-hw - 12.0, 0.0, 0.0),
        Vec3::new(hw + 12.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, hh + 16.0),
    ] {
        let kind = *spec
            .props
            .choose(rng)
            .expect("theme props are non-empty (theme tests)");
        let scale = rng.gen_range(4.0..7.0);
        let slide = rng.gen_range(-0.5..0.5);
        let along = if base.z == 0.0 {
            Vec3::Z * hh * slide
        } else {
            Vec3::X * hw * slide
        };
        let at = base + along + Vec3::Y * rng.gen_range(-5.0..-1.5);
        let col = shade(
            tint(pick(spec.prop_colors, rng), spec.strangeness, rng),
            0.8,
        );
        for &(offset, size, shape) in kind.parts() {
            out.push(trim(
                shape,
                at + Vec3::from(offset) * scale,
                Vec3::from(size) * scale,
                Quat::IDENTITY,
                col,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::layout;
    use crate::dream::theme::ALL_THEMES;
    use rand::SeedableRng;

    fn dressed(t: crate::dream::DreamTheme, seed: u64, fp: bool) -> (Grid, Vec<Block>) {
        let spec = t.spec();
        let mut rng = StdRng::seed_from_u64(seed);
        let l = layout::generate(spec.layout, spec.grid_size, &mut rng);
        let mut out = Vec::new();
        details(&l.grid, &spec, &HashSet::new(), fp, &mut rng, &mut out);
        (l.grid, out)
    }

    #[test]
    fn details_are_trim_only_bounded_and_present() {
        for t in ALL_THEMES {
            for seed in 0..20 {
                for fp in [false, true] {
                    let (_, d) = dressed(t, seed, fp);
                    assert!(!d.is_empty(), "{t:?}/{seed}: undressed");
                    assert!(d.len() <= MAX_DETAIL, "{t:?}/{seed}: {} blocks", d.len());
                    assert!(d.iter().all(|b| b.kind == BlockKind::Trim));
                }
            }
        }
    }

    #[test]
    fn details_are_deterministic() {
        for t in ALL_THEMES {
            let pos = |d: Vec<Block>| d.iter().map(|b| b.pos.to_array()).collect::<Vec<_>>();
            assert_eq!(pos(dressed(t, 7, false).1), pos(dressed(t, 7, false).1));
        }
    }

    /// Over a floor cell's middle, dressing is either at your feet (inlays,
    /// skirting) or at/above the wall tops (caps, ceilings), never at body height.
    #[test]
    fn trim_never_hangs_at_body_height_over_the_floor() {
        for t in ALL_THEMES {
            let wall = t.spec().wall_height;
            for seed in 0..10 {
                for fp in [false, true] {
                    let (g, d) = dressed(t, seed, fp);
                    for c in g.cells_of(Cell::Floor) {
                        let m = g.world(c);
                        for b in &d {
                            let (lo, hi) = (b.pos - b.size * 0.5, b.pos + b.size * 0.5);
                            let over = lo.x < m.x + CELL * 0.3
                                && hi.x > m.x - CELL * 0.3
                                && lo.z < m.z + CELL * 0.3
                                && hi.z > m.z - CELL * 0.3;
                            assert!(
                                !over || hi.y <= 0.3 || lo.y >= wall - 1e-3,
                                "{t:?}/{seed}: trim {:?} {:?} over floor {c:?}",
                                b.shape,
                                b.pos
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn vistas_stand_outside_the_dream() {
        for t in ALL_THEMES {
            let spec = t.spec();
            let mut rng = StdRng::seed_from_u64(1);
            let g = layout::generate(spec.layout, spec.grid_size, &mut rng).grid;
            let (hw, hh) = (g.w as f32 * CELL * 0.5, g.h as f32 * CELL * 0.5);
            let mut v = Vec::new();
            vistas(&g, &spec, &mut rng, &mut v);
            assert!(!v.is_empty());
            for b in &v {
                let (lo, hi) = (b.pos - b.size * 0.5, b.pos + b.size * 0.5);
                assert!(
                    hi.x < -hw || lo.x > hw || lo.z > hh,
                    "{t:?}: vista inside at {:?}",
                    b.pos
                );
            }
        }
    }

    #[test]
    fn first_person_dreams_have_a_lid_over_every_floor_cell() {
        for t in ALL_THEMES
            .into_iter()
            .filter(|t| t.spec().wall_height > 0.0)
        {
            let wall = t.spec().wall_height;
            let (g, d) = dressed(t, 3, true);
            for c in g.cells_of(Cell::Floor) {
                let m = g.world(c);
                assert!(
                    d.iter()
                        .any(|b| (b.pos.y - b.size.y * 0.5 - wall).abs() < 1e-3
                            && (b.pos.x - m.x).abs() <= b.size.x * 0.5
                            && (b.pos.z - m.z).abs() <= b.size.z * 0.5),
                    "{t:?}: no ceiling over {c:?}"
                );
            }
        }
    }
}
