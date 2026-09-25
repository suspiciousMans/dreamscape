//! Layout algorithms. Each produces a grid + spawn + portal; `generate` only
//! returns layouts where the portal is reachable (walking + one-cell jumps).

use super::grid::{Cell, Grid, P};
use super::theme::LayoutKind;
use rand::{rngs::StdRng, seq::SliceRandom, Rng};

/// Minimum route length (cells) so the portal is never right next to spawn.
pub const MIN_PATH_CELLS: usize = 6;
const ATTEMPTS: usize = 20;

pub struct Layout {
    pub grid: Grid,
    pub spawn: P,
    pub portal: P,
    /// True if every attempt failed and we used the always-valid hall.
    pub fallback: bool,
}

pub fn generate(kind: LayoutKind, size: (i32, i32), rng: &mut StdRng) -> Layout {
    for _ in 0..ATTEMPTS {
        let side = rng.gen_range(size.0..=size.1);
        let layout = match kind {
            LayoutKind::OpenHall => open_hall(side, rng),
            LayoutKind::Maze => maze(side, rng),
            LayoutKind::PlatformChain => platform_chain(side, rng),
            LayoutKind::ScatterField => scatter_field(side, rng),
            LayoutKind::Spiral => spiral(side, rng),
            LayoutKind::Mirrored => mirrored(side, rng),
            LayoutKind::Network => network(side, rng),
            LayoutKind::Corridor => corridor(side, rng),
            LayoutKind::Recursive => recursive(side, rng),
        };
        if layout
            .grid
            .path(layout.spawn, layout.portal)
            .is_some_and(|path| path.len() >= MIN_PATH_CELLS)
        {
            return layout;
        }
    }
    Layout {
        fallback: true,
        ..open_hall(9, rng)
    }
}

/// Floor everywhere, walls around the edge.
fn bordered(side: i32) -> Grid {
    let mut g = Grid::new(side, side, Cell::Floor);
    for i in 0..side {
        for p in [(i, 0), (i, side - 1), (0, i), (side - 1, i)] {
            g.set(p, Cell::Wall);
        }
    }
    g
}

/// Big room with pillars; the middle column is always clear, so side >= 9 always passes.
fn open_hall(side: i32, rng: &mut StdRng) -> Layout {
    let mut g = bordered(side);
    let mid = side / 2;
    for y in (2..side - 2).step_by(3) {
        for x in (2..side - 2).step_by(3) {
            if x != mid && rng.gen_bool(0.6) {
                g.set((x, y), Cell::Wall);
            }
        }
    }
    Layout {
        grid: g,
        spawn: (mid, 1),
        portal: (mid, side - 2),
        fallback: false,
    }
}

/// Recursive-backtracker maze with some walls knocked out (liminal loops).
/// Portal goes on the floor cell farthest from spawn.
fn maze(side: i32, rng: &mut StdRng) -> Layout {
    let side = side | 1; // mazes need odd sides
    let mut g = Grid::new(side, side, Cell::Wall);
    let start = (1, 1);
    g.set(start, Cell::Floor);
    let mut stack = vec![start];
    while let Some(&(x, y)) = stack.last() {
        let mut dirs = [(2, 0), (-2, 0), (0, 2), (0, -2)];
        dirs.shuffle(rng);
        let next = dirs
            .iter()
            .map(|&(dx, dy)| ((x + dx, y + dy), (x + dx / 2, y + dy / 2)))
            .find(|&(n, _)| {
                n.0 > 0 && n.1 > 0 && n.0 < side - 1 && n.1 < side - 1 && g.get(n) == Cell::Wall
            });
        match next {
            Some((n, between)) => {
                g.set(between, Cell::Floor);
                g.set(n, Cell::Floor);
                stack.push(n);
            }
            None => {
                stack.pop();
            }
        }
    }
    for _ in 0..side {
        let p = (rng.gen_range(1..side - 1), rng.gen_range(1..side - 1));
        let opens_h = g.get((p.0 - 1, p.1)) == Cell::Floor && g.get((p.0 + 1, p.1)) == Cell::Floor;
        let opens_v = g.get((p.0, p.1 - 1)) == Cell::Floor && g.get((p.0, p.1 + 1)) == Cell::Floor;
        if g.get(p) == Cell::Wall && (opens_h || opens_v) {
            g.set(p, Cell::Floor);
        }
    }
    let portal = g
        .cells_of(Cell::Floor)
        .into_iter()
        .max_by_key(|&p| g.path(start, p).map_or(0, |path| path.len()))
        .unwrap_or(start);
    Layout {
        grid: g,
        spawn: start,
        portal,
        fallback: false,
    }
}

