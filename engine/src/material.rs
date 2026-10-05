use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterialVariant {
    #[serde(rename = "lit")]
    Lit,
    #[serde(rename = "lit_normals")]
    LitNormals,
    #[serde(rename = "unlit")]
    Unlit,
}

impl Default for MaterialVariant {
    fn default() -> Self {
        MaterialVariant::Lit
    }
}

/// Surface description for a mesh. Attach it as an optional ECS component
/// alongside `MeshRenderer` (`world.insert_one(entity, material)`);
/// entities without one render with the profile's standard shader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    #[serde(default)]
    pub base_color: Option<String>,
    #[serde(default)]
    pub normal_map: Option<String>,
    #[serde(default = "default_roughness")]
    pub roughness: f32,
    #[serde(default)]
    pub metallic: f32,
    #[serde(default)]
    pub variant: MaterialVariant,
}

fn default_roughness() -> f32 {
    0.5
}

impl Default for Material {
    fn default() -> Self {
        Material {
            base_color: None,
            normal_map: None,
            roughness: 0.5,
            metallic: 0.0,
            variant: MaterialVariant::Lit,
        }
    }
}

impl Material {
    pub fn to_ron_string(&self) -> Result<String> {
        ron::to_string(self).context("serialize material")
    }
    pub fn from_ron_string(src: &str) -> Result<Self> {
        ron::from_str(src).context("deserialize material")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialFile {
    pub name: String,
    pub material: Material,
}

impl MaterialFile {
    pub fn to_ron_string(&self) -> Result<String> {
        ron::to_string(self).context("serialize material file")
    }
    pub fn from_ron_string(src: &str) -> Result<Self> {
        ron::from_str(src).context("deserialize material file")
    }
}

/// Loads a single material file from a RON path.
pub fn load_from_file(path: &Path) -> anyhow::Result<MaterialFile> {
    let text = std::fs::read_to_string(path)?;
    Ok(ron::from_str(&text)?)
}

/// Saves a material file to a RON path (pretty-printed).
pub fn save_to_file(mat: &MaterialFile, path: &Path) -> anyhow::Result<()> {
    let text = ron::ser::to_string_pretty(mat, ron::ser::PrettyConfig::default())?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Loads every `*.ron` file in `dir`, sorted by filename for a stable
/// cycling order. Files that fail to parse are logged and skipped rather
/// than failing the whole scan.
pub fn load_dir(dir: &Path) -> anyhow::Result<Vec<MaterialFile>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ron"))
        .collect();
    paths.sort();

    let mut materials = Vec::with_capacity(paths.len());
    for path in paths {
        match load_from_file(&path) {
            Ok(mat) => materials.push(mat),
            Err(err) => log::warn!("skipping material {path:?}: {err}"),
        }
    }
    Ok(materials)
}

/// A simple cycling list for loaded materials — mirrors `ProfileCycler`
/// so the F2 panel can cycle through materials the same way it cycles
/// through shader profiles.
pub struct MaterialCycler {
    materials: Vec<MaterialFile>,
    index: usize,
}

impl MaterialCycler {
    pub fn new(materials: Vec<MaterialFile>) -> anyhow::Result<Self> {
        anyhow::ensure!(!materials.is_empty(), "at least one material is required");
        Ok(Self { materials, index: 0 })
    }

    pub fn current(&self) -> &MaterialFile {
        &self.materials[self.index]
    }

    pub fn current_material(&self) -> &Material {
        &self.materials[self.index].material
    }

    pub fn next(&mut self) -> &MaterialFile {
        self.index = (self.index + 1) % self.materials.len();
        self.current()
    }

    pub fn prev(&mut self) -> &MaterialFile {
        self.index = (self.index + self.materials.len() - 1) % self.materials.len();
        self.current()
    }

    pub fn current_index(&self) -> usize {
        self.index
    }

    pub fn all(&self) -> &[MaterialFile] {
        &self.materials
    }

    /// Appends a material and makes it the active one (e.g. after a GUI "Save As").
    pub fn add(&mut self, material: MaterialFile) {
        self.materials.push(material);
        self.index = self.materials.len() - 1;
    }

    pub fn select(&mut self, index: usize) -> &MaterialFile {
        self.index = index.min(self.materials.len() - 1);
        self.current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_load_dir_skips_non_ron() {
        let tmp = std::env::temp_dir().join("jame_test_material_dir");
        let _ = std::fs::create_dir_all(&tmp);
        std::fs::write(tmp.join("mat.ron"), "(name: \"x\", material: (roughness: 0.5))").unwrap();
        std::fs::write(tmp.join("ignore.txt"), "nope").unwrap();
        let found = load_dir(&tmp).unwrap();
        assert_eq!(found.len(), 1);
        let _ = std::fs::remove_file(tmp.join("mat.ron"));
        let _ = std::fs::remove_file(tmp.join("ignore.txt"));
        let _ = std::fs::remove_dir(&tmp);
    }

    #[test]
    fn material_cycler_requires_at_least_one() {
        let res = MaterialCycler::new(Vec::new());
        assert!(res.is_err());
    }

    #[test]
    fn material_cycler_cycles() {
        let mats = vec![
            MaterialFile {
                name: "a".into(),
                material: Material::default(),
            },
            MaterialFile {
                name: "b".into(),
                material: Material::default(),
            },
        ];
        let mut cycler = MaterialCycler::new(mats).unwrap();
        assert_eq!(cycler.current_index(), 0);
        cycler.next();
        assert_eq!(cycler.current_index(), 1);
        cycler.next();
        assert_eq!(cycler.current_index(), 0);
    }
}
