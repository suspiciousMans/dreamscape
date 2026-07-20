use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ai::{Disposition, DialogueNode};
use crate::camera::CameraShakeSpec;
use crate::particles::ParticleEmitterDef;
use crate::physics::PhysicsParams;
use crate::screen_effect::ScreenEffectSpec;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveKind {
    Cube,
    Plane,
}

/// Where an object's geometry comes from: a built-in primitive (shared,
/// generated GPU mesh), an imported OBJ file, or an imported glTF/GLB
/// file — either its first mesh-carrying node (`GltfFile`, for a plain
/// level object) or one specific named node (`GltfNode`, for a rig part —
/// see `engine::mesh::load_gltf` and the F2 panel's "Import glTF as
/// Rig..."). Only the node's first triangle primitive is used either way.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshSource {
    Primitive(PrimitiveKind),
    ObjFile(PathBuf),
    GltfFile(PathBuf),
    GltfNode { path: PathBuf, node: String },
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

/// A trigger's "on enter, load a different level" behavior — the
/// connective piece between `Level`s and trigger volumes. `target_level`
/// is matched against `Level::name`, the same way the F2 panel's "Load:"
/// list picks a level; `spawn_position`/`spawn_yaw_deg` place the player
/// in the new level the way `enter_play_mode`'s own initial spawn does.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LevelTransition {
    pub target_level: String,
    pub spawn_position: [f32; 3],
    #[serde(default)]
    pub spawn_yaw_deg: f32,
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
    /// A `.pss` script to compile and attach as a `Behavior` when spawned,
    /// if any. `#[serde(default)]` so levels saved before this field existed
    /// still load (as `None`/no behavior).
    #[serde(default)]
    pub script: Option<PathBuf>,
    /// An `engine::class::ObjectClass` this object was spawned from, if
    /// any. When set, `mesh`/`texture_path`/`scale`/`is_dynamic`/
    /// `is_trigger`/`animation`/`script` above are resolved **from the
    /// class** at spawn time instead — this object's own copies of those
    /// fields become a fallback only, kept in sync at save time so the
    /// level still loads sensibly if the class is later deleted.
    /// `#[serde(default)]` so levels saved before this field existed still
    /// load (as `None`/unclassed).
    #[serde(default)]
    pub class: Option<PathBuf>,
    /// Only meaningful when `is_trigger` is true: loads a different level
    /// and repositions the player when the player enters this trigger —
    /// see `Sandbox::on_trigger_entered`/`transition_to_level`.
    /// `#[serde(default)]` so levels saved before this field existed still
    /// load (as `None`/no transition).
    #[serde(default)]
    pub level_transition: Option<LevelTransition>,
    /// Only meaningful when `is_trigger` is true: plays a full-screen color
    /// flash/tint when the player enters this trigger — see
    /// `Sandbox::on_trigger_entered`/`engine::screen_effect::ScreenEffectState`.
    /// Independent of `level_transition`: a trigger can have either, both, or
    /// neither. `#[serde(default)]` so levels saved before this field existed
    /// still load (as `None`/no effect).
    #[serde(default)]
    pub screen_effect: Option<ScreenEffectSpec>,
    /// Only meaningful when `is_trigger` is true: jostles the camera when
    /// the player enters this trigger — see `Sandbox::on_trigger_entered`/
    /// `engine::camera::CameraShakeState`. Independent of `screen_effect`/
    /// `level_transition`. `#[serde(default)]` so levels saved before this
    /// field existed still load (as `None`/no shake).
    #[serde(default)]
    pub camera_shake: Option<CameraShakeSpec>,
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

/// A placed instance of a `engine::rig::RigAsset` — a separate list from
/// `objects` because a rig is structurally a multi-entity hierarchy, not a
/// single mesh. No per-instance scale: scale is authored per-part inside
/// the rig asset itself, since scaling only the root's own mesh (not the
/// whole hierarchy's offsets) would be more confusing than useful.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RigInstance {
    pub name: String,
    pub rig_path: PathBuf,
    pub position: [f32; 3],
    pub rotation_euler_deg: [f32; 3],
    pub playing_clip: Option<String>,
}

/// A placed particle emitter — position + `ParticleEmitterDef`. A sibling
/// list on `Level`, not folded into `LevelObject`, since an emitter has no
/// mesh/texture/scale/physics of its own.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LevelParticleEmitter {
    pub name: String,
    pub position: [f32; 3],
    pub def: ParticleEmitterDef,
}

/// A placed character (enemy/NPC/passive) — a sibling list on `Level` like
/// `RigInstance`/`LevelLight`, since a character's shape (disposition,
/// combat/dialogue config) doesn't fit `LevelObject`'s generic mesh/
/// texture/trigger schema. Spawned as a cube (see `Sandbox::spawn_character`)
/// rather than an imported mesh/rig — solid-`color`-filled by default, or
/// textured if `texture_path` is set.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterInstance {
    pub name: String,
    pub position: [f32; 3],
    pub scale: [f32; 3],
    pub color: [f32; 3],
    /// `#[serde(default)]` so levels saved before this field existed still
    /// load (as `None`/solid-color, the prior-only behavior).
    #[serde(default)]
    pub texture_path: Option<PathBuf>,
    pub disposition: Disposition,
    pub move_speed: f32,
    pub wander_radius: f32,
    pub sight_range: f32,
    #[serde(default)]
    pub max_health: Option<f32>,
    #[serde(default)]
    pub damage: Option<f32>,
    #[serde(default)]
    pub attack_range: f32,
    #[serde(default)]
    pub attack_cooldown_secs: f32,
    #[serde(default)]
    pub dialogue_nodes: Vec<DialogueNode>,
}

/// A placed periodic character spawner — a sibling list on `Level` like
/// `LevelLight`/`RigInstance`, since a spawner has no mesh/texture of its
/// own, just a position and a spawn config. See `engine::ai::SpawnerConfig`
/// for what each field does; `Sandbox::spawn_spawner` builds that (plus
/// runtime `SpawnerState`) from this on level load.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpawnerInstance {
    pub name: String,
    pub position: [f32; 3],
    pub template: CharacterInstance,
    pub spawn_interval_secs: f32,
    pub max_alive: u32,
    #[serde(default)]
    pub total_to_spawn: Option<u32>,
    pub spawn_radius: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Level {
    pub name: String,
    pub objects: Vec<LevelObject>,
    #[serde(default)]
    pub lights: Vec<LevelLight>,
    #[serde(default)]
    pub particle_emitters: Vec<LevelParticleEmitter>,
    #[serde(default)]
    pub rig_instances: Vec<RigInstance>,
    #[serde(default)]
    pub characters: Vec<CharacterInstance>,
    #[serde(default)]
    pub spawners: Vec<SpawnerInstance>,
    #[serde(default)]
    pub physics: PhysicsParams,
    /// Background music for this level, relative to the asset root — looped
    /// automatically whenever the level is applied. `None` means silence
    /// (and stops whatever the previously-loaded level was playing).
    #[serde(default)]
    pub music_path: Option<PathBuf>,
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
