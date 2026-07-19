use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::level::{AnimationSpec, MeshSource};

/// A reusable template for everything about an object *except* its
/// placement (name/position/rotation stay per-instance on the
/// `LevelObject` that references a class). Saved as its own `classes/*.ron`
/// file, the same `load_from_file`/`save_to_file`/`load_dir` pattern as
/// `engine::profile`'s `ShaderProfile`.
///
/// Editing a class and clicking "Apply to All Instances" in the F2 panel
/// re-resolves every placed instance from these fields — deliberately an
/// explicit action rather than continuous live-binding, so the mental
/// model stays simple: instances only change when you ask them to.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectClass {
    pub name: String,
    pub mesh: MeshSource,
    pub texture_path: Option<PathBuf>,
    pub scale: [f32; 3],
    pub is_dynamic: bool,
    pub is_trigger: bool,
    pub animation: Option<AnimationSpec>,
    pub script: Option<PathBuf>,
}

pub fn load_from_file(path: &Path) -> anyhow::Result<ObjectClass> {
    let text = std::fs::read_to_string(path)?;
    Ok(ron::from_str(&text)?)
}

pub fn save_to_file(class: &ObjectClass, path: &Path) -> anyhow::Result<()> {
    let text = ron::ser::to_string_pretty(class, ron::ser::PrettyConfig::default())?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Loads every `*.ron` file in `dir`, sorted by filename. Files that fail to
/// parse are logged and skipped rather than failing the whole scan.
pub fn load_dir(dir: &Path) -> anyhow::Result<Vec<ObjectClass>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ron"))
        .collect();
    paths.sort();

    let mut classes = Vec::with_capacity(paths.len());
    for path in paths {
        match load_from_file(&path) {
            Ok(class) => classes.push(class),
            Err(err) => log::warn!("skipping object class {path:?}: {err}"),
        }
    }
    Ok(classes)
}
