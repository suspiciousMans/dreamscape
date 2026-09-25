use std::collections::HashMap;
use std::path::Path;

use glam::{EulerRot, Quat};

use super::{MeshData, Vertex};

/// Decoded RGBA8 pixel data for a glTF primitive's base-color texture —
/// `gltf::import` already decodes embedded (base64) and external image
/// references into raw pixels, so unlike OBJ import there's no separate
/// image-loading step; this just normalizes whatever pixel format the
/// source used into RGBA8 for `GpuTexture::from_rgba8`.
pub struct GltfImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// One glTF node that carries a mesh — the unit both import paths (single
/// object, and rig) build from. `parent_name` mirrors
/// `engine::rig::RigPartDef::parent`: `None` for a node with no parent
/// (or whose parent has no mesh-relevant name), `Some(name)` otherwise —
/// glTF's node hierarchy maps directly onto the rig system's named-parent
/// rigid-part hierarchy.
pub struct GltfMeshEntry {
    pub name: String,
    pub parent_name: Option<String>,
    pub local_position: [f32; 3],
    pub local_rotation_euler_deg: [f32; 3],
    pub scale: [f32; 3],
    pub mesh: MeshData,
    pub image: Option<GltfImage>,
}

pub struct GltfScene {
    pub meshes: Vec<GltfMeshEntry>,
}

/// Loads every mesh-carrying node of a glTF/GLB file into a flat list.
///
/// Deliberately out of scope (static pose only, logged where relevant):
/// **vertex skinning** — the engine has no per-vertex bone-weight
/// attribute or joint-matrix shader support, only rigid-part hierarchies
/// (see `engine::rig`) — and **animation-channel import** — `RigClip`'s
/// `Keyframe` is rotation-only, glTF's TRS animation channels are richer.
/// A skinned/animated source file still imports fine, just at its bind
/// pose with no animation data carried over.
pub fn load_gltf(path: &Path) -> anyhow::Result<GltfScene> {
    let (document, buffers, images) = gltf::import(path)?;
    let mut meshes = Vec::new();

    // Precompute `child node index -> parent's imported name` in a single
    // pass. Only mesh-carrying parents are recorded — every `RigPartDef` in
    // the output is itself mesh-carrying, so a reference to a non-mesh
    // "group" node would dangle; a node parented under such a group
    // therefore imports as a root part at its own *local* transform (a
    // stated limitation, fine for the common "every part is its own mesh
    // node" export). The parent's name uses the SAME synthesized fallback
    // as `node_name` below, so a child of an *unnamed* mesh parent still
    // links to the exact name that parent was imported under instead of
    // losing the link. This also replaces a former O(n^2) per-node scan
    // with an O(n) build + O(1) lookup.
    let mut parent_name_of: HashMap<usize, String> = HashMap::new();
    for node in document.nodes() {
        if node.mesh().is_none() {
            continue;
        }
        let name = node
            .name()
            .map(str::to_string)
            .unwrap_or_else(|| format!("node_{}", node.index()));
        for child in node.children() {
            parent_name_of.insert(child.index(), name.clone());
        }
    }

    for node in document.nodes() {
        let Some(mesh) = node.mesh() else { continue };
        let node_name = node
            .name()
            .map(str::to_string)
            .unwrap_or_else(|| format!("node_{}", node.index()));
        let parent_name = parent_name_of.get(&node.index()).cloned();

        let (translation, rotation, scale) = node.transform().decomposed();
        let (rx, ry, rz) = Quat::from_xyzw(rotation[0], rotation[1], rotation[2], rotation[3])
            .to_euler(EulerRot::XYZ);
        let local_rotation_euler_deg = [rx.to_degrees(), ry.to_degrees(), rz.to_degrees()];

        // Only the first triangle-mode primitive per node is imported —
        // mirrors `load_obj`'s "author multi-part meshes as separate
        // objects" convention; non-triangle primitives (points/lines) are
        // skipped with a warning, matching the OBJ loader's precedent of
        // silently discarding what the engine's renderer can't represent.
        let Some(primitive) = mesh
            .primitives()
            .find(|p| p.mode() == gltf::mesh::Mode::Triangles)
        else {
            log::warn!("gltf node '{node_name}' has no triangle-mode primitive; skipping");
            continue;
        };

        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
        let Some(positions) = reader.read_positions() else {
            log::warn!("gltf node '{node_name}' primitive has no positions; skipping");
            continue;
        };
        let positions: Vec<[f32; 3]> = positions.collect();
        let normals: Vec<[f32; 3]> = reader
            .read_normals()
            .map(Iterator::collect)
            .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);
        let uvs: Vec<[f32; 2]> = reader
            .read_tex_coords(0)
            .map(|iter| iter.into_f32().collect())
            .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
        let indices: Vec<u32> = reader
            .read_indices()
            .map(|iter| iter.into_u32().collect())
            .unwrap_or_else(|| (0..positions.len() as u32).collect());

        let vertices: Vec<Vertex> = (0..positions.len())
            .map(|i| Vertex {
                position: positions[i],
                normal: *normals.get(i).unwrap_or(&[0.0, 1.0, 0.0]),
                uv: *uvs.get(i).unwrap_or(&[0.0, 0.0]),
                color: [0.0, 0.0, 0.0],
            })
            .collect();

        let image = primitive
            .material()
            .pbr_metallic_roughness()
            .base_color_texture()
            .and_then(|info| {
                let source = info.texture().source();
                images.get(source.index()).and_then(|data| {
                    to_rgba8(&data.pixels, data.format).map(|rgba| GltfImage {
                        rgba,
                        width: data.width,
                        height: data.height,
                    })
                })
            });

        meshes.push(GltfMeshEntry {
            name: node_name,
            parent_name,
            local_position: translation,
            local_rotation_euler_deg,
            scale,
            mesh: MeshData { vertices, indices },
            image,
        });
    }

    Ok(GltfScene { meshes })
}

/// Normalizes a decoded glTF image's pixels to RGBA8. `None` for the
/// 16-bit/float formats glTF allows but which a typical color texture
/// never actually uses — rather than guess at a lossy downcast, the
/// caller just skips the texture (logged) and the mesh imports untextured.
fn to_rgba8(pixels: &[u8], format: gltf::image::Format) -> Option<Vec<u8>> {
    use gltf::image::Format;
    match format {
        Format::R8G8B8A8 => Some(pixels.to_vec()),
        Format::R8G8B8 => Some(
            pixels
                .chunks_exact(3)
                .flat_map(|p| [p[0], p[1], p[2], 255])
                .collect(),
        ),
        Format::R8 => Some(pixels.iter().flat_map(|&p| [p, p, p, 255]).collect()),
        Format::R8G8 => Some(
            pixels
                .chunks_exact(2)
                .flat_map(|p| [p[0], p[1], 0, 255])
                .collect(),
        ),
        _ => None,
    }
}
