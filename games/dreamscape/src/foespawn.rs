//! Where the ENEMIES-B stream (moths, mask, leech) may stand.
//!
//! Pure and seeded: every enemy has its own RNG stream, so adding one never
//! shifts the dream's own randomness or another enemy's post. The gates
//! (depth, nightmares) live in each enemy's `spawn_plan`; the prologue and
//! co-op are the game's to check, since they are not facts about a dream.

use engine::glam::Vec3;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::gameplay::CELL;

/// Most new enemies in one dream (the merger caps the combined count).
pub const MAX_NEW: usize = 3;
/// Nothing stands within this many cells of the spawn, portal or shard.
pub const KEEP_CLEAR_CELLS: f32 = 3.0;
/// Nothing stands closer than this many cells to another new enemy.
pub const APART_CELLS: f32 = 2.0;

fn flat_dist(a: Vec3, b: Vec3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// A floor cell, off the route and its neighbours' cells, and clear of
/// `keep_clear` (spawn, portal, shard) and of `taken` (earlier new enemies).
/// `None` when the dream has no such pocket: fewer enemies beats a walled-in
/// route. `floor` holds cell centres on the floor surface.
pub fn place(
    seed: u64,
    stream: u64,
    floor: &[Vec3],
    route: &[Vec3],
    keep_clear: &[Vec3],
    taken: &[Vec3],
) -> Option<Vec3> {
    let mut ok: Vec<Vec3> = floor
        .iter()
        .copied()
        .filter(|f| {
            route.iter().all(|r| flat_dist(*r, *f) > 0.5 * CELL || (r.y - f.y).abs() > 1.5)
                && keep_clear.iter().all(|k| flat_dist(*k, *f) >= KEEP_CLEAR_CELLS * CELL)
                && taken.iter().all(|k| flat_dist(*k, *f) >= APART_CELLS * CELL)
        })
        .collect();
    ok.sort_by(|a, b| (a.x, a.z).partial_cmp(&(b.x, b.z)).expect("finite"));
    if ok.is_empty() {
        return None;
    }
    let mut rng = StdRng::seed_from_u64(seed ^ stream);
    Some(ok[rng.gen_range(0..ok.len())])
}

/// A floor cell next to `from` (same level, one cell away) that is off the
/// route: the other end of a patrol. `None` if there is no free neighbour.
pub fn neighbour(seed: u64, from: Vec3, floor: &[Vec3], route: &[Vec3]) -> Option<Vec3> {
    let mut n: Vec<Vec3> = floor
        .iter()
        .copied()
        .filter(|f| {
            let d = flat_dist(*f, from);
            d > 0.9 * CELL
                && d < 1.1 * CELL
                && (f.y - from.y).abs() < 0.5
                && route.iter().all(|r| flat_dist(*r, *f) > 0.5 * CELL || (r.y - f.y).abs() > 1.5)
        })
        .collect();
    n.sort_by(|a, b| (a.x, a.z).partial_cmp(&(b.x, b.z)).expect("finite"));
    if n.is_empty() {
        return None;
    }
    Some(n[StdRng::seed_from_u64(seed).gen_range(0..n.len())])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Vec<Vec3> {
        (0..12)
            .flat_map(|x| (0..6).map(move |z| Vec3::new(x as f32 * CELL, 0.0, z as f32 * CELL)))
            .collect()
    }

    fn route() -> Vec<Vec3> {
        (0..12).map(|x| Vec3::new(x as f32 * CELL, 0.0, 0.0)).collect()
    }

    #[test]
    fn placement_is_off_route_clear_of_the_keep_points_and_deterministic() {
        let keep = [Vec3::ZERO, Vec3::new(11.0 * CELL, 0.0, 0.0)];
        for seed in 0..50u64 {
            let a = place(seed, 0xA, &grid(), &route(), &keep, &[]).expect("room");
            assert_eq!(Some(a), place(seed, 0xA, &grid(), &route(), &keep, &[]));
            assert!(route().iter().all(|r| flat_dist(*r, a) > 0.5 * CELL), "{a:?} is on the route");
            assert!(keep.iter().all(|k| flat_dist(*k, a) >= 3.0 * CELL), "{a:?} too close");
        }
    }

    #[test]
    fn streams_differ_and_taken_spots_are_respected() {
        let keep = [Vec3::ZERO];
        let a = place(3, 1, &grid(), &route(), &keep, &[]).unwrap();
        let b = place(3, 2, &grid(), &route(), &keep, &[a]).unwrap();
        assert!(flat_dist(a, b) >= APART_CELLS * CELL);
    }

    #[test]
    fn no_room_means_none() {
        let tiny = [Vec3::ZERO, Vec3::new(CELL, 0.0, 0.0)];
        assert_eq!(place(1, 1, &tiny, &route(), &[], &[]), None);
        assert_eq!(place(1, 1, &[], &[], &[], &[]), None);
    }

    #[test]
    fn a_patrol_neighbour_is_one_cell_away_and_off_route() {
        let from = Vec3::new(4.0 * CELL, 0.0, 3.0 * CELL);
        for seed in 0..20 {
            let n = neighbour(seed, from, &grid(), &route()).unwrap();
            assert!((flat_dist(n, from) - CELL).abs() < 0.01);
            assert!(n.z > 0.5 * CELL, "{n:?}");
        }
        assert_eq!(neighbour(1, Vec3::new(0.0, 0.0, 0.0), &[Vec3::ZERO], &route()), None);
    }
}
