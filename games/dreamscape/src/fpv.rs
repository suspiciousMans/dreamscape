//! First-person view: a yaw/pitch camera at eye height, movement relative to
//! where you look, and the compass turned into your frame. Pure maths.
//! Convention: yaw 0 looks along +Z (same way the third-person camera faces);
//! screen-right is `forward × Y` (right-handed, so -X at yaw 0).

use crate::gameplay::{PlayerInputState, MOVE_SPEED};
use engine::glam::Vec3;
use std::f32::consts::{PI, TAU};

/// Eye height above the body's centre (the centre floats 0.5 over the floor).
pub const EYE_HEIGHT: f32 = 0.6;
pub const FOV_DEG: f32 = 75.0;
pub const PITCH_LIMIT: f32 = 1.2;
/// Radians per mouse pixel at sensitivity 0, plus RANGE at sensitivity 1.
pub const MOUSE_BASE: f32 = 0.0012;
pub const MOUSE_RANGE: f32 = 0.0048;
/// Radians per second: right stick fully pushed / arrow keys held.
pub const STICK_TURN: f32 = 2.6;
pub const KEY_TURN: f32 = 2.2;
/// A stalker counts as "seen" inside this half-angle (a bit narrower than
/// the real horizontal half-FOV of ~54°, so the very edge doesn't count).
pub const VIEW_HALF_ANGLE: f32 = 50.0 * PI / 180.0;

pub fn forward(yaw: f32) -> Vec3 {
    Vec3::new(yaw.sin(), 0.0, yaw.cos())
}

pub fn right(yaw: f32) -> Vec3 {
    forward(yaw).cross(Vec3::Y)
}

pub fn velocity(input: &PlayerInputState, yaw: f32) -> Vec3 {
    let (f, r) = (forward(yaw), right(yaw));
    let mut v = Vec3::ZERO;
    if input.forward {
        v += f;
    }
    if input.backward {
        v -= f;
    }
    if input.right {
        v += r;
    }
    if input.left {
        v -= r;
    }
    v.normalize_or_zero() * MOVE_SPEED
}

/// `settings::stick_dir` returns a third-person world direction (x = -stick
/// right, z = stick up). Re-read it in the dreamer's own frame.
pub fn from_world_stick(wx: f32, wz: f32, yaw: f32) -> Vec3 {
    right(yaw) * -wx + forward(yaw) * wz
}

/// Mouse (pixels) -> new (yaw, pitch). Moving the mouse right turns right.
pub fn turn(yaw: f32, pitch: f32, dx: f32, dy: f32, sens: f32, invert_y: bool) -> (f32, f32) {
    let k = MOUSE_BASE + MOUSE_RANGE * sens.clamp(0.0, 1.0);
    let dy = if invert_y { -dy } else { dy };
    (
        (yaw - dx * k).rem_euclid(TAU),
        (pitch - dy * k).clamp(-PITCH_LIMIT, PITCH_LIMIT),
    )
}

/// (eye, target) for `Mat4::look_at_rh`.
pub fn camera(body: Vec3, yaw: f32, pitch: f32) -> (Vec3, Vec3) {
    let eye = body + Vec3::Y * EYE_HEIGHT;
    let dir = Vec3::new(
        pitch.cos() * yaw.sin(),
        pitch.sin(),
        pitch.cos() * yaw.cos(),
    );
    (eye, eye + dir)
}

/// Screen direction (x right, y down) to `target`, in the dreamer's frame:
/// straight ahead points up. `None` when close enough to just see.
pub fn compass(player: Vec3, yaw: f32, target: Vec3, min_dist: f32) -> Option<[f32; 2]> {
    let d = Vec3::new(target.x - player.x, 0.0, target.z - player.z);
    let len = d.length();
    (len >= min_dist).then(|| [d.dot(right(yaw)) / len, -d.dot(forward(yaw)) / len])
}

/// Inside the clear sight radius and inside the view cone.
pub fn sees(player: Vec3, yaw: f32, pos: Vec3, clear: f32) -> bool {
    let d = Vec3::new(pos.x - player.x, 0.0, pos.z - player.z);
    let len = d.length();
    len < clear && (len < 1e-3 || (d / len).dot(forward(yaw)) >= VIEW_HALF_ANGLE.cos())
}