/// Floating platforms in a void: a random walk that sometimes leaves a
/// one-cell gap to jump. No walls. Portal at the end of the walk.
fn platform_chain(side: i32, rng: &mut StdRng) -> Layout {
    let mut g = Grid::new(side, side, Cell::Void);
    let spawn = (side / 2, 1);
    let mut p = spawn;
    g.set(p, Cell::Floor);
    let target = side as usize + 4;
    let mut placed = 1;
    for _ in 0..500 {
        if placed >= target {
            break;
        }
        // (0, 1) listed twice: the walk drifts away from spawn.
        let (dx, dy) = *[(1, 0), (-1, 0), (0, 1), (0, 1)]
            .choose(rng)
            .expect("non-empty");
        let reach = if rng.gen_bool(0.35) { 2 } else { 1 };
        let n = (p.0 + dx * reach, p.1 + dy * reach);
        if n.0 < 1 || n.1 < 1 || n.0 > side - 2 || n.1 > side - 2 {
            continue;
        }
        if g.get(n) == Cell::Void {
            placed += 1;
        }
        g.set(n, Cell::Floor);
        p = n;
    }
    for c in g.cells_of(Cell::Floor) {
        let side_cell = (c.0 + 1, c.1);
        if rng.gen_bool(0.3) && g.get(side_cell) == Cell::Void {
            g.set(side_cell, Cell::Floor); // widen into little islands
        }
    }
    Layout {
        grid: g,
        spawn,
        portal: p,
        fallback: false,
    }
}

/// Open field with short hedge-wall segments scattered through it.
fn scatter_field(side: i32, rng: &mut StdRng) -> Layout {
    let mut g = bordered(side);
    for _ in 0..side {
        let horizontal = rng.gen_bool(0.5);
        let len = rng.gen_range(2..=3);
        let (x0, y0) = (rng.gen_range(2..side - 2), rng.gen_range(2..side - 2));
        for i in 0..len {
            let p = if horizontal {
                (x0 + i, y0)
            } else {
                (x0, y0 + i)
            };
            if p.0 < side - 1 && p.1 < side - 1 {
                g.set(p, Cell::Wall);
            }
        }
    }
    let (spawn, portal) = ((1, 1), (side - 2, side - 2));
    g.set(spawn, Cell::Floor);
    g.set(portal, Cell::Floor);
    Layout {
        grid: g,
        spawn,
        portal,
        fallback: false,
    }
}

/// A square spiral of walkways over the void, winding in from the corner
/// to the portal in the middle. Arms are two void cells apart (too far to
/// jump, so you really do walk the spiral), and a few steps along the arms
/// are missing and must be jumped.
fn spiral(side: i32, rng: &mut StdRng) -> Layout {
    let mut g = Grid::new(side, side, Cell::Void);
    let (mut x0, mut y0, mut x1, mut y1) = (1, 1, side - 2, side - 2);
    let spawn = (1, 1);
    let mut p = spawn;
    let mut arms: Vec<Vec<P>> = Vec::new();
    loop {
        let arm: Vec<P> = (p.0..=x1).map(|x| (x, y0)).collect();
        p = (x1, y0);
        arms.push(arm);
        y0 += 3;
        if y0 > y1 {
            break;
        }
        let arm: Vec<P> = (p.1..=y1).map(|y| (x1, y)).collect();
        p = (x1, y1);
        arms.push(arm);
        x1 -= 3;
        if x1 < x0 {
            break;
        }
        let arm: Vec<P> = (x0..=p.0).rev().map(|x| (x, y1)).collect();
        p = (x0, y1);
        arms.push(arm);
        y1 -= 3;
        if y1 < y0 {
            break;
        }
        let arm: Vec<P> = (y0..=p.1).rev().map(|y| (x0, y)).collect();
        p = (x0, y0);
        arms.push(arm);
        x0 += 3;
        if x0 > x1 {
            break;
        }
    }
    for arm in &arms {
        for &c in arm {
            g.set(c, Cell::Floor);
        }
    }
    // Missing steps: never at an arm's ends (corners stay solid), never two
    // in a row, never next to spawn or the portal.
    for arm in &arms {
        let mut last_gap = 0;
        let inner = arm.len().saturating_sub(2);
        for (k, &c) in arm.iter().enumerate().take(inner).skip(2) {
            if k > last_gap + 2 && c != spawn && c != p && rng.gen_bool(0.14) {
                g.set(c, Cell::Void);
                last_gap = k;
            }
        }
    }
    Layout {
        grid: g,
        spawn,
        portal: p,
        fallback: false,
    }
}

