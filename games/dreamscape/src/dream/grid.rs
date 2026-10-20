//! A dream's floor plan: a 2D grid of cells, plus walkability/pathfinding.

use crate::gameplay::CELL;
use engine::glam::Vec3;
use std::collections::VecDeque;

pub type P = (i32, i32);

/// How high one terrace level is (world units). A plain jump (apex ~1.8,
/// and still ~0.85 on a bad day) always clears one level; three levels
/// (2.1) is a cliff that needs an ability, a double jump or a spring.
pub const LEVEL_H: f32 = 0.7;
/// The tallest step you can walk up with a plain jump, in levels.
pub const MAX_STEP: i32 = 1;

/// The shape of a dream's ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terrain {
    Flat,
    /// Climb tier by tier to a summit.
    Pyramid,
    /// A bowl that caves in: down tier by tier to the pit.
    Crater,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Void,
    Floor,
    Wall,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub w: i32,
    pub h: i32,
    cells: Vec<Cell>,
    /// Terrace level of each cell (0 everywhere on a flat dream). One level
    /// per cell, so nothing is ever stacked above or below anything else.
    levels: Vec<i8>,
}

impl Grid {
    pub fn new(w: i32, h: i32, fill: Cell) -> Self {
        Self {
            w,
            h,
            cells: vec![fill; (w * h) as usize],
            levels: vec![0; (w * h) as usize],
        }
    }

    pub fn level(&self, p: P) -> i32 {
        self.index(p).map_or(0, |i| self.levels[i] as i32)
    }

    pub fn set_level(&mut self, p: P, level: i32) {
        if let Some(i) = self.index(p) {
            self.levels[i] = level as i8;
        }
    }

    pub fn max_level(&self) -> i32 {
        self.levels.iter().copied().max().unwrap_or(0) as i32
    }

    /// Flat, or which way the ground rises.
    pub fn terrain(&self) -> Terrain {
        let (centre, corner) = (self.level((self.w / 2, self.h / 2)), self.level((0, 0)));
        match self.max_level() {
            0 => Terrain::Flat,
            _ if centre > corner => Terrain::Pyramid,
            _ => Terrain::Crater,
        }
    }

    fn index(&self, (x, y): P) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.w && y < self.h).then(|| (y * self.w + x) as usize)
    }

    /// Out-of-bounds reads as Void — the dream just... ends.
    pub fn get(&self, p: P) -> Cell {
        self.index(p).map_or(Cell::Void, |i| self.cells[i])
    }

    pub fn set(&mut self, p: P, cell: Cell) {
        if let Some(i) = self.index(p) {
            self.cells[i] = cell;
        }
    }

    pub fn cells_of(&self, kind: Cell) -> Vec<P> {
        (0..self.h)
            .flat_map(|y| (0..self.w).map(move |x| (x, y)))
            .filter(|&p| self.get(p) == kind)
            .collect()
    }

    /// Centre of a cell's floor surface (y = its terrace height); the grid is
    /// centred on the origin.
    pub fn world(&self, (x, y): P) -> Vec3 {
        Vec3::new(
            (x as f32 - (self.w - 1) as f32 * 0.5) * CELL,
            self.level((x, y)) as f32 * LEVEL_H,
            (y as f32 - (self.h - 1) as f32 * 0.5) * CELL,
        )
    }

    /// Legal moves: one step onto Floor, or a jump over exactly one Void onto
    /// Floor (never over a Wall). The bool is "this move is a jump" (also set
    /// for any step up or down a terrace). Terraces more than `MAX_STEP`
    /// levels apart are a cliff: no move.
    pub fn moves(&self, (x, y): P) -> Vec<(P, bool)> {
        let mut out = Vec::new();
        let here = self.level((x, y));
        let near = |p: P| (self.level(p) - here).abs() <= MAX_STEP;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let step = (x + dx, y + dy);
            match self.get(step) {
                Cell::Floor if near(step) => out.push((step, self.level(step) != here)),
                Cell::Floor => {}
                Cell::Void => {
                    let land = (x + 2 * dx, y + 2 * dy);
                    if self.get(land) == Cell::Floor && near(land) {
                        out.push((land, true));
                    }
                }
                Cell::Wall => {}
            }
        }
        out
    }

    /// Shortest route (BFS). Each entry is (cell, reached_by_jump).
    pub fn path(&self, from: P, to: P) -> Option<Vec<(P, bool)>> {
        if self.get(from) != Cell::Floor || self.get(to) != Cell::Floor {
            return None;
        }
        let idx = |p: P| self.index(p).expect("floor cells are in bounds");
        let mut prev: Vec<Option<(P, bool)>> = vec![None; self.cells.len()];
        let mut seen = vec![false; self.cells.len()];
        let mut queue = VecDeque::from([from]);
        seen[idx(from)] = true;
        while let Some(p) = queue.pop_front() {
            if p == to {
                let mut path = Vec::new();
                let mut cur = to;
                loop {
                    let link = prev[idx(cur)];
                    path.push((cur, link.map_or(false, |(_, jump)| jump)));
                    match link {
                        Some((before, _)) => cur = before,
                        None => break,
                    }
                }
                path.reverse();
                return Some(path);
            }
            for (next, jump) in self.moves(p) {
                let i = idx(next);
                if !seen[i] {
                    seen[i] = true;
                    prev[i] = Some((p, jump));
                    queue.push_back(next);
                }
            }
        }
        None
    }
}

#[cfg(test)]
pub(crate) fn from_rows(rows: &[&str]) -> Grid {
    // '.' floor, '#' wall, ' ' void
    let mut g = Grid::new(rows[0].len() as i32, rows.len() as i32, Cell::Void);
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            let cell = match ch {
                '.' => Cell::Floor,
                '#' => Cell::Wall,
                _ => Cell::Void,
            };
            g.set((x as i32, y as i32), cell);
        }
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_a_corridor() {
        let p = from_rows(&["....."]).path((0, 0), (4, 0)).unwrap();
        assert_eq!(p.len(), 5);
        assert!(p.iter().all(|(_, jump)| !jump));
    }

    #[test]
    fn walls_block() {
        assert!(from_rows(&["..#.."]).path((0, 0), (4, 0)).is_none());
    }

    #[test]
    fn walls_are_routed_around() {
        let p = from_rows(&["..#..", "....."]).path((0, 0), (4, 0)).unwrap();
        assert_eq!(p.len(), 7);
    }

    #[test]
    fn jumps_exactly_one_void() {
        let p = from_rows(&[".. .."]).path((0, 0), (4, 0)).unwrap();
        assert_eq!(p.len(), 4);
        assert_eq!(p.iter().filter(|(_, jump)| *jump).count(), 1);
    }

    #[test]
    fn cannot_jump_two_voids() {
        assert!(from_rows(&[".  ."]).path((0, 0), (3, 0)).is_none());
    }

    #[test]
    fn world_positions_are_centred() {
        let g = Grid::new(3, 3, Cell::Floor);
        assert_eq!(g.world((1, 1)), Vec3::ZERO);
        assert_eq!(g.world((2, 1)), Vec3::new(CELL, 0.0, 0.0));
    }
}