/// Yaw that looks along `v` (for the autopilot and for spawning facing the route).
pub fn yaw_toward(v: Vec3) -> Option<f32> {
    (v.x * v.x + v.z * v.z > 1e-6).then(|| v.x.atan2(v.z))
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::glam::Mat4;

    const YAWS: [f32; 6] = [0.0, 0.7, 1.9, 3.1, 4.4, 5.9];

    fn view(yaw: f32) -> Mat4 {
        let (eye, target) = camera(Vec3::ZERO, yaw, 0.0);
        Mat4::look_at_rh(eye, target, Vec3::Y)
    }

    fn in_view_space(yaw: f32, world_dir: Vec3) -> Vec3 {
        let v = view(yaw);
        v.transform_point3(world_dir) - v.transform_point3(Vec3::ZERO)
    }

    #[test]
    fn forward_moves_where_you_look_and_right_moves_right_on_screen() {
        for yaw in YAWS {
            let f = velocity(
                &PlayerInputState {
                    forward: true,
                    ..Default::default()
                },
                yaw,
            );
            assert!(f.normalize().dot(forward(yaw)) > 0.999);
            assert!(
                in_view_space(yaw, f).z < 0.0,
                "forward goes into the screen"
            );
            let r = velocity(
                &PlayerInputState {
                    right: true,
                    ..Default::default()
                },
                yaw,
            );
            assert!(
                in_view_space(yaw, r).x > 0.0,
                "yaw {yaw}: right key went left"
            );
        }
    }

    #[test]
    fn mouse_right_turns_right_and_pitch_is_clamped() {
        for yaw in YAWS {
            let (y2, _) = turn(yaw, 0.0, 40.0, 0.0, 0.5, false);
            assert!(forward(y2).dot(right(yaw)) > 0.0, "turned the wrong way");
        }
        let (_, p) = turn(0.0, 0.0, 0.0, 1e6, 1.0, false);
        assert_eq!(p, -PITCH_LIMIT, "mouse down looks down");
        let (_, p) = turn(0.0, 0.0, 0.0, 1e6, 1.0, true);
        assert_eq!(p, PITCH_LIMIT, "inverted");
    }

    #[test]
    fn pushing_the_stick_up_walks_forward() {
        let (wx, wz) = crate::settings::stick_dir((0.0, -1.0)).unwrap();
        for yaw in YAWS {
            assert!(from_world_stick(wx, wz, yaw).normalize().dot(forward(yaw)) > 0.99);
        }
    }

    #[test]
    fn compass_matches_the_screen() {
        for yaw in YAWS {
            let ahead = compass(Vec3::ZERO, yaw, forward(yaw) * 20.0, 5.0).unwrap();
            assert!(ahead[1] < -0.99, "ahead is up");
            let side = compass(Vec3::ZERO, yaw, right(yaw) * 20.0, 5.0).unwrap();
            assert!(side[0] > 0.99, "right is right");
            // And it agrees with where the target actually lands on screen.
            let t = forward(yaw) * 10.0 + right(yaw) * 4.0;
            assert!(
                in_view_space(yaw, t).x > 0.0 && compass(Vec3::ZERO, yaw, t, 5.0).unwrap()[0] > 0.0
            );
        }
        assert!(compass(Vec3::ZERO, 0.0, Vec3::Z * 2.0, 5.0).is_none());
    }

    #[test]
    fn sees_what_is_in_front_only() {
        for yaw in YAWS {
            assert!(sees(Vec3::ZERO, yaw, forward(yaw) * 5.0, 10.0));
            assert!(!sees(Vec3::ZERO, yaw, -forward(yaw) * 5.0, 10.0), "behind");
            assert!(!sees(Vec3::ZERO, yaw, right(yaw) * 5.0, 10.0), "90° off");
            assert!(
                !sees(Vec3::ZERO, yaw, forward(yaw) * 15.0, 10.0),
                "past clear sight"
            );
        }
    }

    #[test]
    fn yaw_toward_round_trips() {
        for yaw in YAWS {
            let y = yaw_toward(forward(yaw)).unwrap();
            assert!(forward(y).dot(forward(yaw)) > 0.9999);
        }
        assert!(yaw_toward(Vec3::Y).is_none());
    }
}
