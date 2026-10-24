//! The new combat enemies (Tooth, Weeper, Lullaby): which dreams get them
//! and where they stand. Each enemy owns its rules in its own module; this
//! is only the shared spawn arithmetic. Pure and seeded: the same dream
//! always gets the same enemies, and none of this touches the dream's own
//! random stream.

use crate::combat::Kind;
use crate::dream::DreamTheme;
use crate::gameplay::CELL;
use engine::glam::Vec3;

/// At most this many of each per dream, and this many new enemies in total.
pub const MAX_TEETH: usize = 3;
pub const MAX_WEEPERS: usize = 2;
pub const MAX_LULLABIES: usize = 1;
pub const MAX_TOTAL: usize = 4;
/// Two lairs are at least this far apart (world units).
pub const LAIR_SPACING: f32 = 3.0 * CELL;
/// Lairs keep this far from the spawn, the portal and the shard.
pub const KEEP_CLEAR: f32 = 3.0 * CELL;

fn mix(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^ (x >> 33)
}

/// A number from `seed` on its own named stream.
pub fn stream(seed: u64, salt: u64) -> u64 {
    mix(seed ^ mix(salt))
}

/// Themes that never get wandering enemies (the opening rooms).
pub fn no_wandering_foes(theme: DreamTheme) -> bool {
    matches!(theme, DreamTheme::Lobby | DreamTheme::Awakening)
}

/// Every new enemy's plan for a dream, capped (Lullaby first, then Weepers,
/// then Teeth) at [`MAX_TOTAL`].
pub fn plan(depth: u32, theme: DreamTheme, seed: u64, nightmare: bool) -> Vec<(Kind, usize)> {
    let mut all = Vec::new();
    all.extend(crate::lullaby::spawn_plan(depth, theme, seed, nightmare));
    all.extend(crate::weeper::spawn_plan(depth, theme, seed, nightmare));
    all.extend(crate::tooth::spawn_plan(depth, theme, seed, nightmare));
    let mut room = MAX_TOTAL;
    all.into_iter()
        .filter_map(|(k, n)| {
            let cap = match k {
                Kind::Tooth => MAX_TEETH,
                Kind::Weeper => MAX_WEEPERS,
                Kind::Lullaby => MAX_LULLABIES,
                _ => 0,
            };
            let n = n.min(cap).min(room);
            room -= n;
            (n > 0).then_some((k, n))
        })
        .collect()
}

/// The dev switch `DREAMSCAPE_ENEMY=Tooth|Weeper|Lullaby`.
pub fn forced(name: &str) -> Option<Kind> {
    match name {
        "Tooth" => Some(Kind::Tooth),
        "Weeper" => Some(Kind::Weeper),
        "Lullaby" => Some(Kind::Lullaby),
        _ => None,
    }
}

/// What the dev switch puts in every dream.
pub fn forced_plan(kind: Kind) -> Vec<(Kind, usize)> {
    let n = match kind {
        Kind::Tooth => 2,
        Kind::Weeper => 2,
        _ => 1,
    };
    vec![(kind, n)]
}

/// Picks a lair for each enemy in the plan from the dream's safe `lairs`
/// (off the route, clear of the spawn, portal and shard): seeded, never twice
/// the same lair, never two close together. Fewer lairs than enemies simply
/// means fewer enemies.
pub fn place(plan: &[(Kind, usize)], lairs: &[Vec3], seed: u64) -> Vec<(Kind, Vec3)> {
    let mut order: Vec<usize> = (0..lairs.len()).collect();
    order.sort_by_key(|&i| stream(seed, 0x1a_14 + i as u64));
    let mut used: Vec<Vec3> = Vec::new();
    let mut out = Vec::new();
    for &(kind, n) in plan {
        for _ in 0..n {
            let pick = order.iter().copied().find(|&i| {
                let p = lairs[i];
                used.iter().all(|u| {
                    Vec3::new(u.x - p.x, 0.0, u.z - p.z).length() >= LAIR_SPACING
                })
            });
            if let Some(i) = pick {
                used.push(lairs[i]);
                out.push((kind, lairs[i]));
            }
        }
    }
    out
}

