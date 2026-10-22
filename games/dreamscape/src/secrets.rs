//! Secret rooms, the game's half: what breaks a cracked wall, how it looks,
//! and when you walk through the door it reveals. Pure; `main.rs` wires it
//! to the world. (Where the wall goes is `dream::secret`.)

use crate::gameplay::CELL;
use crate::upgrades::Ability;
use engine::glam::Vec3;
use rand::{rngs::StdRng, Rng, SeedableRng};

/// How near (centre to centre, flat) you must be for a strike to reach the wall.
pub const REACH: f32 = CELL * 1.8;
/// How squarely you must face it (cosine of the angle).
pub const FACING_DOT: f32 = 0.6;
/// How close to the door's centre counts as stepping through.
pub const DOOR_RADIUS: f32 = CELL * 0.6;
/// You notice the crack from this far (flat).
pub const NOTICE: f32 = CELL * 2.6;

/// The first-time hint (once per launch).
pub const HINT: &str = "THE WALL IS CRACKED. SOMETHING QUICK MIGHT GET THROUGH.";
/// Shown when it gives way.
pub const OPENED: &str = "A DOOR. THE MOTH IS EXPECTING YOU.";

fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

/// Only Blink and Dash break a cracked wall.
pub fn breaks_walls(ability: Ability) -> bool {
    matches!(ability, Ability::Blink | Ability::Dash)
}

/// Does `ability`, used (or running) at `player` facing `facing`, strike the
/// cracked wall at `wall`?
pub fn strikes(ability: Ability, player: Vec3, facing: Vec3, wall: Vec3) -> bool {
    if !breaks_walls(ability) {
        return false;
    }
    let to = flat(wall - player);
    let dir = flat(facing);
    if to.length() > REACH || to.length() < 1e-3 || dir.length() < 1e-3 {
        return false;
    }
    to.normalize().dot(dir.normalize()) >= FACING_DOT
}

/// Standing in the doorway.
pub fn at_door(player: Vec3, door: Vec3) -> bool {
    flat(player - door).length() <= DOOR_RADIUS
}

/// Close enough to see the crack.
pub fn notices(player: Vec3, wall: Vec3) -> bool {
    flat(player - wall).length() <= NOTICE
}

/// Draws a jagged hairline crack down the middle of a `size`x`size` RGBA
/// wall texture: a bright core with a dim halo, so it reads as light
/// leaking through rather than as pattern.
pub fn draw_crack(rgba: &mut [u8], size: u32, seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed ^ 0xC4AC_C4AC);
    let n = size as i32;
    let mut x = n / 2;
    let mut paint = |x: i32, y: i32, c: [u8; 3], mix: f32| {
        if x < 0 || x >= n || y < 0 || y >= n {
            return;
        }
        let i = ((y * n + x) * 4) as usize;
        for k in 0..3 {
            rgba[i + k] = (rgba[i + k] as f32 * (1.0 - mix) + c[k] as f32 * mix) as u8;
        }
        rgba[i + 3] = 255;
    };
    for y in 0..n {
        x = (x + rng.gen_range(-1..=1)).clamp(n / 2 - 6, n / 2 + 6);
        paint(x - 2, y, [200, 220, 255], 0.2);
        paint(x + 2, y, [200, 220, 255], 0.2);
        paint(x - 1, y, [200, 220, 255], 0.45);
        paint(x + 1, y, [200, 220, 255], 0.45);
        paint(x, y, [250, 252, 255], 1.0);
        // A hairline branch now and then.
        if rng.gen_ratio(1, 9) {
            let dir = if rng.gen_bool(0.5) { 1 } else { -1 };
            for step in 1..=rng.gen_range(3..8) {
                paint(x + dir * step, y + step / 2, [250, 252, 255], 0.9);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WALL: Vec3 = Vec3::new(0.0, 0.0, 0.0);

    #[test]
    fn only_blink_and_dash_break_it() {
        let player = Vec3::new(CELL, 1.0, 0.0);
        for a in crate::upgrades::ALL_ABILITIES {
            let want = matches!(a, Ability::Blink | Ability::Dash);
            assert_eq!(strikes(a, player, -Vec3::X, WALL), want, "{a:?}");
        }
    }

    #[test]
    fn you_must_be_near_and_facing_it() {
        let near = Vec3::new(CELL, 1.0, 0.0);
        assert!(strikes(Ability::Dash, near, -Vec3::X, WALL));
        assert!(!strikes(Ability::Dash, near, Vec3::X, WALL), "facing away");
        assert!(!strikes(Ability::Dash, near, Vec3::Z, WALL), "facing aside");
        assert!(!strikes(Ability::Blink, Vec3::new(REACH + 0.5, 1.0, 0.0), -Vec3::X, WALL));
        assert!(!strikes(Ability::Dash, near, Vec3::ZERO, WALL));
        assert!(!strikes(Ability::Dash, WALL, -Vec3::X, WALL), "inside it");
    }

    #[test]
    fn height_is_ignored_the_doorway_is_small_and_the_crack_is_noticed_from_a_few_cells() {
        assert!(strikes(Ability::Blink, Vec3::new(CELL, 4.0, 0.0), -Vec3::X, WALL));
        assert!(at_door(Vec3::new(0.5, 1.0, 0.0), WALL));
        assert!(!at_door(Vec3::new(CELL, 1.0, 0.0), WALL));
        assert!(notices(Vec3::new(2.0 * CELL, 1.0, 0.0), WALL));
        assert!(!notices(Vec3::new(4.0 * CELL, 1.0, 0.0), WALL));
    }

    #[test]
    fn the_crack_is_drawn_bright_in_the_middle_and_leaves_the_edges_alone() {
        let size = 64u32;
        let base = vec![40u8; (size * size * 4) as usize];
        let mut a = base.clone();
        draw_crack(&mut a, size, 7);
        let mut b = base.clone();
        draw_crack(&mut b, size, 7);
        assert_eq!(a, b, "deterministic");
        let mut c = base.clone();
        draw_crack(&mut c, size, 8);
        assert_ne!(a, c, "each wall cracks differently");
        let bright = a.chunks(4).filter(|p| p[0] > 240).count();
        assert!((size as usize..size as usize * 3).contains(&bright), "{bright}");
        for y in 0..size {
            for x in [0, 1, size - 2, size - 1] {
                let i = ((y * size + x) * 4) as usize;
                assert_eq!(a[i], 40, "edge pixel ({x},{y}) changed");
            }
        }
    }
}
