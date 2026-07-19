use std::collections::HashMap;
use std::path::{Path, PathBuf};

use glam::{Quat, Vec3};
use hecs::Entity;
use serde::{Deserialize, Serialize};

use crate::ecs::Transform;
use crate::level::MeshSource;

// --- Serializable asset data (its own `rigs/*.ron` files, same pattern as
// `engine::profile`'s ShaderProfile) ---

/// One rigid segment of a segmented ("PS2-style") character rig: a mesh
/// parented to another part via a fixed local offset — no per-vertex skin
/// deformation, joints just don't bend smoothly. `parent` is by name (not
/// index) so parts can be authored/reordered in any order in the file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RigPartDef {
    pub name: String,
    pub parent: Option<String>,
    pub mesh: MeshSource,
    pub texture_path: Option<PathBuf>,
    pub local_position: [f32; 3],
    pub local_rotation_euler_deg: [f32; 3],
    pub scale: [f32; 3],
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Keyframe {
    pub time: f32,
    pub rotation_euler_deg: [f32; 3],
}

/// A named part's rotation over time within one `RigClip`. Keyframes should
/// be kept sorted by `time` — `sample_track` assumes it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JointTrack {
    pub joint_name: String,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RigClip {
    pub name: String,
    pub duration: f32,
    pub looping: bool,
    pub tracks: Vec<JointTrack>,
}

/// A reusable character definition: its part hierarchy plus every clip
/// authored for it. Placed in a level via `engine::level::RigInstance`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RigAsset {
    pub name: String,
    pub parts: Vec<RigPartDef>,
    pub clips: Vec<RigClip>,
}

pub fn load_from_file(path: &Path) -> anyhow::Result<RigAsset> {
    let text = std::fs::read_to_string(path)?;
    Ok(ron::from_str(&text)?)
}

pub fn save_to_file(rig: &RigAsset, path: &Path) -> anyhow::Result<()> {
    let text = ron::ser::to_string_pretty(rig, ron::ser::PrettyConfig::default())?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Loads every `*.ron` file in `dir`, sorted by filename. Files that fail to
/// parse are logged and skipped rather than failing the whole scan.
pub fn load_dir(dir: &Path) -> anyhow::Result<Vec<RigAsset>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ron"))
        .collect();
    paths.sort();

    let mut rigs = Vec::with_capacity(paths.len());
    for path in paths {
        match load_from_file(&path) {
            Ok(rig) => rigs.push(rig),
            Err(err) => log::warn!("skipping rig {path:?}: {err}"),
        }
    }
    Ok(rigs)
}

// --- Runtime ECS components ---

/// A rigid segment parented to another entity via a fixed local offset.
/// `local_position`/`local_rotation_euler_deg`/`local_rotation` are the
/// authoritative *local* pose (set at spawn, hand-posed via the GUI, or
/// driven by clip playback) — `Transform` is treated as pure output for a
/// rigged entity, overwritten every frame by `update_world_transforms` with
/// the resolved *world* pose. This mirrors how `Animator` keeps
/// `base_position`/`base_rotation` separate from the live `Transform`.
pub struct RigPart {
    pub parent: Option<Entity>,
    pub local_position: Vec3,
    /// Authoritative for GUI editing — mirrors `LevelObjectMeta`'s
    /// `rotation_euler_deg` convention, so repeated edits don't drift the
    /// way a naive Euler-to-quaternion round-trip would.
    pub local_rotation_euler_deg: Vec3,
    pub local_rotation: Quat,
}

impl RigPart {
    pub fn set_local_rotation_euler_deg(&mut self, euler_deg: Vec3) {
        self.local_rotation_euler_deg = euler_deg;
        self.local_rotation = crate::ecs::euler_deg_to_quat(euler_deg);
    }
}

