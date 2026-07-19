// A shipped release build has no console attached; a debug build keeps one
// so `cargo run` still shows log output and panic messages as usual.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engine::animation::{AnimationKind, Animator};
use engine::app::{App, Context, Game};
use engine::audio::AudioContext;
use engine::behavior::{Behavior, BehaviorSlot, ScriptApi, ScriptBehavior};
use engine::camera::{
    CameraShakeSpec, CameraShakeState, FirstPersonCamera, HeadBob, LandingDip, OrbitCamera, SpeedFov,
    StrafeTilt,
};
use engine::class::ObjectClass;
use engine::ecs::{euler_deg_to_quat, Entity, Light, LevelObjectMeta, LightKind, MeshRenderer, PlayerController, Transform, World};
use engine::hud::HudState;
use engine::screen_effect::{ScreenEffectSpec, ScreenEffectState};
use engine::glam::{Mat4, Quat, Vec3};
use engine::glow::{self, HasContext};
use engine::level::{AnimationSpec, Level, LevelLight, LevelObject, LevelParticleEmitter, LevelTransition, MeshSource, PrimitiveKind, RigInstance};
use engine::rig::{Keyframe, JointTrack, Rig, RigAnimator, RigAsset, RigClip, RigPart, RigPartDef};
use engine::save::SaveData;
use engine::mesh::{load_obj, primitives, GpuMesh};
use engine::particles::{ParticleEmitter, ParticleEmitterDef};
use engine::physics::{Collider, ColliderShape, PhysicsParams, RigidBody};
use engine::profile::{LightingMode, ProfileCycler, RenderParams, ShaderProfile, TextureFilterMode};
use engine::renderer::{PostParams, Renderer};
use engine::sdl2::controller::Button as ControllerButton;
use engine::sdl2::event::Event;
use engine::sdl2::keyboard::Keycode;
use engine::sdl2::mouse::MouseButton;
use engine::shader::{ShaderVariantCache, AFFINE_UV_BIT};
use engine::texture::{GpuTexture, TextureFilter};
use engine::ui::{draw_hud, draw_pause_menu, render_params_editor, EguiState, PauseMenuAction};

fn default_demo_profiles() -> Vec<ShaderProfile> {
    let make = |name: &str, render: RenderParams| ShaderProfile {
        name: name.to_string(),
        vertex_shader: PathBuf::from("assets/shaders/mesh.vert"),
        fragment_shader: PathBuf::from("assets/shaders/mesh.frag"),
        post_fragment_shader: PathBuf::from("assets/shaders/post_composite.frag"),
        render,
    };

    vec![
        make(
            "ps2_classic",
            RenderParams {
                resolution_scale: 0.35,
                fog_color: [0.08, 0.08, 0.12],
                fog_start: 4.0,
                fog_end: 16.0,
                color_levels: 24,
                dither_strength: 0.25,
                lighting_mode: LightingMode::VertexLit,
                ambient_color: [0.25, 0.25, 0.3],
                vertex_snap_amount: 0.0,
                affine_texture_mapping: false,
                texture_filter: TextureFilterMode::Nearest,
                ..RenderParams::default()
            },
        ),
        make(
            "ps1_wobble",
            RenderParams {
                resolution_scale: 0.18,
                fog_color: [0.05, 0.05, 0.08],
                fog_start: 2.5,
                fog_end: 10.0,
                color_levels: 16,
                dither_strength: 0.2,
                lighting_mode: LightingMode::VertexLit,
                ambient_color: [0.2, 0.2, 0.25],
                vertex_snap_amount: 0.06,
                affine_texture_mapping: true,
                texture_filter: TextureFilterMode::Nearest,
                ..RenderParams::default()
            },
        ),
        make(
            "clean_lowpoly",
            RenderParams {
                resolution_scale: 0.75,
                fog_color: [0.1, 0.1, 0.15],
                fog_start: 8.0,
                fog_end: 30.0,
                color_levels: 256,
                dither_strength: 0.0,
                lighting_mode: LightingMode::VertexLit,
                ambient_color: [0.3, 0.3, 0.35],
                vertex_snap_amount: 0.0,
                affine_texture_mapping: false,
                texture_filter: TextureFilterMode::Nearest,
                ..RenderParams::default()
            },
        ),
    ]
}

/// A mushroom the player can eat — each grants exactly one active ability,
/// replacing whatever was active before. See `Sandbox::eat_mushroom`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MushroomKind {
    Bounce,
    Shrink,
    Glow,
}

impl MushroomKind {
    fn all() -> [MushroomKind; 3] {
        [MushroomKind::Bounce, MushroomKind::Shrink, MushroomKind::Glow]
    }

    fn display_name(self) -> &'static str {
        match self {
            MushroomKind::Bounce => "Bounce",
            MushroomKind::Shrink => "Shrink",
            MushroomKind::Glow => "Glow",
        }
    }

    /// The `ObjectClass` filename stem this kind's pickups are authored
    /// under — used both to build the class path when spawning a pickup
    /// and, in reverse, to recognize a spawned entity as a mushroom of this
    /// kind (see `mushroom_kind_from_class`).
    fn class_name(self) -> &'static str {
        match self {
            MushroomKind::Bounce => "mushroom_bounce",
            MushroomKind::Shrink => "mushroom_shrink",
            MushroomKind::Glow => "mushroom_glow",
        }
    }

    fn color(self) -> [f32; 3] {
        match self {
            MushroomKind::Bounce => [0.9, 0.2, 0.2],
            MushroomKind::Shrink => [0.25, 0.4, 0.95],
            MushroomKind::Glow => [0.95, 0.85, 0.3],
        }
    }

    fn rgba(self) -> [u8; 4] {
        let [r, g, b] = self.color();
        [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255]
    }
}

/// Marks a spawned entity as a mushroom pickup, attached right after the
/// generic `spawn_level_object` call (see `Sandbox::apply_level`) — mirrors
/// the established "attach a native-only extra component after the generic
/// spawn" pattern, since which mushroom kind an entity is isn't part of
/// `LevelObject`'s generic schema (it's recovered from the object's class).
struct MushroomPickup(MushroomKind);

/// Recognizes a spawned object's resolved `class` as one of the three
/// mushroom classes, by filename stem — the reverse of `class_name()`.
fn mushroom_kind_from_class(class: &Option<PathBuf>) -> Option<MushroomKind> {
    let stem = class.as_ref()?.file_stem()?.to_str()?;
    MushroomKind::all().into_iter().find(|kind| kind.class_name() == stem)
}

/// A small solid-colored cube `ObjectClass` for one mushroom kind —
/// `is_trigger: true` so walking into one eats it. Mirrors the shape of the
/// old sandbox's `default_barrel_class`, parameterized instead of repeated
/// three times.
fn default_mushroom_class(kind: MushroomKind) -> ObjectClass {
    ObjectClass {
        name: kind.class_name().to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: None,
        scale: [0.5, 0.5, 0.5],
        is_dynamic: false,
        is_trigger: true,
        animation: None,
        script: None,
    }
}

fn mushroom_pickup_object(name: &str, position: [f32; 3], kind: MushroomKind) -> LevelObject {
    LevelObject {
        name: name.to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: None,
        position,
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [0.5, 0.5, 0.5],
        is_dynamic: false,
        is_trigger: true,
        animation: None,
        script: None,
        class: Some(PathBuf::from(format!("classes/{}.ron", kind.class_name()))),
        level_transition: None,
        screen_effect: None,
        camera_shake: None,
    }
}

/// The entrance hall: player spawns here (`enter_play_mode`'s fixed
/// `(0, 3, 4)` start). A `Bounce` mushroom sits in the open, and a raised
/// platform (top surface at y=1.6) blocks the only way forward — the base
/// jump (~1.0 unit of height) can't reach it, the `Bounce`-boosted jump
/// (~3.7 units) clears it easily. The `LevelTransition` trigger waiting on
/// top leads to `tunnels`.
fn entrance_level() -> Level {
    let floor = LevelObject {
        name: "Entrance Floor".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Plane),
        texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
        position: [0.0, 0.0, 2.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [3.0, 1.0, 4.0],
        is_dynamic: false,
        is_trigger: false,
        animation: None,
        script: None,
        class: None,
        level_transition: None,
        screen_effect: None,
        camera_shake: None,
    };

    let light = LevelLight {
        name: "Entrance Light".to_string(),
        position: [0.0, 3.0, 2.0],
        color: [1.0, 0.85, 0.6],
        intensity: 1.6,
        range: 8.0,
    };

    let bounce_mushroom = mushroom_pickup_object("Bounce Mushroom", [0.0, 1.0, 1.0], MushroomKind::Bounce);

    // Solid block whose top surface sits at y=1.6 — a platform to jump
    // onto, not a wall to jump over, so there's no ambiguity about how to
    // reach the far side once the jump is high enough. Shifted 0.1 forward
    // of where its front face would exactly meet the floor's back edge
    // (z=-2) — the floor is a zero-thickness quad at y=0 while the
    // platform's front face spans y=-1.6..1.6, so an exact z=-2 seam let
    // the two surfaces sit flush and z-fight; overlapping them by 0.1
    // embeds the seam inside solid geometry instead.
    let platform = LevelObject {
        name: "Raised Platform".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
        position: [0.0, 0.0, -3.9],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [6.0, 3.2, 4.0],
        is_dynamic: false,
        is_trigger: false,
        animation: None,
        script: None,
        class: None,
        level_transition: None,
        screen_effect: None,
        camera_shake: None,
    };

    // Sits on the platform's top surface (y=1.6) near its far edge, so
    // landing the jump isn't enough on its own — the player has to walk
    // across the platform to reach it. Embedded 0.1 into the platform
    // (bottom at y=1.5, not exactly the platform's y=1.6 top) for the same
    // reason as the platform/floor seam above — an exactly flush trigger
    // bottom would z-fight against the platform's top face.
    let exit_trigger = LevelObject {
        name: "Tunnels Entrance".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: None,
        position: [0.0, 2.0, -5.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [2.0, 1.0, 2.0],
        is_dynamic: false,
        is_trigger: true,
        animation: None,
        script: None,
        class: None,
        level_transition: Some(LevelTransition {
            target_level: "tunnels".to_string(),
            spawn_position: [0.0, 1.0, 4.0],
            spawn_yaw_deg: 0.0,
        }),
        screen_effect: Some(ScreenEffectSpec {
            color: [1.0, 1.0, 1.0],
            strength: 0.7,
            fade_in_secs: 0.05,
            hold_secs: 0.05,
            fade_out_secs: 0.3,
        }),
        camera_shake: None,
    };

    Level {
        name: "entrance".to_string(),
        objects: vec![floor, bounce_mushroom, platform, exit_trigger],
        lights: vec![light],
        particle_emitters: Vec::new(),
        rig_instances: Vec::new(),
        physics: PhysicsParams::default(),
        music_path: Some(PathBuf::from("music/demo_ambient.wav")),
    }
}

/// A crawl gate: a `Shrink` mushroom, then a ceiling block whose underside
/// starts at y=0.6 — a normal-sized player's collider (radius 0.4, so
/// standing height reaches ~0.9) can't fit under it, a shrunk player
/// (radius 0.2, ~0.5) fits through freely. No side walls needed — the
/// ceiling spans the same width as the floor, and there's simply no floor
/// beyond its edges to walk around on.
fn tunnels_level() -> Level {
    let floor = LevelObject {
        name: "Tunnels Floor".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Plane),
        texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
        position: [0.0, 0.0, -1.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [3.0, 1.0, 6.0],
        is_dynamic: false,
        is_trigger: false,
        animation: None,
        script: None,
        class: None,
        level_transition: None,
        screen_effect: None,
        camera_shake: None,
    };

    let light = LevelLight {
        name: "Tunnels Light".to_string(),
        position: [0.0, 2.5, 2.0],
        color: [0.7, 0.8, 1.0],
        intensity: 1.4,
        range: 7.0,
    };

    let shrink_mushroom = mushroom_pickup_object("Shrink Mushroom", [0.0, 1.0, 2.0], MushroomKind::Shrink);

    let ceiling_gate = LevelObject {
        name: "Low Ceiling".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
        position: [0.0, 2.1, -3.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [6.0, 3.0, 1.0],
        is_dynamic: false,
        is_trigger: false,
        animation: None,
        script: None,
        class: None,
        level_transition: None,
        screen_effect: None,
        camera_shake: None,
    };

    let exit_trigger = LevelObject {
        name: "Grotto Entrance".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: None,
        position: [0.0, 1.0, -6.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [2.0, 1.0, 2.0],
        is_dynamic: false,
        is_trigger: true,
        animation: None,
        script: None,
        class: None,
        level_transition: Some(LevelTransition {
            target_level: "grotto".to_string(),
            spawn_position: [0.0, 1.0, 4.0],
            spawn_yaw_deg: 0.0,
        }),
        screen_effect: Some(ScreenEffectSpec {
            color: [1.0, 1.0, 1.0],
            strength: 0.7,
            fade_in_secs: 0.05,
            hold_secs: 0.05,
            fade_out_secs: 0.3,
        }),
        camera_shake: None,
    };

    Level {
        name: "tunnels".to_string(),
        objects: vec![floor, shrink_mushroom, ceiling_gate, exit_trigger],
        lights: vec![light],
        particle_emitters: Vec::new(),
        rig_instances: Vec::new(),
        physics: PhysicsParams::default(),
        music_path: Some(PathBuf::from("music/demo_ambient.wav")),
    }
}

/// The final room: a `Glow` mushroom and a deliberately dark cave (no point
/// lights placed) leading to the "Heart of the Grove" — the vertical
/// slice's goal. Note: the darkness is atmospheric, not a hard mechanical
/// gate — collision doesn't depend on lighting, so the room is still
/// walkable without `Glow`. A true "can't proceed without light" gate would
/// need a per-level ambient-darkness system that doesn't exist yet.
fn grotto_level() -> Level {
    let floor = LevelObject {
        name: "Grotto Floor".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Plane),
        texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
        position: [0.0, 0.0, -3.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [4.0, 1.0, 9.0],
        is_dynamic: false,
        is_trigger: false,
        animation: None,
        script: None,
        class: None,
        level_transition: None,
        screen_effect: None,
        camera_shake: None,
    };

    let glow_mushroom = mushroom_pickup_object("Glow Mushroom", [0.0, 1.0, 2.0], MushroomKind::Glow);

    // Tall pillars flanking the path, alternating sides — mostly lost in
    // the dark ambient/fog until `Glow`'s much stronger light sweeps past
    // them, giving the ability something concrete to visibly reveal rather
    // than lighting an empty room.
    let make_pillar = |name: &str, x: f32, z: f32| LevelObject {
        name: name.to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
        position: [x, 1.0, z],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [0.6, 2.2, 0.6],
        is_dynamic: false,
        is_trigger: false,
        animation: None,
        script: None,
        class: None,
        level_transition: None,
        screen_effect: None,
        camera_shake: None,
    };
    // Wider x offset and z spacing than before — the previous, tighter
    // layout put the last pillar and the goal close enough along the same
    // viewing ray that the goal's (undistinguished, shared trigger-cyan)
    // silhouette visually blended into the checkered pillar behind it from
    // some angles, reading as clipping even though nothing overlapped in
    // 3D. More clearance plus the goal's own color (below) fixes both.
    let pillar_1 = make_pillar("Cave Pillar 1", -1.8, -1.0);
    let pillar_2 = make_pillar("Cave Pillar 2", 1.8, -3.5);
    let pillar_3 = make_pillar("Cave Pillar 3", -1.8, -6.0);
    let pillar_4 = make_pillar("Cave Pillar 4", 1.8, -8.5);

    let grove_sparkles = LevelParticleEmitter {
        name: "Grove Sparkles".to_string(),
        position: [0.0, 1.8, -10.0],
        def: ParticleEmitterDef::default(),
    };

    let heart_of_the_grove = LevelObject {
        name: "Heart of the Grove".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
        position: [0.0, 1.0, -10.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [1.0, 1.0, 1.0],
        is_dynamic: false,
        is_trigger: true,
        animation: Some(AnimationSpec::Orbit {
            axis: [0.0, 1.0, 0.0],
            speed_deg_per_sec: 60.0,
        }),
        script: None,
        class: None,
        level_transition: None,
        screen_effect: Some(ScreenEffectSpec {
            color: [1.0, 0.85, 0.2],
            strength: 0.6,
            fade_in_secs: 0.1,
            hold_secs: 0.2,
            fade_out_secs: 0.6,
        }),
        // A small rumble alongside the gold flash — pairs a screen effect
        // with a camera shake for a proper "you found it" impact moment.
        camera_shake: Some(CameraShakeSpec {
            intensity: 0.08,
            duration_secs: 0.4,
        }),
    };

    Level {
        name: "grotto".to_string(),
        objects: vec![floor, glow_mushroom, pillar_1, pillar_2, pillar_3, pillar_4, heart_of_the_grove],
        lights: Vec::new(),
        particle_emitters: vec![grove_sparkles],
        rig_instances: Vec::new(),
        physics: PhysicsParams::default(),
        music_path: Some(PathBuf::from("music/demo_ambient.wav")),
    }
}

/// Hand-rolls a minimal mono 16-bit PCM WAV file (RIFF/fmt/data chunks) —
/// no crate needed just to generate a couple of small placeholder sounds
/// for the demo level. Not a general-purpose encoder: fixed to
/// mono/16-bit, which is all `generate_blip_samples`/`generate_ambient_samples`
/// below need.
fn write_wav(path: &Path, samples: &[i16], sample_rate: u32) -> anyhow::Result<()> {
    let data_size = (samples.len() * 2) as u32;
    let byte_rate = sample_rate * 2;
    let mut bytes = Vec::with_capacity(44 + samples.len() * 2);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes()); // block align (mono, 16-bit)
    bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

/// A short sine blip with a linear fade in/out (avoids an audible click at
/// the start/end of a one-shot sound) — the placeholder played by
/// `bob_demo.pss`'s `interact()` alongside its existing procedural `play_tone`.
fn generate_blip_samples(sample_rate: u32) -> Vec<i16> {
    let duration_secs = 0.15;
    let freq = 880.0;
    let fade_secs = 0.01;
    let count = (sample_rate as f32 * duration_secs) as usize;
    let fade_samples = ((sample_rate as f32 * fade_secs) as usize).max(1);
    (0..count)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let envelope = if i < fade_samples {
                i as f32 / fade_samples as f32
            } else if i >= count - fade_samples {
                (count - i) as f32 / fade_samples as f32
            } else {
                1.0
            };
            let value = (t * freq * std::f32::consts::TAU).sin() * envelope * 0.3;
            (value * i16::MAX as f32) as i16
        })
        .collect()
}

/// A soft three-note chord (a root, its octave, and its fifth) sized to
/// loop seamlessly: every component completes a whole number of cycles
/// within `duration_secs`, so the waveform's value and slope match at the
/// loop point with no audible click — no fade needed the way a one-shot
/// sound needs one.
fn generate_ambient_samples(sample_rate: u32) -> Vec<i16> {
    let duration_secs = 2.0;
    let count = (sample_rate as f32 * duration_secs) as usize;
    (0..count)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let wave = |freq: f32| (t * freq * std::f32::consts::TAU).sin();
            let value = (wave(110.0) * 0.4 + wave(220.0) * 0.3 + wave(330.0) * 0.2) * 0.25;
            (value * i16::MAX as f32) as i16
        })
        .collect()
}

/// The starting rig: a simple 6-part cube humanoid (Torso root + Head, two
/// arms, two legs) with one "Wave" clip swinging `RightArm`. Written to
/// `rigs/humanoid.ron` on first run, same bootstrap-if-empty convention as
/// `default_demo_profiles`/`default_level`.
fn default_rig_asset() -> RigAsset {
    let part = |name: &str, parent: Option<&str>, local_position: [f32; 3], scale: [f32; 3]| RigPartDef {
        name: name.to_string(),
        parent: parent.map(str::to_string),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: None,
        local_position,
        local_rotation_euler_deg: [0.0, 0.0, 0.0],
        scale,
    };

    let wave_clip = RigClip {
        name: "Wave".to_string(),
        duration: 1.0,
        looping: true,
        tracks: vec![JointTrack {
            joint_name: "RightArm".to_string(),
            keyframes: vec![
                Keyframe { time: 0.0, rotation_euler_deg: [0.0, 0.0, 0.0] },
                Keyframe { time: 0.5, rotation_euler_deg: [0.0, 0.0, -60.0] },
                Keyframe { time: 1.0, rotation_euler_deg: [0.0, 0.0, 0.0] },
            ],
        }],
    };

    RigAsset {
        name: "humanoid".to_string(),
        parts: vec![
            part("Torso", None, [0.0, 0.0, 0.0], [0.5, 0.7, 0.3]),
            part("Head", Some("Torso"), [0.0, 0.55, 0.0], [0.3, 0.3, 0.3]),
            part("LeftArm", Some("Torso"), [-0.4, 0.2, 0.0], [0.15, 0.5, 0.15]),
            part("RightArm", Some("Torso"), [0.4, 0.2, 0.0], [0.15, 0.5, 0.15]),
            part("LeftLeg", Some("Torso"), [-0.2, -0.6, 0.0], [0.18, 0.55, 0.18]),
            part("RightLeg", Some("Torso"), [0.2, -0.6, 0.0], [0.18, 0.55, 0.18]),
        ],
        clips: vec![wave_clip],
    }
}

fn white_fallback_texture(gl: &glow::Context) -> GpuTexture {
    GpuTexture::from_rgba8(gl, &[255, 255, 255, 255], 1, 1, TextureFilter::Nearest)
        .expect("1x1 solid-color texture upload cannot fail")
}

