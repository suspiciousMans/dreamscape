//! SHARDS hard mode: some shards are a skill check, not a free pickup.
//! Guarded = a sentry watches it; Heavy = slower and louder after the pickup.
#![allow(dead_code)]

use crate::dream::{Block, BlockKind};
use crate::gameplay::CELL;
use engine::glam::Vec3;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// First depth where a shard can be a skill check.
pub const MIN_DEPTH: u32 = 4;
/// How long a heavy shard weighs on you.
pub const HEAVY_SECONDS: f32 = 25.0;
pub const HEAVY_SPEED: f32 = 0.8;
pub const HEAVY_ALERT: f32 = 1.5;
/// Own seeded stream: never touches the dream's generator.
const STREAM: u64 = 0x5A4D_0000_5EED_BEEF;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShardKind {
    #[default]
    Plain,
    Guarded,
    Heavy,
}

/// Chance a shard is not plain: 20% at depth 4, +4% per depth, capped at 60%.
pub fn chance(depth: u32) -> f32 {
    if depth < MIN_DEPTH {
        return 0.0;
    }
    (0.20 + 0.04 * (depth - MIN_DEPTH) as f32).min(0.60)
}

/// What kind of shard this dream holds. `eligible` is false for boss,
/// prologue, tutorial and co-op dreams; a GILDED shard is always plain.
/// One kind per dream, so never both.
pub fn kind_for(seed: u64, depth: u32, gilded: bool, eligible: bool) -> ShardKind {
    if !eligible || gilded || depth < MIN_DEPTH {
        return ShardKind::Plain;
    }
    let mut rng = StdRng::seed_from_u64(seed ^ STREAM);
    if rng.gen::<f32>() >= chance(depth) {
        return ShardKind::Plain;
    }
    if rng.gen_bool(0.5) {
        ShardKind::Guarded
    } else {
        ShardKind::Heavy
    }
}

/// (speed multiplier, alert-radius multiplier) `elapsed` seconds after a
/// heavy pickup; `None` = not carrying one.
pub fn heavy_mults(elapsed: Option<f32>) -> (f32, f32) {
    match elapsed {
        Some(t) if t < HEAVY_SECONDS => (HEAVY_SPEED, HEAVY_ALERT),
        _ => (1.0, 1.0),
    }
}

fn flat_dist(a: Vec3, b: Vec3) -> f32 {
    Vec3::new(a.x - b.x, 0.0, a.z - b.z).length()
}

/// Floor-cell centres (top surface) of a dream, read back from its slabs.
pub fn floor_cells(blocks: &[Block]) -> Vec<Vec3> {
    let mut out: Vec<Vec3> = Vec::new();
    for b in blocks.iter().filter(|b| b.kind == BlockKind::Floor) {
        let n = (b.size.x / CELL).round().max(1.0) as i32;
        let y = b.pos.y + b.size.y * 0.5;
        for i in 0..n {
            let x = b.pos.x + (i as f32 - (n - 1) as f32 * 0.5) * CELL;
            out.push(Vec3::new(x, y, b.pos.z));
        }
    }
    out
}

/// A floor cell 1 to 2 cells from the shard, off the route, away from the
/// `keep_clear` points (spawn, portal), on the shard's level. Sentries are not
/// solid and the cell is off the route, so the way through is untouched.
pub fn guard_cell(
    seed: u64,
    shard: Vec3,
    floor: &[Vec3],
    route: &[Vec3],
    keep_clear: &[Vec3],
) -> Option<Vec3> {
    let mut c: Vec<Vec3> = floor
        .iter()
        .copied()
        .filter(|f| {
            let d = flat_dist(*f, shard);
            d >= CELL * 0.99
                && d <= 2.0 * CELL * 1.01
                && (f.y - shard.y).abs() < 0.5
                && route.iter().all(|r| flat_dist(*r, *f) > 0.5 * CELL)
                && keep_clear.iter().all(|k| flat_dist(*k, *f) > 1.5 * CELL)
        })
        .collect();
    c.sort_by(|a, b| (a.x, a.z).partial_cmp(&(b.x, b.z)).expect("finite"));
    if c.is_empty() {
        return None;
    }
    let mut rng = StdRng::seed_from_u64(seed ^ STREAM ^ 0x6A4D);
    Some(c[rng.gen_range(0..c.len())])
}

