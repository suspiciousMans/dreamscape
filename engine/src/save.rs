use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// A single gameplay checkpoint — enough to drop the player back where they
/// were, plus a small amount of world-state delta (health, which named
/// entities were removed, and a game-defined scalar bag) so a checkpoint
/// doesn't feel like a full level reset. A full per-object/script state
/// snapshot (inventory, arbitrary world flags, ...) is still out of scope
/// for this pass; add it once a concrete game actually needs it rather than
/// guessing at the shape now.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveData {
    pub level_name: String,
    pub player_position: [f32; 3],
    pub player_yaw: f32,
    pub player_pitch: f32,
    pub saved_at_elapsed: f32,
    /// (current, max) health, if the player had a `Health` component when
    /// saved. `#[serde(default)]` so saves from before this field existed
    /// still load (as `None`/full health).
    #[serde(default)]
    pub player_health: Option<(f32, f32)>,
    /// Names of level entities (objects/characters) that were despawned
    /// since the level loaded — replayed by re-despawning the matching
    /// freshly-spawned entity after `apply_level` on load. `#[serde(default)]`
    /// so saves from before this field existed still load (as empty/nothing
    /// despawned).
    #[serde(default)]
    pub despawned_names: Vec<String>,
    /// A small game-defined scalar bag for whatever bit of state a specific
    /// game needs to restore that doesn't fit the fields above (e.g. which
    /// mushroom is currently active). The engine stays generic — it never
    /// reads or writes keys itself. `#[serde(default)]` so saves from before
    /// this field existed still load (as empty).
    #[serde(default)]
    pub extra: HashMap<String, f32>,
}

pub fn load_from_file(path: &Path) -> anyhow::Result<SaveData> {
    let text = std::fs::read_to_string(path)?;
    Ok(ron::from_str(&text)?)
}

pub fn save_to_file(data: &SaveData, path: &Path) -> anyhow::Result<()> {
    let text = ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())?;
    std::fs::write(path, text)?;
    Ok(())
}