/// A solid 1x1 color texture — used to render trigger zones as a flat bright
/// cyan box so they stay visually identifiable in both Edit and Play mode.
/// True see-through would need alpha blending, which the render pipeline
/// doesn't have yet — a known simplification, documented in the guide.
fn solid_color_texture(gl: &glow::Context, rgba: [u8; 4]) -> GpuTexture {
    GpuTexture::from_rgba8(gl, &rgba, 1, 1, TextureFilter::Nearest)
        .expect("1x1 solid-color texture upload cannot fail")
}

const TRIGGER_COLOR: [u8; 4] = [40, 220, 220, 255];

/// Approximate collision bounds from mesh kind + scale, not an exact
/// mesh-fitted bound — fine for boxy low-poly levels, called out in the
/// guide. Shared by `spawn_level_object` and `apply_class_to_instances`
/// (re-resolving a classed object's collider after an "Apply to All
/// Instances" edit).
fn collider_half_extents(mesh: &MeshSource, scale: [f32; 3]) -> Vec3 {
    match mesh {
        MeshSource::Primitive(PrimitiveKind::Plane) => Vec3::new(scale[0], 0.1, scale[2]),
        _ => Vec3::from(scale) * 0.5,
    }
}

/// The `ScriptApi` a script (or a native `Behavior`) sees, scoped to
/// exactly one entity — built fresh for each dispatch, never stored.
struct SandboxScriptApi<'a> {
    transform: &'a mut Transform,
    audio: Option<&'a AudioContext>,
    elapsed: f32,
    hud: &'a mut HudState,
    screen_effects: &'a mut ScreenEffectState,
    camera_shake: &'a mut CameraShakeState,
    asset_root: &'a Path,
}

impl ScriptApi for SandboxScriptApi<'_> {
    fn log(&mut self, message: &str) {
        log::info!("[script] {message}");
    }

    fn position(&self) -> (f32, f32, f32) {
        (self.transform.position.x, self.transform.position.y, self.transform.position.z)
    }

    fn set_position(&mut self, x: f32, y: f32, z: f32) {
        self.transform.position = Vec3::new(x, y, z);
    }

    fn move_by(&mut self, dx: f32, dy: f32, dz: f32) {
        self.transform.position += Vec3::new(dx, dy, dz);
    }

    fn play_tone(&mut self, frequency_hz: f32, duration_secs: f32) {
        if let Some(audio) = self.audio {
            audio.play_tone(frequency_hz, duration_secs);
        }
    }

    fn play_sfx(&mut self, path: &str) {
        if let Some(audio) = self.audio {
            let full_path = self.asset_root.join(path);
            if let Err(err) = audio.play_sfx_file(&full_path) {
                log::warn!("failed to play sfx {full_path:?}: {err}");
            }
        }
    }

    fn elapsed(&self) -> f32 {
        self.elapsed
    }

    fn set_hud_bar(&mut self, name: &str, fraction: f32) {
        self.hud.set_bar(name, fraction);
    }

    fn show_toast(&mut self, message: &str, seconds: f32) {
        self.hud.show_toast(message, seconds);
    }

    fn screen_flash(
        &mut self,
        r: f32,
        g: f32,
        b: f32,
        strength: f32,
        fade_in_secs: f32,
        hold_secs: f32,
        fade_out_secs: f32,
    ) {
        self.screen_effects.trigger(ScreenEffectSpec {
            color: [r, g, b],
            strength,
            fade_in_secs,
            hold_secs,
            fade_out_secs,
        });
    }

    fn camera_shake(&mut self, intensity: f32, duration_secs: f32) {
        self.camera_shake.trigger(CameraShakeSpec { intensity, duration_secs });
    }
}

/// Launches the standalone script editor (`tools/script_editor`) pointed at
/// `path`, non-blocking. Dev-time convenience: assumes the sibling binary
/// lives next to this one, true when both are built from the same
/// workspace target directory — not something a shipped game needs.
fn spawn_script_editor(path: &Path) {
    let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) else {
        log::error!("could not determine the current executable's directory");
        return;
    };
    let editor_name = if cfg!(windows) { "script_editor.exe" } else { "script_editor" };
    let editor_path = exe_dir.join(editor_name);
    match std::process::Command::new(&editor_path).arg(path).spawn() {
        Ok(_) => log::info!("launched script editor for {path:?}"),
        Err(err) => log::error!("failed to launch script editor at {editor_path:?} (is it built?): {err}"),
    }
}

struct MeshUniforms {
    model: Option<glow::UniformLocation>,
    view: Option<glow::UniformLocation>,
    proj: Option<glow::UniformLocation>,
    light_dir: Option<glow::UniformLocation>,
    ambient_color: Option<glow::UniformLocation>,
    lighting_mode: Option<glow::UniformLocation>,
    vertex_snap_amount: Option<glow::UniformLocation>,
    fog_start: Option<glow::UniformLocation>,
    fog_end: Option<glow::UniformLocation>,
    fog_color: Option<glow::UniformLocation>,
    tex: Option<glow::UniformLocation>,
    point_light_pos: Vec<Option<glow::UniformLocation>>,
    point_light_color: Vec<Option<glow::UniformLocation>>,
    point_light_intensity: Vec<Option<glow::UniformLocation>>,
    point_light_range: Vec<Option<glow::UniformLocation>>,
    point_light_count: Option<glow::UniformLocation>,
}

/// Matches the fixed-size point-light arrays declared in `mesh.vert`.
const MAX_POINT_LIGHTS: usize = 4;

impl MeshUniforms {
    fn resolve(gl: &glow::Context, program: glow::Program) -> Self {
        unsafe {
            Self {
                model: gl.get_uniform_location(program, "uModel"),
                view: gl.get_uniform_location(program, "uView"),
                proj: gl.get_uniform_location(program, "uProj"),
                light_dir: gl.get_uniform_location(program, "uLightDir"),
                ambient_color: gl.get_uniform_location(program, "uAmbientColor"),
                lighting_mode: gl.get_uniform_location(program, "uLightingMode"),
                vertex_snap_amount: gl.get_uniform_location(program, "uVertexSnapAmount"),
                fog_start: gl.get_uniform_location(program, "uFogStart"),
                fog_end: gl.get_uniform_location(program, "uFogEnd"),
                fog_color: gl.get_uniform_location(program, "uFogColor"),
                tex: gl.get_uniform_location(program, "uTex"),
                point_light_pos: (0..MAX_POINT_LIGHTS)
                    .map(|i| gl.get_uniform_location(program, &format!("uPointLightPos[{i}]")))
                    .collect(),
                point_light_color: (0..MAX_POINT_LIGHTS)
                    .map(|i| gl.get_uniform_location(program, &format!("uPointLightColor[{i}]")))
                    .collect(),
                point_light_intensity: (0..MAX_POINT_LIGHTS)
                    .map(|i| gl.get_uniform_location(program, &format!("uPointLightIntensity[{i}]")))
                    .collect(),
                point_light_range: (0..MAX_POINT_LIGHTS)
                    .map(|i| gl.get_uniform_location(program, &format!("uPointLightRange[{i}]")))
                    .collect(),
                point_light_count: gl.get_uniform_location(program, "uPointLightCount"),
            }
        }
    }
}

/// Where `assets/` and `profiles/` are read from at runtime.
///
/// A shipped `.exe` has them copied alongside it, so we look there first.
/// `cargo run`/`cargo build` (debug only) falls back to the crate's own
/// source directory so assets don't need to be copied into `target/debug` on
/// every change — that fallback is compiled out of release builds entirely,
/// so a shipped binary never has the dev machine's path baked into it.
fn resolve_asset_root() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));

    if exe_dir.join("assets").is_dir() {
        return exe_dir;
    }

    #[cfg(debug_assertions)]
    {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if manifest_dir.join("assets").is_dir() {
            return manifest_dir;
        }
    }

    exe_dir
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorMode {
    Edit,
    Play,
}

/// Whether the dev-time level/shader editor should ever be reachable —
/// `true` in debug builds, `false` in release. `cargo build --release`
/// (what `package.ps1` already uses for shipping) is treated as "this is
/// the game a player gets," so it never enters `EditorMode::Edit`: no
/// panels, no F1/F2/Tab/F5/F3, no orbit-camera drag. `cfg!()` is a
/// compile-time constant, so every `if editor_available()` branch below
/// is dead code in a release build, not just an unreachable runtime path.
fn editor_available() -> bool {
    cfg!(debug_assertions)
}

/// Tags a rig instance's root entity (see `spawn_rig_instance`) so it can be
/// listed in the F2 "Rigs" section separately from the flat "Scene Objects"
/// outliner, and so `build_level_from_ecs` can reconstruct `Level.rig_instances`.
struct RigRoot {
    name: String,
    rig_path: PathBuf,
}

/// Tags an entity spawned from an `engine::class::ObjectClass` so
/// "Apply to All Instances" can find every member of a class, and so
/// `build_level_from_ecs` can write `LevelObject::class` back out.
struct ClassMember {
    class_path: PathBuf,
}

struct Sandbox {
    asset_root: PathBuf,
    profiles_dir: PathBuf,
    levels_dir: PathBuf,
    shader_cache: Option<ShaderVariantCache>,
    shader_paths: (PathBuf, PathBuf, PathBuf),
    render_params: RenderParams,
    world: World,
    camera: OrbitCamera,
    renderer: Option<Renderer>,
    profiles: Option<ProfileCycler>,
    ui: Option<EguiState>,
    ui_visible: bool,
    save_as_name: String,
    levels: Vec<Level>,
    current_level_name: String,
    level_save_as_name: String,
    level_ui_visible: bool,
    selected_entity: Option<Entity>,
    cube_mesh: Option<Arc<GpuMesh>>,
    plane_mesh: Option<Arc<GpuMesh>>,
    particle_quad_mesh: Option<Arc<GpuMesh>>,
    physics_params: PhysicsParams,
    mode: EditorMode,
    /// Only meaningful while `mode == EditorMode::Play` — freezes gameplay
    /// simulation (see `Game::update`) and shows the pause menu overlay
    /// (see `Game::render`) without despawning the player or restoring the
    /// pre-Play snapshot the way `exit_play_mode` does.
    paused: bool,
    fp_camera: FirstPersonCamera,
    /// Optional walking view-bob, applied on top of `fp_camera` in
    /// `player_eye_position`. Toggle/tune live via the F1 panel's "Camera"
    /// section.
    head_bob: HeadBob,
    /// Momentary camera jitter — fired via `ScriptApi::camera_shake` or a
    /// trigger's `LevelObjectMeta::camera_shake`, applied in
    /// `player_eye_position` alongside `head_bob`/`landing_dip`.
    camera_shake: CameraShakeState,
    /// Camera roll while strafing, eased toward a target each frame in
    /// `update_player_input` and written into `fp_camera.roll`.
    strafe_tilt: StrafeTilt,
    /// Additive FOV widening at speed, added on top of `base_fov_radians`
    /// (not `fp_camera.fov_y_radians` directly, so it never fights with
    /// ability-driven FOV changes like `eat_mushroom`'s).
    speed_fov: SpeedFov,
    /// The FOV `speed_fov`'s kick is added on top of each frame — set by
    /// `eat_mushroom` (or just left at its default) instead of writing
    /// `fp_camera.fov_y_radians` directly.
    base_fov_radians: f32,
    /// Camera dip-and-recover spring, kicked by `land()` (see the landing-
    /// detection check around `engine::physics::step` in `Game::update`).
    landing_dip: LandingDip,
    /// `RigidBody::grounded` as of last frame — compared against this
    /// frame's value (after `physics::step`) to detect the exact frame the
    /// player lands, since `grounded` alone doesn't say whether it just
    /// became true.
    was_grounded: bool,
    player_entity: Option<Entity>,
    pre_play_snapshot: Option<Level>,
    // `None` if no audio output device was available — degrades silently
    // rather than failing the whole game.
    audio: Option<AudioContext>,
    /// Trigger-vs-entity pairs overlapping as of last frame, diffed each
    /// frame against `physics::step`'s return value to fire enter/exit.
    trigger_overlaps: HashSet<(Entity, Entity)>,
    /// Total seconds since startup — stored (rather than read from `Context`
    /// each time) since scripts can be dispatched from places without a
    /// `Context` handy (spawning, the F2 panel).
    elapsed_time: f32,
    /// F2 panel: whether the selected object's script is being edited
    /// inline, and the text buffer backing that `TextEdit`.
    script_editor_open: bool,
    script_editor_buffer: String,
    rigs_dir: PathBuf,
    rigs: Vec<RigAsset>,
    /// F2 panel: the currently selected rig *instance* (its root entity)
    /// and, within it, the currently selected *part* for hand-posing — kept
    /// separate from `selected_entity` since rigs get their own "Rigs"
    /// section rather than sharing the flat Scene Objects outliner.
    selected_rig: Option<Entity>,
    selected_rig_part: Option<Entity>,
    rig_new_name: String,
    new_part_name: String,
    new_part_parent: String,
    new_clip_name: String,
    new_clip_duration: f32,
    new_clip_looping: bool,
    classes_dir: PathBuf,
    classes: Vec<ObjectClass>,
    /// F2 panel: the currently selected *class* (an index into `classes`,
    /// not an ECS entity — a class is plain data, not a live thing).
    selected_class: Option<usize>,
    class_new_name: String,
    /// F2 panel: transient input buffer for the "Add Particle Emitter"
    /// naming — actual per-emitter properties are edited on the selected
    /// entity's own `ParticleEmitter` component, not buffered here.
    particle_emitter_new_name: String,
    /// In-game HUD state (title/bars/toasts) — drawn in Play mode only,
    /// mutated by scripts/native `Behavior`s through `ScriptApi`.
    hud: HudState,
    /// The currently-playing full-screen color flash/tint, if any — ticked
    /// every frame and composited into the frame in `render()`. Triggered by
    /// scripts/native `Behavior`s via `ScriptApi::screen_flash` or by a
    /// trigger's `LevelObjectMeta::screen_effect`.
    screen_effects: ScreenEffectState,
    /// Which mushroom the player last ate, if any — drives `hud.title` and
    /// is checked by `eat_mushroom` before applying a new one. Not itself
    /// level data; persists across `LevelTransition`s since it lives here,
    /// not on the level.
    active_mushroom: Option<MushroomKind>,
    saves_dir: PathBuf,
    /// The current level's background music, if any — mirrors
    /// `Level::music_path` and is set/cleared by `apply_level`, edited via
    /// the F2 panel's "Assign Music.../Stop Music", and written back out by
    /// `build_level_from_ecs`.
    current_music_path: Option<PathBuf>,
    /// Persists the F2 panel's volume slider across level loads, since
    /// `play_music_file` itself has no volume parameter — re-applied via
    /// `set_music_volume` every time music (re)starts.
    music_volume: f32,
}

impl Sandbox {
    fn new() -> Self {
        let asset_root = resolve_asset_root();
        let profiles_dir = asset_root.join("profiles");
        let levels_dir = asset_root.join("levels");
        let rigs_dir = asset_root.join("rigs");
        let classes_dir = asset_root.join("classes");
        let saves_dir = asset_root.join("saves");
        Self {
            asset_root,
            profiles_dir,
            levels_dir,
            rigs_dir,
            classes_dir,
            saves_dir,
            shader_cache: None,
            shader_paths: (PathBuf::new(), PathBuf::new(), PathBuf::new()),
            render_params: RenderParams::default(),
            world: World::new(),
            camera: OrbitCamera::new(Vec3::ZERO, 4.0),
            renderer: None,
            profiles: None,
            ui: None,
            ui_visible: true,
            save_as_name: String::from("custom"),
            levels: Vec::new(),
            current_level_name: String::from("default"),
            level_save_as_name: String::from("level_1"),
            level_ui_visible: true,
            selected_entity: None,
            cube_mesh: None,
            particle_quad_mesh: None,
            plane_mesh: None,
            physics_params: PhysicsParams::default(),
            mode: EditorMode::Edit,
            paused: false,
            fp_camera: FirstPersonCamera::new(),
            head_bob: HeadBob::new(),
            camera_shake: CameraShakeState::default(),
            strafe_tilt: StrafeTilt::new(),
            speed_fov: SpeedFov::new(),
            base_fov_radians: 60f32.to_radians(),
            landing_dip: LandingDip::new(),
            was_grounded: false,
            player_entity: None,
            pre_play_snapshot: None,
            trigger_overlaps: HashSet::new(),
            elapsed_time: 0.0,
            script_editor_open: false,
            script_editor_buffer: String::new(),
            rigs: Vec::new(),
            selected_rig: None,
            selected_rig_part: None,
            rig_new_name: String::from("humanoid"),
            new_part_name: String::from("Part"),
            new_part_parent: String::new(),
            new_clip_name: String::from("clip_1"),
            new_clip_duration: 1.0,
            new_clip_looping: true,
            classes: Vec::new(),
            selected_class: None,
            class_new_name: String::from("class_1"),
            particle_emitter_new_name: String::from("Sparkles"),
            hud: HudState::default(),
            screen_effects: ScreenEffectState::default(),
            active_mushroom: None,
            current_music_path: None,
            music_volume: 1.0,
            audio: match AudioContext::new() {
                Ok(audio) => Some(audio),
                Err(err) => {
                    log::warn!("no audio output available, sounds will be silent: {err}");
                    None
                }
            },
        }
    }

    fn play_tone(&self, frequency_hz: f32, duration_secs: f32) {
        if let Some(audio) = &self.audio {
            audio.play_tone(frequency_hz, duration_secs);
        }
    }

    /// Builds a `ScriptApi` scoped to `entity` and calls `f` with it and its
    /// `BehaviorSlot` — the one place that bridges `Behavior`/`ScriptApi` to
    /// the ECS. Uses the same "combined tuple `query_one`" idiom as every
    /// other simultaneous two-component fetch in this file, so it needs no
    /// new borrow-checker workaround. Returns `false` if `entity` has no
    /// `Transform` + `BehaviorSlot` (not scripted, or already despawned).
    fn with_behavior(&mut self, entity: Entity, f: impl FnOnce(&mut dyn Behavior, &mut dyn ScriptApi)) -> bool {
        let elapsed = self.elapsed_time;
        let audio = self.audio.as_ref();
        let hud = &mut self.hud;
        let screen_effects = &mut self.screen_effects;
        let camera_shake = &mut self.camera_shake;
        let asset_root = self.asset_root.as_path();
        if let Ok(mut query) = self.world.query_one::<(&mut Transform, &mut BehaviorSlot)>(entity) {
            if let Some((transform, BehaviorSlot(behavior))) = query.get() {
                let mut api = SandboxScriptApi { transform, audio, elapsed, hud, screen_effects, camera_shake, asset_root };
                f(behavior.as_mut(), &mut api);
                return true;
            }
        }
        false
    }

    /// (Re)compiles the script at `relative_path` and attaches it as
    /// `entity`'s `BehaviorSlot`, replacing any existing one — used by
    /// "Attach/Change Script...", "Reload", and after an inline "Save".
    /// Updates `LevelObjectMeta::script_path` so it round-trips when the
    /// level is saved, and fires `on_ready` immediately so new logic takes
    /// effect live rather than waiting for the next hook call.
    fn attach_script_to_entity(&mut self, entity: Entity, relative_path: &Path) {
        let full_path = self.asset_root.join(relative_path);
        let source = match std::fs::read_to_string(&full_path) {
            Ok(source) => source,
            Err(err) => {
                log::error!("failed to read script {full_path:?}: {err}");
                return;
            }
        };
        let behavior = match ScriptBehavior::from_source(&source) {
            Ok(behavior) => behavior,
            Err(errors) => {
                for error in errors {
                    log::error!("script error in {full_path:?}: {error}");
                }
                return;
            }
        };
        let _ = self.world.insert_one(entity, BehaviorSlot(Box::new(behavior)));
        if let Ok(mut query) = self.world.query_one::<&mut LevelObjectMeta>(entity) {
            if let Some(meta) = query.get() {
                meta.script_path = Some(relative_path.to_path_buf());
            }
        }
        self.with_behavior(entity, |behavior, api| behavior.on_ready(api));
        log::info!("attached script {relative_path:?}");
    }

    /// Loads the profile's shader files, swaps the composite shader, and
    /// resizes the offscreen target for its resolution scale. Only needs
    /// `gl`/`drawable_size` (not a full `&mut Context`) so it can be called
    /// from inside the egui closure in `render` without borrow conflicts.
    fn apply_profile(
        &mut self,
        gl: &glow::Context,
        drawable_size: (u32, u32),
        profile: &ShaderProfile,
    ) -> anyhow::Result<()> {
        let vertex_body = std::fs::read_to_string(self.asset_root.join(&profile.vertex_shader))?;
        let fragment_body =
            std::fs::read_to_string(self.asset_root.join(&profile.fragment_shader))?;
        self.shader_cache = Some(ShaderVariantCache::new(vertex_body, fragment_body));

        let post_fragment_src =
            std::fs::read_to_string(self.asset_root.join(&profile.post_fragment_shader))?;
        match self.renderer.as_mut() {
            Some(renderer) => {
                renderer.set_composite_shader(gl, &post_fragment_src)?;
                renderer.set_resolution_scale(profile.render.resolution_scale);
                renderer.resize_if_needed(gl, drawable_size)?;
            }
            None => {
                self.renderer = Some(Renderer::new(
                    gl,
                    drawable_size,
                    profile.render.resolution_scale,
                    &post_fragment_src,
                )?);
            }
        }

        self.shader_paths = (
            profile.vertex_shader.clone(),
            profile.fragment_shader.clone(),
            profile.post_fragment_shader.clone(),
        );
        self.render_params = profile.render;
        log::info!("applied shader profile '{}'", profile.name);
        Ok(())
    }

