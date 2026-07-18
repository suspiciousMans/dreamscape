use glam::{Quat, Vec3};

use crate::ecs::Transform;

/// Two procedural motion kinds — not a full keyframe/timeline system, just
/// enough to make level objects feel alive with zero authored keyframes.
#[derive(Clone, Copy, Debug)]
pub enum AnimationKind {
    /// Continuous rotation around `axis` at `speed_deg_per_sec`.
    Orbit { axis: Vec3, speed_deg_per_sec: f32 },
    /// Sinusoidal position offset along `axis`, `amplitude` units each way,
    /// completing one full cycle every `period_secs`.
    Bob {
        axis: Vec3,
        amplitude: f32,
        period_secs: f32,
    },
}

/// Drives an entity's `Transform` from a fixed base pose + `kind` + elapsed
/// time, rather than mutating the `Transform` incrementally — so the motion
/// is stable/reversible and never drifts.
pub struct Animator {
    pub kind: AnimationKind,
    pub base_position: Vec3,
    pub base_rotation: Quat,
    pub elapsed: f32,
}

/// Advances every animated entity's `elapsed` time and recomputes its
/// `Transform` from `Animator::base_position`/`base_rotation` + `kind`. Runs
/// unconditionally every frame (both Edit and Play mode) — a live preview
/// while editing costs nothing extra and is a nice default. A non-dynamic
/// animated object (no `RigidBody`) still pushes anything that touches it in
/// Play mode: the physics collision loop checks every `Collider` regardless
/// of `RigidBody` presence, so this doubles as a free kinematic mover with no
/// special-casing.
pub fn step(world: &mut hecs::World, dt: f32) {
    for (_entity, (transform, animator)) in
        world.query::<(&mut Transform, &mut Animator)>().iter()
    {
        animator.elapsed += dt;
        match animator.kind {
            AnimationKind::Orbit { axis, speed_deg_per_sec } => {
                let angle = (speed_deg_per_sec * animator.elapsed).to_radians();
                let spin = Quat::from_axis_angle(axis.normalize_or_zero(), angle);
                transform.position = animator.base_position;
                transform.rotation = spin * animator.base_rotation;
            }
            AnimationKind::Bob { axis, amplitude, period_secs } => {
                let phase = if period_secs > 0.0 {
                    (animator.elapsed / period_secs) * std::f32::consts::TAU
                } else {
                    0.0
                };
                let offset = axis.normalize_or_zero() * amplitude * phase.sin();
                transform.position = animator.base_position + offset;
                transform.rotation = animator.base_rotation;
            }
        }
    }
}