/// A rig part's mesh/texture source, kept alongside `RigPart` so a live rig
/// can be serialized back into a `RigAsset` (see the sandbox's
/// `build_rig_asset_from_ecs`) without a `LevelObjectMeta` — a rig part
/// isn't a level object and deliberately doesn't show up in the flat scene
/// outliner.
pub struct RigPartMeta {
    pub mesh_source: MeshSource,
    pub texture_path: Option<PathBuf>,
}

/// Lives on a rig instance's root entity: every part's name, for
/// `step_rig_animation` to resolve a clip track's `joint_name` to an
/// `Entity`.
pub struct Rig {
    pub parts_by_name: HashMap<String, Entity>,
}

/// Lives on a rig instance's root entity. `clips` is cloned in from the
/// `RigAsset` at spawn time — clips don't change at runtime except through
/// the F2 panel, which edits this copy directly and (on "Save Rig") writes
/// it back out to the asset file.
pub struct RigAnimator {
    pub clips: Vec<RigClip>,
    pub current_clip: Option<usize>,
    pub time: f32,
    pub playing: bool,
    pub speed: f32,
    /// The clip `current_clip` just replaced, frozen at the moment of the
    /// switch (see `play_clip`) — `step_rig_animation` blends from its pose
    /// at `previous_time` toward the new clip over `blend_duration` seconds,
    /// then clears this so switching clips never pops.
    pub previous_clip: Option<usize>,
    pub previous_time: f32,
    pub blend_elapsed: f32,
    pub blend_duration: f32,
}

impl Default for RigAnimator {
    fn default() -> Self {
        Self {
            clips: Vec::new(),
            current_clip: None,
            time: 0.0,
            playing: true,
            speed: 1.0,
            previous_clip: None,
            previous_time: 0.0,
            blend_elapsed: 0.0,
            blend_duration: 0.2,
        }
    }
}

impl RigAnimator {
    /// Switches to clip `index`, blending smoothly from whatever was
    /// playing instead of cutting instantly — snapshots the current clip
    /// and time as `previous_clip`/`previous_time` so `step_rig_animation`
    /// can interpolate between old and new. A no-op if `index` is already
    /// playing (nothing to blend from itself).
    pub fn play_clip(&mut self, index: usize) {
        if self.current_clip == Some(index) {
            return;
        }
        self.previous_clip = self.current_clip;
        self.previous_time = self.time;
        self.blend_elapsed = 0.0;
        self.current_clip = Some(index);
        self.time = 0.0;
    }
}

/// Slerps between a track's two bracketing keyframes at `time`. Returns
/// `None` for an empty track. Clamps to the first/last keyframe outside
/// their time range rather than extrapolating or wrapping — wrapping is the
/// caller's job (see `step_rig_animation`, which wraps `time` itself against
/// the clip's `duration` for looping clips before sampling).
fn sample_track(track: &JointTrack, time: f32) -> Option<Quat> {
    let keyframes = &track.keyframes;
    if keyframes.is_empty() {
        return None;
    }
    if time <= keyframes[0].time {
        return Some(crate::ecs::euler_deg_to_quat(Vec3::from(keyframes[0].rotation_euler_deg)));
    }
    let last = keyframes.len() - 1;
    if time >= keyframes[last].time {
        return Some(crate::ecs::euler_deg_to_quat(Vec3::from(keyframes[last].rotation_euler_deg)));
    }
    for window in keyframes.windows(2) {
        let [a, b] = window else { unreachable!() };
        if time >= a.time && time <= b.time {
            let span = (b.time - a.time).max(1e-6);
            let t = (time - a.time) / span;
            let rotation_a = crate::ecs::euler_deg_to_quat(Vec3::from(a.rotation_euler_deg));
            let rotation_b = crate::ecs::euler_deg_to_quat(Vec3::from(b.rotation_euler_deg));
            return Some(rotation_a.slerp(rotation_b, t));
        }
    }
    None
}