    fn save_current_profile(&self) {
        if let Some(cycler) = self.profiles.as_ref() {
            let profile = cycler.current();
            let path = self.profiles_dir.join(format!("{}.ron", profile.name));
            match engine::profile::save_to_file(profile, &path) {
                Ok(()) => log::info!("saved profile '{}' to {path:?}", profile.name),
                Err(err) => log::error!("failed to save profile '{}': {err}", profile.name),
            }
        }
    }

    fn save_as_new_profile(&mut self) {
        let name = self.save_as_name.trim();
        if name.is_empty() {
            log::warn!("cannot save a profile with an empty name");
            return;
        }
        let (vertex_shader, fragment_shader, post_fragment_shader) = self.shader_paths.clone();
        let profile = ShaderProfile {
            name: name.to_string(),
            vertex_shader,
            fragment_shader,
            post_fragment_shader,
            render: self.render_params,
        };
        let path = self.profiles_dir.join(format!("{}.ron", profile.name));
        match engine::profile::save_to_file(&profile, &path) {
            Ok(()) => {
                log::info!("saved new profile '{}' to {path:?}", profile.name);
                if let Some(cycler) = self.profiles.as_mut() {
                    cycler.add(profile);
                }
            }
            Err(err) => log::error!("failed to save new profile '{name}': {err}"),
        }
    }

    // --- Level editor ---

    /// Resolves an object's mesh (cached primitive, or an OBJ loaded fresh)
    /// and texture (or a white fallback), then spawns it with a `Transform` +
    /// `MeshRenderer` + `LevelObjectMeta` (the last so it round-trips back
    /// into a `LevelObject` when the level is saved).
    /// Resolves a `MeshSource` to a GPU mesh — a cached, shared primitive, or
    /// an OBJ loaded fresh. Shared by `spawn_level_object` and
    /// `spawn_rig_part` so both go through the same cube/plane cache.
    fn resolve_mesh(&mut self, gl: &glow::Context, source: &MeshSource) -> anyhow::Result<Arc<GpuMesh>> {
        Ok(match source {
            MeshSource::Primitive(PrimitiveKind::Cube) => {
                if self.cube_mesh.is_none() {
                    self.cube_mesh = Some(Arc::new(GpuMesh::upload(gl, &primitives::cube())?));
                }
                Arc::clone(self.cube_mesh.as_ref().unwrap())
            }
            MeshSource::Primitive(PrimitiveKind::Plane) => {
                if self.plane_mesh.is_none() {
                    self.plane_mesh = Some(Arc::new(GpuMesh::upload(gl, &primitives::plane())?));
                }
                Arc::clone(self.plane_mesh.as_ref().unwrap())
            }
            MeshSource::ObjFile(path) => {
                // Only the first sub-mesh is used per level object (matches
                // the engine's existing single-submesh assumption elsewhere);
                // author multi-part OBJs as separate level objects instead.
                let data = load_obj(&self.asset_root.join(path))?;
                Arc::new(GpuMesh::upload(gl, &data[0])?)
            }
        })
    }

    /// Resolves an optional texture path to a GPU texture, or a solid white
    /// 1x1 fallback if `path` is `None` or fails to load. Shared by
    /// `spawn_level_object` and `spawn_rig_part`.
    fn resolve_texture(&self, gl: &glow::Context, path: Option<&Path>) -> Arc<GpuTexture> {
        match path {
            Some(path) => {
                match GpuTexture::load_from_file(gl, &self.asset_root.join(path), TextureFilter::Nearest) {
                    Ok(tex) => Arc::new(tex),
                    Err(err) => {
                        log::error!("failed to load texture {path:?}: {err}; using white fallback");
                        Arc::new(white_fallback_texture(gl))
                    }
                }
            }
            None => Arc::new(white_fallback_texture(gl)),
        }
    }

    /// Resolves an `ObjectClass` referenced by `class_path` — matched by
    /// file stem against `self.classes` (same convention as
    /// `find_rig_asset`: `save_to_file` always writes `classes/<name>.ron`,
    /// so the stem recovers the name without a separate path table).
    fn find_class(&self, class_path: &Path) -> Option<&ObjectClass> {
        let stem = class_path.file_stem()?.to_str()?;
        self.classes.iter().find(|class| class.name == stem)
    }

    /// If `obj.class` references a loaded class, returns a copy of `obj`
    /// with mesh/texture/scale/dynamic/trigger/animation/script overridden
    /// from the class — everything about an object except its placement
    /// (name/position/rotation stay `obj`'s own). Falls back to `obj`
    /// unchanged if unclassed, or if the referenced class isn't loaded
    /// (logged, not fatal — the object still spawns as a plain object).
    fn resolve_class_overrides(&self, obj: &LevelObject) -> LevelObject {
        let Some(class_path) = &obj.class else { return obj.clone() };
        let Some(class) = self.find_class(class_path) else {
            log::warn!(
                "object '{}' references missing class {class_path:?}; spawning with its own fallback fields",
                obj.name
            );
            return obj.clone();
        };
        LevelObject {
            mesh: class.mesh.clone(),
            texture_path: class.texture_path.clone(),
            scale: class.scale,
            is_dynamic: class.is_dynamic,
            is_trigger: class.is_trigger,
            animation: class.animation,
            script: class.script.clone(),
            ..obj.clone()
        }
    }

    fn spawn_level_object(&mut self, gl: &glow::Context, obj: &LevelObject) -> anyhow::Result<Entity> {
        let resolved = self.resolve_class_overrides(obj);
        let obj = &resolved;

        let mesh = self.resolve_mesh(gl, &obj.mesh)?;

        let texture = if obj.is_trigger {
            Arc::new(solid_color_texture(gl, TRIGGER_COLOR))
        } else {
            self.resolve_texture(gl, obj.texture_path.as_deref())
        };

        let half_extents = collider_half_extents(&obj.mesh, obj.scale);

        let rotation_euler_deg = Vec3::from(obj.rotation_euler_deg);
        let entity = self.world.spawn((
            Transform {
                position: Vec3::from(obj.position),
                rotation: euler_deg_to_quat(rotation_euler_deg),
                scale: Vec3::from(obj.scale),
            },
            MeshRenderer {
                mesh,
                texture: Some(texture),
            },
            LevelObjectMeta {
                name: obj.name.clone(),
                mesh_source: obj.mesh.clone(),
                texture_path: obj.texture_path.clone(),
                rotation_euler_deg,
                script_path: obj.script.clone(),
                level_transition: obj.level_transition.clone(),
                screen_effect: obj.screen_effect.clone(),
                camera_shake: obj.camera_shake.clone(),
            },
            Collider {
                shape: ColliderShape::Aabb { half_extents },
                is_trigger: obj.is_trigger,
            },
        ));

        if obj.is_dynamic {
            let _ = self.world.insert_one(entity, RigidBody::default());
        }

        if let Some(script_path) = &obj.script {
            let full_path = self.asset_root.join(script_path);
            match std::fs::read_to_string(&full_path) {
                Ok(source) => match ScriptBehavior::from_source(&source) {
                    Ok(behavior) => {
                        let _ = self.world.insert_one(entity, BehaviorSlot(Box::new(behavior)));
                        self.with_behavior(entity, |behavior, api| behavior.on_ready(api));
                    }
                    Err(errors) => {
                        for error in errors {
                            log::error!("script error in {full_path:?}: {error}");
                        }
                    }
                },
                Err(err) => log::error!("failed to read script {full_path:?}: {err}"),
            }
        }

        if let Some(spec) = obj.animation {
            let kind = match spec {
                AnimationSpec::Orbit { axis, speed_deg_per_sec } => {
                    AnimationKind::Orbit { axis: Vec3::from(axis), speed_deg_per_sec }
                }
                AnimationSpec::Bob { axis, amplitude, period_secs } => {
                    AnimationKind::Bob { axis: Vec3::from(axis), amplitude, period_secs }
                }
            };
            let _ = self.world.insert_one(
                entity,
                Animator {
                    kind,
                    base_position: Vec3::from(obj.position),
                    base_rotation: euler_deg_to_quat(rotation_euler_deg),
                    elapsed: 0.0,
                },
            );
        }

        if let Some(class_path) = &obj.class {
            let _ = self.world.insert_one(entity, ClassMember { class_path: class_path.clone() });
        }

        Ok(entity)
    }

