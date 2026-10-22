//! TACTICS (hard mode): enemies cooperate. Pure decisions only; main.rs wires
//! them in (`// === tactics ===` blocks). Chasers stay slower than the player:
//! the danger is combinations, not speed.

use crate::gameplay::CELL;
use crate::specials::{self, Elite};
use engine::glam::Vec3;

/// A chasing pacer's shout reaches this far.
pub const SHOUT_RADIUS: f32 = 6.0 * CELL;
/// A pacer can be shouted at once per this many seconds.
pub const SHOUT_COOLDOWN: f32 = 8.0;
pub const SHOUT_DEPTH: u32 = 3;
pub const JESTER_DEPTH: u32 = 4;
pub const MIMIC_DEPTH: u32 = 5;
pub const PACK_DEPTH: u32 = 6;
/// Extra mimic replay delay (seconds) once tactics apply.
pub const MIMIC_EXTRA_DELAY: f32 = 3.0;
/// Sentry turn-rate multiplier while it has a mimic in its cone.
pub const SENTRY_BOOST: f32 = 1.5;
/// A jester likes to throw you within this of a stalker or sentry.
pub const JESTER_ASSIST_RADIUS: f32 = specials::SENTRY_RANGE;
/// An elite's pack: this many followers within this reach.
pub const PACK_MAX: usize = 3;
pub const PACK_RADIUS: f32 = 9.0 * CELL;

fn flat_dist(a: Vec3, b: Vec3) -> f32 {
    Vec3::new(a.x - b.x, 0.0, a.z - b.z).length()
}

/// Do tactics apply at all? (Not in co-op, not in the gentle first dreams.)
pub fn active(depth: u32, coop: bool) -> bool {
    !coop && depth >= SHOUT_DEPTH
}

/// Per-enemy shout cooldowns, indexed like `enemies`.
#[derive(Default, Debug, Clone)]
pub struct State {
    pub cooldowns: Vec<f32>,
}

impl State {
    pub fn tick(&mut self, n: usize, dt: f32) {
        self.cooldowns.resize(n, 0.0);
        for c in &mut self.cooldowns {
            *c = (*c - dt).max(0.0);
        }
    }
    pub fn cooldown(&self, k: usize) -> f32 {
        self.cooldowns.get(k).copied().unwrap_or(0.0)
    }
    pub fn mark(&mut self, k: usize) {
        if k >= self.cooldowns.len() {
            self.cooldowns.resize(k + 1, 0.0);
        }
        self.cooldowns[k] = SHOUT_COOLDOWN;
    }
}

/// Indices (ascending) of pacers that hear `shouter` at `from`.
/// `others` = (index, position, shout cooldown left).
pub fn shout_targets(
    depth: u32,
    shouter: usize,
    from: Vec3,
    others: &[(usize, Vec3, f32)],
) -> Vec<usize> {
    if depth < SHOUT_DEPTH {
        return Vec::new();
    }
    let mut v: Vec<usize> = others
        .iter()
        .filter(|&&(k, p, cd)| k != shouter && cd <= 0.0 && flat_dist(p, from) <= SHOUT_RADIUS)
        .map(|&(k, _, _)| k)
        .collect();
    v.sort_unstable();
    v
}

/// Which throw landing the jester takes. `landings[i]` is `Some(point)` when
/// direction i is safe floor (None = wall/void: never chosen). From
/// JESTER_DEPTH, prefers the first landing within a watcher's reach; else the
/// first safe one (the old behaviour).
pub fn jester_dir(depth: u32, landings: &[Option<Vec3>], watchers: &[Vec3]) -> Option<usize> {
    if depth >= JESTER_DEPTH {
        let hit = landings.iter().position(|l| {
            l.is_some_and(|p| watchers.iter().any(|&w| flat_dist(p, w) <= JESTER_ASSIST_RADIUS))
        });
        if hit.is_some() {
            return hit;
        }
    }
    landings.iter().position(|l| l.is_some())
}

/// Extra mimic delay at this depth.
pub fn mimic_extra_delay(depth: u32) -> f32 {
    if depth >= MIMIC_DEPTH {
        MIMIC_EXTRA_DELAY
    } else {
        0.0
    }
}

/// Sentry turn multiplier: faster while a mimic is in its cone.
pub fn sentry_boost(depth: u32, at: Vec3, facing: Vec3, mimic: Option<Vec3>) -> f32 {
    match mimic {
        Some(m) if depth >= MIMIC_DEPTH && specials::sees(at, facing, m) => SENTRY_BOOST,
        _ => 1.0,
    }
}