/// What a new enemy says out loud when its state changes: `was` and `now` are
/// (telling, lunging or slamming, tears in the air). Pure, so each tell, lunge
/// and spit is heard once.
pub fn cues(kind: crate::combat::Kind, was: (bool, bool, usize), now: (bool, bool, usize)) -> Vec<crate::sounds::Sound> {
    use crate::combat::Kind;
    use crate::sounds::Sound;
    let mut out = Vec::new();
    match kind {
        Kind::Tooth => {
            if now.0 && !was.0 {
                out.push(Sound::ToothTell);
            }
            if now.1 && !was.1 {
                out.push(Sound::ToothLunge);
            }
        }
        Kind::Weeper => {
            if now.2 > was.2 {
                out.push(Sound::WeeperSpit);
            }
        }
        Kind::Lullaby => {
            if now.0 && !was.0 {
                out.push(Sound::LullabyTell);
            }
            if now.1 && !was.1 {
                out.push(Sound::LullabySlam);
            }
        }
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::Kind;
    use crate::sounds::Sound;

    #[test]
    fn each_tell_lunge_slam_and_spit_is_heard_once_when_it_begins() {
        let idle = (false, false, 0);
        assert_eq!(cues(Kind::Tooth, idle, (true, false, 0)), vec![Sound::ToothTell]);
        assert!(cues(Kind::Tooth, (true, false, 0), (true, false, 0)).is_empty(), "still telling: silent");
        assert_eq!(cues(Kind::Tooth, (true, false, 0), (false, true, 0)), vec![Sound::ToothLunge]);
        assert_eq!(cues(Kind::Lullaby, idle, (true, false, 0)), vec![Sound::LullabyTell]);
        assert_eq!(cues(Kind::Lullaby, (true, false, 0), (false, true, 0)), vec![Sound::LullabySlam]);
        assert_eq!(cues(Kind::Weeper, idle, (true, false, 0)), Vec::<Sound>::new(), "its tell is silent");
        assert_eq!(cues(Kind::Weeper, idle, (false, false, 1)), vec![Sound::WeeperSpit]);
        assert!(cues(Kind::Weeper, (false, false, 2), (false, false, 1)).is_empty(), "a tear landing is not a spit");
        assert!(cues(Kind::Mask, idle, (true, true, 3)).is_empty(), "other enemies are not its business");
    }

    use crate::dream::ALL_THEMES;

    fn count(plan: &[(Kind, usize)], kind: Kind) -> usize {
        plan.iter().filter(|p| p.0 == kind).map(|p| p.1).sum()
    }

    #[test]
    fn nothing_spawns_before_each_enemys_depth() {
        for theme in ALL_THEMES {
            for seed in 0..200 {
                assert!(count(&plan(2, theme, seed, false), Kind::Tooth) == 0);
                assert!(count(&plan(3, theme, seed, false), Kind::Weeper) == 0);
                assert!(count(&plan(4, theme, seed, false), Kind::Lullaby) == 0);
                assert!(plan(2, theme, seed, false).is_empty());
            }
        }
    }

    #[test]
    fn each_enemy_appears_at_its_depth_for_some_seed() {
        let any = |depth, kind| (0..200).any(|s| count(&plan(depth, DreamTheme::Garden, s, false), kind) > 0);
        assert!(any(3, Kind::Tooth));
        assert!(any(4, Kind::Weeper));
        assert!(any(5, Kind::Lullaby));
    }

    #[test]
    fn nightmares_and_the_opening_rooms_get_none() {
        for seed in 0..200 {
            for depth in 0..30 {
                assert!(plan(depth, DreamTheme::Garden, seed, true).is_empty());
                assert!(plan(depth, DreamTheme::Lobby, seed, false).is_empty());
                assert!(plan(depth, DreamTheme::Awakening, seed, false).is_empty());
                assert!(crate::tooth::spawn_plan(depth, DreamTheme::Garden, seed, true).is_empty());
                assert!(crate::weeper::spawn_plan(depth, DreamTheme::Garden, seed, true).is_empty());
                assert!(crate::lullaby::spawn_plan(depth, DreamTheme::Garden, seed, true).is_empty());
            }
        }
    }

    #[test]
    fn the_caps_hold_for_every_depth_theme_and_seed() {
        let mut most = 0;
        for theme in ALL_THEMES {
            for depth in 0..40 {
                for seed in 0..120 {
                    let p = plan(depth, theme, seed, false);
                    assert!(count(&p, Kind::Lullaby) <= 1, "{theme:?} {depth} {seed}");
                    assert!(count(&p, Kind::Weeper) <= 2);
                    assert!(count(&p, Kind::Tooth) <= 3);
                    let total: usize = p.iter().map(|x| x.1).sum();
                    assert!(total <= MAX_TOTAL, "{p:?}");
                    most = most.max(total);
                }
            }
        }
        assert_eq!(most, MAX_TOTAL, "the cap is reachable, so it is being tested");
    }

    #[test]
    fn plans_are_deterministic_and_the_spawn_rate_is_moderate() {
        assert_eq!(plan(9, DreamTheme::Garden, 77, false), plan(9, DreamTheme::Garden, 77, false));
        let with = (0..400).filter(|&s| !plan(9, DreamTheme::Garden, s, false).is_empty()).count();
        assert!((150..400).contains(&with), "{with}/400 dreams have new enemies");
        let lull = (0..400).filter(|&s| count(&plan(9, DreamTheme::Garden, s, false), Kind::Lullaby) > 0).count();
        assert!(lull < with);
    }

    #[test]
    fn the_dev_switch_names_map_to_kinds() {
        assert_eq!(forced("Tooth"), Some(Kind::Tooth));
        assert_eq!(forced("Weeper"), Some(Kind::Weeper));
        assert_eq!(forced("Lullaby"), Some(Kind::Lullaby));
        assert_eq!(forced("Pacer"), None);
        assert_eq!(forced_plan(Kind::Lullaby), vec![(Kind::Lullaby, 1)]);
    }

    #[test]
    fn placement_uses_distinct_spaced_lairs_and_never_invents_one() {
        let lairs: Vec<Vec3> = (0..10).map(|i| Vec3::new(i as f32 * CELL, 0.0, 0.0)).collect();
        let p = [(Kind::Lullaby, 1), (Kind::Weeper, 2), (Kind::Tooth, 3)];
        let placed = place(&p, &lairs, 5);
        assert!(!placed.is_empty());
        for (_, at) in &placed {
            assert!(lairs.contains(at));
        }
        for (i, (_, a)) in placed.iter().enumerate() {
            for (_, b) in &placed[i + 1..] {
                assert!(a.distance(*b) >= LAIR_SPACING - 1e-3);
            }
        }
        assert_eq!(placed, place(&p, &lairs, 5));
        assert!(place(&p, &[], 5).is_empty());
        assert_eq!(place(&[(Kind::Tooth, 3)], &lairs[..1], 5).len(), 1, "one lair, one tooth");
    }
}