    /// Spawns a point light: `Transform` + `Light` + `LevelObjectMeta` (name
    /// only, no mesh/texture) so it shares the F2 outliner's selection code
    /// with mesh objects while carrying its own color/intensity/range fields.
    fn spawn_level_light(&mut self, light: &LevelLight) -> Entity {
        self.world.spawn((
            Transform {
                position: Vec3::from(light.position),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            Light {
                color: Vec3::from(light.color),
                intensity: light.intensity,
                kind: LightKind::Point { range: light.range },
            },
            LevelObjectMeta {
                name: light.name.clone(),
                mesh_source: MeshSource::Primitive(PrimitiveKind::Cube),
                texture_path: None,
                rotation_euler_deg: Vec3::ZERO,
                script_path: None,
                level_transition: None,
        screen_effect: None,
        camera_shake: None,
            },
        ))
    }

    /// Spawns a placed particle emitter: `Transform` + `LevelObjectMeta`
    /// (a filler `mesh_source`, no `MeshRenderer` — same trick
    /// `spawn_level_light` uses so it shares the outliner/selection code)
    /// + `ParticleEmitter`.
    fn spawn_level_particle_emitter(&mut self, emitter: &LevelParticleEmitter) -> Entity {
        self.world.spawn((
            Transform {
                position: Vec3::from(emitter.position),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            ParticleEmitter::new(emitter.def),
            LevelObjectMeta {
                name: emitter.name.clone(),
                mesh_source: MeshSource::Primitive(PrimitiveKind::Cube),
                texture_path: None,
                rotation_euler_deg: Vec3::ZERO,
                script_path: None,
                level_transition: None,
        screen_effect: None,
        camera_shake: None,
            },
        ))
    }

    fn apply_level(&mut self, gl: &glow::Context, level: &Level) -> anyhow::Result<()> {
        self.world.clear();
        self.selected_entity = None;
        self.selected_rig = None;
        self.selected_rig_part = None;
        for obj in &level.objects {
            match self.spawn_level_object(gl, obj) {
                Ok(entity) => {
                    // A mushroom's kind isn't part of `LevelObject`'s generic
                    // schema — it's recovered from the resolved class name,
                    // then attached as a marker plus a per-kind solid-color
                    // texture override (every trigger's texture is otherwise
                    // forced to the same shared cyan in `spawn_level_object`).
                    if let Some(kind) = mushroom_kind_from_class(&obj.class) {
                        let _ = self.world.insert_one(entity, MushroomPickup(kind));
                        if let Ok(mut query) = self.world.query_one::<&mut MeshRenderer>(entity) {
                            if let Some(renderer) = query.get() {
                                renderer.texture = Some(Arc::new(solid_color_texture(gl, kind.rgba())));
                            }
                        }
                    }
                    // The vertical slice's one-off goal object — otherwise
                    // it'd share the same flat cyan every other unlabeled
                    // trigger gets, which reads as ambiguous/overlapping
                    // against nearby geometry rather than a distinct goal.
                    if obj.name == "Heart of the Grove" {
                        if let Ok(mut query) = self.world.query_one::<&mut MeshRenderer>(entity) {
                            if let Some(renderer) = query.get() {
                                renderer.texture = Some(Arc::new(solid_color_texture(gl, [255, 200, 40, 255])));
                            }
                        }
                    }
                }
                Err(err) => log::error!("failed to spawn level object '{}': {err}", obj.name),
            }
        }
        for light in &level.lights {
            self.spawn_level_light(light);
        }
        for emitter in &level.particle_emitters {
            self.spawn_level_particle_emitter(emitter);
        }
        for instance in &level.rig_instances {
            let Some(asset) = self.find_rig_asset(&instance.rig_path).cloned() else {
                log::error!(
                    "failed to spawn rig instance '{}': rig {:?} not loaded",
                    instance.name,
                    instance.rig_path
                );
                continue;
            };
            if let Err(err) = self.spawn_rig_instance(gl, &asset, instance) {
                log::error!("failed to spawn rig instance '{}': {err}", instance.name);
            }
        }
        self.current_level_name = level.name.clone();
        self.physics_params = level.physics;
        self.current_music_path = level.music_path.clone();
        let music_volume = self.music_volume;
        if let Some(audio) = self.audio.as_mut() {
            match &self.current_music_path {
                Some(path) => {
                    let full_path = self.asset_root.join(path);
                    match audio.play_music_file(&full_path, true) {
                        Ok(()) => audio.set_music_volume(music_volume),
                        Err(err) => log::warn!("failed to play music {full_path:?}: {err}"),
                    }
                }
                None => audio.stop_music(),
            }
        }
        log::info!(
            "loaded level '{}' ({} objects, {} lights, {} particle emitters, {} rig instances)",
            level.name,
            level.objects.len(),
            level.lights.len(),
            level.particle_emitters.len(),
            level.rig_instances.len()
        );
        Ok(())
    }

    /// Spawns one rig part: `Transform` + `MeshRenderer` + `RigPart` (parent
    /// left unwired — `spawn_rig_instance` fills it in once every part in
    /// the rig exists) + `RigPartMeta` (so the part can be serialized back
    /// out later). The initial `Transform` is just a reasonable placeholder;
    /// `engine::rig::update_world_transforms` overwrites it the very next
    /// frame regardless.
    fn spawn_rig_part(&mut self, gl: &glow::Context, def: &RigPartDef) -> anyhow::Result<Entity> {
        let mesh = self.resolve_mesh(gl, &def.mesh)?;
        let texture = self.resolve_texture(gl, def.texture_path.as_deref());
        let local_rotation_euler_deg = Vec3::from(def.local_rotation_euler_deg);
        let local_rotation = euler_deg_to_quat(local_rotation_euler_deg);
        let local_position = Vec3::from(def.local_position);

        let entity = self.world.spawn((
            Transform {
                position: local_position,
                rotation: local_rotation,
                scale: Vec3::from(def.scale),
            },
            MeshRenderer { mesh, texture: Some(texture) },
            RigPart { parent: None, local_position, local_rotation_euler_deg, local_rotation },
            engine::rig::RigPartMeta { mesh_source: def.mesh.clone(), texture_path: def.texture_path.clone() },
        ));
        Ok(entity)
    }

    /// Spawns every part of `asset`, wires up the parent hierarchy by name,
    /// and tags the root with `Rig`/`RigAnimator`/`RigRoot` — the root being
    /// whichever part has `parent: None` (the first one found, if an asset
    /// somehow has more than one, which authoring through the F2 panel never
    /// produces). `instance`'s position/rotation become the root's local
    /// pose (a root's local pose *is* its world pose).
    fn spawn_rig_instance(&mut self, gl: &glow::Context, asset: &RigAsset, instance: &RigInstance) -> anyhow::Result<Entity> {
        let mut parts_by_name: HashMap<String, Entity> = HashMap::with_capacity(asset.parts.len());
        for def in &asset.parts {
            let entity = self.spawn_rig_part(gl, def)?;
            parts_by_name.insert(def.name.clone(), entity);
        }

        // Second pass: every part entity now exists, so parent references
        // can be resolved regardless of authoring order in the file.
        for def in &asset.parts {
            let Some(&entity) = parts_by_name.get(&def.name) else { continue };
            let parent = def.parent.as_ref().and_then(|name| parts_by_name.get(name).copied());
            if let Ok(mut part) = self.world.get::<&mut RigPart>(entity) {
                part.parent = parent;
            }
        }

        let root_def = asset
            .parts
            .iter()
            .find(|def| def.parent.is_none())
            .ok_or_else(|| anyhow::anyhow!("rig '{}' has no root part (a part with parent: None)", asset.name))?;
        let root_entity = *parts_by_name
            .get(&root_def.name)
            .ok_or_else(|| anyhow::anyhow!("failed to resolve root entity for rig '{}'", asset.name))?;

        let rotation_euler_deg = Vec3::from(instance.rotation_euler_deg);
        if let Ok(mut root_part) = self.world.get::<&mut RigPart>(root_entity) {
            root_part.local_position = Vec3::from(instance.position);
            root_part.set_local_rotation_euler_deg(rotation_euler_deg);
        }

        let current_clip = instance
            .playing_clip
            .as_ref()
            .and_then(|name| asset.clips.iter().position(|clip| &clip.name == name));

        // Three separate `insert_one` calls, not one call with a tuple:
        // `insert_one`'s generic bound would happily accept a tuple as a
        // single opaque component instead of unpacking it, silently
        // breaking every later single-component query/filter against these
        // types (the same footgun documented on `spawn_native_behavior_demo`).
        let _ = self.world.insert_one(root_entity, Rig { parts_by_name });
        let _ = self.world.insert_one(
            root_entity,
            RigAnimator {
                clips: asset.clips.clone(),
                current_clip,
                time: 0.0,
                playing: current_clip.is_some(),
                speed: 1.0,
                ..Default::default()
            },
        );
        let _ = self
            .world
            .insert_one(root_entity, RigRoot { name: instance.name.clone(), rig_path: instance.rig_path.clone() });

        Ok(root_entity)
    }

    /// The rig-world mirror of `build_level_from_ecs`: reconstructs a
    /// `RigAsset` from a placed instance's live entities (current part
    /// offsets + whatever clips its `RigAnimator` holds), for "Save Rig".
    fn build_rig_asset_from_ecs(&self, root: Entity, name: String) -> Option<RigAsset> {
        let rig = self.world.get::<&Rig>(root).ok()?;
        let entity_to_name: HashMap<Entity, String> =
            rig.parts_by_name.iter().map(|(part_name, &entity)| (entity, part_name.clone())).collect();

        let mut parts = Vec::new();
        for (part_name, &entity) in &rig.parts_by_name {
            let Ok(rig_part) = self.world.get::<&RigPart>(entity) else { continue };
            let Ok(meta) = self.world.get::<&engine::rig::RigPartMeta>(entity) else { continue };
            let Ok(transform) = self.world.get::<&Transform>(entity) else { continue };
            parts.push(RigPartDef {
                name: part_name.clone(),
                parent: rig_part.parent.and_then(|parent_entity| entity_to_name.get(&parent_entity).cloned()),
                mesh: meta.mesh_source.clone(),
                texture_path: meta.texture_path.clone(),
                local_position: rig_part.local_position.to_array(),
                local_rotation_euler_deg: rig_part.local_rotation_euler_deg.to_array(),
                scale: transform.scale.to_array(),
            });
        }

        let clips = self.world.get::<&RigAnimator>(root).map(|animator| animator.clips.clone()).unwrap_or_default();
        Some(RigAsset { name, parts, clips })
    }

    /// Finds a loaded `RigAsset` by the relative path a `RigInstance`
    /// references it with — matched by file stem against `self.rigs`
    /// (which, like `self.levels`/`self.profiles`, only remembers each
    /// asset's `name`, not the path it was loaded from; `save_to_file`
    /// always writes `rigs/<name>.ron`, so the file stem recovers it).
    fn find_rig_asset(&self, rig_path: &Path) -> Option<&RigAsset> {
        let stem = rig_path.file_stem()?.to_str()?;
        self.rigs.iter().find(|rig| rig.name == stem)
    }

    fn build_level_from_ecs(&self) -> Level {
        let mut objects = Vec::new();
        for (entity, (transform, meta)) in self
            .world
            .query::<(&Transform, &LevelObjectMeta)>()
            .without::<&Light>()
            .without::<&ParticleEmitter>()
            .iter()
        {
            // An animated object's live `Transform` is mid-motion — save the
            // `Animator`'s fixed base pose instead, so saving mid-animation
            // doesn't capture a random instant.
            let position = self
                .world
                .get::<&Animator>(entity)
                .map(|animator| animator.base_position)
                .unwrap_or(transform.position);

            let animation = self.world.get::<&Animator>(entity).ok().map(|animator| {
                match animator.kind {
                    AnimationKind::Orbit { axis, speed_deg_per_sec } => {
                        AnimationSpec::Orbit { axis: axis.to_array(), speed_deg_per_sec }
                    }
                    AnimationKind::Bob { axis, amplitude, period_secs } => {
                        AnimationSpec::Bob { axis: axis.to_array(), amplitude, period_secs }
                    }
                }
            });

            objects.push(LevelObject {
                name: meta.name.clone(),
                mesh: meta.mesh_source.clone(),
                texture_path: meta.texture_path.clone(),
                position: position.to_array(),
                rotation_euler_deg: meta.rotation_euler_deg.to_array(),
                scale: transform.scale.to_array(),
                is_dynamic: self.world.get::<&RigidBody>(entity).is_ok(),
                is_trigger: self
                    .world
                    .get::<&Collider>(entity)
                    .is_ok_and(|collider| collider.is_trigger),
                animation,
                script: meta.script_path.clone(),
                class: self.world.get::<&ClassMember>(entity).ok().map(|cm| cm.class_path.clone()),
                level_transition: meta.level_transition.clone(),
                screen_effect: meta.screen_effect.clone(),
                camera_shake: meta.camera_shake.clone(),
            });
        }

        let mut lights = Vec::new();
        for (_entity, (transform, meta, light)) in
            self.world.query::<(&Transform, &LevelObjectMeta, &Light)>().iter()
        {
            let range = match light.kind {
                LightKind::Point { range } => range,
                LightKind::Directional { .. } => 10.0,
            };
            lights.push(LevelLight {
                name: meta.name.clone(),
                position: transform.position.to_array(),
                color: light.color.to_array(),
                intensity: light.intensity,
                range,
            });
        }

        let mut particle_emitters = Vec::new();
        for (_entity, (transform, meta, emitter)) in
            self.world.query::<(&Transform, &LevelObjectMeta, &ParticleEmitter)>().iter()
        {
            particle_emitters.push(LevelParticleEmitter {
                name: meta.name.clone(),
                position: transform.position.to_array(),
                def: emitter.def,
            });
        }

        let mut rig_instances = Vec::new();
        for (_entity, (root, part, animator)) in
            self.world.query::<(&RigRoot, &RigPart, &RigAnimator)>().iter()
        {
            let playing_clip = animator.current_clip.and_then(|i| animator.clips.get(i)).map(|clip| clip.name.clone());
            rig_instances.push(RigInstance {
                name: root.name.clone(),
                rig_path: root.rig_path.clone(),
                position: part.local_position.to_array(),
                rotation_euler_deg: part.local_rotation_euler_deg.to_array(),
                playing_clip,
            });
        }

        Level {
            name: self.current_level_name.clone(),
            objects,
            lights,
            particle_emitters,
            rig_instances,
            physics: self.physics_params,
            music_path: self.current_music_path.clone(),
        }
    }

    fn save_current_level(&self) {
        let level = self.build_level_from_ecs();
        let path = self.levels_dir.join(format!("{}.ron", level.name));
        match engine::level::save_to_file(&level, &path) {
            Ok(()) => log::info!("saved level '{}' to {path:?}", level.name),
            Err(err) => log::error!("failed to save level '{}': {err}", level.name),
        }
    }

    fn save_level_as(&mut self) {
        let name = self.level_save_as_name.trim();
        if name.is_empty() {
            log::warn!("cannot save a level with an empty name");
            return;
        }
        let mut level = self.build_level_from_ecs();
        level.name = name.to_string();
        let path = self.levels_dir.join(format!("{}.ron", level.name));
        match engine::level::save_to_file(&level, &path) {
            Ok(()) => {
                log::info!("saved new level '{}' to {path:?}", level.name);
                self.current_level_name = level.name.clone();
                self.levels.push(level);
            }
            Err(err) => log::error!("failed to save new level '{name}': {err}"),
        }
    }

    fn add_primitive(&mut self, gl: &glow::Context, kind: PrimitiveKind) {
        let count = self.world.query::<&LevelObjectMeta>().iter().count();
        let label = match kind {
            PrimitiveKind::Cube => "Cube",
            PrimitiveKind::Plane => "Plane",
        };
        let obj = LevelObject {
            name: format!("{label}_{}", count + 1),
            mesh: MeshSource::Primitive(kind),
            texture_path: None,
            position: [count as f32 * 1.5 - 1.5, 0.0, 0.0],
            rotation_euler_deg: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            is_dynamic: matches!(kind, PrimitiveKind::Cube),
            is_trigger: false,
            animation: None,
            script: None,
            class: None,
            level_transition: None,
        screen_effect: None,
        camera_shake: None,
        };
        match self.spawn_level_object(gl, &obj) {
            Ok(entity) => self.selected_entity = Some(entity),
            Err(err) => log::error!("failed to add {label}: {err}"),
        }
    }

    /// Adds a cube-shaped, non-solid trigger zone — the same "Add Object"
    /// spawn path as `add_primitive`, but pre-configured `is_trigger: true`
    /// and rendered in a distinct cyan so it stays identifiable in the editor.
    fn add_trigger_zone(&mut self, gl: &glow::Context) {
        let count = self.world.query::<&LevelObjectMeta>().iter().count();
        let obj = LevelObject {
            name: format!("Trigger_{}", count + 1),
            mesh: MeshSource::Primitive(PrimitiveKind::Cube),
            texture_path: None,
            position: [count as f32 * 1.5 - 1.5, 0.0, 0.0],
            rotation_euler_deg: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            is_dynamic: false,
            is_trigger: true,
            animation: None,
            script: None,
            class: None,
            level_transition: None,
        screen_effect: None,
        camera_shake: None,
        };
        match self.spawn_level_object(gl, &obj) {
            Ok(entity) => self.selected_entity = Some(entity),
            Err(err) => log::error!("failed to add trigger zone: {err}"),
        }
    }

    /// Opens a file picker for an OBJ (then an optional texture) and adds it
    /// as a new level object — unlike the old "Import Model", this does not
    /// clear the existing scene.
    fn import_model_as_object(&mut self, gl: &glow::Context) {
        let Some(model_path) = rfd::FileDialog::new()
            .add_filter("Wavefront OBJ", &["obj"])
            .set_directory(&self.asset_root)
            .pick_file()
        else {
            return;
        };

        let texture_path = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg"])
            .set_directory(&self.asset_root)
            .pick_file();

        let name = model_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Imported".to_string());
        let count = self.world.query::<&LevelObjectMeta>().iter().count();

        let obj = LevelObject {
            name,
            mesh: MeshSource::ObjFile(engine::level::relativize(&model_path, &self.asset_root)),
            texture_path: texture_path
                .map(|path| engine::level::relativize(&path, &self.asset_root)),
            position: [count as f32 * 1.5 - 1.5, 0.0, 0.0],
            rotation_euler_deg: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            is_dynamic: false,
            is_trigger: false,
            animation: None,
            script: None,
            class: None,
            level_transition: None,
        screen_effect: None,
        camera_shake: None,
        };

        match self.spawn_level_object(gl, &obj) {
            Ok(entity) => {
                self.selected_entity = Some(entity);
                log::info!("imported {model_path:?} as a new level object");
            }
            Err(err) => log::error!("failed to import model {model_path:?}: {err}"),
        }
    }

    /// F2 "Assign Music..." — picks an audio file, starts it looping
    /// immediately (so the change is audible right away, matching how
    /// "Assign Texture..." updates the live entity), and records it as
    /// `current_music_path` so it round-trips when the level is saved.
    fn assign_music(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Audio", &["wav", "ogg", "mp3", "flac"])
            .set_directory(&self.asset_root)
            .pick_file()
        else {
            return;
        };
        self.current_music_path = Some(engine::level::relativize(&path, &self.asset_root));
        let volume = self.music_volume;
        if let Some(audio) = self.audio.as_mut() {
            match audio.play_music_file(&path, true) {
                Ok(()) => audio.set_music_volume(volume),
                Err(err) => log::error!("failed to play music {path:?}: {err}"),
            }
        }
    }

    fn stop_current_music(&mut self) {
        self.current_music_path = None;
        if let Some(audio) = self.audio.as_mut() {
            audio.stop_music();
        }
    }

    fn assign_texture_to_entity(&mut self, gl: &glow::Context, entity: Entity) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg"])
            .set_directory(&self.asset_root)
            .pick_file()
        else {
            return;
        };

        let texture = match GpuTexture::load_from_file(gl, &path, TextureFilter::Nearest) {
            Ok(tex) => Arc::new(tex),
            Err(err) => {
                log::error!("failed to load texture {path:?}: {err}");
                return;
            }
        };

        let relative_path = engine::level::relativize(&path, &self.asset_root);
        if let Ok(mut query) = self.world.query_one::<(&mut MeshRenderer, &mut LevelObjectMeta)>(entity) {
            if let Some((renderer, meta)) = query.get() {
                renderer.texture = Some(texture);
                meta.texture_path = Some(relative_path);
            }
        }
    }

    // --- Play mode ---

    const PLAYER_EYE_HEIGHT: f32 = 0.5;

    fn player_eye_position(&self) -> Vec3 {
        self.player_entity
            .and_then(|entity| self.world.get::<&Transform>(entity).ok())
            .map(|transform| {
                transform.position
                    + Vec3::new(0.0, Self::PLAYER_EYE_HEIGHT, 0.0)
                    + self.head_bob.offset(self.fp_camera.right())
                    + self.landing_dip.offset()
                    + self.camera_shake.offset(self.fp_camera.right(), Vec3::Y)
            })
            .unwrap_or(Vec3::ZERO)
    }

    /// Snapshots the level (so Stop can restore it exactly), then spawns an
    /// invisible player rig (sphere collider + rigid body, no mesh) above the
    /// level and switches to the first-person camera. Turns on relative
    /// mouse mode for unbounded FPS look (restored to normal on exit).
    fn enter_play_mode(&mut self, ctx: &mut Context) {
        if self.mode == EditorMode::Play {
            return;
        }
        self.pre_play_snapshot = Some(self.build_level_from_ecs());
        self.fp_camera = FirstPersonCamera::new();
        self.trigger_overlaps.clear();

        let start_position = Vec3::new(0.0, 3.0, 4.0);
        let entity = self.world.spawn((
            Transform {
                position: start_position,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            RigidBody::default(),
            Collider {
                shape: ColliderShape::Sphere { radius: 0.4 },
                is_trigger: false,
            },
            PlayerController::default(),
        ));
        self.player_entity = Some(entity);

        ctx.platform.sdl.mouse().set_relative_mouse_mode(true);
        self.mode = EditorMode::Play;
        self.paused = false;
        log::info!("entered play mode");
    }

    /// Despawns the player and restores the exact pre-Play level state —
    /// hitting Stop never leaves physics-knocked-around objects behind.
    fn exit_play_mode(&mut self, ctx: &mut Context) {
        if self.mode == EditorMode::Edit {
            return;
        }
        if let Some(player) = self.player_entity.take() {
            let _ = self.world.despawn(player);
        }
        if let Some(snapshot) = self.pre_play_snapshot.take() {
            let gl = ctx.gl();
            if let Err(err) = self.apply_level(gl, &snapshot) {
                log::error!("failed to restore pre-play level state: {err}");
            }
        }
        ctx.platform.sdl.mouse().set_relative_mouse_mode(false);
        self.mode = EditorMode::Edit;
        self.paused = false;
        log::info!("exited play mode, restored edit-time state");
    }

    /// Freezes gameplay simulation (see the `paused` check in `Game::update`)
    /// and releases the mouse so the pause menu's buttons are clickable —
    /// Play mode's relative-mouse-mode FPS look would otherwise swallow
    /// clicks instead of moving a real cursor.
    fn pause(&mut self, ctx: &mut Context) {
        self.paused = true;
        ctx.platform.sdl.mouse().set_relative_mouse_mode(false);
    }

    /// Re-captures the mouse and lets gameplay simulation resume.
    fn resume(&mut self, ctx: &mut Context) {
        self.paused = false;
        ctx.platform.sdl.mouse().set_relative_mouse_mode(true);
    }

    /// Pause menu's "Restart Level" (offered instead of "Exit to Editor"
    /// when there's no editor to exit to — see `draw_pause_menu`) — reuses
    /// the existing snapshot/restore pair to reset the current Play
    /// session. Briefly passes through `EditorMode::Edit` inside this one
    /// call, but that's never rendered: no frame happens between the two
    /// calls.
    fn restart_level(&mut self, ctx: &mut Context) {
        self.exit_play_mode(ctx);
        self.enter_play_mode(ctx);
    }

    fn checkpoint_path(&self) -> PathBuf {
        self.saves_dir.join("checkpoint.ron")
    }

    /// F9 — snapshots the player's position/look direction to a single
    /// checkpoint file. Deliberately just one slot (`saves/checkpoint.ron`,
    /// overwritten each time), not a multi-slot save system — there's no
    /// per-object/script world state captured, so this is a "get back to
    /// roughly where I was" checkpoint, not a full save file.
    fn save_checkpoint(&mut self) {
        let Some(player) = self.player_entity else {
            log::warn!("F9: no player to checkpoint (not in Play mode?)");
            return;
        };
        let Ok(position) = self.world.get::<&Transform>(player).map(|t| t.position) else {
            return;
        };
        let data = SaveData {
            level_name: self.current_level_name.clone(),
            player_position: position.to_array(),
            player_yaw: self.fp_camera.yaw,
            player_pitch: self.fp_camera.pitch,
            saved_at_elapsed: self.elapsed_time,
        };
        let path = self.checkpoint_path();
        match engine::save::save_to_file(&data, &path) {
            Ok(()) => {
                log::info!("checkpoint saved to {path:?}");
                self.hud.show_toast("Checkpoint saved", 1.5);
            }
            Err(err) => log::error!("failed to save checkpoint: {err}"),
        }
    }

    /// F10 — loads the checkpoint and re-applies its player position/look
    /// direction, entering Play mode first if needed (there's no player
    /// entity to reposition in Edit mode). Does not re-load a different
    /// level if the checkpoint was saved in one — out of scope for a
    /// checkpoint this simple, just logged as a heads-up.
    fn load_checkpoint(&mut self, ctx: &mut Context) {
        let path = self.checkpoint_path();
        let data = match engine::save::load_from_file(&path) {
            Ok(data) => data,
            Err(err) => {
                log::warn!("F10: no checkpoint to load ({err})");
                return;
            }
        };
        if data.level_name != self.current_level_name {
            log::warn!(
                "checkpoint was saved in level '{}', but '{}' is currently loaded — repositioning within the current level anyway",
                data.level_name, self.current_level_name
            );
        }
        if self.mode == EditorMode::Edit {
            self.enter_play_mode(ctx);
        }
        if let Some(player) = self.player_entity {
            if let Ok(mut transform) = self.world.get::<&mut Transform>(player) {
                transform.position = Vec3::from(data.player_position);
            }
        }
        self.fp_camera.yaw = data.player_yaw;
        self.fp_camera.pitch = data.player_pitch;
        log::info!("checkpoint loaded from {path:?}");
        self.hud.show_toast("Checkpoint loaded", 1.5);
    }

    /// Reads input each frame in Play mode and drives the player: mouse-look,
    /// WASD movement, Space to jump. **This is the place to add your own
    /// game's input handling** — swap the movement scheme, add sprint/crouch,
    /// wire up new keys, etc. `interact` below is the sibling hook for
    /// one-shot actions (currently bound to E).
    fn update_player_input(&mut self, ctx: &mut Context, dt: f32) {
        let Some(player) = self.player_entity else {
            return;
        };

        // Gamepad right stick blends additively with mouse look — whichever
        // is actually being used drives the camera, no mode switch needed.
        // `CONTROLLER_LOOK_SPEED` is in radians/sec at full stick deflection.
        const CONTROLLER_LOOK_SPEED: f32 = 2.5;
        let (mouse_dx, mouse_dy) = ctx.input.mouse_delta();
        let (right_stick_x, right_stick_y) = ctx.input.right_stick();
        self.fp_camera.look(
            mouse_dx as f32 * 0.0025 + right_stick_x * CONTROLLER_LOOK_SPEED * dt,
            -mouse_dy as f32 * 0.0025 - right_stick_y * CONTROLLER_LOOK_SPEED * dt,
        );

        let forward = self.fp_camera.forward();
        let right = self.fp_camera.right();
        // Flatten to the horizontal plane so looking up/down doesn't change ground speed.
        let forward_flat = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

        let mut move_dir = Vec3::ZERO;
        if ctx.input.is_key_down(Keycode::W) {
            move_dir += forward_flat;
        }
        if ctx.input.is_key_down(Keycode::S) {
            move_dir -= forward_flat;
        }
        if ctx.input.is_key_down(Keycode::D) {
            move_dir += right_flat;
        }
        if ctx.input.is_key_down(Keycode::A) {
            move_dir -= right_flat;
        }
        // Left stick blends in the same way — additive with WASD, whichever
        // is actually being pushed drives movement. `y` is SDL's raw
        // positive-down convention, so forward is `-left_stick_y`.
        let (left_stick_x, left_stick_y) = ctx.input.left_stick();
        move_dir += forward_flat * -left_stick_y + right_flat * left_stick_x;
        move_dir = move_dir.normalize_or_zero();

        // How much of this frame's movement is sideways, -1 (left) to 1
        // (right) — reused as-is for the strafe-tilt target rather than
        // tracking A/D separately, so diagonal movement tilts proportionally.
        let strafe_input = move_dir.dot(right_flat);
        self.strafe_tilt.update(strafe_input, dt);
        self.fp_camera.roll = self.strafe_tilt.roll_radians();
        self.camera_shake.tick(dt);
        self.landing_dip.update(dt);

        let mut jumped = false;
        if let Ok(mut query) = self.world.query_one::<(&mut RigidBody, &PlayerController)>(player) {
            if let Some((body, controller)) = query.get() {
                let horizontal_velocity = move_dir * controller.move_speed;
                body.velocity.x = horizontal_velocity.x;
                body.velocity.z = horizontal_velocity.z;
                self.head_bob.update(horizontal_velocity.length(), body.grounded, dt);
                self.speed_fov.update(horizontal_velocity.length(), dt);
                self.fp_camera.fov_y_radians = self.base_fov_radians + self.speed_fov.kick_radians();

                let jump_pressed = ctx.input.just_pressed(Keycode::Space)
                    || ctx.input.controller_just_pressed(ControllerButton::A);
                if jump_pressed && body.grounded {
                    body.velocity.y = controller.jump_speed;
                    jumped = true;
                }
            }
        }
        if jumped {
            self.play_tone(660.0, 0.12);
        }
    }

    const EDITOR_CAMERA_MOVE_SPEED: f32 = 6.0;

    /// WASD flies the Edit-mode `OrbitCamera`'s orbit target around on the
    /// horizontal plane (mirrors `update_player_input`'s WASD-relative-to-
    /// look-direction handling, flattened the same way so looking up/down
    /// doesn't change how fast you glide); Space/Shift raise/lower it
    /// straight along world Y. Mouse-drag orbit and scroll zoom are
    /// unaffected — this only moves what the camera orbits *around*.
    fn update_editor_camera_input(&mut self, ctx: &mut Context, dt: f32) {
        let forward = self.camera.forward();
        let right = self.camera.right();
        let forward_flat = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

        let mut move_dir = Vec3::ZERO;
        if ctx.input.is_key_down(Keycode::W) {
            move_dir += forward_flat;
        }
        if ctx.input.is_key_down(Keycode::S) {
            move_dir -= forward_flat;
        }
        if ctx.input.is_key_down(Keycode::D) {
            move_dir += right_flat;
        }
        if ctx.input.is_key_down(Keycode::A) {
            move_dir -= right_flat;
        }
        if ctx.input.is_key_down(Keycode::Space) {
            move_dir += Vec3::Y;
        }
        if ctx.input.is_key_down(Keycode::LShift) || ctx.input.is_key_down(Keycode::RShift) {
            move_dir -= Vec3::Y;
        }
        self.camera.target += move_dir.normalize_or_zero() * Self::EDITOR_CAMERA_MOVE_SPEED * dt;
    }

    /// Pushes the nearest dynamic level object within reach with an outward
    /// impulse — the example "interact with the environment" action, tying
    /// the physics engine and the input hook together. Replace this with
    /// your own game's interaction (pickup, dialogue, open a door, ...).
    fn interact(&mut self) {
        let Some(player) = self.player_entity else {
            return;
        };
        let Some(player_position) = self.world.get::<&Transform>(player).ok().map(|t| t.position)
        else {
            return;
        };
        let Some(controller) = self
            .world
            .get::<&PlayerController>(player)
            .ok()
            .map(|c| *c)
        else {
            return;
        };

        // Eligible targets are anything with a `RigidBody` (the original
        // "push it" demo) *or* a `BehaviorSlot` (a scripted/native object
        // that wants its own `on_interact`) — a plain query tuple can't
        // express "either," so the eligibility check is inline instead.
        let mut nearest: Option<(Entity, f32)> = None;
        for (entity, (transform, _meta)) in self.world.query::<(&Transform, &LevelObjectMeta)>().iter() {
            let interactable = self.world.get::<&RigidBody>(entity).is_ok()
                || self.world.get::<&BehaviorSlot>(entity).is_ok();
            if !interactable {
                continue;
            }
            let distance = transform.position.distance(player_position);
            if distance <= controller.interact_radius
                && nearest.is_none_or(|(_, best)| distance < best)
            {
                nearest = Some((entity, distance));
            }
        }

        let Some((entity, distance)) = nearest else {
            return;
        };

        // A scripted/native-behavior object handles its own interaction
        // instead of the generic push — it opted in by having a `Behavior`.
        if self.with_behavior(entity, |behavior, api| behavior.on_interact(api)) {
            log::info!("interacted with a behavior-driven object {distance:.2}m away");
            return;
        }

        let mut pushed = false;
        if let Ok(mut query) = self.world.query_one::<(&Transform, &mut RigidBody)>(entity) {
            if let Some((transform, body)) = query.get() {
                let mut push_dir = transform.position - player_position;
                push_dir.y = 0.0;
                let push_dir = push_dir.normalize_or_zero();
                let push_dir = if push_dir == Vec3::ZERO { Vec3::X } else { push_dir };
                body.velocity += push_dir * controller.interact_impulse
                    + Vec3::Y * (controller.interact_impulse * 0.3);
                log::info!("interacted with an object {distance:.2}m away");
                pushed = true;
            }
        }
        if pushed {
            self.play_tone(220.0, 0.1);
            if let Ok(position) = self.world.get::<&Transform>(entity).map(|t| t.position) {
                self.spawn_burst_at(position, ParticleEmitterDef { rate_per_sec: 0.0, ..ParticleEmitterDef::default() }, 12);
            }
            self.hud.show_toast("Pushed!", 1.5);
        }
    }

    /// Spawns a transient, self-cleaning burst of particles at `position` —
    /// no `LevelObjectMeta` (so it never shows up in Scene Objects or gets
    /// saved as level data) and no mesh/texture of its own, just a
    /// `ParticleEmitter` that `engine::particles::step` despawns once every
    /// particle it made has aged out (see `Game::update`). Ties particles +
    /// physics + the `interact` hook together, the same demo spirit as the
    /// jump/push tones.
    fn spawn_burst_at(&mut self, position: Vec3, def: ParticleEmitterDef, count: u32) {
        let mut emitter = ParticleEmitter::new(def);
        emitter.spawn_burst(position, count);
        self.world.spawn((Transform { position, rotation: Quat::IDENTITY, scale: Vec3::ONE }, emitter));
    }

    /// Sibling to `interact` for the other kind of "things that happen when
    /// the player does something": entering/exiting a non-solid trigger
    /// zone. Replace this with your own game's trigger logic (checkpoints,
    /// damage zones, ...). Also dispatches to the trigger's `Behavior`, if
    /// it has one — script-driven trigger logic alongside the built-in
    /// log+tone demo, and (if the trigger has one) a level transition —
    /// see `transition_to_level`. Returns `true` if a transition fired, so
    /// the caller knows this frame's trigger-overlap bookkeeping (computed
    /// against a world that `transition_to_level` just wiped) is stale and
    /// must be skipped rather than applied.
    fn on_trigger_entered(&mut self, gl: &glow::Context, trigger: Entity) -> bool {
        let mushroom_kind = self.world.get::<&MushroomPickup>(trigger).ok().map(|pickup| pickup.0);
        if let Some(kind) = mushroom_kind {
            self.eat_mushroom(kind);
            let _ = self.world.despawn(trigger);
            self.trigger_overlaps.retain(|&(_, t)| t != trigger);
            return true;
        }

        let name = self
            .world
            .get::<&LevelObjectMeta>(trigger)
            .map(|meta| meta.name.clone())
            .unwrap_or_else(|_| "Trigger".to_string());
        log::info!("entered trigger '{name}'");
        self.play_tone(880.0, 0.08);
        self.with_behavior(trigger, |behavior, api| behavior.on_trigger_enter(api, "Player"));

        let effect = self
            .world
            .get::<&LevelObjectMeta>(trigger)
            .ok()
            .and_then(|meta| meta.screen_effect);
        if let Some(effect) = effect {
            self.screen_effects.trigger(effect);
        }

        let shake = self
            .world
            .get::<&LevelObjectMeta>(trigger)
            .ok()
            .and_then(|meta| meta.camera_shake);
        if let Some(shake) = shake {
            self.camera_shake.trigger(shake);
        }

        let transition = self
            .world
            .get::<&LevelObjectMeta>(trigger)
            .ok()
            .and_then(|meta| meta.level_transition.clone());
        if let Some(transition) = transition {
            self.transition_to_level(gl, &transition);
            return true;
        }
        false
    }

    /// Grants `kind`'s ability, replacing whatever was active before —
    /// resets `PlayerController`/`Collider`/`FirstPersonCamera.fov_y_radians`
    /// to their spawn-time base values first (all read live every frame,
    /// nothing cached elsewhere) so effects never stack, then applies the
    /// new kind's modifiers. Deliberately drastic rather than subtle in
    /// every dimension available (move/jump feel, collider size, FOV,
    /// lighting) — with no visible player body in first person, a small
    /// change in any single one of these reads as ambiguous or "nothing
    /// happened"; a big change in all of them together reads as an obvious
    /// transformation. Also toggles the player's own point light (`Glow`),
    /// updates the HUD's persistent title, shows a toast, plays a pickup
    /// blip, and fires a strong screen flash in the mushroom's color.
    fn eat_mushroom(&mut self, kind: MushroomKind) {
        if let Some(player) = self.player_entity {
            if let Ok(mut query) = self.world.query_one::<(&mut PlayerController, &mut Collider)>(player) {
                if let Some((controller, collider)) = query.get() {
                    *controller = PlayerController::default();
                    let mut radius = 0.4;
                    match kind {
                        // A cartoonish, unmistakable super-bounce, with a
                        // touch more run speed to match the energy.
                        MushroomKind::Bounce => {
                            controller.jump_speed = 12.0;
                            controller.move_speed = 5.0;
                        }
                        // Small AND quick (a scurrying-mouse feel) with a
                        // weaker jump — being tiny, a huge leap wouldn't
                        // make sense, and the contrast sells the size change.
                        MushroomKind::Shrink => {
                            radius = 0.15;
                            controller.move_speed = 5.5;
                            controller.jump_speed = 3.0;
                        }
                        MushroomKind::Glow => {}
                    }
                    if let ColliderShape::Sphere { radius: r } = &mut collider.shape {
                        *r = radius;
                    }
                }
            }
            let _ = self.world.remove_one::<Light>(player);
            if kind == MushroomKind::Glow {
                let _ = self.world.insert_one(
                    player,
                    Light {
                        color: Vec3::new(1.0, 0.9, 0.45),
                        intensity: 5.0,
                        kind: LightKind::Point { range: 14.0 },
                    },
                );
            }
        }

        // A wider FOV reads as "the world got bigger around you" — a cheap,
        // strong visual cue with no player-model rendering to lean on. Sets
        // `base_fov_radians`, not `fp_camera.fov_y_radians` directly —
        // `update_player_input` adds `speed_fov`'s kick on top of this each
        // frame, so the two never fight over the same field.
        self.base_fov_radians = match kind {
            MushroomKind::Bounce => 70f32.to_radians(),
            MushroomKind::Shrink => 75f32.to_radians(),
            MushroomKind::Glow => 60f32.to_radians(),
        };

        self.active_mushroom = Some(kind);
        self.hud.title = Some(format!("{} Mushroom", kind.display_name()));
        self.hud.show_toast(&format!("Ate a {} Mushroom!", kind.display_name()), 2.0);
        self.play_tone(660.0, 0.12);
        self.screen_effects.trigger(ScreenEffectSpec {
            color: kind.color(),
            strength: 0.7,
            fade_in_secs: 0.05,
            hold_secs: 0.15,
            fade_out_secs: 0.45,
        });
    }

    /// Loads `transition.target_level` and repositions the player at
    /// `transition.spawn_position`/`spawn_yaw_deg` — the level-transition
    /// half of a trigger's job (see `on_trigger_entered`). Deliberately
    /// leaves `pre_play_snapshot` untouched: it still points at whatever
    /// level Play mode was entered from, so F3/Escape-stop after a mid-play
    /// transition correctly returns the editor to the level being edited,
    /// not wherever the player ended up.
    fn transition_to_level(&mut self, gl: &glow::Context, transition: &LevelTransition) {
        let Some(level) = self.levels.iter().find(|l| l.name == transition.target_level).cloned() else {
            log::error!(
                "level transition: no level named '{}' is loaded",
                transition.target_level
            );
            return;
        };
        if let Err(err) = self.apply_level(gl, &level) {
            log::error!("level transition to '{}' failed: {err}", transition.target_level);
            return;
        }

        // `apply_level` just wiped the player along with everything else —
        // respawn one at the transition's spawn point, mirroring
        // `enter_play_mode`'s own initial spawn exactly.
        let entity = self.world.spawn((
            Transform {
                position: Vec3::from(transition.spawn_position),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            RigidBody::default(),
            Collider {
                shape: ColliderShape::Sphere { radius: 0.4 },
                is_trigger: false,
            },
            PlayerController::default(),
        ));
        self.player_entity = Some(entity);
        self.fp_camera.pitch = 0.0;
        self.fp_camera.yaw = transition.spawn_yaw_deg.to_radians();
        // The old trigger entities this set refers to no longer exist —
        // leaving it stale risks a dead `Entity` aliasing a freshly-spawned
        // one after `world.clear()` resets id generations.
        self.trigger_overlaps.clear();
        self.hud.show_toast(&format!("Entering {}", level.name), 1.5);
    }

    fn on_trigger_exited(&mut self, trigger: Entity) {
        let name = self
            .world
            .get::<&LevelObjectMeta>(trigger)
            .map(|meta| meta.name.clone())
            .unwrap_or_else(|_| "Trigger".to_string());
        log::info!("exited trigger '{name}'");
        self.play_tone(440.0, 0.08);
        self.with_behavior(trigger, |behavior, api| behavior.on_trigger_exit(api, "Player"));
    }

    fn draw_level_editor_ui(&mut self, ui: &mut egui::Ui, gl: &glow::Context) {
        ui.heading("Level");
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                self.save_current_level();
            }
            ui.text_edit_singleline(&mut self.level_save_as_name);
            if ui.button("Save As New").clicked() {
                self.save_level_as();
            }
        });

        if !self.levels.is_empty() {
            ui.label("Load:");
            let mut clicked_index = None;
            for (i, level) in self.levels.iter().enumerate() {
                if ui
                    .selectable_label(level.name == self.current_level_name, &level.name)
                    .clicked()
                {
                    clicked_index = Some(i);
                }
            }
            if let Some(i) = clicked_index {
                let level = self.levels[i].clone();
                if let Err(err) = self.apply_level(gl, &level) {
                    log::error!("failed to load level '{}': {err}", level.name);
                }
            }
        }

        ui.separator();
        ui.heading("Music");
        ui.label(format!(
            "Track: {}",
            self.current_music_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "None".to_string())
        ));
        ui.horizontal(|ui| {
            if ui.button("Assign Music...").clicked() {
                self.assign_music();
            }
            if self.current_music_path.is_some() && ui.button("Stop Music").clicked() {
                self.stop_current_music();
            }
        });
        if self.current_music_path.is_some()
            && ui.add(egui::Slider::new(&mut self.music_volume, 0.0..=1.0).text("Volume")).changed()
        {
            let volume = self.music_volume;
            if let Some(audio) = self.audio.as_mut() {
                audio.set_music_volume(volume);
            }
        }

