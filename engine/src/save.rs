use std::path::Path;

use serde::{Deserialize, Serialize};

/// A single gameplay checkpoint — enough to drop the player back where they
/// were, not a full world snapshot. Per-object/script custom state
/// (inventory, world flags, ...) is deliberately out of scope for this pass;
/// add it once a concrete game actually needs it rather than guessing at the
/// shape now.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveData {
    pub level_name: String,
    pub player_position: [f32; 3],
    pub player_yaw: f32,
    pub player_pitch: f32,
    pub saved_at_elapsed: f32,
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