/// Advances every playing `RigAnimator`'s clock and samples its current
/// clip's tracks into the named parts' `RigPart::local_rotation`. Gated on
/// `playing` specifically: pausing lets the F2 panel hand-pose a part
/// without the sampler immediately overwriting it — that hand-pose *is* the
/// keyframe-authoring workflow (see `RigAnimator`/F2 "Set Keyframe Here").
pub fn step_rig_animation(world: &mut hecs::World, dt: f32) {
    let mut updates: Vec<(Entity, Vec3)> = Vec::new();

    for (_entity, (animator, rig)) in world.query::<(&mut RigAnimator, &Rig)>().iter() {
        if !animator.playing {
            continue;
        }
        let Some(clip_index) = animator.current_clip else { continue };
        let Some(clip) = animator.clips.get(clip_index) else { continue };

        animator.time += dt * animator.speed;
        if clip.duration > 0.0 {
            if clip.looping {
                animator.time = animator.time.rem_euclid(clip.duration);
            } else {
                animator.time = animator.time.clamp(0.0, clip.duration);
            }
        }

        // Blending out a just-replaced clip so switching clips doesn't pop —
        // `blend_from` is a frozen (clip index, time) snapshot of whatever
        // was playing at the moment `play_clip` was called.
        let blend_from = animator.previous_clip.map(|index| (index, animator.previous_time));
        if animator.previous_clip.is_some() {
            animator.blend_elapsed += dt;
            if animator.blend_elapsed >= animator.blend_duration {
                animator.previous_clip = None;
            }
        }
        let blend_t = if animator.blend_duration > 0.0 {
            (animator.blend_elapsed / animator.blend_duration).clamp(0.0, 1.0)
        } else {
            1.0
        };

        for track in &clip.tracks {
            let Some(&part_entity) = rig.parts_by_name.get(&track.joint_name) else { continue };
            let Some(current_rotation) = sample_track(track, animator.time) else { continue };
            let rotation = match blend_from {
                Some((previous_index, previous_time)) => {
                    let previous_rotation = animator
                        .clips
                        .get(previous_index)
                        .and_then(|prev_clip| prev_clip.tracks.iter().find(|t| t.joint_name == track.joint_name))
                        .and_then(|prev_track| sample_track(prev_track, previous_time))
                        .unwrap_or(current_rotation);
                    previous_rotation.slerp(current_rotation, blend_t)
                }
                None => current_rotation,
            };
            let (euler_x, euler_y, euler_z) = rotation.to_euler(glam::EulerRot::XYZ);
            updates.push((
                part_entity,
                Vec3::new(euler_x.to_degrees(), euler_y.to_degrees(), euler_z.to_degrees()),
            ));
        }
    }

    for (entity, euler_deg) in updates {
        if let Ok(mut part) = world.get::<&mut RigPart>(entity) {
            part.set_local_rotation_euler_deg(euler_deg);
        }
    }
}

