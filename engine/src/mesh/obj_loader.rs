use std::path::Path;

use super::{MeshData, Vertex};

/// Loads every shape in an OBJ file as a separate `MeshData`, sharing one
/// vertex-per-index layout (position/normal/uv). Missing normals default to
/// +Y and missing texcoords default to (0,0) rather than failing.
pub fn load_obj(path: &Path) -> anyhow::Result<Vec<MeshData>> {
    let (models, _materials) = tobj::load_obj(
        path,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
    )?;

    let mut out = Vec::with_capacity(models.len());
    for model in models {
        let mesh = model.mesh;
        let vertex_count = mesh.positions.len() / 3;
        let has_normals = mesh.normals.len() == vertex_count * 3;
        let has_uvs = mesh.texcoords.len() == vertex_count * 2;

        let vertices = (0..vertex_count)
            .map(|i| Vertex {
                position: [
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                ],
                normal: if has_normals {
                    [
                        mesh.normals[i * 3],
                        mesh.normals[i * 3 + 1],
                        mesh.normals[i * 3 + 2],
                    ]
                } else {
                    [0.0, 1.0, 0.0]
                },
                uv: if has_uvs {
                    [mesh.texcoords[i * 2], mesh.texcoords[i * 2 + 1]]
                } else {
                    [0.0, 0.0]
                },
            })
            .collect();

        out.push(MeshData {
            vertices,
            indices: mesh.indices,
        });
    }

    Ok(out)
}
