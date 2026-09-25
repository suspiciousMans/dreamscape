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
    /// A hoop standing in the XY plane (hole along Z).
    Torus,
    /// A doorframe: two posts under a half-ring, in the XY plane.
    Arch,
    /// Lumpy cluster of blobs.
    Cloud,
    /// A crescent moon in the XY plane, horns pointing +X.
    Crescent,
    /// A floating rock: bumpy top, tapering to a point underneath.
    Island,
    /// Three steps climbing toward +Z.
    Stairs,
}

pub const ALL_SHAPES: [Shape; 11] = [
    Shape::Cube,
    Shape::Octahedron,
    Shape::Orb,
    Shape::Cylinder,
    Shape::Cone,
    Shape::Torus,
    Shape::Arch,
    Shape::Cloud,
    Shape::Crescent,
    Shape::Island,
    Shape::Stairs,
];

const SEGMENTS: usize = 10;

pub fn build(shape: Shape) -> MeshData {
    match shape {
        Shape::Cube => engine::mesh::primitives::cube(),
        Shape::Octahedron => octahedron(),
        Shape::Orb => orb(6, SEGMENTS),
        Shape::Cylinder => cylinder(SEGMENTS),
        Shape::Cone => cone(SEGMENTS),
        Shape::Torus => torus(16, 8),
        Shape::Arch => arch(8),
        Shape::Cloud => cloud(),
        Shape::Crescent => crescent(10),
        Shape::Island => island(),
        Shape::Stairs => stairs(),
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
    if n.length() < 1e-6 {
        // Where two pieces meet at a point (crescent horns): nothing to draw.
        return;
    }
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

/// A convex solid given as 8 corners: bottom face (0..4) then top face
/// (4..8), both in the same winding order. Faces point away from its centre.
fn hexahedron(m: &mut MeshData, c: [Vec3; 8]) {
    let inside = c.iter().copied().sum::<Vec3>() / 8.0;
    let quad = |m: &mut MeshData, a: Vec3, b: Vec3, cc: Vec3, d: Vec3| {
        tri(m, a, b, cc, inside);
        tri(m, a, cc, d, inside);
    };
    quad(m, c[0], c[1], c[2], c[3]);
    quad(m, c[4], c[5], c[6], c[7]);
    for i in 0..4 {
        let j = (i + 1) % 4;
        quad(m, c[i], c[j], c[4 + j], c[4 + i]);
    }
}

fn cuboid(m: &mut MeshData, min: Vec3, max: Vec3) {
    let p = |x: f32, y: f32, z: f32| Vec3::new(x, y, z);
    hexahedron(
        m,
        [
            p(min.x, min.y, min.z),
            p(max.x, min.y, min.z),
            p(max.x, min.y, max.z),
            p(min.x, min.y, max.z),
            p(min.x, max.y, min.z),
            p(max.x, max.y, min.z),
            p(max.x, max.y, max.z),
            p(min.x, max.y, max.z),
        ],
    );
}

/// A flat 2D quad strip (XY) extruded through the whole depth (Z = ±0.5).
fn extrude_quad(m: &mut MeshData, q: [Vec3; 4]) {
    let back = q.map(|p| Vec3::new(p.x, p.y, -0.5));
    let front = q.map(|p| Vec3::new(p.x, p.y, 0.5));
    hexahedron(
        m,
        [
            back[0], back[1], back[2], back[3], front[0], front[1], front[2], front[3],
        ],
    );
}

fn torus(major: usize, minor: usize) -> MeshData {
    let mut m = MeshData::default();
    let (r_major, r_tube) = (0.36, 0.14);
    let p = |i: usize, j: usize| {
        let u = TAU * i as f32 / major as f32;
        let v = TAU * j as f32 / minor as f32;
        let r = r_major + r_tube * v.cos();
        Vec3::new(r * u.cos(), r * u.sin(), 0.5 * v.sin())
    };
    for i in 0..major {
        let u = TAU * (i as f32 + 0.5) / major as f32;
        let core = Vec3::new(r_major * u.cos(), r_major * u.sin(), 0.0);
        for j in 0..minor {
            let (a, b, c, d) = (p(i, j), p(i + 1, j), p(i + 1, j + 1), p(i, j + 1));
            tri(&mut m, a, b, c, core);
            tri(&mut m, a, c, d, core);
        }
    }
    m
}

fn arch(segs: usize) -> MeshData {
    let mut m = MeshData::default();
    let (inner, outer) = (0.3, 0.5);
    // Posts.
    cuboid(
        &mut m,
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(-0.3, 0.0, 0.5),
    );
    cuboid(&mut m, Vec3::new(0.3, -0.5, -0.5), Vec3::new(0.5, 0.0, 0.5));
    // Half-ring lintel, one extruded block per segment.
    let at = |r: f32, a: f32| Vec3::new(r * a.cos(), r * a.sin(), 0.0);
    for k in 0..segs {
        let a0 = PI * k as f32 / segs as f32;
        let a1 = PI * (k + 1) as f32 / segs as f32;
        extrude_quad(
            &mut m,
            [at(inner, a0), at(outer, a0), at(outer, a1), at(inner, a1)],
        );
    }
    m
}

/// A squashed low-poly blob centred at `c` with radii `r`.
fn blob(m: &mut MeshData, c: Vec3, r: Vec3) {
    let (rings, segs) = (4, 8);
    let p = |i: usize, j: usize| {
        let th = PI * i as f32 / rings as f32;
        let ph = TAU * j as f32 / segs as f32;
        c + Vec3::new(th.sin() * ph.cos(), th.cos(), th.sin() * ph.sin()) * r
    };
    for i in 0..rings {
        for j in 0..segs {
            let (a, b, cc, d) = (p(i, j), p(i + 1, j), p(i + 1, j + 1), p(i, j + 1));
            if i + 1 != rings {
                tri(m, a, b, cc, c);
            }
            if i != 0 {
                tri(m, a, cc, d, c);
            }
        }
    }
}

fn cloud() -> MeshData {
    let mut m = MeshData::default();
    for (c, r) in [
        ([0.0, -0.1, 0.0], [0.42, 0.38, 0.4]),
        ([-0.28, -0.18, 0.05], [0.22, 0.22, 0.22]),
        ([0.28, -0.16, -0.05], [0.22, 0.24, 0.22]),
        ([0.06, 0.18, 0.0], [0.3, 0.32, 0.3]),
        ([0.0, -0.22, 0.28], [0.24, 0.2, 0.22]),
        ([-0.05, -0.2, -0.28], [0.24, 0.2, 0.22]),
    ] {
        blob(&mut m, Vec3::from(c), Vec3::from(r));
    }
    m
}

fn crescent(segs: usize) -> MeshData {
    let mut m = MeshData::default();
    // Outer circle r 0.5 at the origin, bitten by a circle r 0.42 at x = 0.22.
    // They cross at (0.2727, ±0.419): the horns.
    let (bite_x, bite_r) = (0.22_f32, 0.42_f32);
    let hx = (0.25 - bite_r * bite_r + bite_x * bite_x) / (2.0 * bite_x);
    let hy = (0.25 - hx * hx).sqrt();
    let t0 = hy.atan2(hx);
    let s0 = hy.atan2(hx - bite_x);
    for k in 0..segs {
        let f = |k: usize| k as f32 / segs as f32;
        let outer = |t: f32| {
            let a = t0 + t * (TAU - 2.0 * t0);
            Vec3::new(0.5 * a.cos(), 0.5 * a.sin(), 0.0)
        };
        let inner = |t: f32| {
            let a = s0 + t * (TAU - 2.0 * s0);
            Vec3::new(bite_x + bite_r * a.cos(), bite_r * a.sin(), 0.0)
        };
        let (a, b) = (f(k), f(k + 1));
        extrude_quad(&mut m, [outer(a), outer(b), inner(b), inner(a)]);
    }
    m
}

fn island() -> MeshData {
    let mut m = MeshData::default();
    let segs = 8;
    // Alternating radii make the rim rocky; even spokes reach the full 0.5.
    let rim = |j: usize, y: f32, scale: f32| {
        let a = TAU * j as f32 / segs as f32;
        let r = if j.is_multiple_of(2) { 0.5 } else { 0.42 } * scale;
        Vec3::new(r * a.cos(), y, r * a.sin())
    };
    let (top, tip) = (Vec3::Y * 0.5, Vec3::Y * -0.5);
    for j in 0..segs {
        let (t0, t1) = (rim(j, 0.25, 1.0), rim(j + 1, 0.25, 1.0));
        let (m0, m1) = (rim(j, -0.05, 0.62), rim(j + 1, -0.05, 0.62));
        tri(&mut m, top, t0, t1, Vec3::ZERO);
        tri(&mut m, t0, m0, m1, Vec3::ZERO);
        tri(&mut m, t0, m1, t1, Vec3::ZERO);
        tri(&mut m, m0, tip, m1, Vec3::ZERO);
    }
    m
}

fn stairs() -> MeshData {
    let mut m = MeshData::default();
    for k in 0..3 {
        let z0 = -0.5 + k as f32 / 3.0;
        let top = -0.5 + (k + 1) as f32 / 3.0;
        cuboid(
            &mut m,
            Vec3::new(-0.5, -0.5, z0),
            Vec3::new(0.5, top, z0 + 1.0 / 3.0),
        );
    }
    m
}

impl Shape {
    /// Every face of these points away from the origin (the others are
    /// unions of convex pieces, checked piece by piece via signed volume).
    #[cfg(test)]
    fn star_shaped(self) -> bool {
        matches!(
            self,
            Shape::Cube
                | Shape::Octahedron
                | Shape::Orb
                | Shape::Cylinder
                | Shape::Cone
                | Shape::Island
        )
    }
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
                    !s.star_shaped() || stored.dot((a + b + c) / 3.0) > 0.0,
                    "{s:?} tri {k}: faces inward"
                );
            }
        }
    }

    /// Outward-facing closed pieces enclose positive volume (divergence
    /// theorem); a piece wound inside-out would subtract.
    #[test]
    fn every_shape_encloses_positive_volume() {
        for s in ALL_SHAPES {
            let vol: f32 = tris(&build(s))
                .iter()
                .map(|[a, b, c]| a.dot(b.cross(*c)) / 6.0)
                .sum();
            assert!(vol > 0.05, "{s:?} volume {vol}");
        }
    }
}
