use std::path::PathBuf;
use std::sync::Arc;

use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::camera::CameraShakeSpec;
use crate::level::{LevelTransition, MeshSource};
use crate::mesh::GpuMesh;
use crate::screen_effect::ScreenEffectSpec;
use crate::texture::GpuTexture;

/// Converts GUI-edited Euler degrees to the quaternion `Transform::rotation`
/// actually stores. Kept as one shared function (rather than each caller
/// rolling its own) so every Euler-authoring path — level objects, rig
/// parts — agrees on axis order.
pub fn euler_deg_to_quat(euler_deg: Vec3) -> Quat {
    Quat::from_euler(
        EulerRot::XYZ,
        euler_deg.x.to_radians(),
        euler_deg.y.to_radians(),
        euler_deg.z.to_radians(),
    )
}

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

/// Shared handles so one loaded mesh/texture can back many entities without
/// re-uploading GPU resources per instance.
pub struct MeshRenderer {
    pub mesh: Arc<GpuMesh>,
    pub texture: Option<Arc<GpuTexture>>,
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub fov_y_radians: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov_y_radians: 45f32.to_radians(),
            near: 0.1,
            far: 500.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LightKind {
    Directional { direction: Vec3 },
    Point { range: f32 },
}

#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub color: Vec3,
    pub intensity: f32,
    pub kind: LightKind,
}

/// Marks an entity as a level-editor-placed object and carries enough
/// information to serialize it back into a `LevelObject`. `rotation_euler_deg`
/// is the authoritative editable rotation (the GUI edits this directly);
/// `Transform::rotation` is recomputed from it on every edit rather than
/// converting back and forth, which would drift the value over repeated edits.
#[derive(Clone, Debug)]
pub struct LevelObjectMeta {
    pub name: String,
    pub mesh_source: MeshSource,
    pub texture_path: Option<PathBuf>,
    pub rotation_euler_deg: Vec3,
    /// Path to the `.pss` script backing this object's `BehaviorSlot`, if
    /// any — kept here (mirroring `texture_path`) so the F2 panel and
    /// `build_level_from_ecs` can round-trip it without inspecting the
    /// `BehaviorSlot` component itself.
    pub script_path: Option<PathBuf>,
    /// Mirrors `LevelObject::level_transition` for the same reason as
    /// `script_path` — round-trippable by the F2 panel and
    /// `build_level_from_ecs` without inspecting other components.
    pub level_transition: Option<LevelTransition>,
    /// Mirrors `LevelObject::screen_effect` for the same reason as
    /// `level_transition` — round-trippable by the F2 panel and
    /// `build_level_from_ecs` without inspecting other components.
    pub screen_effect: Option<ScreenEffectSpec>,
    /// Mirrors `LevelObject::camera_shake` for the same reason as
    /// `screen_effect` — round-trippable by the F2 panel and
    /// `build_level_from_ecs` without inspecting other components.
    pub camera_shake: Option<CameraShakeSpec>,
}

/// Tags the player entity in Play mode. Plain tunable fields — this is the
/// obvious place to retune movement feel for your own game.
#[derive(Clone, Copy, Debug)]
pub struct PlayerController {
    pub move_speed: f32,
    pub jump_speed: f32,
    pub interact_radius: f32,
    pub interact_impulse: f32,
}

impl Default for PlayerController {
    fn default() -> Self {
        Self {
            move_speed: 4.0,
            jump_speed: 4.5,
            interact_radius: 2.0,
            interact_impulse: 6.0,
        }
    }
}