        ui.separator();
        ui.heading("Add Object");
        ui.horizontal(|ui| {
            if ui.button("Add Cube").clicked() {
                self.add_primitive(gl, PrimitiveKind::Cube);
            }
            if ui.button("Add Plane").clicked() {
                self.add_primitive(gl, PrimitiveKind::Plane);
            }
        });
        if ui.button("Import Model as Object...").clicked() {
            self.import_model_as_object(gl);
        }
        if ui.button("Add Trigger Zone").clicked() {
            self.add_trigger_zone(gl);
        }
        if ui.button("Add Light").clicked() {
            let count = self.world.query::<&Light>().iter().count();
            let light = LevelLight {
                name: format!("Light_{}", count + 1),
                position: [0.0, 2.0, 0.0],
                color: [1.0, 1.0, 1.0],
                intensity: 1.0,
                range: 5.0,
            };
            let entity = self.spawn_level_light(&light);
            self.selected_entity = Some(entity);
        }
        if ui.button("Add Particle Emitter").clicked() {
            let count = self.world.query::<&ParticleEmitter>().iter().count();
            let emitter = LevelParticleEmitter {
                name: format!("{}_{}", self.particle_emitter_new_name, count + 1),
                position: [0.0, 1.5, 0.0],
                def: ParticleEmitterDef::default(),
            };
            let entity = self.spawn_level_particle_emitter(&emitter);
            self.selected_entity = Some(entity);
        }

        ui.separator();
        ui.heading("Scene Objects");
        let mut clicked_entity = None;
        for (entity, meta) in self.world.query::<&LevelObjectMeta>().iter() {
            let selected = self.selected_entity == Some(entity);
            if ui.selectable_label(selected, &meta.name).clicked() {
                clicked_entity = Some(entity);
            }
        }
        if let Some(entity) = clicked_entity {
            self.selected_entity = Some(entity);
        }

        ui.separator();
        if let Some(entity) = self.selected_entity {
            if self.world.contains(entity) {
                self.draw_selected_object_ui(ui, gl, entity);
            } else {
                self.selected_entity = None;
            }
        } else {
            ui.label("No object selected.");
        }

