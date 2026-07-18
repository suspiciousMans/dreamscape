use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::physics::PhysicsParams;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveKind {
    Cube,
    Plane,
}

/// Where an object's geometry comes from: a built-in primitive (shared,
/// generated GPU mesh) or an imported OBJ file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshSource {
    Primitive(PrimitiveKind),
    ObjFile(PathBuf),
}

/// Mirrors `engine::animation::AnimationKind`, serializable so it can be
/// saved/loaded as part of a `LevelObject`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AnimationSpec {
    Orbit {
        axis: [f32; 3],
        speed_deg_per_sec: f32,
    },
    Bob {
        axis: [f32; 3],
        amplitude: f32,
        period_secs: f32,
    },
}

/// One placed object in a level: what it looks like (mesh/texture) and its
/// transform. Rotation is stored as Euler degrees (not a quaternion) because
/// that's what the GUI edits directly — converting only at spawn/edit time
/// avoids repeated Euler↔quaternion round-trips drifting the value.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LevelObject {
    pub name: String,
    pub mesh: MeshSource,
    pub texture_path: Option<PathBuf>,
    pub position: [f32; 3],
    pub rotation_euler_deg: [f32; 3],
    pub scale: [f32; 3],
    /// Whether this object gets a `RigidBody` (falls/collides/pushable) when
    /// spawned, or is static (collided against, but never moves). `#[serde(default)]`
    /// so levels saved before this field existed still load (as `false`/static).
    #[serde(default)]
    pub is_dynamic: bool,
    /// Whether this object's `Collider` is a non-solid trigger zone (reported
    /// via `physics::step`'s overlap list, never physically collided with).
    #[serde(default)]
    pub is_trigger: bool,
    /// Procedural motion (orbit/bob), if any. `#[serde(default)]` so levels
    /// saved before this field existed still load (as `None`/unanimated).
    #[serde(default)]
    pub animation: Option<AnimationSpec>,
}

/// A placed point light: a separate shape from `LevelObject` because a light
/// has no mesh/texture/scale, just a position and a color/intensity/range.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LevelLight {
    pub name: String,
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Level {
    pub name: String,
    pub objects: Vec<LevelObject>,
    #[serde(default)]
    pub lights: Vec<LevelLight>,
    #[serde(default)]
    pub physics: PhysicsParams,
}

pub fn load_from_file(path: &Path) -> anyhow::Result<Level> {
    let text = std::fs::read_to_string(path)?;
    Ok(ron::from_str(&text)?)
}

pub fn save_to_file(level: &Level, path: &Path) -> anyhow::Result<()> {
    let text = ron::ser::to_string_pretty(level, ron::ser::PrettyConfig::default())?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Loads every `*.ron` file in `dir`, sorted by filename. Files that fail to
/// parse are logged and skipped rather than failing the whole scan.
pub fn load_dir(dir: &Path) -> anyhow::Result<Vec<Level>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ron"))
        .collect();
    paths.sort();

    let mut levels = Vec::with_capacity(paths.len());
    for path in paths {
        match load_from_file(&path) {
            Ok(level) => levels.push(level),
            Err(err) => log::warn!("skipping level {path:?}: {err}"),
        }
    }
    Ok(levels)
}

/// Stores `path` relative to `root` when possible (portable across
/// machines), falling back to the absolute path if it's outside `root` (e.g.
/// picked from somewhere like Downloads instead of the asset folder).
pub fn relativize(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}
