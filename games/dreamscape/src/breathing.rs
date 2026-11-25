//! BREATHING: the walls that are nowhere near your way through the dream
//! swell and settle by up to half a cell, as if the room were inhaling.
//!
//! The sway is real, not just draw-time: `engine::physics::step` reads every
//! collider's live `Transform` each frame, so a swaying wall's collider sways
//! with it and can nudge a dreamer who stands too close. What keeps that from
//! ever costing a route is geometric and cheap to check: a wall only breathes
//! when it is at least `CLEAR_CELLS` from every route cell, and it never leans
//! further than `AMPLITUDE`, so a moving wall's *centre* always keeps
//! `CLEAR_CELLS * CELL - AMPLITUDE` (one and a half cells) from the route, and
//! its face at least one cell, even at the far end of its breath. Route walls
//! themselves never move at all, which is what keeps the walk through the dream
//! as legible as it was in a still one.

use crate::dream::Dream;
use crate::gameplay::CELL;
use engine::glam::Vec3;
use std::f32::consts::TAU;

/// How far a breathing wall leans from where it stands: half a cell.
pub const AMPLITUDE: f32 = CELL * 0.5;
/// Walls at least this many cells from the route breathe; closer ones are
/// still. Two cells is the smallest figure that still leaves a full cell of
/// margin once the wall has leaned all the way over.
pub const CLEAR_CELLS: f32 = 2.0;
/// Seconds per breath, fastest and slowest (walls are desynchronised so the
/// room doesn't pulse as one piece of scenery).
pub const PERIOD_FAST: f32 = 5.0;
pub const PERIOD_SLOW: f32 = 9.0;
/// How much of the lean goes sideways rather than across. The sideways term
/// rides the same angle as the crossways one, so a wall's total lean never
/// exceeds `AMPLITUDE` in any direction.
const SIDE_LEAN: f32 = 0.7;

/// A wall that breathes. `base` is where it stands with no breath in it.
#[derive(Clone, Copy, Debug)]
pub struct Breathe {
    pub base: Vec3,
    pub amp: f32,
    pub period: f32,
    pub phase: f32,
}

/// Every point the dreamer has to be able to stand on (or reach): the route,
/// the shard detour, both ends, the shard and fragments, and a nightmare's
/// sigils. A wall near any of these never breathes.
pub fn route_points(dream: &Dream) -> Vec<Vec3> {
    let mut out: Vec<Vec3> = dream
        .route
        .iter()
        .chain(dream.lucid_route.iter())
        .map(|w| w.pos)
        .collect();
    out.extend([dream.spawn, dream.portal]);
    out.extend(dream.shard);
    out.extend(dream.fragments.iter().copied());
    out.extend(dream.sigils.iter().copied());
    out
}

/// How far `at` is from the nearest route point, in the plane (height is not
/// part of the promise: a wall above a route cell is still a wall over it).
pub fn clearance(at: Vec3, route: &[Vec3]) -> f32 {
    route
        .iter()
        .map(|p| Vec3::new(p.x - at.x, 0.0, p.z - at.z).length())
        .fold(f32::INFINITY, f32::min)
}

/// How far a wall `clearance` from the route may lean: nothing at all until it
/// is `CLEAR_CELLS` away, then the full half cell.
pub fn amp_for(clearance: f32) -> f32 {
    if clearance >= CLEAR_CELLS * CELL {
        AMPLITUDE
    } else {
        0.0
    }
}

/// A stable 0..1 from a position, so a wall's breath is the same every load.
fn hash(at: Vec3) -> f32 {
    let h = (at.x * 12.9898 + at.z * 78.233).sin() * 43_758.547;
    h - h.floor()
}

/// A second, uncorrelated 0..1 for the phase.
fn hash2(at: Vec3) -> f32 {
    let h = (at.x * 39.346 + at.z * 11.135).sin() * 24_634.634;
    h - h.floor()
}

/// Seconds per breath for a wall at `at`, between the fast and slow limits.
pub fn period_for(at: Vec3) -> f32 {
    PERIOD_FAST + hash(at) * (PERIOD_SLOW - PERIOD_FAST)
}

/// Where in its cycle a wall at `at` starts.
pub fn phase_for(at: Vec3) -> f32 {
    hash2(at) * TAU
}