        ui.separator();
        ui.heading("Rigs");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.rig_new_name);
            if ui.button("New Rig").clicked() {
                self.create_new_rig(gl);
            }
        });
        if !self.rigs.is_empty() {
            ui.label("Add instance of:");
            let mut add_instance_of = None;
            for (i, rig) in self.rigs.iter().enumerate() {
                if ui.selectable_label(false, &rig.name).clicked() {
                    add_instance_of = Some(i);
                }
            }
            if let Some(i) = add_instance_of {
                self.add_rig_instance(gl, i);
            }
        }

        let mut clicked_rig = None;
        for (entity, root) in self.world.query::<&RigRoot>().iter() {
            let selected = self.selected_rig == Some(entity);
            if ui.selectable_label(selected, &root.name).clicked() {
                clicked_rig = Some(entity);
            }
        }
        if let Some(entity) = clicked_rig {
            self.selected_rig = Some(entity);
            self.selected_rig_part = None;
        }

        ui.separator();
        if let Some(root) = self.selected_rig {
            if self.world.contains(root) {
                self.draw_selected_rig_ui(ui, gl, root);
            } else {
                self.selected_rig = None;
                self.selected_rig_part = None;
            }
        } else {
            ui.label("No rig selected.");
        }

        ui.separator();
        ui.heading("Classes");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.class_new_name);
            if ui.button("New Class").clicked() {
                self.create_new_class();
            }
        });
        let mut clicked_class = None;
        let mut add_instance_of_class = None;
        for (i, class) in self.classes.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui.selectable_label(self.selected_class == Some(i), &class.name).clicked() {
                    clicked_class = Some(i);
                }
                if ui.small_button("+ Instance").clicked() {
                    add_instance_of_class = Some(i);
                }
            });
        }
        if let Some(i) = clicked_class {
            self.selected_class = Some(i);
        }
        if let Some(i) = add_instance_of_class {
            self.spawn_object_from_class(gl, i);
        }

        ui.separator();
        if let Some(index) = self.selected_class {
            if index < self.classes.len() {
                self.draw_class_editor_ui(ui, gl, index);
            } else {
                self.selected_class = None;
            }
        } else {
            ui.label("No class selected.");
        }

        ui.separator();
        ui.small("F2 toggle level editor \u{b7} F3 play/stop \u{b7} click an object below to select it");
    }

    fn draw_selected_object_ui(&mut self, ui: &mut egui::Ui, gl: &glow::Context, entity: Entity) {
        if self.world.get::<&Light>(entity).is_ok() {
            self.draw_selected_light_ui(ui, entity);
            return;
        }
        if self.world.get::<&ParticleEmitter>(entity).is_ok() {
            self.draw_selected_particle_emitter_ui(ui, entity);
            return;
        }
        let classed_path = self.world.get::<&ClassMember>(entity).ok().map(|cm| cm.class_path.clone());
        if let Some(class_path) = classed_path {
            self.draw_classed_object_ui(ui, entity, &class_path);
            return;
        }

        let mut delete = false;
        let mut assign_texture = false;
        let mut is_dynamic = self.world.get::<&RigidBody>(entity).is_ok();
        let mut dynamic_changed = false;
        let mut is_trigger = self
            .world
            .get::<&Collider>(entity)
            .is_ok_and(|collider| collider.is_trigger);
        let mut trigger_changed = false;
        let mut texture_path_for_trigger_toggle = None;
        let mut preview_screen_effect: Option<ScreenEffectSpec> = None;
        let mut preview_camera_shake: Option<CameraShakeSpec> = None;

        let existing_animator = self.world.get::<&Animator>(entity).ok().map(|animator| animator.kind);
        let existing_base = self
            .world
            .get::<&Animator>(entity)
            .ok()
            .map(|animator| (animator.base_position, animator.base_rotation));
        let mut anim_selected: usize = match existing_animator {
            None => 0,
            Some(AnimationKind::Orbit { .. }) => 1,
            Some(AnimationKind::Bob { .. }) => 2,
        };
        let mut anim_axis: [f32; 3] = match existing_animator {
            Some(AnimationKind::Orbit { axis, .. }) => axis.to_array(),
            Some(AnimationKind::Bob { axis, .. }) => axis.to_array(),
            None => [0.0, 1.0, 0.0],
        };
        let mut anim_speed_deg_per_sec: f32 = match existing_animator {
            Some(AnimationKind::Orbit { speed_deg_per_sec, .. }) => speed_deg_per_sec,
            _ => 90.0,
        };
        let mut anim_amplitude: f32 = match existing_animator {
            Some(AnimationKind::Bob { amplitude, .. }) => amplitude,
            _ => 0.5,
        };
        let mut anim_period_secs: f32 = match existing_animator {
            Some(AnimationKind::Bob { period_secs, .. }) => period_secs,
            _ => 2.0,
        };
        let mut anim_changed = false;
        let mut base_position = Vec3::ZERO;
        let mut base_rotation = Quat::IDENTITY;

        let mut script_attach = false;
        let mut script_reload = false;
        let mut script_save_inline = false;
        let mut script_open_editor = false;
        let mut script_inline_just_opened = false;
        let mut current_script_path: Option<PathBuf> = None;

        {
            if let Ok(mut query) =
                self.world
                    .query_one::<(&mut Transform, &mut LevelObjectMeta, &mut Collider)>(entity)
            {
                if let Some((transform, meta, collider)) = query.get() {
                    let mut name = meta.name.clone();
                    if ui.text_edit_singleline(&mut name).changed() {
                        meta.name = name;
                    }

                    ui.label("Position");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut transform.position.x).speed(0.05).prefix("x: "));
                        ui.add(egui::DragValue::new(&mut transform.position.y).speed(0.05).prefix("y: "));
                        ui.add(egui::DragValue::new(&mut transform.position.z).speed(0.05).prefix("z: "));
                    });

                    ui.label("Rotation (deg)");
                    let mut rot_changed = false;
                    ui.horizontal(|ui| {
                        rot_changed |= ui
                            .add(egui::DragValue::new(&mut meta.rotation_euler_deg.x).speed(1.0).prefix("x: "))
                            .changed();
                        rot_changed |= ui
                            .add(egui::DragValue::new(&mut meta.rotation_euler_deg.y).speed(1.0).prefix("y: "))
                            .changed();
                        rot_changed |= ui
                            .add(egui::DragValue::new(&mut meta.rotation_euler_deg.z).speed(1.0).prefix("z: "))
                            .changed();
                    });
                    if rot_changed {
                        transform.rotation = euler_deg_to_quat(meta.rotation_euler_deg);
                    }

                    ui.label("Scale");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut transform.scale.x).speed(0.02).prefix("x: "));
                        ui.add(egui::DragValue::new(&mut transform.scale.y).speed(0.02).prefix("y: "));
                        ui.add(egui::DragValue::new(&mut transform.scale.z).speed(0.02).prefix("z: "));
                    });

                    if ui.checkbox(&mut is_dynamic, "Dynamic Rigid Body").changed() {
                        dynamic_changed = true;
                    }
                    if ui.checkbox(&mut is_trigger, "Is Trigger (non-solid)").changed() {
                        collider.is_trigger = is_trigger;
                        trigger_changed = true;
                        texture_path_for_trigger_toggle = meta.texture_path.clone();
                    }

                    if is_trigger {
                        ui.separator();
                        ui.label("Level Transition");
                        if let Some(transition) = &mut meta.level_transition {
                            ui.label("Target level:");
                            let mut clicked_target = None;
                            for level in &self.levels {
                                if ui
                                    .selectable_label(transition.target_level == level.name, &level.name)
                                    .clicked()
                                {
                                    clicked_target = Some(level.name.clone());
                                }
                            }
                            if let Some(target) = clicked_target {
                                transition.target_level = target;
                            }
                            ui.label("Spawn Position");
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut transition.spawn_position[0]).speed(0.1).prefix("x: "));
                                ui.add(egui::DragValue::new(&mut transition.spawn_position[1]).speed(0.1).prefix("y: "));
                                ui.add(egui::DragValue::new(&mut transition.spawn_position[2]).speed(0.1).prefix("z: "));
                            });
                            ui.add(egui::DragValue::new(&mut transition.spawn_yaw_deg).speed(1.0).prefix("Spawn yaw (deg): "));
                            if ui.button("Clear Transition").clicked() {
                                meta.level_transition = None;
                            }
                        } else if ui.button("Add Level Transition").clicked() {
                            meta.level_transition = Some(LevelTransition {
                                target_level: self.levels.first().map(|l| l.name.clone()).unwrap_or_default(),
                                spawn_position: [0.0, 1.0, 0.0],
                                spawn_yaw_deg: 0.0,
                            });
                        }

                        ui.separator();
                        ui.label("Screen Effect");
                        if let Some(effect) = &mut meta.screen_effect {
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                ui.color_edit_button_rgb(&mut effect.color);
                            });
                            ui.add(egui::Slider::new(&mut effect.strength, 0.0..=1.0).text("Strength"));
                            ui.add(egui::DragValue::new(&mut effect.fade_in_secs).speed(0.05).range(0.0..=10.0).prefix("Fade in (s): "));
                            ui.add(egui::DragValue::new(&mut effect.hold_secs).speed(0.05).range(0.0..=10.0).prefix("Hold (s): "));
                            ui.add(egui::DragValue::new(&mut effect.fade_out_secs).speed(0.05).range(0.0..=10.0).prefix("Fade out (s): "));
                            if ui.button("Preview").clicked() {
                                preview_screen_effect = Some(*effect);
                            }
                            if ui.button("Clear Screen Effect").clicked() {
                                meta.screen_effect = None;
                            }
                        } else if ui.button("Add Screen Effect").clicked() {
                            meta.screen_effect = Some(ScreenEffectSpec {
                                color: [1.0, 1.0, 1.0],
                                strength: 0.6,
                                fade_in_secs: 0.05,
                                hold_secs: 0.05,
                                fade_out_secs: 0.3,
                            });
                        }

                        ui.separator();
                        ui.label("Camera Shake");
                        if let Some(shake) = &mut meta.camera_shake {
                            ui.add(egui::Slider::new(&mut shake.intensity, 0.0..=0.5).text("Intensity"));
                            ui.add(egui::DragValue::new(&mut shake.duration_secs).speed(0.05).range(0.0..=5.0).prefix("Duration (s): "));
                            if ui.button("Preview").clicked() {
                                preview_camera_shake = Some(*shake);
                            }
                            if ui.button("Clear Camera Shake").clicked() {
                                meta.camera_shake = None;
                            }
                        } else if ui.button("Add Camera Shake").clicked() {
                            meta.camera_shake = Some(CameraShakeSpec {
                                intensity: 0.1,
                                duration_secs: 0.4,
                            });
                        }
                    }

                    ui.label("Animation");
                    egui::ComboBox::from_id_salt("animation_kind")
                        .selected_text(match anim_selected {
                            1 => "Orbit",
                            2 => "Bob",
                            _ => "None",
                        })
                        .show_ui(ui, |ui| {
                            anim_changed |= ui.selectable_value(&mut anim_selected, 0, "None").changed();
                            anim_changed |= ui.selectable_value(&mut anim_selected, 1, "Orbit").changed();
                            anim_changed |= ui.selectable_value(&mut anim_selected, 2, "Bob").changed();
                        });
                    if anim_selected != 0 {
                        ui.label("Axis");
                        ui.horizontal(|ui| {
                            anim_changed |= ui.add(egui::DragValue::new(&mut anim_axis[0]).speed(0.05).prefix("x: ")).changed();
                            anim_changed |= ui.add(egui::DragValue::new(&mut anim_axis[1]).speed(0.05).prefix("y: ")).changed();
                            anim_changed |= ui.add(egui::DragValue::new(&mut anim_axis[2]).speed(0.05).prefix("z: ")).changed();
                        });
                    }
                    if anim_selected == 1 {
                        anim_changed |= ui
                            .add(egui::Slider::new(&mut anim_speed_deg_per_sec, -360.0..=360.0).text("Speed (deg/s)"))
                            .changed();
                    } else if anim_selected == 2 {
                        anim_changed |= ui
                            .add(egui::Slider::new(&mut anim_amplitude, 0.0..=5.0).text("Amplitude"))
                            .changed();
                        anim_changed |= ui
                            .add(egui::Slider::new(&mut anim_period_secs, 0.1..=10.0).text("Period (s)"))
                            .changed();
                    }
                    // Only used if there's no existing `Animator` to preserve
                    // a base from (see the post-block logic below) — i.e.
                    // freshly turning animation on, so the object animates
                    // starting from wherever it currently sits.
                    base_position = transform.position;
                    base_rotation = transform.rotation;

                    ui.label("Script");
                    current_script_path = meta.script_path.clone();
                    ui.label(
                        meta.script_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "None".to_string()),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Attach/Change Script...").clicked() {
                            script_attach = true;
                        }
                        if meta.script_path.is_some() && ui.button("Reload").clicked() {
                            script_reload = true;
                        }
                    });
                    if meta.script_path.is_some() {
                        if ui.checkbox(&mut self.script_editor_open, "Edit Inline").changed()
                            && self.script_editor_open
                        {
                            script_inline_just_opened = true;
                        }
                        if self.script_editor_open {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.script_editor_buffer)
                                    .desired_rows(12)
                                    .code_editor(),
                            );
                            if ui.button("Save").clicked() {
                                script_save_inline = true;
                            }
                        }
                        if ui.button("Open in Script Editor").clicked() {
                            script_open_editor = true;
                        }
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Assign Texture...").clicked() {
                            assign_texture = true;
                        }
                        if ui.button("Delete").clicked() {
                            delete = true;
                        }
                    });
                }
            }
        }

        if dynamic_changed {
            if is_dynamic {
                let _ = self.world.insert_one(entity, RigidBody::default());
            } else {
                let _ = self.world.remove_one::<RigidBody>(entity);
            }
        }
        if trigger_changed {
            let new_texture = if is_trigger {
                Arc::new(solid_color_texture(gl, TRIGGER_COLOR))
            } else {
                match &texture_path_for_trigger_toggle {
                    Some(path) => match GpuTexture::load_from_file(gl, &self.asset_root.join(path), TextureFilter::Nearest) {
                        Ok(tex) => Arc::new(tex),
                        Err(err) => {
                            log::error!("failed to load texture {path:?}: {err}; using white fallback");
                            Arc::new(white_fallback_texture(gl))
                        }
                    },
                    None => Arc::new(white_fallback_texture(gl)),
                }
            };
            if let Ok(mut query) = self.world.query_one::<&mut MeshRenderer>(entity) {
                if let Some(renderer) = query.get() {
                    renderer.texture = Some(new_texture);
                }
            }
        }
        if let Some(effect) = preview_screen_effect {
            self.screen_effects.trigger(effect);
        }
        if let Some(shake) = preview_camera_shake {
            self.camera_shake.trigger(shake);
        }
        if anim_changed {
            if anim_selected == 0 {
                let _ = self.world.remove_one::<Animator>(entity);
            } else {
                // Preserve the existing base pose across param tweaks/kind
                // switches (the object shouldn't jump); only a fresh
                // None -> animated transition captures the current transform.
                let (base_position, base_rotation) = existing_base.unwrap_or((base_position, base_rotation));
                let kind = if anim_selected == 1 {
                    AnimationKind::Orbit { axis: Vec3::from(anim_axis), speed_deg_per_sec: anim_speed_deg_per_sec }
                } else {
                    AnimationKind::Bob { axis: Vec3::from(anim_axis), amplitude: anim_amplitude, period_secs: anim_period_secs }
                };
                let _ = self.world.insert_one(
                    entity,
                    Animator { kind, base_position, base_rotation, elapsed: 0.0 },
                );
            }
        }
        if script_inline_just_opened {
            if let Some(path) = &current_script_path {
                match std::fs::read_to_string(self.asset_root.join(path)) {
                    Ok(source) => self.script_editor_buffer = source,
                    Err(err) => log::error!("failed to read script for inline editing: {err}"),
                }
            }
        }
        if script_attach {
            if let Some(picked) = rfd::FileDialog::new()
                .add_filter("PS2 Script", &["pss"])
                .set_directory(&self.asset_root)
                .pick_file()
            {
                let relative = engine::level::relativize(&picked, &self.asset_root);
                self.attach_script_to_entity(entity, &relative);
            }
        }
        if script_reload {
            if let Some(path) = current_script_path.clone() {
                self.attach_script_to_entity(entity, &path);
            }
        }
        if script_save_inline {
            if let Some(path) = current_script_path.clone() {
                let full_path = self.asset_root.join(&path);
                match std::fs::write(&full_path, &self.script_editor_buffer) {
                    Ok(()) => self.attach_script_to_entity(entity, &path),
                    Err(err) => log::error!("failed to save script {full_path:?}: {err}"),
                }
            }
        }
        if script_open_editor {
            if let Some(path) = &current_script_path {
                spawn_script_editor(&self.asset_root.join(path));
            }
        }
        if assign_texture {
            self.assign_texture_to_entity(gl, entity);
        }
        if delete {
            let _ = self.world.despawn(entity);
            self.selected_entity = None;
        }
    }

    /// Sibling to `draw_selected_object_ui` for the light case — a point
    /// light has no mesh/texture/scale, just position + color/intensity/range.
    fn draw_selected_light_ui(&mut self, ui: &mut egui::Ui, entity: Entity) {
        let mut delete = false;
        if let Ok(mut query) = self.world.query_one::<(&mut Transform, &mut LevelObjectMeta, &mut Light)>(entity) {
            if let Some((transform, meta, light)) = query.get() {
                let mut name = meta.name.clone();
                if ui.text_edit_singleline(&mut name).changed() {
                    meta.name = name;
                }

                ui.label("Position");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut transform.position.x).speed(0.05).prefix("x: "));
                    ui.add(egui::DragValue::new(&mut transform.position.y).speed(0.05).prefix("y: "));
                    ui.add(egui::DragValue::new(&mut transform.position.z).speed(0.05).prefix("z: "));
                });

                let mut color = [light.color.x, light.color.y, light.color.z];
                if ui.color_edit_button_rgb(&mut color).changed() {
                    light.color = Vec3::from(color);
                }

                ui.add(egui::Slider::new(&mut light.intensity, 0.0..=5.0).text("Intensity"));

                if let LightKind::Point { range } = &mut light.kind {
                    ui.add(egui::Slider::new(range, 0.5..=30.0).text("Range"));
                }

                if ui.button("Delete").clicked() {
                    delete = true;
                }
            }
        }
        if delete {
            let _ = self.world.despawn(entity);
            self.selected_entity = None;
        }
    }

    fn draw_selected_particle_emitter_ui(&mut self, ui: &mut egui::Ui, entity: Entity) {
        let mut delete = false;
        let mut test_burst = false;
        if let Ok(mut query) = self.world.query_one::<(&mut Transform, &mut LevelObjectMeta, &mut ParticleEmitter)>(entity) {
            if let Some((transform, meta, emitter)) = query.get() {
                let mut name = meta.name.clone();
                if ui.text_edit_singleline(&mut name).changed() {
                    meta.name = name;
                }

                ui.label("Position");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut transform.position.x).speed(0.05).prefix("x: "));
                    ui.add(egui::DragValue::new(&mut transform.position.y).speed(0.05).prefix("y: "));
                    ui.add(egui::DragValue::new(&mut transform.position.z).speed(0.05).prefix("z: "));
                });

                ui.label(format!("Live particles: {}", emitter.particles.len()));

                ui.add(egui::Slider::new(&mut emitter.def.rate_per_sec, 0.0..=50.0).text("Rate/sec (0 = burst only)"));
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut emitter.def.lifetime_min).speed(0.05).prefix("lifetime min: "));
                    ui.add(egui::DragValue::new(&mut emitter.def.lifetime_max).speed(0.05).prefix("max: "));
                });
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut emitter.def.speed_min).speed(0.05).prefix("speed min: "));
                    ui.add(egui::DragValue::new(&mut emitter.def.speed_max).speed(0.05).prefix("max: "));
                });
                ui.add(egui::Slider::new(&mut emitter.def.spread_deg, 0.0..=180.0).text("Spread (deg)"));
                ui.add(egui::Slider::new(&mut emitter.def.gravity_scale, 0.0..=2.0).text("Gravity scale"));
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut emitter.def.start_size).speed(0.01).prefix("start size: "));
                    ui.add(egui::DragValue::new(&mut emitter.def.end_size).speed(0.01).prefix("end size: "));
                });
                ui.horizontal(|ui| {
                    ui.label("Start color");
                    ui.color_edit_button_rgba_unmultiplied(&mut emitter.def.start_color);
                    ui.label("End color");
                    ui.color_edit_button_rgba_unmultiplied(&mut emitter.def.end_color);
                });
                let mut max_particles = emitter.def.max_particles as f32;
                if ui.add(egui::Slider::new(&mut max_particles, 1.0..=256.0).text("Max particles")).changed() {
                    emitter.def.max_particles = max_particles.round() as u32;
                }

                ui.horizontal(|ui| {
                    if ui.button("Test Burst (12)").clicked() {
                        test_burst = true;
                    }
                    if ui.button("Delete").clicked() {
                        delete = true;
                    }
                });
            }
        }
        if test_burst {
            if let Ok(mut query) = self.world.query_one::<(&Transform, &mut ParticleEmitter)>(entity) {
                if let Some((transform, emitter)) = query.get() {
                    let position = transform.position;
                    emitter.spawn_burst(position, 12);
                }
            }
        }
        if delete {
            let _ = self.world.despawn(entity);
            self.selected_entity = None;
        }
    }

    // --- Rigs ---

    fn create_new_rig(&mut self, gl: &glow::Context) {
        let name = self.rig_new_name.trim().to_string();
        if name.is_empty() {
            log::warn!("cannot create a rig with an empty name");
            return;
        }
        if self.rigs.iter().any(|rig| rig.name == name) {
            log::warn!("a rig named '{name}' already exists");
            return;
        }
        let asset = RigAsset {
            name: name.clone(),
            parts: vec![RigPartDef {
                name: "Root".to_string(),
                parent: None,
                mesh: MeshSource::Primitive(PrimitiveKind::Cube),
                texture_path: None,
                local_position: [0.0, 0.0, 0.0],
                local_rotation_euler_deg: [0.0, 0.0, 0.0],
                scale: [0.4, 0.4, 0.4],
            }],
            clips: Vec::new(),
        };
        let path = self.rigs_dir.join(format!("{name}.ron"));
        if let Err(err) = engine::rig::save_to_file(&asset, &path) {
            log::error!("failed to save new rig '{name}': {err}");
            return;
        }
        self.rigs.push(asset);
        let index = self.rigs.len() - 1;
        self.add_rig_instance(gl, index);
    }

    /// Places a new instance of `self.rigs[asset_index]`, offset along X by
    /// however many instances already exist so repeated clicks don't stack
    /// on top of each other.
    fn add_rig_instance(&mut self, gl: &glow::Context, asset_index: usize) {
        let Some(asset) = self.rigs.get(asset_index).cloned() else { return };
        let count = self.world.query::<&RigRoot>().iter().count();
        let instance = RigInstance {
            name: format!("{}_{}", asset.name, count + 1),
            rig_path: PathBuf::from(format!("rigs/{}.ron", asset.name)),
            position: [count as f32 * 1.5, 1.0, 2.0],
            rotation_euler_deg: [0.0, 0.0, 0.0],
            playing_clip: None,
        };
        match self.spawn_rig_instance(gl, &asset, &instance) {
            Ok(root) => {
                self.selected_rig = Some(root);
                self.selected_rig_part = None;
            }
            Err(err) => log::error!("failed to add rig instance: {err}"),
        }
    }

    /// Spawns a new part (always a `Cube` primitive — resize/retexture it
    /// afterward via its own Local Position/Scale, same as any other
    /// primitive) parented to `self.new_part_parent`, falling back to the
    /// rig's root if that name doesn't match any existing part. Registers
    /// it in the root's `Rig::parts_by_name` immediately so it's selectable
    /// and animatable without needing a reload.
    fn add_rig_part(&mut self, gl: &glow::Context, root: Entity, root_name: &str) {
        let name = self.new_part_name.trim().to_string();
        if name.is_empty() {
            log::warn!("cannot add a rig part with an empty name");
            return;
        }
        let valid_parents: HashSet<String> = self
            .world
            .get::<&Rig>(root)
            .map(|rig| rig.parts_by_name.keys().cloned().collect())
            .unwrap_or_default();
        let parent_name = if valid_parents.contains(&self.new_part_parent) {
            self.new_part_parent.clone()
        } else {
            root_name.to_string()
        };

        let def = RigPartDef {
            name: name.clone(),
            parent: Some(parent_name.clone()),
            mesh: MeshSource::Primitive(PrimitiveKind::Cube),
            texture_path: None,
            local_position: [0.0, 0.3, 0.0],
            local_rotation_euler_deg: [0.0, 0.0, 0.0],
            scale: [0.2, 0.2, 0.2],
        };
        let entity = match self.spawn_rig_part(gl, &def) {
            Ok(entity) => entity,
            Err(err) => {
                log::error!("failed to add rig part '{name}': {err}");
                return;
            }
        };

        let parent_entity = self.world.get::<&Rig>(root).ok().and_then(|rig| rig.parts_by_name.get(&parent_name).copied());
        if let Ok(mut part) = self.world.get::<&mut RigPart>(entity) {
            part.parent = parent_entity;
        }
        if let Ok(mut rig) = self.world.get::<&mut Rig>(root) {
            rig.parts_by_name.insert(name, entity);
        }
        self.selected_rig_part = Some(entity);
    }

    /// Upserts a keyframe at the current scrub time for the selected part,
    /// into the selected clip's track for that part (creating the track if
    /// this is its first keyframe) — the entire keyframe-authoring workflow
    /// boils down to: pause, pose a part by hand, click this.
    fn set_keyframe_for_selected_part(&mut self, root: Entity) {
        let Some(part_entity) = self.selected_rig_part else { return };
        let Some(part_name) = self.world.get::<&Rig>(root).ok().and_then(|rig| {
            rig.parts_by_name.iter().find(|(_, &e)| e == part_entity).map(|(name, _)| name.clone())
        }) else {
            log::warn!("selected part is not registered on this rig");
            return;
        };
        let Some(local_rotation_euler_deg) = self.world.get::<&RigPart>(part_entity).ok().map(|part| part.local_rotation_euler_deg) else {
            return;
        };

        let Ok(mut animator) = self.world.get::<&mut RigAnimator>(root) else { return };
        let Some(clip_index) = animator.current_clip else {
            log::warn!("select or create a clip before setting a keyframe");
            return;
        };
        let time = animator.time;
        let Some(clip) = animator.clips.get_mut(clip_index) else { return };

        let keyframe = Keyframe { time, rotation_euler_deg: local_rotation_euler_deg.to_array() };
        match clip.tracks.iter_mut().find(|track| track.joint_name == part_name) {
            Some(track) => match track.keyframes.iter_mut().find(|k| (k.time - time).abs() < 1e-3) {
                Some(existing) => *existing = keyframe,
                None => {
                    track.keyframes.push(keyframe);
                    track.keyframes.sort_by(|a, b| a.time.total_cmp(&b.time));
                }
            },
            None => clip.tracks.push(JointTrack { joint_name: part_name.clone(), keyframes: vec![keyframe] }),
        }
        log::info!("set keyframe for '{part_name}' at t={time:.2}");
    }

    fn draw_selected_rig_ui(&mut self, ui: &mut egui::Ui, gl: &glow::Context, root: Entity) {
        // Distinct from `rig_name` (the *instance*'s display name, e.g.
        // "Humanoid"): `asset_name` is the underlying `RigAsset`'s own name
        // (e.g. "humanoid", derived from `RigRoot::rig_path`'s file stem —
        // the same convention `find_rig_asset` uses), and `root_part_name`
        // is the root *part*'s name within the rig (e.g. "Torso"). Using
        // the instance name for either by mistake previously corrupted the
        // saved asset file's `name` field (breaking `find_rig_asset` for
        // any instance renamed differently from its asset) and could parent
        // a new part to a nonexistent "part" named after the instance.
        let mut asset_name = String::new();
        let mut root_part_name = String::new();
        let mut add_part = false;
        let mut new_clip = false;
        let mut save_rig = false;
        let mut delete_rig = false;

        {
            if let Ok(mut query) = self.world.query_one::<(&mut RigPart, &mut RigAnimator, &Rig, &RigRoot)>(root) {
                if let Some((root_part, animator, rig, rig_root)) = query.get() {
                    let rig_name = rig_root.name.clone();
                    asset_name = rig_root
                        .rig_path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .map(str::to_string)
                        .unwrap_or_else(|| rig_name.clone());
                    root_part_name = rig
                        .parts_by_name
                        .iter()
                        .find(|(_, &entity)| entity == root)
                        .map(|(name, _)| name.clone())
                        .unwrap_or_else(|| rig_name.clone());
                    let mut part_names: Vec<String> = rig.parts_by_name.keys().cloned().collect();
                    part_names.sort();

                    ui.label(format!("Rig: {rig_name}"));
                    ui.label("Root Local Position");
                    ui.horizontal(|ui| {
                        let mut pos = root_part.local_position;
                        let mut changed = false;
                        changed |= ui.add(egui::DragValue::new(&mut pos.x).speed(0.05).prefix("x: ")).changed();
                        changed |= ui.add(egui::DragValue::new(&mut pos.y).speed(0.05).prefix("y: ")).changed();
                        changed |= ui.add(egui::DragValue::new(&mut pos.z).speed(0.05).prefix("z: ")).changed();
                        if changed {
                            root_part.local_position = pos;
                        }
                    });
                    ui.label("Root Local Rotation (deg)");
                    ui.horizontal(|ui| {
                        let mut rot = root_part.local_rotation_euler_deg;
                        let mut changed = false;
                        changed |= ui.add(egui::DragValue::new(&mut rot.x).speed(1.0).prefix("x: ")).changed();
                        changed |= ui.add(egui::DragValue::new(&mut rot.y).speed(1.0).prefix("y: ")).changed();
                        changed |= ui.add(egui::DragValue::new(&mut rot.z).speed(1.0).prefix("z: ")).changed();
                        if changed {
                            root_part.set_local_rotation_euler_deg(rot);
                        }
                    });

                    ui.separator();
                    ui.heading("Clips");
                    let mut clicked_clip = None;
                    for (i, clip) in animator.clips.iter().enumerate() {
                        if ui.selectable_label(animator.current_clip == Some(i), &clip.name).clicked() {
                            clicked_clip = Some(i);
                        }
                    }
                    if let Some(i) = clicked_clip {
                        animator.play_clip(i);
                        animator.playing = true;
                    }
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.new_clip_name);
                        if ui.button("New Clip").clicked() {
                            new_clip = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.new_clip_duration).speed(0.1).prefix("duration: "));
                        ui.checkbox(&mut self.new_clip_looping, "looping");
                    });

                    if let Some(i) = animator.current_clip {
                        let playing = animator.playing;
                        if let Some(clip) = animator.clips.get_mut(i) {
                            ui.horizontal(|ui| {
                                if ui.button(if playing { "Pause" } else { "Play" }).clicked() {
                                    animator.playing = !playing;
                                }
                                ui.checkbox(&mut clip.looping, "Loop");
                            });
                            ui.add(egui::Slider::new(&mut animator.speed, 0.0..=3.0).text("Speed"));
                            let mut time = animator.time;
                            let max_time = clip.duration.max(0.01);
                            if ui.add(egui::Slider::new(&mut time, 0.0..=max_time).text("Time (scrub)")).changed() {
                                animator.time = time;
                                animator.playing = false;
                            }
                        }
                    }

                    ui.separator();
                    ui.heading("Parts");
                    let mut clicked_part = None;
                    for name in &part_names {
                        if let Some(&entity) = rig.parts_by_name.get(name) {
                            if ui.selectable_label(self.selected_rig_part == Some(entity), name).clicked() {
                                clicked_part = Some(entity);
                            }
                        }
                    }
                    if let Some(entity) = clicked_part {
                        self.selected_rig_part = Some(entity);
                    }

                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.new_part_name);
                        egui::ComboBox::from_id_salt("new_part_parent")
                            .selected_text(if self.new_part_parent.is_empty() { rig_name.as_str() } else { self.new_part_parent.as_str() })
                            .show_ui(ui, |ui| {
                                for name in &part_names {
                                    ui.selectable_value(&mut self.new_part_parent, name.clone(), name);
                                }
                            });
                        if ui.button("Add Part").clicked() {
                            add_part = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Save Rig").clicked() {
                            save_rig = true;
                        }
                        if ui.button("Delete Rig Instance").clicked() {
                            delete_rig = true;
                        }
                    });
                }
            }
        }

        if let Some(part_entity) = self.selected_rig_part {
            if self.world.contains(part_entity) {
                ui.separator();
                ui.label("Selected Part");
                let mut set_keyframe = false;
                if let Ok(mut query) = self.world.query_one::<&mut RigPart>(part_entity) {
                    if let Some(part) = query.get() {
                        ui.label("Local Position");
                        ui.horizontal(|ui| {
                            let mut pos = part.local_position;
                            let mut changed = false;
                            changed |= ui.add(egui::DragValue::new(&mut pos.x).speed(0.05).prefix("x: ")).changed();
                            changed |= ui.add(egui::DragValue::new(&mut pos.y).speed(0.05).prefix("y: ")).changed();
                            changed |= ui.add(egui::DragValue::new(&mut pos.z).speed(0.05).prefix("z: ")).changed();
                            if changed {
                                part.local_position = pos;
                            }
                        });
                        ui.label("Local Rotation (deg)");
                        ui.horizontal(|ui| {
                            let mut rot = part.local_rotation_euler_deg;
                            let mut changed = false;
                            changed |= ui.add(egui::DragValue::new(&mut rot.x).speed(1.0).prefix("x: ")).changed();
                            changed |= ui.add(egui::DragValue::new(&mut rot.y).speed(1.0).prefix("y: ")).changed();
                            changed |= ui.add(egui::DragValue::new(&mut rot.z).speed(1.0).prefix("z: ")).changed();
                            if changed {
                                part.set_local_rotation_euler_deg(rot);
                            }
                        });
                    }
                }
                if ui.button("Set Keyframe Here").clicked() {
                    set_keyframe = true;
                }
                if set_keyframe {
                    self.set_keyframe_for_selected_part(root);
                }
            } else {
                self.selected_rig_part = None;
            }
        }

        if new_clip {
            let name = self.new_clip_name.trim().to_string();
            if name.is_empty() {
                log::warn!("cannot create a clip with an empty name");
            } else if let Ok(mut animator) = self.world.get::<&mut RigAnimator>(root) {
                animator.clips.push(RigClip {
                    name,
                    duration: self.new_clip_duration.max(0.1),
                    looping: self.new_clip_looping,
                    tracks: Vec::new(),
                });
                animator.current_clip = Some(animator.clips.len() - 1);
                animator.time = 0.0;
            }
        }
        if add_part {
            self.add_rig_part(gl, root, &root_part_name);
        }
        if save_rig {
            if let Some(asset) = self.build_rig_asset_from_ecs(root, asset_name) {
                let path = self.rigs_dir.join(format!("{}.ron", asset.name));
                match engine::rig::save_to_file(&asset, &path) {
                    Ok(()) => {
                        log::info!("saved rig '{}' to {path:?}", asset.name);
                        match self.rigs.iter_mut().find(|rig| rig.name == asset.name) {
                            Some(existing) => *existing = asset,
                            None => self.rigs.push(asset),
                        }
                    }
                    Err(err) => log::error!("failed to save rig: {err}"),
                }
            }
        }
        if delete_rig {
            let part_entities: Vec<Entity> = self
                .world
                .get::<&Rig>(root)
                .map(|rig| rig.parts_by_name.values().copied().collect())
                .unwrap_or_default();
            for entity in part_entities {
                let _ = self.world.despawn(entity);
            }
            let _ = self.world.despawn(root);
            self.selected_rig = None;
            self.selected_rig_part = None;
        }
    }

    // --- Classes ---

    fn create_new_class(&mut self) {
        let name = self.class_new_name.trim().to_string();
        if name.is_empty() {
            log::warn!("cannot create a class with an empty name");
            return;
        }
        if self.classes.iter().any(|class| class.name == name) {
            log::warn!("a class named '{name}' already exists");
            return;
        }
        let class = ObjectClass {
            name: name.clone(),
            mesh: MeshSource::Primitive(PrimitiveKind::Cube),
            texture_path: None,
            scale: [1.0, 1.0, 1.0],
            is_dynamic: false,
            is_trigger: false,
            animation: None,
            script: None,
        };
        let path = self.classes_dir.join(format!("{name}.ron"));
        if let Err(err) = engine::class::save_to_file(&class, &path) {
            log::error!("failed to save new class '{name}': {err}");
            return;
        }
        self.classes.push(class);
        self.selected_class = Some(self.classes.len() - 1);
    }

    /// Places a new `LevelObject` referencing `self.classes[class_index]` —
    /// its look/behavior come entirely from the class; only name/position
    /// are this instance's own.
    fn spawn_object_from_class(&mut self, gl: &glow::Context, class_index: usize) {
        let Some(class) = self.classes.get(class_index).cloned() else { return };
        let count = self.world.query::<&ClassMember>().iter().count();
        let obj = LevelObject {
            name: format!("{}_{}", class.name, count + 1),
            mesh: class.mesh.clone(),
            texture_path: class.texture_path.clone(),
            position: [count as f32 * 1.5 - 1.5, 1.0, 5.5],
            rotation_euler_deg: [0.0, 0.0, 0.0],
            scale: class.scale,
            is_dynamic: class.is_dynamic,
            is_trigger: class.is_trigger,
            animation: class.animation,
            script: class.script.clone(),
            class: Some(PathBuf::from(format!("classes/{}.ron", class.name))),
            level_transition: None,
        screen_effect: None,
        camera_shake: None,
        };
        match self.spawn_level_object(gl, &obj) {
            Ok(entity) => self.selected_entity = Some(entity),
            Err(err) => log::error!("failed to spawn instance of class '{}': {err}", class.name),
        }
    }

    fn assign_texture_to_class(&mut self, index: usize) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg"])
            .set_directory(&self.asset_root)
            .pick_file()
        else {
            return;
        };
        let relative_path = engine::level::relativize(&path, &self.asset_root);
        if let Some(class) = self.classes.get_mut(index) {
            class.texture_path = Some(relative_path);
        }
    }

    /// Re-resolves mesh/texture/scale/trigger-flag/dynamic-flag/animation/
    /// script for every live `ClassMember` entity referencing this class —
    /// the "edit the class, every instance updates" behavior, run only when
    /// explicitly asked for (not continuous live-binding).
    fn apply_class_to_instances(&mut self, gl: &glow::Context, index: usize) {
        let Some(class) = self.classes.get(index).cloned() else { return };
        // Asset-root-relative, matching the convention `spawn_object_from_class`
        // and saved level files store `ClassMember::class_path`/`LevelObject::class` in.
        let class_path = PathBuf::from(format!("classes/{}.ron", class.name));

        let entities: Vec<Entity> = self
            .world
            .query::<&ClassMember>()
            .iter()
            .filter(|(_, member)| member.class_path == class_path)
            .map(|(entity, _)| entity)
            .collect();

        for entity in entities {
            let mesh = match self.resolve_mesh(gl, &class.mesh) {
                Ok(mesh) => mesh,
                Err(err) => {
                    log::error!("failed to apply class '{}' to an instance: {err}", class.name);
                    continue;
                }
            };
            let texture = if class.is_trigger {
                Arc::new(solid_color_texture(gl, TRIGGER_COLOR))
            } else {
                self.resolve_texture(gl, class.texture_path.as_deref())
            };
            let half_extents = collider_half_extents(&class.mesh, class.scale);

            if let Ok(mut query) = self
                .world
                .query_one::<(&mut Transform, &mut MeshRenderer, &mut LevelObjectMeta, &mut Collider)>(entity)
            {
                if let Some((transform, renderer, meta, collider)) = query.get() {
                    transform.scale = Vec3::from(class.scale);
                    renderer.mesh = mesh;
                    renderer.texture = Some(texture);
                    meta.mesh_source = class.mesh.clone();
                    meta.texture_path = class.texture_path.clone();
                    collider.shape = ColliderShape::Aabb { half_extents };
                    collider.is_trigger = class.is_trigger;
                }
            }

            if class.is_dynamic {
                let _ = self.world.insert_one(entity, RigidBody::default());
            } else {
                let _ = self.world.remove_one::<RigidBody>(entity);
            }

            if let Some(spec) = class.animation {
                let kind = match spec {
                    AnimationSpec::Orbit { axis, speed_deg_per_sec } => {
                        AnimationKind::Orbit { axis: Vec3::from(axis), speed_deg_per_sec }
                    }
                    AnimationSpec::Bob { axis, amplitude, period_secs } => {
                        AnimationKind::Bob { axis: Vec3::from(axis), amplitude, period_secs }
                    }
                };
                // Preserve the entity's current position/rotation as the
                // animation base — re-applying a class shouldn't teleport
                // an already-placed instance back to the class's origin.
                let base = self.world.get::<&Transform>(entity).ok().map(|t| (t.position, t.rotation));
                if let Some((base_position, base_rotation)) = base {
                    let _ = self.world.insert_one(entity, Animator { kind, base_position, base_rotation, elapsed: 0.0 });
                }
            } else {
                let _ = self.world.remove_one::<Animator>(entity);
            }

            if let Some(script_path) = &class.script {
                self.attach_script_to_entity(entity, script_path);
            } else {
                let _ = self.world.remove_one::<BehaviorSlot>(entity);
            }
        }

        log::info!("applied class '{}' to its instances", class.name);
    }

    fn draw_class_editor_ui(&mut self, ui: &mut egui::Ui, gl: &glow::Context, index: usize) {
        let mut assign_texture = false;
        let mut apply_to_instances = false;
        let mut save_class = false;

        {
            let Some(class) = self.classes.get_mut(index) else { return };
            ui.label(format!("Class: {}", class.name));

            ui.label("Mesh");
            ui.horizontal(|ui| {
                if ui.selectable_label(matches!(class.mesh, MeshSource::Primitive(PrimitiveKind::Cube)), "Cube").clicked() {
                    class.mesh = MeshSource::Primitive(PrimitiveKind::Cube);
                }
                if ui.selectable_label(matches!(class.mesh, MeshSource::Primitive(PrimitiveKind::Plane)), "Plane").clicked() {
                    class.mesh = MeshSource::Primitive(PrimitiveKind::Plane);
                }
            });
            ui.label(format!(
                "Texture: {}",
                class.texture_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "None".to_string())
            ));
            if ui.button("Assign Texture...").clicked() {
                assign_texture = true;
            }

            ui.label("Scale");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut class.scale[0]).speed(0.02).prefix("x: "));
                ui.add(egui::DragValue::new(&mut class.scale[1]).speed(0.02).prefix("y: "));
                ui.add(egui::DragValue::new(&mut class.scale[2]).speed(0.02).prefix("z: "));
            });

            ui.checkbox(&mut class.is_dynamic, "Dynamic Rigid Body");
            ui.checkbox(&mut class.is_trigger, "Is Trigger (non-solid)");

            let mut anim_selected: usize = match class.animation {
                None => 0,
                Some(AnimationSpec::Orbit { .. }) => 1,
                Some(AnimationSpec::Bob { .. }) => 2,
            };
            let mut anim_axis: [f32; 3] = match class.animation {
                Some(AnimationSpec::Orbit { axis, .. }) => axis,
                Some(AnimationSpec::Bob { axis, .. }) => axis,
                None => [0.0, 1.0, 0.0],
            };
            let mut anim_speed: f32 = match class.animation {
                Some(AnimationSpec::Orbit { speed_deg_per_sec, .. }) => speed_deg_per_sec,
                _ => 90.0,
            };
            let mut anim_amplitude: f32 = match class.animation {
                Some(AnimationSpec::Bob { amplitude, .. }) => amplitude,
                _ => 0.5,
            };
            let mut anim_period: f32 = match class.animation {
                Some(AnimationSpec::Bob { period_secs, .. }) => period_secs,
                _ => 2.0,
            };
            ui.label("Animation");
            egui::ComboBox::from_id_salt("class_animation_kind")
                .selected_text(match anim_selected {
                    1 => "Orbit",
                    2 => "Bob",
                    _ => "None",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut anim_selected, 0, "None");
                    ui.selectable_value(&mut anim_selected, 1, "Orbit");
                    ui.selectable_value(&mut anim_selected, 2, "Bob");
                });
            if anim_selected != 0 {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut anim_axis[0]).speed(0.05).prefix("x: "));
                    ui.add(egui::DragValue::new(&mut anim_axis[1]).speed(0.05).prefix("y: "));
                    ui.add(egui::DragValue::new(&mut anim_axis[2]).speed(0.05).prefix("z: "));
                });
            }
            if anim_selected == 1 {
                ui.add(egui::Slider::new(&mut anim_speed, -360.0..=360.0).text("Speed (deg/s)"));
            } else if anim_selected == 2 {
                ui.add(egui::Slider::new(&mut anim_amplitude, 0.0..=5.0).text("Amplitude"));
                ui.add(egui::Slider::new(&mut anim_period, 0.1..=10.0).text("Period (s)"));
            }
            class.animation = match anim_selected {
                1 => Some(AnimationSpec::Orbit { axis: anim_axis, speed_deg_per_sec: anim_speed }),
                2 => Some(AnimationSpec::Bob { axis: anim_axis, amplitude: anim_amplitude, period_secs: anim_period }),
                _ => None,
            };

            ui.label(format!(
                "Script: {}",
                class.script.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "None".to_string())
            ));
            if ui.button("Attach Script...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PS2 Script", &["pss"])
                    .set_directory(&self.asset_root)
                    .pick_file()
                {
                    class.script = Some(engine::level::relativize(&path, &self.asset_root));
                }
            }
            if class.script.is_some() && ui.button("Clear Script").clicked() {
                class.script = None;
            }

            ui.horizontal(|ui| {
                if ui.button("Save Class").clicked() {
                    save_class = true;
                }
                if ui.button("Apply to All Instances").clicked() {
                    apply_to_instances = true;
                }
            });
        }

        if assign_texture {
            self.assign_texture_to_class(index);
        }
        if save_class {
            if let Some(class) = self.classes.get(index) {
                let path = self.classes_dir.join(format!("{}.ron", class.name));
                match engine::class::save_to_file(class, &path) {
                    Ok(()) => log::info!("saved class '{}' to {path:?}", class.name),
                    Err(err) => log::error!("failed to save class '{}': {err}", class.name),
                }
            }
        }
        if apply_to_instances {
            self.apply_class_to_instances(gl, index);
        }
    }

    /// The selected-object panel for an entity spawned from a class: only
    /// Name/Position/Rotation are directly editable (the rest is inherited
    /// from the class) plus a way to jump to the class or detach from it.
    fn draw_classed_object_ui(&mut self, ui: &mut egui::Ui, entity: Entity, class_path: &Path) {
        let mut delete = false;
        let mut unlink = false;
        let mut edit_class = false;

        {
            if let Ok(mut query) = self.world.query_one::<(&mut Transform, &mut LevelObjectMeta)>(entity) {
                if let Some((transform, meta)) = query.get() {
                    let mut name = meta.name.clone();
                    if ui.text_edit_singleline(&mut name).changed() {
                        meta.name = name;
                    }

                    ui.label("Position");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut transform.position.x).speed(0.05).prefix("x: "));
                        ui.add(egui::DragValue::new(&mut transform.position.y).speed(0.05).prefix("y: "));
                        ui.add(egui::DragValue::new(&mut transform.position.z).speed(0.05).prefix("z: "));
                    });

                    ui.label("Rotation (deg)");
                    let mut rot_changed = false;
                    ui.horizontal(|ui| {
                        rot_changed |= ui.add(egui::DragValue::new(&mut meta.rotation_euler_deg.x).speed(1.0).prefix("x: ")).changed();
                        rot_changed |= ui.add(egui::DragValue::new(&mut meta.rotation_euler_deg.y).speed(1.0).prefix("y: ")).changed();
                        rot_changed |= ui.add(egui::DragValue::new(&mut meta.rotation_euler_deg.z).speed(1.0).prefix("z: ")).changed();
                    });
                    if rot_changed {
                        transform.rotation = euler_deg_to_quat(meta.rotation_euler_deg);
                    }
                }
            }
        }

        let class_name = self.find_class(class_path).map(|c| c.name.clone()).unwrap_or_else(|| "?".to_string());
        ui.label(format!("Class: {class_name}"));
        ui.horizontal(|ui| {
            if ui.button("Edit Class").clicked() {
                edit_class = true;
            }
            if ui.button("Unlink from Class").clicked() {
                unlink = true;
            }
            if ui.button("Delete").clicked() {
                delete = true;
            }
        });

        if edit_class {
            self.selected_class = self.classes.iter().position(|c| c.name == class_name);
        }
        if unlink {
            let _ = self.world.remove_one::<ClassMember>(entity);
        }
        if delete {
            let _ = self.world.despawn(entity);
            self.selected_entity = None;
        }
    }

    // --- Shader profile editor ---

    fn draw_shader_editor_ui(&mut self, ui: &mut egui::Ui, gl: &glow::Context, drawable_size: (u32, u32)) {
        ui.heading("Shader Profiles");
        if let Some(cycler) = self.profiles.as_mut() {
            let current_index = cycler.current_index();
            let mut clicked_index = None;
            for (i, profile) in cycler.all().iter().enumerate() {
                if ui.selectable_label(i == current_index, &profile.name).clicked() {
                    clicked_index = Some(i);
                }
            }
            if let Some(i) = clicked_index {
                let profile = cycler.select(i).clone();
                if let Err(err) = self.apply_profile(gl, drawable_size, &profile) {
                    log::error!("failed to apply profile '{}': {err}", profile.name);
                }
            }
        }

        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                self.save_current_profile();
            }
            ui.text_edit_singleline(&mut self.save_as_name);
            if ui.button("Save As New").clicked() {
                self.save_as_new_profile();
            }
        });

        ui.separator();
        ui.heading("Render Params");
        render_params_editor(ui, &mut self.render_params);
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.set_resolution_scale(self.render_params.resolution_scale);
        }

        ui.separator();
        ui.heading("Physics");
        ui.add(
            egui::Slider::new(&mut self.physics_params.gravity, 0.0..=30.0).text("Gravity"),
        );
        ui.add(
            egui::Slider::new(&mut self.physics_params.linear_damping, 0.0..=2.0)
                .text("Linear damping"),
        );
        ui.small("Saved/loaded with the level (F2 panel), not the shader profile.");

        ui.separator();
        ui.heading("Camera");
        ui.checkbox(&mut self.head_bob.enabled, "Head bob");
        ui.add(
            egui::Slider::new(&mut self.head_bob.amplitude, 0.0..=0.15).text("Bob amplitude"),
        );
        ui.add(
            egui::Slider::new(&mut self.head_bob.sway_amplitude, 0.0..=0.1).text("Sway amplitude"),
        );
        ui.add(
            egui::Slider::new(&mut self.head_bob.cycles_per_unit, 0.1..=2.0)
                .text("Bob cycles per unit walked"),
        );

        ui.add_space(6.0);
        ui.checkbox(&mut self.landing_dip.enabled, "Landing dip");
        ui.add(
            egui::Slider::new(&mut self.landing_dip.fall_speed_to_dip, 0.0..=0.1)
                .text("Dip per fall speed"),
        );
        ui.add(egui::Slider::new(&mut self.landing_dip.max_dip, 0.0..=0.6).text("Max dip"));

        ui.add_space(6.0);
        ui.checkbox(&mut self.strafe_tilt.enabled, "Strafe tilt");
        ui.add(
            egui::Slider::new(&mut self.strafe_tilt.max_roll_deg, 0.0..=15.0)
                .text("Max roll (deg)"),
        );

        ui.add_space(6.0);
        ui.checkbox(&mut self.speed_fov.enabled, "Sprint FOV kick");
        ui.add(
            egui::Slider::new(&mut self.speed_fov.max_kick_deg, 0.0..=20.0).text("Max FOV kick (deg)"),
        );
        ui.add(
            egui::Slider::new(&mut self.speed_fov.speed_for_max_kick, 0.5..=20.0)
                .text("Speed for max kick"),
        );

        ui.add_space(6.0);
        if ui.button("Test camera shake").clicked() {
            self.camera_shake.trigger(CameraShakeSpec { intensity: 0.15, duration_secs: 0.5 });
        }
        ui.small("Play-session settings, not saved with the level or profile.");

        ui.separator();
        ui.small(
            "F1 toggle UI \u{b7} F2 level editor \u{b7} F3 play/stop \u{b7} Tab cycle profile \u{b7} F5 save \u{b7} F9/F10 checkpoint save/load \u{b7} Esc quit",
        );
    }
}

