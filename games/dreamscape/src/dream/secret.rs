//! Secret rooms: a cracked wall, placed off the route, that hides a door.
//!
//! Pure data. The wall stays an ordinary `Wall` cell in the grid (so the
//! floor plan, the route and every other stream are untouched); the dream
//! only records which wall cell is cracked and which floor cell you reach it
//! from. Breaking it and the shop behind it live in the game, not here.

use super::grid::{Cell, Grid, P};
use super::theme::LayoutKind;
use engine::glam::Vec3;
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use std::collections::HashSet;

/// Chance that a descent dream hides a cracked wall: about one in 4-5.
/// The only knob for secret frequency.
pub const SECRET_CHANCE: f64 = 0.22;
/// Secrets never appear above this depth (the first dreams teach the basics).
pub const SECRET_MIN_DEPTH: u32 = 2;

/// A cracked wall in a dream.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Secret {
    /// Floor-level centre of the cracked wall cell.
    pub wall: Vec3,
    /// Floor-level centre of the open floor cell you reach it from.
    pub approach: Vec3,
}

/// Whether a dream of this depth rolls a secret at `chance` (its own RNG
/// stream, so asking never changes any other part of the dream).
pub fn rolls(seed: u64, depth: u32, chance: f64) -> bool {
    depth >= SECRET_MIN_DEPTH
        && chance > 0.0
        && StdRng::seed_from_u64(seed ^ 0x5EC2_E7D0).gen_bool(chance.min(1.0))
}

/// Maze-like and room-like layouts can hide a wall. Platform chains,
/// spirals, scatter fields, networks over the void and terraces cannot.
pub fn hosts(kind: LayoutKind) -> bool {
    matches!(
        kind,
        LayoutKind::OpenHall
            | LayoutKind::Maze
            | LayoutKind::Mirrored
            | LayoutKind::Corridor
            | LayoutKind::Recursive
    )
}

/// A wall cell with exactly one open floor neighbour that is off every
/// route, with nothing but wall or void straight behind it. Biased to the
/// far half of the dream. Returns (wall cell, approach cell).
pub(super) fn pick(grid: &Grid, taken: &HashSet<P>, spawn: P, seed: u64) -> Option<(P, P)> {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x5EC2_E7A1);
    let mut candidates: Vec<(usize, P, P)> = Vec::new();
    for wall in grid.cells_of(Cell::Wall) {
        let floors: Vec<(i32, i32)> = [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .into_iter()
            .filter(|&(dx, dy)| grid.get((wall.0 + dx, wall.1 + dy)) == Cell::Floor)
            .collect();
        let [(dx, dy)] = floors[..] else { continue };
        let approach = (wall.0 + dx, wall.1 + dy);
        let behind = (wall.0 - dx, wall.1 - dy);
        if taken.contains(&approach) || grid.get(behind) == Cell::Floor {
            continue;
        }
        // Stand level with the wall: no cliff between you and the crack.
        if grid.level(approach) != grid.level(wall) {
            continue;
        }
        if let Some(p) = grid.path(spawn, approach) {
            candidates.push((p.len(), wall, approach));
        }
    }
    candidates.sort();
    let far_half = candidates.get(candidates.len() / 2..)?;
    far_half.choose(&mut rng).map(|&(_, w, a)| (w, a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_before_depth_two_and_never_at_zero_chance() {
        for seed in 0..500 {
            assert!(!rolls(seed, 0, 1.0) && !rolls(seed, 1, 1.0));
            assert!(!rolls(seed, 5, 0.0));
            assert!(rolls(seed, 2, 1.0));
        }
    }

    #[test]
    fn the_real_chance_is_about_one_in_four_or_five() {
        let hits = (0..4000).filter(|&s| rolls(s, 3, SECRET_CHANCE)).count();
        assert!((600..1150).contains(&hits), "{hits}/4000");
    }

    #[test]
    fn only_maze_and_room_layouts_host() {
        assert!(hosts(LayoutKind::Maze) && hosts(LayoutKind::Recursive));
        for k in [
            LayoutKind::PlatformChain,
            LayoutKind::Spiral,
            LayoutKind::ScatterField,
            LayoutKind::Network,
            LayoutKind::Terraces,
        ] {
            assert!(!hosts(k), "{k:?}");
        }
    }

    #[test]
    fn picks_a_dead_end_wall_off_the_taken_cells() {
        let g = super::super::grid::from_rows(&["#####", "#...#", "#####"]);
        let taken: HashSet<P> = [(1, 1), (2, 1), (3, 1)].into_iter().collect();
        assert!(pick(&g, &taken, (1, 1), 1).is_none(), "route fills the room");
        let taken: HashSet<P> = [(1, 1)].into_iter().collect();
        let (wall, approach) = pick(&g, &taken, (1, 1), 1).unwrap();
        assert_eq!(g.get(wall), Cell::Wall);
        assert!(!taken.contains(&approach));
    }
}