/// A hall of mirrors: walls on the left half are reflected onto the right.
/// Crossing walls every few rows only open on the sides (never the middle),
/// so the route zig-zags between reflections.
fn mirrored(side: i32, rng: &mut StdRng) -> Layout {
    let side = side | 1; // an odd side has a true middle column
    let mut g = bordered(side);
    let mid = side / 2;
    let mirror = |x: i32| side - 1 - x;
    for y in (3..side - 2).step_by(3) {
        for x in 1..side - 1 {
            g.set((x, y), Cell::Wall);
        }
        let door = rng.gen_range(1..mid);
        g.set((door, y), Cell::Floor);
        g.set((mirror(door), y), Cell::Floor);
    }
    // Reflected pillars between the crossing walls.
    for _ in 0..side / 2 {
        let (x, y) = (rng.gen_range(2..mid), rng.gen_range(2..side - 2));
        if y % 3 != 0 && g.get((x, y - 1)) != Cell::Wall && g.get((x, y + 1)) != Cell::Wall {
            g.set((x, y), Cell::Wall);
            g.set((mirror(x), y), Cell::Wall);
        }
    }
    let (spawn, portal) = ((mid, 1), (mid, side - 2));
    g.set(spawn, Cell::Floor);
    g.set(portal, Cell::Floor);
    Layout {
        grid: g,
        spawn,
        portal,
        fallback: false,
    }
}

/// Mycelium: round-ish clearings floating in the void, each joined to the
/// next by a one-cell root bridge. Spawn in the first, portal in the last.
fn network(side: i32, rng: &mut StdRng) -> Layout {
    let mut g = Grid::new(side, side, Cell::Void);
    let n = rng.gen_range(4..=6);
    let mut centres: Vec<P> = Vec::new();
    for _ in 0..200 {
        if centres.len() >= n {
            break;
        }
        let c = (rng.gen_range(2..side - 2), rng.gen_range(2..side - 2));
        if centres
            .iter()
            .all(|o| (o.0 - c.0).abs() + (o.1 - c.1).abs() >= 5)
        {
            centres.push(c);
        }
    }
    // Visit clearings nearest-first from the one closest to a corner.
    centres.sort_by_key(|c| c.0 + c.1);
    let mut ordered = vec![centres.remove(0)];
    while !centres.is_empty() {
        let last = *ordered.last().expect("non-empty");
        let (i, _) = centres
            .iter()
            .enumerate()
            .min_by_key(|(_, c)| (c.0 - last.0).abs() + (c.1 - last.1).abs())
            .expect("non-empty");
        ordered.push(centres.remove(i));
    }
    for &c in &ordered {
        let r = rng.gen_range(1..=2);
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r + 1 {
                    let p = (c.0 + dx, c.1 + dy);
                    if p.0 > 0 && p.1 > 0 && p.0 < side - 1 && p.1 < side - 1 {
                        g.set(p, Cell::Floor);
                    }
                }
            }
        }
    }
    // Root bridges: an L from each clearing to the next.
    for w in ordered.windows(2) {
        let (a, b) = (w[0], w[1]);
        let corner = if rng.gen_bool(0.5) {
            (b.0, a.1)
        } else {
            (a.0, b.1)
        };
        for (from, to) in [(a, corner), (corner, b)] {
            let (dx, dy) = ((to.0 - from.0).signum(), (to.1 - from.1).signum());
            let mut p = from;
            g.set(p, Cell::Floor);
            while p != to {
                p = (p.0 + dx, p.1 + dy);
                g.set(p, Cell::Floor);
            }
        }
    }
    Layout {
        spawn: ordered[0],
        portal: *ordered.last().expect("non-empty"),
        grid: g,
        fallback: false,
    }
}

/// The Tunnel: one corridor snaking back and forth across the dream, walls
/// on both sides, with the odd alcove off to the side.
fn corridor(side: i32, rng: &mut StdRng) -> Layout {
    let side = side | 1;
    let mut g = Grid::new(side, side, Cell::Wall);
    let spawn = (1, 1);
    let mut p = spawn;
    g.set(p, Cell::Floor);
    let mut right = true;
    let mut y = 1;
    loop {
        // Across: all the way, or stopping a little short.
        let end = if right {
            side - 2 - rng.gen_range(0..=1)
        } else {
            1 + rng.gen_range(0..=1)
        };
        while p.0 != end {
            p.0 += if right { 1 } else { -1 };
            g.set(p, Cell::Floor);
        }
        if y + 2 > side - 2 {
            break;
        }
        // Down two (one wall row between the passes).
        for _ in 0..2 {
            y += 1;
            p.1 = y;
            g.set(p, Cell::Floor);
        }
        right = !right;
    }
    let portal = p;
    // Alcoves: single cells off the corridor, into the wall rows.
    for c in g.cells_of(Cell::Floor) {
        for d in [(0, 1), (0, -1)] {
            let a = (c.0 + d.0, c.1 + d.1);
            let beyond = (a.0 + d.0, a.1 + d.1);
            if a.1 > 0
                && a.1 < side - 1
                && g.get(a) == Cell::Wall
                && g.get(beyond) != Cell::Floor
                && rng.gen_bool(0.08)
            {
                g.set(a, Cell::Floor);
            }
        }
    }
    Layout {
        grid: g,
        spawn,
        portal,
        fallback: false,
    }
}

