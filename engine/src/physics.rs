use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::ecs::Transform;

/// Global tuning knobs for the simulation. Deliberately just two numbers —
/// this is meant to be an easily-editable physics engine, not a
/// feature-complete one.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PhysicsParams {
    pub gravity: f32,
    /// Fraction of velocity removed per second (crude air/ground drag).
    pub linear_damping: f32,
}

impl Default for PhysicsParams {
    fn default() -> Self {
        Self {
            gravity: 9.8,
            linear_damping: 0.15,
        }
    }
}

/// Marks an entity as simulated. Its *absence* is what makes an object with
/// a `Collider` static (collided against, but never moved) — deliberately
/// not a separate `is_dynamic` flag, so there's only one thing to keep in
/// sync rather than two.
#[derive(Clone, Copy, Debug, Default)]
pub struct RigidBody {
    pub velocity: Vec3,
    pub grounded: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum ColliderShape {
    Sphere { radius: f32 },
    Aabb { half_extents: Vec3 },
}

#[derive(Clone, Copy, Debug)]
pub struct Collider {
    pub shape: ColliderShape,
    /// A trigger is never physically solid — overlapping it never pushes
    /// anything out or zeroes velocity, it's only ever reported in `step`'s
    /// returned overlap list for game logic (see `Sandbox::on_trigger_entered`
    /// in the sandbox for the demo use).
    pub is_trigger: bool,
}

/// Advances the simulation by `dt` seconds: integrates every dynamic body
/// (gravity + damping + move), then resolves collisions by pushing dynamic
/// bodies out of whatever they overlap along the axis of least penetration —
/// except pairs involving a trigger, which are reported (dynamic entity,
/// other entity) in the returned list instead of being physically resolved.
///
/// Simplifying assumptions (fine for a simple engine, worth knowing about):
/// axis-aligned boxes only (no rotation/torque), and only one resolution
/// pass per frame (a dynamic body resting on two overlapping surfaces can
/// jitter slightly rather than settle perfectly) — good enough for a level
/// full of boxes and a player sphere, not a general-purpose solver.
pub fn step(
    world: &mut hecs::World,
    dt: f32,
    params: &PhysicsParams,
) -> Vec<(hecs::Entity, hecs::Entity)> {
    for (_entity, (transform, body)) in world.query::<(&mut Transform, &mut RigidBody)>().iter() {
        body.velocity.y -= params.gravity * dt;
        let damping = (1.0 - params.linear_damping * dt).max(0.0);
        body.velocity *= damping;
        transform.position += body.velocity * dt;
        body.grounded = false;
    }

    // Collected up front so resolution can freely use `query_one` per entity
    // below without holding this query's borrow of `world` at the same time.
    let colliders: Vec<(hecs::Entity, Vec3, ColliderShape, bool, bool)> = world
        .query::<(&Transform, &Collider, Option<&RigidBody>)>()
        .iter()
        .map(|(entity, (transform, collider, body))| {
            (
                entity,
                transform.position,
                collider.shape,
                body.is_some(),
                collider.is_trigger,
            )
        })
        .collect();

    let mut trigger_overlaps = Vec::new();

    for &(entity, position, shape, is_dynamic, is_trigger) in &colliders {
        if !is_dynamic {
            continue;
        }
        for &(other_entity, other_position, other_shape, other_is_dynamic, other_is_trigger) in
            &colliders
        {
            if other_entity == entity {
                continue;
            }
            let Some(overlap) = resolve_overlap(position, shape, other_position, other_shape)
            else {
                continue;
            };

            if is_trigger || other_is_trigger {
                trigger_overlaps.push((entity, other_entity));
                continue;
            }

            if let Ok(mut query) = world.query_one::<(&mut Transform, &mut RigidBody)>(entity) {
                if let Some((transform, body)) = query.get() {
                    // When both bodies are dynamic, each one runs this loop as
                    // the outer entity and pushes *itself* out — against the
                    // same frozen snapshot positions — so applying the full
                    // depth to each would separate the pair by ~2x the
                    // penetration and pop them apart. Split the correction so
                    // the two half-pushes sum to exactly one penetration depth.
                    let push = if other_is_dynamic {
                        overlap.push * 0.5
                    } else {
                        overlap.push
                    };
                    transform.position += push;
                    let into_surface = body.velocity.dot(overlap.normal);
                    if into_surface < 0.0 {
                        body.velocity -= overlap.normal * into_surface;
                    }
                    if overlap.normal.y > 0.5 {
                        body.grounded = true;
                    }
                }
            }
        }
    }

    trigger_overlaps
}

struct Overlap {
    /// How far (and which way) to move the first shape to separate them.
    push: Vec3,
    /// Points away from `other_shape`, toward `shape`.
    normal: Vec3,
}

fn resolve_overlap(
    position: Vec3,
    shape: ColliderShape,
    other_position: Vec3,
    other_shape: ColliderShape,
) -> Option<Overlap> {
    match (shape, other_shape) {
        (
            ColliderShape::Sphere { radius },
            ColliderShape::Sphere {
                radius: other_radius,
            },
        ) => sphere_vs_sphere(position, radius, other_position, other_radius),
        (
            ColliderShape::Aabb { half_extents },
            ColliderShape::Aabb {
                half_extents: other_half,
            },
        ) => aabb_vs_aabb(position, half_extents, other_position, other_half),
        (ColliderShape::Sphere { radius }, ColliderShape::Aabb { half_extents }) => {
            sphere_vs_aabb(position, radius, other_position, half_extents)
        }
        (ColliderShape::Aabb { half_extents }, ColliderShape::Sphere { radius }) => {
            sphere_vs_aabb(other_position, radius, position, half_extents).map(|o| Overlap {
                push: -o.push,
                normal: -o.normal,
            })
        }
    }
}

fn sphere_vs_sphere(a: Vec3, radius_a: f32, b: Vec3, radius_b: f32) -> Option<Overlap> {
    let delta = a - b;
    let distance = delta.length();
    let min_distance = radius_a + radius_b;
    if distance < min_distance && distance > 1e-5 {
        let normal = delta / distance;
        Some(Overlap {
            push: normal * (min_distance - distance),
            normal,
        })
    } else {
        None
    }
}

fn aabb_vs_aabb(a: Vec3, half_a: Vec3, b: Vec3, half_b: Vec3) -> Option<Overlap> {
    let delta = a - b;
    let overlap = half_a + half_b - delta.abs();
    if overlap.x <= 0.0 || overlap.y <= 0.0 || overlap.z <= 0.0 {
        return None;
    }
    // Push out along whichever axis has the smallest overlap.
    if overlap.x < overlap.y && overlap.x < overlap.z {
        let normal = Vec3::new(delta.x.signum(), 0.0, 0.0);
        Some(Overlap {
            push: normal * overlap.x,
            normal,
        })
    } else if overlap.y < overlap.z {
        let normal = Vec3::new(0.0, delta.y.signum(), 0.0);
        Some(Overlap {
            push: normal * overlap.y,
            normal,
        })
    } else {
        let normal = Vec3::new(0.0, 0.0, delta.z.signum());
        Some(Overlap {
            push: normal * overlap.z,
            normal,
        })
    }
}

fn sphere_vs_aabb(
    sphere_pos: Vec3,
    radius: f32,
    box_pos: Vec3,
    half_extents: Vec3,
) -> Option<Overlap> {
    let local = sphere_pos - box_pos;
    let closest_local = local.clamp(-half_extents, half_extents);
    let diff = local - closest_local;
    let distance = diff.length();
    if distance > 1e-5 {
        // Normal case: the sphere center is outside the box, so `diff` points
        // from the nearest surface point out to the center.
        if distance < radius {
            let normal = diff / distance;
            Some(Overlap {
                push: normal * (radius - distance),
                normal,
            })
        } else {
            None
        }
    } else {
        // The sphere center is *inside* the box: the clamped closest point is
        // the center itself, so `diff` is ~zero and there's no surface
        // direction to eject along. Without this branch the sphere would
        // simply pass through (a fast fall with a large `dt`, or an entity
        // spawned overlapping thick geometry). Eject along the axis whose
        // face the center is nearest to — the minimum-translation axis, same
        // approach `aabb_vs_aabb` uses — by that remaining depth plus the
        // full radius so the sphere clears the box entirely.
        let penetration = half_extents - local.abs();
        let (axis, depth) = if penetration.x < penetration.y && penetration.x < penetration.z {
            (Vec3::X, penetration.x)
        } else if penetration.y < penetration.z {
            (Vec3::Y, penetration.y)
        } else {
            (Vec3::Z, penetration.z)
        };
        // Push toward whichever side of the mid-plane the center sits on
        // (`signum()` of an exact-zero component is +1, a fine arbitrary pick).
        let normal = axis * local.dot(axis).signum();
        Some(Overlap {
            push: normal * (depth + radius),
            normal,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_center_inside_box_is_ejected() {
        // Deep-penetration case: a sphere whose center sits *inside* the AABB
        // must still produce a separating push rather than returning None and
        // silently tunnelling through.
        let sphere = ColliderShape::Sphere { radius: 0.5 };
        let aabb = ColliderShape::Aabb {
            half_extents: Vec3::splat(1.0),
        };
        // Center just above the box's own center — still well inside it.
        let overlap = resolve_overlap(Vec3::new(0.0, 0.2, 0.0), sphere, Vec3::ZERO, aabb)
            .expect("a sphere centered inside the box must be resolved");
        // Nearest face is +Y, so it ejects upward.
        assert!(
            overlap.normal.abs_diff_eq(Vec3::Y, 1e-4),
            "normal was {:?}",
            overlap.normal
        );
        assert!(
            overlap.push.y > 0.0,
            "push should be upward, was {:?}",
            overlap.push
        );
    }

    #[test]
    fn sphere_grazing_a_face_still_resolves() {
        // Regression guard for the ordinary center-outside path.
        let sphere = ColliderShape::Sphere { radius: 0.5 };
        let aabb = ColliderShape::Aabb {
            half_extents: Vec3::splat(1.0),
        };
        // Top face is at y = 1.0; a center at y = 1.3 with radius 0.5 overlaps by 0.2.
        let overlap = resolve_overlap(Vec3::new(0.0, 1.3, 0.0), sphere, Vec3::ZERO, aabb)
            .expect("a sphere grazing the top face must be resolved");
        assert!(
            overlap.normal.abs_diff_eq(Vec3::Y, 1e-4),
            "normal was {:?}",
            overlap.normal
        );
        assert!(
            (overlap.push.y - 0.2).abs() < 1e-3,
            "push.y was {}",
            overlap.push.y
        );
    }

    #[test]
    fn distant_shapes_do_not_overlap() {
        let sphere = ColliderShape::Sphere { radius: 0.5 };
        let aabb = ColliderShape::Aabb {
            half_extents: Vec3::splat(1.0),
        };
        assert!(resolve_overlap(Vec3::new(0.0, 5.0, 0.0), sphere, Vec3::ZERO, aabb).is_none());
    }
}
