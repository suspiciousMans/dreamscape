use glam::Vec3;

use super::{MeshData, Vertex};

/// Appends one quad (as two triangles) with a per-corner UV and a single
/// flat normal — matches the hand-authored cube OBJ's per-face convention.
fn face(vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>, corners: [Vec3; 4], normal: Vec3) {
    let base = vertices.len() as u32;
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for (corner, uv) in corners.into_iter().zip(uvs) {
        vertices.push(Vertex {
            position: corner.to_array(),
            normal: normal.to_array(),
            uv,
        });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

/// A unit cube (-0.5..=0.5 on each axis) — scale it via `Transform::scale`.
pub fn cube() -> MeshData {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let v1 = Vec3::new(-0.5, -0.5, 0.5);
    let v2 = Vec3::new(0.5, -0.5, 0.5);
    let v3 = Vec3::new(0.5, 0.5, 0.5);
    let v4 = Vec3::new(-0.5, 0.5, 0.5);
    let v5 = Vec3::new(-0.5, -0.5, -0.5);
    let v6 = Vec3::new(0.5, -0.5, -0.5);
    let v7 = Vec3::new(0.5, 0.5, -0.5);
    let v8 = Vec3::new(-0.5, 0.5, -0.5);

    face(&mut vertices, &mut indices, [v1, v2, v3, v4], Vec3::Z);
    face(&mut vertices, &mut indices, [v6, v5, v8, v7], Vec3::NEG_Z);
    face(&mut vertices, &mut indices, [v2, v6, v7, v3], Vec3::X);
    face(&mut vertices, &mut indices, [v5, v1, v4, v8], Vec3::NEG_X);
    face(&mut vertices, &mut indices, [v4, v3, v7, v8], Vec3::Y);
    face(&mut vertices, &mut indices, [v5, v6, v2, v1], Vec3::NEG_Y);

    MeshData { vertices, indices }
}

/// A 2x2 unit quad in the XZ plane facing +Y — a simple floor/ground piece.
pub fn plane() -> MeshData {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let p1 = Vec3::new(-1.0, 0.0, 1.0);
    let p2 = Vec3::new(1.0, 0.0, 1.0);
    let p3 = Vec3::new(1.0, 0.0, -1.0);
    let p4 = Vec3::new(-1.0, 0.0, -1.0);
    face(&mut vertices, &mut indices, [p1, p2, p3, p4], Vec3::Y);

    MeshData { vertices, indices }
}