/// Resolves every `RigPart`'s world position/rotation from its parent chain
/// and writes it into `Transform`. A character is only a handful of parts,
/// so a straightforward memoized recursive resolve is plenty fast — no need
/// for a topologically-sorted update order. Runs unconditionally every
/// frame (both Edit and Play mode), the same "live preview" philosophy as
/// `engine::animation::step`.
pub fn update_world_transforms(world: &mut hecs::World) {
    let snapshot: Vec<(Entity, Option<Entity>, Vec3, Quat)> = world
        .query::<&RigPart>()
        .iter()
        .map(|(entity, part)| (entity, part.parent, part.local_position, part.local_rotation))
        .collect();

    let mut resolved: HashMap<Entity, (Vec3, Quat)> = HashMap::with_capacity(snapshot.len());

    fn resolve(
        entity: Entity,
        snapshot: &[(Entity, Option<Entity>, Vec3, Quat)],
        resolved: &mut HashMap<Entity, (Vec3, Quat)>,
    ) -> (Vec3, Quat) {
        if let Some(&value) = resolved.get(&entity) {
            return value;
        }
        let Some(&(_, parent, local_position, local_rotation)) =
            snapshot.iter().find(|(e, ..)| *e == entity)
        else {
            return (Vec3::ZERO, Quat::IDENTITY);
        };
        let value = match parent {
            None => (local_position, local_rotation),
            Some(parent_entity) => {
                let (parent_position, parent_rotation) = resolve(parent_entity, snapshot, resolved);
                (
                    parent_position + parent_rotation * local_position,
                    parent_rotation * local_rotation,
                )
            }
        };
        resolved.insert(entity, value);
        value
    }

    for &(entity, ..) in &snapshot {
        let (world_position, world_rotation) = resolve(entity, &snapshot, &mut resolved);
        if let Ok(mut transform) = world.get::<&mut Transform>(entity) {
            transform.position = world_position;
            transform.rotation = world_rotation;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(keyframes: Vec<(f32, [f32; 3])>) -> JointTrack {
        JointTrack {
            joint_name: "Arm".to_string(),
            keyframes: keyframes
                .into_iter()
                .map(|(time, rotation_euler_deg)| Keyframe { time, rotation_euler_deg })
                .collect(),
        }
    }

    #[test]
    fn sample_track_clamps_before_first_and_after_last() {
        let t = track(vec![(1.0, [0.0, 0.0, 0.0]), (2.0, [0.0, 0.0, 90.0])]);
        let before = sample_track(&t, 0.0).unwrap();
        let after = sample_track(&t, 5.0).unwrap();
        assert!(before.abs_diff_eq(Quat::IDENTITY, 1e-4));
        assert!(after.abs_diff_eq(Quat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, 90f32.to_radians()), 1e-4));
    }

    #[test]
    fn sample_track_interpolates_midpoint() {
        let t = track(vec![(0.0, [0.0, 0.0, 0.0]), (2.0, [0.0, 0.0, 90.0])]);
        let midpoint = sample_track(&t, 1.0).unwrap();
        let expected = Quat::IDENTITY.slerp(
            Quat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, 90f32.to_radians()),
            0.5,
        );
        assert!(midpoint.abs_diff_eq(expected, 1e-4));
    }

    #[test]
    fn sample_track_empty_returns_none() {
        let t = track(vec![]);
        assert!(sample_track(&t, 0.0).is_none());
    }

    #[test]
    fn update_world_transforms_offsets_child_by_parent_rotation() {
        let mut world = hecs::World::new();
        let root = world.spawn((
            Transform::default(),
            RigPart {
                parent: None,
                local_position: Vec3::ZERO,
                local_rotation_euler_deg: Vec3::new(0.0, 90.0, 0.0),
                local_rotation: crate::ecs::euler_deg_to_quat(Vec3::new(0.0, 90.0, 0.0)),
            },
        ));
        let child = world.spawn((
            Transform::default(),
            RigPart {
                parent: Some(root),
                local_position: Vec3::new(1.0, 0.0, 0.0),
                local_rotation_euler_deg: Vec3::ZERO,
                local_rotation: Quat::IDENTITY,
            },
        ));

        update_world_transforms(&mut world);

        // The root spins the local +X offset onto roughly -Z (a 90-degree yaw).
        let child_transform = world.get::<&Transform>(child).unwrap();
        assert!(child_transform.position.abs_diff_eq(Vec3::new(0.0, 0.0, -1.0), 1e-3));
    }

    #[test]
    fn step_rig_animation_loops_and_respects_playing_flag() {
        let mut world = hecs::World::new();
        let arm = world.spawn((
            Transform::default(),
            RigPart {
                parent: None,
                local_position: Vec3::ZERO,
                local_rotation_euler_deg: Vec3::ZERO,
                local_rotation: Quat::IDENTITY,
            },
        ));
        let mut parts_by_name = HashMap::new();
        parts_by_name.insert("Arm".to_string(), arm);
        let clip = RigClip {
            name: "Wave".to_string(),
            duration: 1.0,
            looping: true,
            tracks: vec![track(vec![(0.0, [0.0, 0.0, 0.0]), (1.0, [0.0, 0.0, 90.0])])],
        };
        let root = world.spawn((
            Rig { parts_by_name },
            RigAnimator { clips: vec![clip], current_clip: Some(0), time: 0.9, playing: true, ..Default::default() },
        ));

        // Advancing past the clip's duration should wrap, not clamp, since looping = true.
        step_rig_animation(&mut world, 0.2);
        let time_after = world.get::<&RigAnimator>(root).unwrap().time;
        assert!(time_after < 1.0, "expected wrapped time, got {time_after}");

        // Pausing should freeze the sampled pose exactly where it was.
        world.get::<&mut RigAnimator>(root).unwrap().playing = false;
        let euler_before = world.get::<&RigPart>(arm).unwrap().local_rotation_euler_deg;
        step_rig_animation(&mut world, 0.5);
        let euler_after = world.get::<&RigPart>(arm).unwrap().local_rotation_euler_deg;
        assert_eq!(euler_before, euler_after);
    }

    #[test]
    fn play_clip_blends_smoothly_instead_of_popping() {
        let mut world = hecs::World::new();
        let arm = world.spawn((
            Transform::default(),
            RigPart {
                parent: None,
                local_position: Vec3::ZERO,
                local_rotation_euler_deg: Vec3::ZERO,
                local_rotation: Quat::IDENTITY,
            },
        ));
        let mut parts_by_name = HashMap::new();
        parts_by_name.insert("Arm".to_string(), arm);
        let clip_a = RigClip {
            name: "A".to_string(),
            duration: 1.0,
            looping: true,
            tracks: vec![track(vec![(0.0, [0.0, 0.0, 0.0])])],
        };
        let clip_b = RigClip {
            name: "B".to_string(),
            duration: 1.0,
            looping: true,
            tracks: vec![track(vec![(0.0, [0.0, 0.0, 90.0])])],
        };
        let root = world.spawn((
            Rig { parts_by_name },
            RigAnimator { clips: vec![clip_a, clip_b], current_clip: Some(0), playing: true, ..Default::default() },
        ));

        world.get::<&mut RigAnimator>(root).unwrap().play_clip(1);

        // Right at the switch, the blend hasn't progressed yet — the pose
        // should still read as the old clip's, not pop to the new one.
        step_rig_animation(&mut world, 0.0);
        let just_after = world.get::<&RigPart>(arm).unwrap().local_rotation;
        assert!(just_after.abs_diff_eq(Quat::IDENTITY, 1e-3), "expected still-old pose right at the switch, got {just_after:?}");

        // Halfway through the default 0.2s blend window, the pose should sit
        // roughly midway between old and new.
        step_rig_animation(&mut world, 0.1);
        let midpoint = world.get::<&RigPart>(arm).unwrap().local_rotation;
        let expected_mid =
            Quat::IDENTITY.slerp(Quat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, 90f32.to_radians()), 0.5);
        assert!(midpoint.abs_diff_eq(expected_mid, 1e-3), "expected blend midpoint, got {midpoint:?}");

        // Once the blend window has fully elapsed, the pose matches the new
        // clip exactly and the animator stops tracking a previous clip.
        step_rig_animation(&mut world, 0.2);
        let after_blend = world.get::<&RigPart>(arm).unwrap().local_rotation;
        let expected_new = Quat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, 90f32.to_radians());
        assert!(after_blend.abs_diff_eq(expected_new, 1e-3), "expected fully-new pose after blend, got {after_blend:?}");
        assert!(world.get::<&RigAnimator>(root).unwrap().previous_clip.is_none());
    }

    #[test]
    fn play_clip_is_a_no_op_for_the_already_playing_clip() {
        let mut animator = RigAnimator { current_clip: Some(0), time: 0.42, ..Default::default() };
        animator.play_clip(0);
        assert_eq!(animator.time, 0.42, "switching to the already-playing clip shouldn't reset time");
        assert!(animator.previous_clip.is_none());
    }
}
