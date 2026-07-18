use std::path::PathBuf;
use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};

use crate::level::MeshSource;
use crate::mesh::GpuMesh;
use crate::texture::GpuTexture;

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
