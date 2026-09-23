//! Gameplay tuning + pure helpers. No GL calls — everything here is unit-tested.

use engine::glam::Vec3;
use engine::profile::ShaderProfile;

/// Side length of one dream-grid cell, in world units.
pub const CELL: f32 = 3.0;
/// Floor slab thickness; slab tops sit at y = 0.
pub const SLAB: f32 = 0.2;
pub const PLAYER_RADIUS: f32 = 0.5;
pub const MOVE_SPEED: f32 = 6.0;
pub const JUMP_SPEED: f32 = 6.0;
pub const KILL_Y: f32 = -10.0;
pub const ENEMY_TOUCH_RADIUS: f32 = 1.0;
/// Behind (-Z) and well above the player, so 3-unit maze walls don't hide them.
pub const CAMERA_OFFSET: Vec3 = Vec3::new(0.0, 12.0, -6.0);
pub const CAMERA_LERP_PER_SEC: f32 = 5.0;
/// A long frame (e.g. during dream generation) must not become one giant
/// physics step that tunnels through a 0.2-thick floor.
pub const MAX_DT: f32 = 1.0 / 30.0;

#[derive(Debug, Clone, Default)]
pub struct PlayerInputState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
}

/// Marks the entity that ends the current dream when the player overlaps it.
#[derive(Debug, Clone, Copy)]
pub struct PortalMarker;

/// Camera sits at -Z looking toward +Z, so (right-handed) screen-right is
/// world -X. W = +Z = away from the camera.
pub fn horizontal_velocity(input: &PlayerInputState) -> Vec3 {
    let mut dir = Vec3::ZERO;
    if input.forward {
        dir.z += 1.0;
    }
    if input.backward {
        dir.z -= 1.0;
    }
    if input.right {
        dir.x -= 1.0;
    }
    if input.left {
        dir.x += 1.0;
    }
    dir.normalize_or_zero() * MOVE_SPEED
}

/// Debug/E2E only: walk straight at `target` on the XZ plane.
pub fn autopilot_velocity(pos: Vec3, target: Vec3) -> Vec3 {
    Vec3::new(target.x - pos.x, 0.0, target.z - pos.z).normalize_or_zero() * MOVE_SPEED
}

pub fn follow_camera(current: Vec3, player: Vec3, dt: f32) -> Vec3 {
    current.lerp(
        player + CAMERA_OFFSET,
        (CAMERA_LERP_PER_SEC * dt).clamp(0.0, 1.0),
    )
}

pub fn fell_out(pos: Vec3) -> bool {
    pos.y < KILL_Y
}

pub fn touches(a: Vec3, b: Vec3, radius: f32) -> bool {
    a.distance(b) < radius
}