impl Game for Sandbox {
    fn init(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        unsafe {
            ctx.gl().enable(glow::DEPTH_TEST);
        }

        std::fs::create_dir_all(&self.profiles_dir)?;
        let mut profiles = engine::profile::load_dir(&self.profiles_dir)?;
        if profiles.is_empty() {
            log::info!(
                "no shader profiles found in {:?}; writing default demo profiles",
                self.profiles_dir
            );
            profiles = default_demo_profiles();
            for profile in &profiles {
                let path = self.profiles_dir.join(format!("{}.ron", profile.name));
                engine::profile::save_to_file(profile, &path)?;
            }
        }

        let cycler = ProfileCycler::new(profiles)?;
        let first_profile = cycler.current().clone();
        {
            let gl = ctx.gl();
            let drawable_size = ctx.drawable_size();
            self.apply_profile(gl, drawable_size, &first_profile)?;
        }
        self.profiles = Some(cycler);

        self.ui = Some(EguiState::new(ctx.gl_arc())?);

        std::fs::create_dir_all(&self.rigs_dir)?;
        let mut rigs = engine::rig::load_dir(&self.rigs_dir)?;
        if rigs.is_empty() {
            log::info!("no rigs found in {:?}; writing default demo rig", self.rigs_dir);
            let rig = default_rig_asset();
            let path = self.rigs_dir.join(format!("{}.ron", rig.name));
            engine::rig::save_to_file(&rig, &path)?;
            rigs = vec![rig];
        }
        self.rigs = rigs;

        std::fs::create_dir_all(&self.classes_dir)?;
        let mut classes = engine::class::load_dir(&self.classes_dir)?;
        for kind in MushroomKind::all() {
            if !classes.iter().any(|c| c.name == kind.class_name()) {
                log::info!("no '{}' class found in {:?}; writing it", kind.class_name(), self.classes_dir);
                let class = default_mushroom_class(kind);
                let path = self.classes_dir.join(format!("{}.ron", class.name));
                engine::class::save_to_file(&class, &path)?;
                classes.push(class);
            }
        }
        self.classes = classes;

        // No bootstrap content here (unlike profiles/rigs/classes/levels) —
        // there's nothing sensible to pre-populate a checkpoint with before
        // the player's ever played, so this just ensures F9 has somewhere
        // to write.
        std::fs::create_dir_all(&self.saves_dir)?;

        // Placeholder audio the demo level references (a push-interact blip
        // and a looping ambient track) — hand-generated rather than shipped
        // as binary assets, same bootstrap-if-missing convention as
        // profiles/rigs/classes/levels above.
        let demo_sfx_path = self.asset_root.join("sfx/demo_blip.wav");
        if !demo_sfx_path.exists() {
            log::info!("no demo sfx found at {demo_sfx_path:?}; generating a placeholder blip");
            if let Err(err) = write_wav(&demo_sfx_path, &generate_blip_samples(44100), 44100) {
                log::error!("failed to write demo sfx: {err}");
            }
        }
        let demo_music_path = self.asset_root.join("music/demo_ambient.wav");
        if !demo_music_path.exists() {
            log::info!("no demo music found at {demo_music_path:?}; generating a placeholder ambient loop");
            if let Err(err) = write_wav(&demo_music_path, &generate_ambient_samples(44100), 44100) {
                log::error!("failed to write demo music: {err}");
            }
        }

        std::fs::create_dir_all(&self.levels_dir)?;
        let mut levels = engine::level::load_dir(&self.levels_dir)?;
        if levels.is_empty() {
            log::info!(
                "no levels found in {:?}; writing the dungeon's entrance level",
                self.levels_dir
            );
            let level = entrance_level();
            let path = self.levels_dir.join(format!("{}.ron", level.name));
            engine::level::save_to_file(&level, &path)?;
            levels = vec![level];
        }
        if !levels.iter().any(|l| l.name == "tunnels") {
            log::info!("no 'tunnels' level found in {:?}; writing it", self.levels_dir);
            let level = tunnels_level();
            let path = self.levels_dir.join(format!("{}.ron", level.name));
            engine::level::save_to_file(&level, &path)?;
            levels.push(level);
        }
        if !levels.iter().any(|l| l.name == "grotto") {
            log::info!("no 'grotto' level found in {:?}; writing it", self.levels_dir);
            let level = grotto_level();
            let path = self.levels_dir.join(format!("{}.ron", level.name));
            engine::level::save_to_file(&level, &path)?;
            levels.push(level);
        }
        let first_level = levels[0].clone();
        {
            let gl = ctx.gl();
            self.apply_level(gl, &first_level)?;
        }
        self.levels = levels;

        // A shipped/release build has no editor to land in — go straight
        // to Play so the very first frame a player sees is the game.
        if !editor_available() {
            self.enter_play_mode(ctx);
        }

        Ok(())
    }

