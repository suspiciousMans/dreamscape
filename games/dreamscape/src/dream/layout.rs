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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    const KINDS: [LayoutKind; 4] = [
        LayoutKind::OpenHall,
        LayoutKind::Maze,
        LayoutKind::PlatformChain,
        LayoutKind::ScatterField,
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
    fn same_seed_same_layout() {
        for kind in KINDS {
            let (a, b) = (gen(kind, 99), gen(kind, 99));
            assert_eq!(a.grid, b.grid);
            assert_eq!((a.spawn, a.portal), (b.spawn, b.portal));
        }
    }
}