/// The sentry angle (see `Sentry::facing`) that points from `guard` at `shard`.
pub fn facing_angle(guard: Vec3, shard: Vec3) -> f32 {
    let d = shard - guard;
    d.x.atan2(d.z)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::{generate_with, Goals, Pressure, ALL_THEMES};
    use engine::glam::Vec3;

    #[test]
    fn plain_below_min_depth_and_when_ineligible() {
        for seed in 0..200u64 {
            for depth in 0..MIN_DEPTH {
                assert_eq!(kind_for(seed, depth, false, true), ShardKind::Plain);
            }
            assert_eq!(kind_for(seed, 30, true, true), ShardKind::Plain, "gilded");
            assert_eq!(kind_for(seed, 30, false, false), ShardKind::Plain, "ineligible");
        }
    }

    #[test]
    fn deterministic_and_rises_with_depth() {
        let count = |depth| {
            (0..2000u64)
                .filter(|&s| kind_for(s, depth, false, true) != ShardKind::Plain)
                .count()
        };
        for s in 0..50 {
            assert_eq!(kind_for(s, 9, false, true), kind_for(s, 9, false, true));
        }
        assert!(count(4) > 0);
        assert!(count(4) < count(12));
        assert!(count(12) <= count(40));
        let heavy = (0..2000u64).filter(|&s| kind_for(s, 12, false, true) == ShardKind::Heavy).count();
        let guarded = (0..2000u64).filter(|&s| kind_for(s, 12, false, true) == ShardKind::Guarded).count();
        assert!(heavy > 100 && guarded > 100);
    }

    #[test]
    fn heavy_mults_fade_after_the_window() {
        assert_eq!(heavy_mults(None), (1.0, 1.0));
        assert_eq!(heavy_mults(Some(0.0)), (HEAVY_SPEED, HEAVY_ALERT));
        assert_eq!(heavy_mults(Some(HEAVY_SECONDS - 0.1)), (HEAVY_SPEED, HEAVY_ALERT));
        assert_eq!(heavy_mults(Some(HEAVY_SECONDS)), (1.0, 1.0));
        assert!((HEAVY_SPEED - 0.8).abs() < 1e-6 && (HEAVY_ALERT - 1.5).abs() < 1e-6);
    }

    #[test]
    fn guard_cell_is_in_range_and_clear_of_the_route() {
        let c = crate::gameplay::CELL;
        let shard = Vec3::ZERO;
        let floor: Vec<Vec3> = (-4..=4)
            .flat_map(|x| (-4..=4).map(move |z| Vec3::new(x as f32 * c, 0.0, z as f32 * c)))
            .collect();
        let route = [Vec3::new(-c, 0.0, 0.0), Vec3::ZERO, Vec3::new(c, 0.0, 0.0)];
        let g = guard_cell(7, shard, &floor, &route, &[]).expect("a guard cell");
        let d = (g - shard).length();
        assert!(d >= c * 0.99 && d <= 2.0 * c * 1.01 + 0.01, "d={d}");
        assert!(route.iter().all(|r| r.distance(g) > 0.5 * c));
        assert_eq!(Some(g), guard_cell(7, shard, &floor, &route, &[]));
        assert_eq!(guard_cell(7, shard, &[], &route, &[]), None);
        // keep-clear points (spawn, portal) are respected
        let near = [g];
        assert_ne!(guard_cell(7, shard, &floor, &route, &near), Some(g));
    }

    #[test]
    fn facing_points_at_the_shard() {
        let a = facing_angle(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0));
        let f = Vec3::new(a.sin(), 0.0, a.cos());
        assert!(f.dot(Vec3::X) > 0.999);
    }

    /// The patient-walker guarantee: a guard never sits on the route, so the
    /// route spawn -> shard -> portal is exactly the one the generator made.
    #[test]
    fn a_guard_never_blocks_the_route_on_several_layouts() {
        let mut layouts = 0;
        for theme in ALL_THEMES {
            let mut seen = false;
            for seed in 0..40u64 {
                let d = generate_with(theme, seed, 8, None, true, Pressure::default(), Goals::default());
                let Some(shard) = d.shard else { continue };
                let floor = floor_cells(&d.blocks);
                assert!(!floor.is_empty());
                let route: Vec<Vec3> = d.lucid_route.iter().map(|w| w.pos).collect();
                let keep = [d.spawn, d.portal];
                let Some(g) = guard_cell(seed, shard, &floor, &route, &keep) else { continue };
                assert!(route.iter().all(|r| flat_d(*r, g) > 1.0), "{theme:?} {seed}");
                assert!(flat_d(g, shard) <= crate::specials::SENTRY_RANGE);
                // it really stands on a floor cell
                assert!(floor.iter().any(|f| flat_d(*f, g) < 0.01));
                // and the sentry beam covers the shard
                let a = facing_angle(g, shard);
                let face = Vec3::new(a.sin(), 0.0, a.cos());
                assert!(crate::specials::sees(g, face, shard));
                seen = true;
            }
            if seen {
                layouts += 1;
            }
        }
        assert!(layouts >= 3, "only {layouts} layouts had a guard cell");
    }

    fn flat_d(a: Vec3, b: Vec3) -> f32 {
        Vec3::new(a.x - b.x, 0.0, a.z - b.z).length()
    }
}