/// Followers (up to PACK_MAX, nearest first, ties by index) of an elite
/// leader that has just noticed you. `others` = (index, position).
pub fn pack_follow(
    depth: u32,
    leader: usize,
    leader_elite: Option<Elite>,
    from: Vec3,
    others: &[(usize, Vec3)],
) -> Vec<usize> {
    if depth < PACK_DEPTH || leader_elite.is_none() {
        return Vec::new();
    }
    let mut c: Vec<(f32, usize)> = others
        .iter()
        .filter(|&&(k, p)| k != leader && flat_dist(p, from) <= PACK_RADIUS)
        .map(|&(k, p)| (flat_dist(p, from), k))
        .collect();
    c.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    c.truncate(PACK_MAX);
    c.into_iter().map(|(_, k)| k).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f32, z: f32) -> Vec3 {
        Vec3::new(x, 0.5, z)
    }

    #[test]
    fn shouts_reach_near_ready_pacers_only() {
        let others = [
            (0, at(0.0, 0.0), 0.0),
            (1, at(SHOUT_RADIUS - 0.1, 0.0), 0.0),
            (2, at(SHOUT_RADIUS + 0.5, 0.0), 0.0),
            (3, at(1.0, 1.0), 2.0),
        ];
        assert_eq!(shout_targets(3, 0, at(0.0, 0.0), &others), vec![1]);
    }

    #[test]
    fn no_shouting_in_the_gentle_dreams() {
        let others = [(1, at(1.0, 0.0), 0.0)];
        assert!(shout_targets(2, 0, at(0.0, 0.0), &others).is_empty());
        assert!(!active(2, false) && !active(5, true) && active(3, false));
    }

    #[test]
    fn shout_is_deterministic_and_sorted() {
        let others = [(5, at(1.0, 0.0), 0.0), (2, at(2.0, 0.0), 0.0)];
        let a = shout_targets(4, 0, at(0.0, 0.0), &others);
        assert_eq!(a, shout_targets(4, 0, at(0.0, 0.0), &others));
        assert_eq!(a, vec![2, 5]);
    }

    #[test]
    fn cooldowns_tick_down_and_gate() {
        let mut s = State::default();
        s.mark(2);
        assert_eq!(s.cooldown(2), SHOUT_COOLDOWN);
        s.tick(3, SHOUT_COOLDOWN + 0.1);
        assert_eq!(s.cooldown(2), 0.0);
    }

    #[test]
    fn jester_prefers_a_watched_landing_but_never_a_blocked_one() {
        let l = [None, Some(at(10.0, 0.0)), Some(at(0.0, 10.0)), None];
        assert_eq!(jester_dir(4, &l, &[at(0.0, 10.5)]), Some(2));
        assert_eq!(jester_dir(4, &l, &[]), Some(1));
        assert_eq!(jester_dir(4, &l, &[at(99.0, 99.0)]), Some(1));
        // before depth 4: first safe one, watchers ignored
        assert_eq!(jester_dir(3, &l, &[at(0.0, 10.5)]), Some(1));
        assert_eq!(jester_dir(9, &[None, None], &[at(0.0, 0.0)]), None);
        for d in 0..12 {
            for w in [vec![], vec![at(10.0, 0.0)], vec![at(0.0, 10.0)]] {
                let i = jester_dir(d, &l, &w).unwrap();
                assert!(l[i].is_some());
            }
        }
    }

    #[test]
    fn mimic_is_slower_and_sentries_notice_it_deep_down() {
        assert_eq!(mimic_extra_delay(4), 0.0);
        assert_eq!(mimic_extra_delay(5), MIMIC_EXTRA_DELAY);
        let (s, f) = (Vec3::ZERO, Vec3::Z);
        let m = Some(Vec3::new(0.0, 0.0, CELL));
        assert_eq!(sentry_boost(5, s, f, m), SENTRY_BOOST);
        assert_eq!(sentry_boost(4, s, f, m), 1.0);
        assert_eq!(sentry_boost(5, s, f, None), 1.0);
        assert_eq!(sentry_boost(5, s, f, Some(Vec3::new(0.0, 0.0, -CELL))), 1.0);
    }

    #[test]
    fn elite_packs_are_two_or_three_nearest_followers() {
        let o: Vec<_> = (0..6).map(|k| (k, at(k as f32, 0.0))).collect();
        let p = pack_follow(6, 0, Some(Elite::Fast), at(0.0, 0.0), &o);
        assert_eq!(p, vec![1, 2, 3]);
        assert_eq!(p, pack_follow(6, 0, Some(Elite::Fast), at(0.0, 0.0), &o));
        assert!(pack_follow(5, 0, Some(Elite::Fast), at(0.0, 0.0), &o).is_empty());
        assert!(pack_follow(6, 0, None, at(0.0, 0.0), &o).is_empty());
        let far = [(1, at(PACK_RADIUS + 1.0, 0.0))];
        assert!(pack_follow(8, 0, Some(Elite::Big), at(0.0, 0.0), &far).is_empty());
    }
}
