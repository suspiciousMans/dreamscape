//! Low-poly procedural meshes, flat-shaded (one normal per triangle) for the
//! PS2 look. Every shape fills the unit cube -0.5..=0.5, so `Transform.scale`
//! is its bounding size and matches its AABB collider.

use engine::glam::Vec3;
use engine::mesh::{MeshData, Vertex};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shape {
    Cube,
    Octahedron,
    Orb,
    Cylinder,
    Cone,
}

pub const ALL_SHAPES: [Shape; 5] = [
    Shape::Cube,
    Shape::Octahedron,
    Shape::Orb,
    Shape::Cylinder,
    Shape::Cone,
];

const SEGMENTS: usize = 10;

pub fn build(shape: Shape) -> MeshData {
    match shape {
        Shape::Cube => engine::mesh::primitives::cube(),
        Shape::Octahedron => octahedron(),
        Shape::Orb => orb(6, SEGMENTS),
        Shape::Cylinder => cylinder(SEGMENTS),
        Shape::Cone => cone(SEGMENTS),
    }
}

fn planar_uv(p: Vec3) -> [f32; 2] {
    [p.x + p.z + 0.5, p.y + 0.5]
}

/// One flat triangle, wound counter-clockwise seen from outside (the engine's
/// front face, same as `primitives::cube`). `inside` is any point on the inner
/// side; the winding is flipped if the given order faces inward.
fn tri(m: &mut MeshData, a: Vec3, b: Vec3, c: Vec3, inside: Vec3) {
    let mut n = (b - a).cross(c - a);
    let (b, c) = if n.dot((a + b + c) / 3.0 - inside) < 0.0 {
        n = -n;
        (c, b)
    } else {
        (b, c)
    };
    let n = n.normalize_or_zero();
    let base = m.vertices.len() as u32;
    for p in [a, b, c] {
        m.vertices.push(Vertex {
            position: p.to_array(),
            normal: n.to_array(),
            uv: planar_uv(p),
            color: [0.0; 3],
        });
    }
    m.indices.extend_from_slice(&[base, base + 1, base + 2]);
}

fn octahedron() -> MeshData {
    let mut m = MeshData::default();
    for sx in [-0.5, 0.5] {
        for sy in [-0.5, 0.5] {
            for sz in [-0.5, 0.5] {
                tri(&mut m, Vec3::X * sx, Vec3::Y * sy, Vec3::Z * sz, Vec3::ZERO);
            }
        }
    }
    m
}

/// Low-poly UV sphere. Pole triangles are emitted once (the degenerate half
/// of each pole quad is skipped).
fn orb(rings: usize, segs: usize) -> MeshData {
    let mut m = MeshData::default();
    let p = |i: usize, j: usize| {
        let th = PI * i as f32 / rings as f32;
        let ph = TAU * j as f32 / segs as f32;
        Vec3::new(th.sin() * ph.cos(), th.cos(), th.sin() * ph.sin()) * 0.5
    };
    for i in 0..rings {
        for j in 0..segs {
            let (a, b, c, d) = (p(i, j), p(i + 1, j), p(i + 1, j + 1), p(i, j + 1));
            if i + 1 != rings {
                tri(&mut m, a, b, c, Vec3::ZERO);
            }
            if i != 0 {
                tri(&mut m, a, c, d, Vec3::ZERO);
            }
        }
    }
    m
}

fn ring(j: usize, segs: usize, y: f32) -> Vec3 {
    let a = TAU * j as f32 / segs as f32;
    Vec3::new(0.5 * a.cos(), y, 0.5 * a.sin())
}

fn cylinder(segs: usize) -> MeshData {
    let mut m = MeshData::default();
    for j in 0..segs {
        let (b0, b1) = (ring(j, segs, -0.5), ring(j + 1, segs, -0.5));
        let (t0, t1) = (ring(j, segs, 0.5), ring(j + 1, segs, 0.5));
        tri(&mut m, b0, b1, t1, Vec3::ZERO);
        tri(&mut m, b0, t1, t0, Vec3::ZERO);
        tri(&mut m, Vec3::Y * 0.5, t0, t1, Vec3::ZERO);
        tri(&mut m, Vec3::Y * -0.5, b1, b0, Vec3::ZERO);
    }
    m
}

fn cone(segs: usize) -> MeshData {
    let mut m = MeshData::default();
    for j in 0..segs {
        let (b0, b1) = (ring(j, segs, -0.5), ring(j + 1, segs, -0.5));
        tri(&mut m, b0, b1, Vec3::Y * 0.5, Vec3::ZERO);
        tri(&mut m, Vec3::Y * -0.5, b1, b0, Vec3::ZERO);
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tris(m: &MeshData) -> Vec<[Vec3; 3]> {
        m.indices
            .chunks(3)
            .map(|t| [0, 1, 2].map(|k| Vec3::from(m.vertices[t[k] as usize].position)))
            .collect()
    }

    #[test]
    fn indices_form_valid_triangles() {
        for s in ALL_SHAPES {
            let m = build(s);
            assert!(!m.indices.is_empty(), "{s:?}");
            assert_eq!(m.indices.len() % 3, 0, "{s:?}");
            assert!(
                m.indices.iter().all(|&i| (i as usize) < m.vertices.len()),
                "{s:?}"
            );
        }
    }

    #[test]
    fn fits_and_fills_the_unit_cube() {
        for s in ALL_SHAPES {
            let mut max = Vec3::ZERO;
            for v in &build(s).vertices {
                let p = Vec3::from(v.position).abs();
                assert!(
                    p.max_element() <= 0.5 + 1e-4,
                    "{s:?} vertex {p:?} outside unit cube"
                );
                max = max.max(p);
            }
            assert!(max.min_element() >= 0.47, "{s:?} only reaches {max:?}");
        }
    }

    #[test]
    fn no_degenerate_triangles() {
        for s in ALL_SHAPES {
            for [a, b, c] in tris(&build(s)) {
                assert!(
                    (b - a).cross(c - a).length() > 1e-6,
                    "{s:?} degenerate {a:?} {b:?} {c:?}"
                );
            }
        }
    }

    #[test]
    fn normals_are_unit_length() {
        for s in ALL_SHAPES {
            for v in &build(s).vertices {
                let len = Vec3::from(v.normal).length();
                assert!((len - 1.0).abs() < 1e-4, "{s:?} normal length {len}");
            }
        }
    }

    /// Cube is the engine's own mesh, so it anchors the convention: every
    /// shape must wind the same way (CCW from outside, facing outward).
    #[test]
    fn winding_matches_the_engine_cube() {
        for s in ALL_SHAPES {
            let m = build(s);
            for (k, [a, b, c]) in tris(&m).into_iter().enumerate() {
                let geo = (b - a).cross(c - a);
                let stored = Vec3::from(m.vertices[m.indices[k * 3] as usize].normal);
                assert!(
                    geo.dot(stored) > 0.0,
                    "{s:?} tri {k}: winding disagrees with normal"
                );
                assert!(
                    stored.dot((a + b + c) / 3.0) > 0.0,
                    "{s:?} tri {k}: faces inward"
                );
            }
        }
    }
}