/// Where a wall with this breath stands `age` seconds into the dream, relative
/// to its base. Bounded by `amp` on both axes.
pub fn offset(amp: f32, period: f32, phase: f32, age: f32) -> Vec3 {
    let s = TAU * age / period.max(0.05) + phase;
    Vec3::new(amp * s.sin(), 0.0, amp * SIDE_LEAN * s.cos())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::{
        generate_with, BlockKind, Goals, Pressure, ALL_THEMES,
    };

    #[test]
    fn a_still_wall_never_moves() {
        for age in [0.0, 0.3, 2.0, 11.7] {
            assert_eq!(offset(0.0, 7.0, 0.4, age), Vec3::ZERO);
        }
    }

    #[test]
    fn a_breathing_wall_never_leans_further_than_half_a_cell() {
        let amp = AMPLITUDE;
        for i in 0..200 {
            let age = i as f32 * 0.07;
            let o = offset(amp, 6.4, 1.1, age);
            assert!(o.x.abs() <= amp + 1e-5, "x {} at {age}", o.x);
            assert!(o.z.abs() <= amp + 1e-5, "z {} at {age}", o.z);
            assert!(o.length() <= amp + 1e-5);
            assert_eq!(o.y, 0.0, "breathing is horizontal");
        }
    }

    #[test]
    fn a_breath_repeats_every_period() {
        let (amp, period, phase) = (0.9, 7.5, 0.8);
        for i in 0..40 {
            let age = i as f32 * 0.3;
            let a = offset(amp, period, phase, age);
            let b = offset(amp, period, phase, age + period);
            assert!((a - b).length() < 1e-4, "{age}: {a:?} vs {b:?}");
        }
    }

    #[test]
    fn walls_breathe_only_once_they_are_two_cells_off_the_route() {
        assert_eq!(amp_for(0.0), 0.0);
        assert_eq!(amp_for(CLEAR_CELLS * CELL - 0.01), 0.0);
        assert_eq!(amp_for(CLEAR_CELLS * CELL), AMPLITUDE);
        assert_eq!(amp_for(40.0), AMPLITUDE);
    }

    #[test]
    fn clearance_is_the_flat_distance_to_the_nearest_route_point() {
        let route = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(9.0, 4.0, 0.0)];
        assert_eq!(clearance(Vec3::new(0.0, 12.0, 0.0), &route), 0.0);
        assert!((clearance(Vec3::new(3.0, 0.0, 4.0), &route) - 5.0).abs() < 1e-5);
        assert!((clearance(Vec3::new(9.0, 0.0, 2.0), &route) - 2.0).abs() < 1e-5);
        assert_eq!(clearance(Vec3::ZERO, &[]), f32::INFINITY);
    }

    #[test]
    fn a_breath_is_stable_for_a_position_and_differs_between_walls() {
        let a = Vec3::new(3.0, 0.0, 9.0);
        let b = Vec3::new(-6.0, 0.0, 15.0);
        assert_eq!(period_for(a), period_for(a));
        assert!((PERIOD_FAST..=PERIOD_SLOW).contains(&period_for(a)));
        assert!((PERIOD_FAST..=PERIOD_SLOW).contains(&period_for(b)));
        assert_ne!(phase_for(a), phase_for(b));
        assert!((0.0..TAU).contains(&phase_for(a)));
    }

    /// The real guarantee: on real generated dreams, no wall that breathes is
    /// close enough to the route for its lean to pinch the way through, and
    /// every route cell is untouchable by definition (`amp_for(0) == 0`).
    #[test]
    fn breathing_never_pinches_a_route_shut() {
        let mut breathing = 0;
        for theme in ALL_THEMES {
            for seed in 0..30u64 {
                let d = generate_with(
                    theme,
                    seed,
                    6,
                    None,
                    true,
                    Pressure::default(),
                    Goals::default(),
                );
                let route = route_points(&d);
                assert!(!route.is_empty(), "{theme:?} {seed}: no route");
                for w in &route {
                    assert_eq!(amp_for(clearance(*w, &route)), 0.0);
                }
                for b in d.blocks.iter().filter(|b| b.kind == BlockKind::Wall) {
                    let clear = clearance(b.pos, &route);
                    let amp = amp_for(clear);
                    if amp <= 0.0 {
                        continue;
                    }
                    breathing += 1;
                    assert!(
                        clear - amp >= 1.5 * CELL,
                        "{theme:?} seed {seed}: wall {clear} from the route leans {amp}"
                    );
                }
            }
        }
        assert!(breathing > 100, "almost no wall ever breathes: {breathing}");
    }
}