    fn handle_event(&mut self, ctx: &mut Context, event: &Event) {
        if let Some(ui) = self.ui.as_mut() {
            ui.handle_event(event);
        }
        let ui_wants_pointer = self
            .ui
            .as_ref()
            .map(|ui| ui.wants_pointer_input())
            .unwrap_or(false);
        let ui_wants_keyboard = self
            .ui
            .as_ref()
            .map(|ui| ui.wants_keyboard_input())
            .unwrap_or(false);

        let editing = editor_available() && self.mode == EditorMode::Edit;

        match *event {
            Event::MouseMotion { xrel, yrel, .. }
                if editing && !ui_wants_pointer && ctx.input.is_button_down(MouseButton::Left) =>
            {
                self.camera.orbit(-xrel as f32 * 0.01, -yrel as f32 * 0.01);
            }
            Event::MouseWheel { y, .. } if editing && !ui_wants_pointer => {
                self.camera.zoom(-y as f32 * 0.3);
            }
            Event::KeyDown {
                keycode: Some(Keycode::Escape),
                repeat: false,
                ..
            } => match self.mode {
                EditorMode::Play if !self.paused => self.pause(ctx),
                EditorMode::Play => self.resume(ctx),
                EditorMode::Edit => ctx.should_quit = true,
            },
            Event::KeyDown {
                keycode: Some(Keycode::F1),
                repeat: false,
                ..
            } if editing => {
                self.ui_visible = !self.ui_visible;
            }
            Event::KeyDown {
                keycode: Some(Keycode::F2),
                repeat: false,
                ..
            } if editing => {
                self.level_ui_visible = !self.level_ui_visible;
            }
            Event::KeyDown {
                keycode: Some(Keycode::F3),
                repeat: false,
                ..
            } if editor_available() => match self.mode {
                EditorMode::Edit => self.enter_play_mode(ctx),
                EditorMode::Play => self.exit_play_mode(ctx),
            },
            Event::KeyDown {
                keycode: Some(Keycode::F9),
                repeat: false,
                ..
            } if self.mode == EditorMode::Play && !self.paused => {
                self.save_checkpoint();
            }
            Event::KeyDown {
                keycode: Some(Keycode::F10),
                repeat: false,
                ..
            } => {
                self.load_checkpoint(ctx);
            }
            Event::KeyDown {
                keycode: Some(Keycode::Tab),
                repeat: false,
                ..
            } if editing && !ui_wants_keyboard => {
                if let Some(cycler) = self.profiles.as_mut() {
                    let profile = cycler.next().clone();
                    let gl = ctx.gl();
                    let drawable_size = ctx.drawable_size();
                    if let Err(err) = self.apply_profile(gl, drawable_size, &profile) {
                        log::error!("failed to apply profile '{}': {err}", profile.name);
                    }
                }
            }
            Event::KeyDown {
                keycode: Some(Keycode::F5),
                repeat: false,
                ..
            } if editing && !ui_wants_keyboard => {
                self.save_current_profile();
            }
            Event::KeyDown {
                keycode: Some(Keycode::E),
                repeat: false,
                ..
            } if self.mode == EditorMode::Play && !self.paused => {
                self.interact();
            }
            Event::ControllerButtonDown { button: ControllerButton::X, .. }
                if self.mode == EditorMode::Play && !self.paused =>
            {
                self.interact();
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) -> anyhow::Result<()> {
        self.elapsed_time = ctx.time.elapsed;

        // Runs in both Edit and Play mode — a live preview while editing
        // costs nothing extra and is a nice default. The one exception is a
        // genuinely *paused* Play session: unlike Edit mode, that's meant to
        // freeze everything, not just player input, so particles/rig clips/
        // HUD countdowns actually stop rather than keep animating behind
        // the pause menu.
        let paused_in_play = self.mode == EditorMode::Play && self.paused;
        if !paused_in_play {
            engine::animation::step(&mut self.world, dt);
            // Sample playing rig clips into each part's local rotation, then
            // resolve every rig part's world `Transform` from its parent
            // chain — same "always on, live preview" philosophy, and order
            // matters: a clip sampled this frame should be reflected in this
            // frame's rendered pose, not lag one frame behind.
            engine::rig::step_rig_animation(&mut self.world, dt);
            engine::rig::update_world_transforms(&mut self.world);
            // Transient one-shot burst emitters (see `spawn_burst_at`)
            // despawn themselves once every particle they made has aged out
            // — they aren't level data, so nothing else would ever clean
            // them up.
            for entity in engine::particles::step(&mut self.world, dt) {
                let _ = self.world.despawn(entity);
            }
            self.hud.tick(dt);
            self.screen_effects.tick(dt);
        }

        if self.mode == EditorMode::Edit {
            let ui_wants_keyboard = self.ui.as_ref().map(|ui| ui.wants_keyboard_input()).unwrap_or(false);
            if !ui_wants_keyboard {
                self.update_editor_camera_input(ctx, dt);
            }
        }

        if self.mode == EditorMode::Play && !self.paused {
            self.update_player_input(ctx, dt);

            // Collected up front (mirrors the physics-collider pattern
            // above) so dispatching `on_update` can freely use `&mut self`
            // per entity without holding this query's borrow of `world`.
            let scripted: Vec<Entity> = self.world.query::<&BehaviorSlot>().iter().map(|(entity, _)| entity).collect();
            for entity in scripted {
                self.with_behavior(entity, |behavior, api| behavior.on_update(api, dt));
            }

            // Captured before `step` resolves collisions — landing zeroes
            // out the vertical velocity component, so this is the only
            // point where "how fast were we falling" is still readable.
            let pre_step_fall_speed = self
                .player_entity
                .and_then(|entity| self.world.get::<&RigidBody>(entity).ok())
                .map(|body| body.velocity.y)
                .unwrap_or(0.0);

            let overlaps = engine::physics::step(&mut self.world, dt, &self.physics_params);

            let Some(player) = self.player_entity else {
                return Ok(());
            };

            let now_grounded = self.world.get::<&RigidBody>(player).map(|body| body.grounded).unwrap_or(false);
            if now_grounded && !self.was_grounded && pre_step_fall_speed < -0.1 {
                self.landing_dip.land(-pre_step_fall_speed);
            }
            self.was_grounded = now_grounded;

            let current: HashSet<(Entity, Entity)> = overlaps
                .into_iter()
                .filter(|&(dynamic, _other)| dynamic == player)
                .collect();

            let entered: Vec<Entity> = current
                .difference(&self.trigger_overlaps)
                .map(|&(_player, trigger)| trigger)
                .collect();
            let exited: Vec<Entity> = self
                .trigger_overlaps
                .difference(&current)
                .map(|&(_player, trigger)| trigger)
                .collect();

            // If a trigger causes a level transition mid-loop, the world it
            // was computed against is gone — stop processing this frame's
            // remaining trigger events and skip overwriting
            // `trigger_overlaps` with a now-stale snapshot (`transition_to_level`
            // already cleared it).
            let gl = ctx.gl();
            let mut transitioned = false;
            for trigger in entered {
                if self.on_trigger_entered(gl, trigger) {
                    transitioned = true;
                    break;
                }
            }
            if !transitioned {
                for trigger in exited {
                    self.on_trigger_exited(trigger);
                }
                self.trigger_overlaps = current;
            }
        }
        Ok(())
    }

    fn render(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        let drawable_size = ctx.drawable_size();
        let aspect = ctx.aspect_ratio();
        let (view, proj) = match self.mode {
            EditorMode::Edit => (self.camera.view_matrix(), self.camera.projection_matrix(aspect)),
            EditorMode::Play => {
                let eye = self.player_eye_position();
                (self.fp_camera.view_matrix(eye), self.fp_camera.projection_matrix(aspect))
            }
        };
        let params = self.render_params;

        let gl = ctx.gl();
        let renderer = self.renderer.as_mut().expect("renderer set up in init");
        renderer.resize_if_needed(gl, drawable_size)?;
        renderer.begin_scene(gl);

        let shader_cache = self
            .shader_cache
            .as_mut()
            .expect("shader cache set up in init");
        let flags = if params.affine_texture_mapping {
            AFFINE_UV_BIT
        } else {
            0
        };
        let program = shader_cache.get_or_compile(gl, flags)?;
        let uniforms = MeshUniforms::resolve(gl, program);

        unsafe {
            gl.clear_color(params.fog_color[0], params.fog_color[1], params.fog_color[2], 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }

        // Drawn first, depth-write disabled, so mesh geometry drawn after
        // this always occludes it normally — see `SkyboxPass` for why no
        // cubemap/dome mesh is needed.
        let inv_view_proj = (proj * view).inverse();
        renderer.draw_skybox(gl, inv_view_proj.to_cols_array(), params.sky_horizon_color, params.sky_zenith_color);

        unsafe {
            // Scoped to just this opaque mesh loop — left untouched for the
            // skybox (a single fullscreen triangle already drawn above) and
            // for particle billboards (drawn after, via their own begin/end).
            if params.backface_culling {
                gl.enable(glow::CULL_FACE);
                gl.cull_face(glow::BACK);
            } else {
                gl.disable(glow::CULL_FACE);
            }

            gl.use_program(Some(program));
            gl.uniform_matrix_4_f32_slice(uniforms.view.as_ref(), false, &view.to_cols_array());
            gl.uniform_matrix_4_f32_slice(uniforms.proj.as_ref(), false, &proj.to_cols_array());
            let light_dir = Vec3::from(params.light_dir).normalize_or_zero();
            gl.uniform_3_f32(
                uniforms.light_dir.as_ref(),
                light_dir.x,
                light_dir.y,
                light_dir.z,
            );
            gl.uniform_3_f32(
                uniforms.ambient_color.as_ref(),
                params.ambient_color[0],
                params.ambient_color[1],
                params.ambient_color[2],
            );
            gl.uniform_1_i32(
                uniforms.lighting_mode.as_ref(),
                match params.lighting_mode {
                    LightingMode::Unlit => 0,
                    LightingMode::VertexLit => 1,
                },
            );
            gl.uniform_1_f32(uniforms.vertex_snap_amount.as_ref(), params.vertex_snap_amount);
            gl.uniform_1_f32(uniforms.fog_start.as_ref(), params.fog_start);
            gl.uniform_1_f32(uniforms.fog_end.as_ref(), params.fog_end);
            gl.uniform_3_f32(
                uniforms.fog_color.as_ref(),
                params.fog_color[0],
                params.fog_color[1],
                params.fog_color[2],
            );

            let point_lights: Vec<(Vec3, Vec3, f32, f32)> = self
                .world
                .query::<(&Transform, &Light)>()
                .iter()
                .filter_map(|(_entity, (transform, light))| match light.kind {
                    LightKind::Point { range } => {
                        Some((transform.position, light.color, light.intensity, range))
                    }
                    LightKind::Directional { .. } => None,
                })
                .take(MAX_POINT_LIGHTS)
                .collect();

            for (i, (position, color, intensity, range)) in point_lights.iter().enumerate() {
                gl.uniform_3_f32(uniforms.point_light_pos[i].as_ref(), position.x, position.y, position.z);
                gl.uniform_3_f32(uniforms.point_light_color[i].as_ref(), color.x, color.y, color.z);
                gl.uniform_1_f32(uniforms.point_light_intensity[i].as_ref(), *intensity);
                gl.uniform_1_f32(uniforms.point_light_range[i].as_ref(), *range);
            }
            gl.uniform_1_i32(uniforms.point_light_count.as_ref(), point_lights.len() as i32);

            for (_entity, (transform, mesh_renderer)) in
                self.world.query::<(&Transform, &MeshRenderer)>().iter()
            {
                gl.uniform_matrix_4_f32_slice(
                    uniforms.model.as_ref(),
                    false,
                    &transform.matrix().to_cols_array(),
                );

                if let Some(texture) = &mesh_renderer.texture {
                    texture.bind(gl, 0);
                    gl.uniform_1_i32(uniforms.tex.as_ref(), 0);
                }

                mesh_renderer.mesh.draw(gl);
            }

            gl.disable(glow::CULL_FACE);
        }

        // Camera-facing (billboarded) particles, drawn after opaque meshes
        // so they blend over the already-drawn scene. `view.inverse()`'s
        // rotation is the camera's own world-space orientation — applying
        // it to every particle's quad makes it face the viewer regardless
        // of camera angle.
        if self.particle_quad_mesh.is_none() {
            self.particle_quad_mesh = Some(Arc::new(GpuMesh::upload(gl, &primitives::quad())?));
        }
        let quad_mesh = self.particle_quad_mesh.as_ref().unwrap().clone();
        let (_, billboard_rotation, _) = view.inverse().to_scale_rotation_translation();
        let mut particle_draws: Vec<(Arc<GpuMesh>, [f32; 16], [f32; 4])> = Vec::new();
        for (_entity, (_transform, emitter)) in self.world.query::<(&Transform, &ParticleEmitter)>().iter() {
            for particle in &emitter.particles {
                let (size, color) = emitter.appearance(particle);
                let model = Mat4::from_scale_rotation_translation(Vec3::splat(size), billboard_rotation, particle.position);
                particle_draws.push((quad_mesh.clone(), model.to_cols_array(), color));
            }
        }
        renderer.draw_particles(gl, &view.to_cols_array(), &proj.to_cols_array(), particle_draws.into_iter());

        let (tint_color, tint_strength) = self.screen_effects.current();
        renderer.present(
            gl,
            drawable_size,
            &PostParams {
                color_levels: params.color_levels as f32,
                dither_strength: params.dither_strength,
                tint_color,
                tint_strength,
            },
        );

        // Editor overlay, drawn on top of the final (already-pixelated) image
        // at full window resolution so the UI itself stays crisp.
        let mut ui_state = self.ui.take().expect("ui set up in init");
        let editing = editor_available() && self.mode == EditorMode::Edit;
        let ui_visible = self.ui_visible && editing;
        let level_ui_visible = self.level_ui_visible && editing;
        let playing = self.mode == EditorMode::Play;
        let mut pause_menu_action: Option<PauseMenuAction> = None;
        let full_output = ui_state.run(drawable_size, |egui_ctx| {
            if level_ui_visible {
                egui::SidePanel::left("level_editor")
                    .default_width(280.0)
                    .show(egui_ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            self.draw_level_editor_ui(ui, gl);
                        });
                    });
            }
            if ui_visible {
                egui::SidePanel::right("editor")
                    .default_width(300.0)
                    .show(egui_ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            self.draw_shader_editor_ui(ui, gl, drawable_size);
                        });
                    });
            }
            if playing {
                egui::Area::new("play_mode_indicator".into())
                    .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 12.0))
                    .show(egui_ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            let escape_hint = if self.paused { "Esc to resume" } else { "Esc to pause" };
                            let f3_hint = if editor_available() { " \u{b7} F3 stop" } else { "" };
                            ui.label(format!(
                                "Play Mode \u{2014} {escape_hint}{f3_hint} \u{b7} WASD + mouse \u{b7} Space jump \u{b7} E interact \u{b7} F9 checkpoint \u{b7} F10 load checkpoint"
                            ));
                        });
                    });
                // HUD only makes sense with a player around to show progress
                // for — mirrors the play_mode_indicator's own gating. Left
                // visible (though frozen, see `Game::update`) while paused,
                // same as the rest of the frozen world behind the menu.
                draw_hud(egui_ctx, &self.hud);
                if self.paused {
                    pause_menu_action = draw_pause_menu(egui_ctx, editor_available());
                }
            }
        });
        ui_state.paint(drawable_size, full_output);
        self.ui = Some(ui_state);

        match pause_menu_action {
            Some(PauseMenuAction::Resume) => self.resume(ctx),
            Some(PauseMenuAction::ExitToEditor) => self.exit_play_mode(ctx),
            Some(PauseMenuAction::RestartLevel) => self.restart_level(ctx),
            Some(PauseMenuAction::QuitGame) => ctx.should_quit = true,
            None => {}
        }

        Ok(())
    }
}

/// Debug builds keep the console and log there as usual. Release builds have
/// no console (see `windows_subsystem` above), so log to a file next to the
/// executable instead — otherwise there'd be no way to diagnose a problem
/// after the fact.
fn init_logging() {
    #[cfg(debug_assertions)]
    {
        env_logger::init();
    }

    #[cfg(not(debug_assertions))]
    {
        let log_path = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join("game.log")))
            .unwrap_or_else(|| PathBuf::from("game.log"));

        match std::fs::File::create(&log_path) {
            Ok(file) => {
                env_logger::Builder::from_default_env()
                    .target(env_logger::Target::Pipe(Box::new(file)))
                    .init();
            }
            Err(_) => env_logger::init(),
        }
    }
}

/// Debug builds already show panics/errors in the console; release builds
/// don't have one, so also pop a native dialog (reusing `rfd`, already a
/// dependency for model import) so a crash isn't completely silent.
fn show_fatal_error_dialog(message: &str) {
    #[cfg(not(debug_assertions))]
    {
        rfd::MessageDialog::new()
            .set_title("Jame Engine mushroom_man - Error")
            .set_description(message)
            .set_level(rfd::MessageLevel::Error)
            .show();
    }
    #[cfg(debug_assertions)]
    {
        let _ = message;
    }
}

fn main() -> anyhow::Result<()> {
    init_logging();

    std::panic::set_hook(Box::new(|info| {
        log::error!("panic: {info}");
        show_fatal_error_dialog(&info.to_string());
    }));

    let result = App::run("Jame Engine mushroom_man", 1280, 720, Sandbox::new());
    if let Err(err) = &result {
        log::error!("fatal error: {err:?}");
        show_fatal_error_dialog(&format!("{err:?}"));
    }
    result
}