/// Fractal Cathedral: square rooms nested inside each other, each with one
/// door on a random side. The portal waits in the innermost room.
fn recursive(side: i32, rng: &mut StdRng) -> Layout {
    let side = side | 1;
    let mut g = bordered(side);
    let mid = side / 2;
    let mut k = 2;
    while side - 1 - 2 * k >= 2 {
        let (lo, hi) = (k, side - 1 - k);
        for i in lo..=hi {
            for p in [(i, lo), (i, hi), (lo, i), (hi, i)] {
                g.set(p, Cell::Wall);
            }
        }
        // A door somewhere along one side (never a corner).
        let along = rng.gen_range(lo + 1..hi);
        let door = match rng.gen_range(0..4) {
            0 => (along, lo),
            1 => (along, hi),
            2 => (lo, along),
            _ => (hi, along),
        };
        g.set(door, Cell::Floor);
        k += 2;
    }
    let (spawn, portal) = ((1, 1), (mid, mid));
    g.set(portal, Cell::Floor);
    Layout {
        grid: g,
        spawn,
        portal,
        fallback: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    const KINDS: [LayoutKind; 9] = [
        LayoutKind::Network,
        LayoutKind::Corridor,
        LayoutKind::Recursive,
        LayoutKind::OpenHall,
        LayoutKind::Maze,
        LayoutKind::PlatformChain,
        LayoutKind::ScatterField,
        LayoutKind::Spiral,
        LayoutKind::Mirrored,
    ];

    fn gen(kind: LayoutKind, seed: u64) -> Layout {
        generate(kind, (9, 15), &mut StdRng::seed_from_u64(seed))
    }

    #[test]
    fn every_layout_has_a_long_enough_route() {
        for kind in KINDS {
            for seed in 0..300 {
                let l = gen(kind, seed);
                let path = l
                    .grid
                    .path(l.spawn, l.portal)
                    .unwrap_or_else(|| panic!("{kind:?} seed {seed}: portal unreachable"));
                assert!(
                    path.len() >= MIN_PATH_CELLS,
                    "{kind:?} seed {seed}: route {}",
                    path.len()
                );
            }
        }
    }

    #[test]
    fn fallback_is_rare() {
        for kind in KINDS {
            let n = (0..300).filter(|&s| gen(kind, s).fallback).count();
            assert!(
                n < 15,
                "{kind:?} fell back {n}/300 times — the algorithm needs fixing, not the test"
            );
        }
    }

    #[test]
    fn mirror_halls_are_symmetric() {
        for seed in 0..50 {
            let g = gen(LayoutKind::Mirrored, seed).grid;
            for y in 0..g.h {
                for x in 0..g.w {
                    assert_eq!(g.get((x, y)), g.get((g.w - 1 - x, y)), "seed {seed}");
                }
            }
        }
    }

    #[test]
    fn spirals_end_in_the_middle_and_have_gaps_to_jump() {
        let mut jumps = 0;
        for seed in 0..50 {
            let l = gen(LayoutKind::Spiral, seed);
            let c = (l.grid.w / 2, l.grid.h / 2);
            assert!(
                (l.portal.0 - c.0).abs() <= 3 && (l.portal.1 - c.1).abs() <= 3,
                "seed {seed}: portal {:?} not near the middle {c:?}",
                l.portal
            );
            jumps += l
                .grid
                .path(l.spawn, l.portal)
                .unwrap()
                .iter()
                .filter(|s| s.1)
                .count();
        }
        assert!(jumps > 20, "spirals barely need jumping ({jumps})");
    }

    #[test]
    fn corridors_are_long_and_cathedrals_nest() {
        for seed in 0..40 {
            let l = gen(LayoutKind::Corridor, seed);
            let path = l.grid.path(l.spawn, l.portal).unwrap();
            assert!(
                path.len() as i32 >= l.grid.w * 2,
                "seed {seed}: corridor only {} long",
                path.len()
            );
            let c = gen(LayoutKind::Recursive, seed);
            let (mid, w) = (c.grid.w / 2, c.grid.w);
            assert_eq!(c.portal, (mid, mid));
            // Walk from the portal outward: you cross a wall ring every 2 cells.
            let walls = (1..mid)
                .filter(|&x| c.grid.get((mid - x, mid)) == Cell::Wall)
                .count();
            assert!(walls >= 1 || w < 9, "seed {seed}: no nested rooms");
        }
    }

    #[test]
    fn same_seed_same_layout() {
        for kind in KINDS {
            let (a, b) = (gen(kind, 99), gen(kind, 99));
            assert_eq!(a.grid, b.grid);
            assert_eq!((a.spawn, a.portal), (b.spawn, b.portal));
        }
    }
}