pub fn profile_index(profiles: &[ShaderProfile], name: &str) -> Option<usize> {
    profiles.iter().position(|p| p.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::ecs::Transform;
    use engine::glam::Mat4;
    use engine::physics::{step, Collider, ColliderShape, PhysicsParams, RigidBody};

    fn screen_delta(world_velocity: Vec3) -> Vec3 {
        let view = Mat4::look_at_rh(CAMERA_OFFSET, Vec3::ZERO, Vec3::Y);
        view.transform_point3(world_velocity) - view.transform_point3(Vec3::ZERO)
    }

    #[test]
    fn right_key_moves_right_on_screen() {
        let v = horizontal_velocity(&PlayerInputState {
            right: true,
            ..Default::default()
        });
        assert!(
            screen_delta(v).x > 0.0,
            "D moved {:?} in view space",
            screen_delta(v)
        );
    }

    #[test]
    fn forward_key_moves_away_from_camera() {
        let v = horizontal_velocity(&PlayerInputState {
            forward: true,
            ..Default::default()
        });
        assert!(
            screen_delta(v).z < 0.0,
            "W moved {:?} in view space",
            screen_delta(v)
        );
    }

    #[test]
    fn diagonal_is_not_faster() {
        let v = horizontal_velocity(&PlayerInputState {
            forward: true,
            right: true,
            ..Default::default()
        });
        assert!((v.length() - MOVE_SPEED).abs() < 1e-4);
    }

    #[test]
    fn autopilot_heads_for_target_on_the_ground_plane() {
        let v = autopilot_velocity(Vec3::new(0.0, 0.6, 0.0), Vec3::new(10.0, 1.0, 0.0));
        assert!((v - Vec3::new(MOVE_SPEED, 0.0, 0.0)).length() < 1e-4);
    }

    #[test]
    fn camera_converges_on_offset_target() {
        let player = Vec3::new(3.0, 0.5, 7.0);
        let mut cam = Vec3::ZERO;
        for _ in 0..300 {
            cam = follow_camera(cam, player, 1.0 / 60.0);
        }
        assert!((cam - (player + CAMERA_OFFSET)).length() < 0.01);
    }

    #[test]
    fn camera_lags_instead_of_snapping() {
        let cam = follow_camera(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), 1.0 / 60.0);
        assert!(cam.x > 0.0 && cam.x < 10.0);
    }

    #[test]
    fn camera_sees_over_a_3_high_wall_just_behind_the_player() {
        // A player centred in a maze corridor has a wall face 1.5 units behind them.
        let t = 1.5 / -CAMERA_OFFSET.z; // fraction of the way to the camera
        let sight_line_y = 0.5 + CAMERA_OFFSET.y * t;
        assert!(sight_line_y > 3.2, "sight line at y={sight_line_y}");
    }

    #[test]
    fn fail_states() {
        assert!(fell_out(Vec3::new(0.0, KILL_Y - 0.1, 0.0)));
        assert!(!fell_out(Vec3::new(0.0, 0.5, 0.0)));
        assert!(touches(
            Vec3::ZERO,
            Vec3::new(0.5, 0.0, 0.0),
            ENEMY_TOUCH_RADIUS
        ));
        assert!(!touches(
            Vec3::ZERO,
            Vec3::new(3.0, 0.0, 0.0),
            ENEMY_TOUCH_RADIUS
        ));
    }

    /// A one-cell floor slab centred at (x, -SLAB/2, 0), top face at y = 0.
    fn slab(world: &mut hecs::World, x: f32) {
        world.spawn((
            Transform::from_position(Vec3::new(x, -SLAB * 0.5, 0.0)),
            Collider {
                shape: ColliderShape::Aabb {
                    half_extents: Vec3::new(CELL * 0.5, SLAB * 0.5, CELL * 0.5),
                },
                is_trigger: false,
            },
        ));
    }

    fn player(world: &mut hecs::World, at: Vec3) -> hecs::Entity {
        world.spawn((
            Transform::from_position(at),
            RigidBody::default(),
            Collider {
                shape: ColliderShape::Sphere {
                    radius: PLAYER_RADIUS,
                },
                is_trigger: false,
            },
        ))
    }

    #[test]
    fn player_lands_on_a_slab_and_is_grounded() {
        let mut world = hecs::World::new();
        slab(&mut world, 0.0);
        let p = player(&mut world, Vec3::new(0.0, 1.5, 0.0));
        for _ in 0..180 {
            step(&mut world, 1.0 / 60.0, &PhysicsParams::default());
        }
        let y = world.get::<&Transform>(p).unwrap().position.y;
        assert!((y - PLAYER_RADIUS).abs() < 0.05, "resting y = {y}");
        assert!(world.get::<&RigidBody>(p).unwrap().grounded);
    }

    #[test]
    fn player_can_jump_a_one_cell_gap() {
        let mut world = hecs::World::new();
        slab(&mut world, 0.0);
        slab(&mut world, 2.0 * CELL); // exactly one empty cell between the slabs
        let p = player(&mut world, Vec3::new(0.0, 1.0, 0.0));
        let params = PhysicsParams::default();
        for _ in 0..60 {
            step(&mut world, 1.0 / 60.0, &params);
        }
        world.get::<&mut RigidBody>(p).unwrap().velocity.y = JUMP_SPEED;
        for _ in 0..240 {
            let x = world.get::<&Transform>(p).unwrap().position.x;
            world.get::<&mut RigidBody>(p).unwrap().velocity.x =
                if x < 2.0 * CELL { MOVE_SPEED } else { 0.0 };
            step(&mut world, 1.0 / 60.0, &params);
        }
        let pos = world.get::<&Transform>(p).unwrap().position;
        assert!(pos.y > 0.3, "fell into the gap: {pos:?}");
        assert!((pos.x - 2.0 * CELL).abs() < 1.0, "landed at {pos:?}");
    }

    #[test]
    fn walking_into_a_portal_reports_an_overlap() {
        let mut world = hecs::World::new();
        slab(&mut world, 0.0);
        let portal = world.spawn((
            Transform::from_position(Vec3::new(0.0, 1.0, 0.0)),
            Collider {
                shape: ColliderShape::Aabb {
                    half_extents: Vec3::new(0.75, 1.0, 0.75),
                },
                is_trigger: true,
            },
            PortalMarker,
        ));
        let p = player(&mut world, Vec3::new(0.0, 0.5, 0.0));
        let overlaps = step(&mut world, 1.0 / 60.0, &PhysicsParams::default());
        assert!(overlaps.contains(&(p, portal)), "overlaps = {overlaps:?}");
    }
}
