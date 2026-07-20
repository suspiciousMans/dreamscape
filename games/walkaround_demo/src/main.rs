// A shipped release build has no console attached; a debug build keeps one
// so `cargo run` still shows log output and panic messages as usual.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engine::ai::{AiEvent, CharacterBrain, CharacterMeta, Dialogue, DialogueNode, Disposition, Health, SpawnerConfig, SpawnerState};
use engine::animation::{AnimationKind, Animator};
use engine::app::{App, Context, Game};
use engine::audio::AudioContext;
use engine::behavior::{Behavior, BehaviorSlot, NativeBobBehavior, ScriptApi, ScriptBehavior};
use engine::camera::{
    CameraShakeSpec, CameraShakeState, FirstPersonCamera, HeadBob, LandingDip, OrbitCamera, SpeedFov,
    StrafeTilt,
};
use engine::class::ObjectClass;
use engine::ecs::{euler_deg_to_quat, Entity, Light, LevelObjectMeta, LightKind, MeshRenderer, PlayerController, Transform, World};
use engine::hotreload::HotReloadWatcher;
use engine::hud::HudState;
use engine::screen_effect::{ScreenEffectSpec, ScreenEffectState};
use engine::glam::{Mat4, Quat, Vec3};
use engine::glow::{self, HasContext};
use engine::level::{AnimationSpec, CharacterInstance, Level, LevelLight, LevelObject, LevelParticleEmitter, LevelTransition, MeshSource, PrimitiveKind, RigInstance, SpawnerInstance};
use engine::rig::{Keyframe, JointTrack, Rig, RigAnimator, RigAsset, RigClip, RigPart, RigPartDef};
use engine::save::SaveData;
use engine::mesh::{load_gltf, load_obj, primitives, GpuMesh};
use engine::net::{
    spawn_ring_offset, CharacterSnapshot, InputState, NetClient, NetHost, NetId, ObjectSnapshot, PlayerSnapshot,
    ServerMessage,
};
use engine::particles::{ParticleEmitter, ParticleEmitterDef};
use engine::pathfinding::NavGrid;
use engine::physics::{Collider, ColliderShape, PhysicsParams, RigidBody};
use engine::profile::{LightingMode, ProfileCycler, RenderParams, ShaderProfile, TextureFilterMode};
use engine::renderer::{PostParams, Renderer};
use engine::sdl2::controller::Button as ControllerButton;
use engine::sdl2::event::Event;
use engine::sdl2::keyboard::Keycode;
use engine::sdl2::mouse::MouseButton;
use engine::shader::{ShaderVariantCache, AFFINE_UV_BIT};
use engine::texture::{GpuTexture, TextureFilter};
use engine::ui::{draw_hud, draw_pause_menu, hud_style_editor, render_params_editor, EguiState, PauseMenuAction};

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

/// The starting level: the same 3-cube demo scene the engine has always
/// shipped with, now expressed as real, editable/savable `LevelObject`s
/// instead of hardcoded ECS spawns — plus a static floor so Play mode has
/// something for the (now dynamic) cubes to fall onto.
fn default_level() -> Level {
    let make_cube = |name: &str, x: f32| LevelObject {
        name: name.to_string(),
        mesh: MeshSource::ObjFile(PathBuf::from("assets/models/test_scene.obj")),
        texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
        position: [x, 1.2, 0.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [1.0, 1.0, 1.0],
        is_dynamic: true,
        is_trigger: false,
        animation: None,
        script: None,
        class: None,
        level_transition: None,
        screen_effect: None,
        camera_shake: None,
    };

    let floor = LevelObject {
        name: "Floor".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Plane),
        // Textured (not the default white fallback) so it doesn't read as a
        // blank patch butted up against the checkerboard cubes — a plain
        // white prop right next to a textured one is what actually made
        // things look "hollow"/broken, not a mesh or shader bug.
        texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
        position: [0.0, 0.0, 0.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [8.0, 1.0, 8.0],
        is_dynamic: false,
        is_trigger: false,
        animation: None,
        script: None,
        class: None,
        level_transition: None,
        screen_effect: None,
        camera_shake: None,
    };

    // Static (no RigidBody) but animated — demonstrates that an animated
    // object with no physics still moves under its own motion, purely
    // decorative here since nothing touches it.
    let orbiting_cube = LevelObject {
        name: "Orbiting Cube".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
        position: [0.0, 3.0, -2.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [0.6, 0.6, 0.6],
        is_dynamic: false,
        is_trigger: false,
        animation: Some(AnimationSpec::Orbit {
            axis: [0.0, 1.0, 0.0],
            speed_deg_per_sec: 90.0,
        }),
        script: None,
        class: None,
        level_transition: None,
        screen_effect: None,
        camera_shake: None,
    };

    let demo_light = LevelLight {
        name: "Light 1".to_string(),
        position: [0.0, 2.5, 1.5],
        color: [1.0, 0.75, 0.4],
        intensity: 1.5,
        range: 6.0,
    };

    // Sits in the player's path between the Play-mode spawn point and the
    // cubes, so walking forward demonstrates trigger enter/exit for free.
    let trigger = LevelObject {
        name: "Trigger Zone".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: None,
        position: [0.0, 1.0, 2.5],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [1.5, 1.0, 1.5],
        is_dynamic: false,
        is_trigger: true,
        animation: None,
        script: None,
        class: None,
        level_transition: None,
        // A quick cyan flash matching the trigger's own render color —
        // demonstrates screen effects the moment a player reaches Play
        // mode's first trigger, no extra geometry needed.
        screen_effect: Some(ScreenEffectSpec {
            color: [0.2, 0.8, 1.0],
            strength: 0.5,
            fade_in_secs: 0.05,
            hold_secs: 0.05,
            fade_out_secs: 0.35,
        }),
        // A light rumble alongside the flash — demonstrates camera shake
        // the same free way the flash demonstrates screen effects.
        camera_shake: Some(CameraShakeSpec {
            intensity: 0.06,
            duration_secs: 0.3,
        }),
    };

    // Bobs via `scripts/bob_demo.pss` — stands next to the compiled-in
    // "Native Behavior Cube" (see `spawn_native_behavior_demo`) so
    // interacting with both makes the script/Rust parity concrete.
    let script_demo_cube = LevelObject {
        name: "Script Demo Cube".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
        position: [-2.4, 1.0, -2.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [0.6, 0.6, 0.6],
        is_dynamic: false,
        is_trigger: false,
        animation: None,
        script: Some(PathBuf::from("scripts/bob_demo.pss")),
        class: None,
        level_transition: None,
        screen_effect: None,
        camera_shake: None,
    };

    // Past the demo cubes/rig, in the player's forward path — demonstrates
    // level transitions: walking in loads "second_room" and lands the
    // player just inside its own "Return" trigger (see `second_room_level`).
    let level_exit = LevelObject {
        name: "Level Exit".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: None,
        position: [0.0, 1.0, -4.5],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [1.5, 1.0, 1.5],
        is_dynamic: false,
        is_trigger: true,
        animation: None,
        script: None,
        class: None,
        level_transition: Some(LevelTransition {
            target_level: "second_room".to_string(),
            spawn_position: [0.0, 1.0, 1.5],
            spawn_yaw_deg: 0.0,
        }),
        // A quick white flash makes the level swap read as a proper
        // "portal" transition rather than an abrupt cut.
        screen_effect: Some(ScreenEffectSpec {
            color: [1.0, 1.0, 1.0],
            strength: 0.7,
            fade_in_secs: 0.05,
            hold_secs: 0.05,
            fade_out_secs: 0.3,
        }),
        camera_shake: None,
    };

    let humanoid_instance = RigInstance {
        name: "Humanoid".to_string(),
        rig_path: PathBuf::from("rigs/humanoid.ron"),
        position: [3.2, 1.5, -2.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        playing_clip: Some("Wave".to_string()),
    };

    // Two instances of the "barrel" class — demonstrates the class/family
    // system: mesh/texture/scale/physics all come from `classes/barrel.ron`
    // at spawn time, not from the fields below (kept only as a fallback in
    // case the class is ever deleted). Edit the class in F2 and click
    // "Apply to All Instances" to see both barrels update together.
    let make_barrel = |name: &str, x: f32| LevelObject {
        name: name.to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: None,
        position: [x, 1.0, 4.5],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [1.0, 1.0, 1.0],
        is_dynamic: false,
        is_trigger: false,
        animation: None,
        script: None,
        class: Some(PathBuf::from("classes/barrel.ron")),
        level_transition: None,
        screen_effect: None,
        camera_shake: None,
    };

    // A small continuous sparkle jet — placed near the light so it reads as
    // decoration rather than needing its own explanation.
    let sparkle_emitter = LevelParticleEmitter {
        name: "Sparkles".to_string(),
        position: [0.0, 2.5, 1.5],
        def: ParticleEmitterDef::default(),
    };

    // A minimal demo touch for the character/AI system, matching every
    // other feature's presence here — a `Passive` critter that wanders
    // near the cubes and flees if the player approaches.
    let wanderer = CharacterInstance {
        name: "Wanderer".to_string(),
        position: [2.5, 1.0, -1.0],
        scale: [0.6, 0.6, 0.6],
        color: [0.3, 0.8, 0.4],
        texture_path: None,
        disposition: Disposition::Passive,
        move_speed: 2.0,
        wander_radius: 2.5,
        sight_range: 4.0,
        max_health: None,
        damage: None,
        attack_range: 1.0,
        attack_cooldown_secs: 1.0,
        dialogue_nodes: Vec::new(),
    };

    Level {
        name: "default".to_string(),
        objects: vec![
            floor,
            // A couple of static props for the walkaround demo
            LevelObject {
                name: "Stone Pillar".to_string(),
                mesh: MeshSource::Primitive(PrimitiveKind::Cube),
                texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
                position: [2.0, 1.0, 0.0],
                rotation_euler_deg: [0.0, 0.0, 0.0],
                scale: [0.8, 2.0, 0.8],
                is_dynamic: false,
                is_trigger: false,
                animation: None,
                script: None,
                class: None,
                level_transition: None,
                screen_effect: None,
                camera_shake: None,
            },
            LevelObject {
                name: "Bench".to_string(),
                mesh: MeshSource::Primitive(PrimitiveKind::Cube),
                texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
                position: [-2.0, 0.5, 1.5],
                rotation_euler_deg: [0.0, 0.0, 0.0],
                scale: [1.5, 0.3, 0.5],
                is_dynamic: false,
                is_trigger: false,
                animation: None,
                script: None,
                class: None,
                level_transition: None,
                screen_effect: None,
                camera_shake: None,
            },
            make_barrel("Barrel 1", -0.7),
            make_barrel("Barrel 2", 0.7),
        ],
        lights: vec![demo_light],
        particle_emitters: vec![sparkle_emitter],
        rig_instances: Vec::new(),
        characters: Vec::new(),
        spawners: Vec::new(),
        physics: PhysicsParams::default(),
        music_path: Some(PathBuf::from("music/demo_ambient.wav")),
    }
}

/// A small second level demonstrating level-to-level transitions:
/// `default_level`'s "Level Exit" trigger lands here, and this room's own
/// "Return" trigger goes back — see `Sandbox::transition_to_level`.
fn second_room_level() -> Level {
    let floor = LevelObject {
        name: "Second Room Floor".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Plane),
        texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
        position: [0.0, 0.0, 0.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [4.0, 1.0, 4.0],
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
        name: "Second Room Light".to_string(),
        position: [0.0, 2.5, 0.0],
        color: [0.6, 0.75, 1.0],
        intensity: 1.5,
        range: 6.0,
    };

    // Set back from the arrival spawn point so walking in doesn't
    // instantly re-trigger the return; walking forward reaches it.
    let return_trigger = LevelObject {
        name: "Return".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: None,
        position: [0.0, 1.0, -1.5],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [1.5, 1.0, 1.5],
        is_dynamic: false,
        is_trigger: true,
        animation: None,
        script: None,
        class: None,
        level_transition: Some(LevelTransition {
            target_level: "default".to_string(),
            spawn_position: [0.0, 1.0, -3.0],
            spawn_yaw_deg: 0.0,
        }),
        screen_effect: None,
        camera_shake: None,
    };

    Level {
        name: "second_room".to_string(),
        objects: vec![floor, return_trigger],
        lights: vec![light],
        particle_emitters: Vec::new(),
        rig_instances: Vec::new(),
        characters: Vec::new(),
        spawners: Vec::new(),
        physics: PhysicsParams::default(),
        music_path: None,
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

/// The starting object class: a plain dynamic cube standing in for a
/// "barrel" (no dedicated art yet — reuses the built-in cube primitive).
/// Written to `classes/barrel.ron` on first run, same bootstrap-if-empty
/// convention as `default_demo_profiles`/`default_level`/`default_rig_asset`.
fn default_barrel_class() -> ObjectClass {
    ObjectClass {
        name: "barrel".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        // Same reasoning as the floor: an untextured (solid white) prop
        // standing right next to the checkerboard cubes read as a hollow/
        // broken box rather than a separate plain object.
        texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
        scale: [0.8, 0.8, 0.8],
        is_dynamic: true,
        is_trigger: false,
        animation: None,
        script: None,
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

/// Who requested the currently-open texture asset browser — set when a
/// "Browse Texture..." button opens it, read (and cleared) once the user
/// picks a thumbnail, so the picked path can be applied to the right
/// target across however many frames the browser stays open.
enum PendingTexturePick {
    ForEntity(Entity),
    ForClass(usize),
    ForCharacter(Entity),
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

/// Maps the number-row keys to a 0-based index, for the dialogue
/// choice-picker overlay (`Sandbox::draw_dialogue_choices`).
fn number_key_index(keycode: Keycode) -> Option<usize> {
    match keycode {
        Keycode::Num1 => Some(0),
        Keycode::Num2 => Some(1),
        Keycode::Num3 => Some(2),
        Keycode::Num4 => Some(3),
        Keycode::Num5 => Some(4),
        Keycode::Num6 => Some(5),
        Keycode::Num7 => Some(6),
        Keycode::Num8 => Some(7),
        Keycode::Num9 => Some(8),
        _ => None,
    }
}

/// Tags the compiled-in native-behavior demo cube so `build_level_from_ecs`
/// can exclude it — it isn't level data, just always re-added by
/// `spawn_native_behavior_demo`.
struct NativeBehaviorDemoMarker;

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

/// A host-assigned network identity, attached to any entity whose state
/// needs to be replicated to other peers — players, `CharacterMeta`
/// characters, dynamic props. Static level geometry never gets one: it's
/// already fully described by the `Level` sent in `Welcome`/
/// `LevelTransition`, which `apply_level` reconstructs identically on
/// every peer (see `engine::net`'s "Entity replication model").
struct Networked(NetId);

/// Tags a host-side entity representing a connected client's avatar —
/// distinguishes it from the host's own local `player_entity` (which has
/// no mesh, being first-person) and from NPCs (`CharacterMeta`). Only
/// ever exists under `NetMode::Host`; look it up by `NetId` via
/// `Sandbox::remote_players` rather than querying for this marker.
struct RemotePlayer;

/// Client-side: the latest authoritative position/rotation for a
/// networked entity OTHER than the local player, nudged toward every
/// frame (`Sandbox::smooth_networked_transforms`) instead of snapped to
/// instantly on arrival — snapshots land at a fixed 20Hz, and a hard
/// `Transform` overwrite reads as visible stepping for anything the
/// player is watching move (another player, an AI character, a pushed
/// physics prop). The local player's own entity is handled separately
/// (see `update_player_input`'s dead reckoning plus `net_local_target`
/// for the correction it layers on top), since it has real per-frame
/// local input to stay responsive to.
struct NetTarget {
    position: Vec3,
    rotation: Option<Quat>,
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
    /// Debug FPS/frame-time/entity/draw-call overlay, toggled from the F1
    /// panel — always available (Edit and Play mode), unlike `draw_hud`
    /// which is gameplay UI. `frame_times` is a small ring buffer of
    /// recent `dt` samples, averaged each frame for a less jittery FPS
    /// reading than a single-frame instantaneous value.
    profiler_visible: bool,
    frame_times: std::collections::VecDeque<f32>,
    /// `None` if the file watcher failed to start (degrades silently, same
    /// convention as `audio: Option<AudioContext>`) or in a shipped build
    /// (`init` only starts it when `editor_available()`).
    hot_reload: Option<HotReloadWatcher>,
    /// F2 panel: in-editor texture thumbnail browser, opened by a "Browse
    /// Texture..." button (alongside the existing OS-dialog "Assign
    /// Texture..." button, not replacing it) and `pending_texture_pick`
    /// tracking which entity/class it was opened for.
    texture_asset_browser: engine::ui::AssetBrowserState,
    pending_texture_pick: Option<PendingTexturePick>,
    /// Play mode: set by `interact()` when the character just spoken to has
    /// a choice pending on their current dialogue node — while set, a
    /// choice-picker overlay is drawn and number keys 1-9 pick an option
    /// (see `handle_event`'s `Keycode::Num1..=Num9` arm).
    active_dialogue: Option<Entity>,
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
    /// each frame rather than writing `fp_camera.fov_y_radians` directly.
    speed_fov: SpeedFov,
    /// The FOV `speed_fov`'s kick is added on top of each frame.
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
    /// Baked from static level geometry in `apply_level` — `None` only
    /// before the first level ever loads. Used by `engine::ai::step` for
    /// character pathfinding.
    nav_grid: Option<NavGrid>,
    /// F2 panel: the currently selected character entity, mirroring
    /// `selected_rig` — characters get their own "Characters" section
    /// rather than sharing the flat Scene Objects outliner, since they
    /// have no `LevelObjectMeta`.
    selected_character: Option<Entity>,
    /// F2 panel: the currently selected spawner entity, mirroring
    /// `selected_character` — spawners get their own "Spawners" section
    /// since they have no `LevelObjectMeta`/`CharacterMeta`.
    selected_spawner: Option<Entity>,
    /// Multi-select sets for batch delete/duplicate (Ctrl+click toggles
    /// membership; a plain click resets the set to just that one entity).
    /// `selected_entity`/`selected_character` remain "which one's detail
    /// panel is shown" (always the most recently clicked), independent of
    /// how many are in the batch-operation set.
    selected_entities: HashSet<Entity>,
    selected_characters: HashSet<Entity>,
    /// Undo/redo: full-level snapshots (the same `Level` shape
    /// `build_level_from_ecs`/`apply_level` already round-trip for save/
    /// load), pushed by `push_undo_snapshot` before a discrete edit
    /// (add/delete/duplicate) or a selection change (bundling a
    /// slider-drag session on one object into a single undo step). Capped
    /// so it can't grow unbounded across a long editing session.
    undo_stack: Vec<Level>,
    redo_stack: Vec<Level>,
    /// Last frame's selection tuple — compared each `draw_level_editor_ui`
    /// call to detect "the user switched to editing something else",
    /// which is when a fresh undo snapshot is pushed.
    last_undo_selection: (Option<Entity>, Option<Entity>, Option<Entity>, Option<usize>),
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
    /// Names of level entities despawned since the current level was
    /// (re-)applied — captured into `SaveData::despawned_names` at
    /// checkpoint time and replayed on load so a reloaded checkpoint
    /// doesn't bring back things the player already removed. Cleared by
    /// `apply_level`.
    despawned_since_load: Vec<String>,
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
    /// Which side of a listen-server session this instance is playing,
    /// if any — see `engine::net`. `Offline` is the default (unchanged
    /// single-player behavior).
    net_mode: NetMode,
    /// Client-side: maps a replicated entity's host-assigned `NetId` to
    /// this process's own local `hecs::Entity` for it — built from
    /// `ServerMessage::Welcome`'s `named_net_ids` after `apply_level`, and
    /// extended as `PlayerJoined`/`CharacterSpawned` events arrive.
    net_id_to_entity: HashMap<NetId, Entity>,
    /// Host-side: maps a connected client's `NetId` to the `RemotePlayer`
    /// entity spawned for them, so `NetHost::poll_inputs()`'s results can
    /// be applied to the right entity each frame.
    remote_players: HashMap<NetId, Entity>,
    /// F2-panel-style transient input buffers for the multiplayer panel's
    /// "Address"/"Name" fields, persisted across frames the same way
    /// `level_save_as_name` already is.
    net_join_address: String,
    net_player_name: String,
    /// Host-side: `NetId`s of networked characters/objects despawned since
    /// the last `Snapshot` broadcast — flushed into that snapshot's
    /// `removed` list and cleared, so clients despawn the matching local
    /// entity instead of leaving a stale cube behind forever.
    net_removed: Vec<NetId>,
    /// Client-side sibling to `active_dialogue` — dialogue state itself is
    /// host-authoritative under `NetMode::Client`, so instead of reading a
    /// local `Dialogue` component, `draw_dialogue_choices` and the number-
    /// key handler read this instead: `(speaker, text, choices)` from the
    /// last `ServerMessage::Dialogue`, cleared by `DialogueClosed` or a
    /// picked choice.
    net_dialogue: Option<(NetId, String, Vec<(String, usize)>)>,
    /// Client-side smoothing target for the local player's own position
    /// — the latest authoritative value from `Snapshot`, nudged toward
    /// every frame the same way `NetTarget` smooths every other entity,
    /// rather than hard-snapped. A hard snap on every snapshot arrival
    /// was tried first and made the joining client's own view visibly
    /// wobble every ~50ms, since even tiny natural client/host timing
    /// differences (not just real collisions) show up as a full pop;
    /// smoothing hides that jitter while still correcting real drift
    /// (e.g. the host stopping the player at a wall) within a few
    /// frames. Applied on top of `update_player_input`'s per-frame X/Z
    /// dead reckoning, which stays instantly responsive to input.
    /// `None` until the first `Snapshot` arrives.
    net_local_target: Option<Vec3>,
    /// Host-side: `NetId`s of connected players granted permission to
    /// trigger a level transition themselves by walking into an exit —
    /// by default only the host's own local player can (see
    /// `Game::update`'s trigger-overlap handling), since a remote
    /// player's physical overlap is otherwise never checked at all.
    /// Toggled per-player from the multiplayer panel's player list.
    remote_level_switch_permission: HashMap<NetId, bool>,
    /// Client-side: attempts left in an automatic reconnect sequence
    /// after an unexpected drop (see `update_networking`'s
    /// `!client.is_connected()` handling) — `0` means no sequence is
    /// active. Reuses `net_join_address`/`net_player_name`, the same
    /// values `join_game` reads from the multiplayer panel's text
    /// fields, so it retries the exact same connection the player
    /// originally made.
    net_reconnect_attempts_left: u32,
    /// Seconds until the next automatic reconnect attempt.
    net_reconnect_timer: f32,
}

/// Which side of a listen-server multiplayer session `Sandbox` is
/// currently playing, if any — see the `engine::net` module for the
/// underlying transport. `Host`/`Client` wrap the connection state;
/// `Offline` (the default) is unchanged single-player behavior.
enum NetMode {
    Offline,
    Host(NetHost),
    Client(NetClient),
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
            profiler_visible: true,
            frame_times: std::collections::VecDeque::new(),
            hot_reload: None,
            texture_asset_browser: engine::ui::AssetBrowserState::default(),
            pending_texture_pick: None,
            active_dialogue: None,
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
            nav_grid: None,
            selected_character: None,
            selected_spawner: None,
            selected_entities: HashSet::new(),
            selected_characters: HashSet::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_undo_selection: (None, None, None, None),
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
            despawned_since_load: Vec::new(),
            current_music_path: None,
            music_volume: 1.0,
            net_mode: NetMode::Offline,
            net_id_to_entity: HashMap::new(),
            remote_players: HashMap::new(),
            net_join_address: format!("127.0.0.1:{}", engine::net::DEFAULT_PORT),
            net_player_name: String::from("Player"),
            net_removed: Vec::new(),
            net_dialogue: None,
            net_local_target: None,
            remote_level_switch_permission: HashMap::new(),
            net_reconnect_attempts_left: 0,
            net_reconnect_timer: 0.0,
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

    /// Compares an asset-root-relative path (as stored on `LevelObjectMeta`/
    /// `shader_paths`) against an absolute path a `HotReloadWatcher` event
    /// reported changed. Canonicalizes both sides when possible (handles
    /// separator/case differences) and falls back to a direct comparison
    /// if canonicalization fails (e.g. the file was just deleted).
    fn hot_reload_path_matches(&self, relative: &Path, changed: &Path) -> bool {
        let full = self.asset_root.join(relative);
        match (full.canonicalize(), changed.canonicalize()) {
            (Ok(a), Ok(b)) => a == b,
            _ => full == *changed,
        }
    }

    /// Reacts to a `HotReloadWatcher::poll_events` result: reapplies the
    /// current shader profile if one of its files changed, re-attaches any
    /// live entity's script if its `.pss` file changed, and reloads any
    /// live entity's texture if its image file changed.
    fn handle_hot_reload(&mut self, ctx: &mut Context, changed_paths: &[PathBuf]) {
        let gl = ctx.gl();

        let shader_changed = changed_paths.iter().any(|path| {
            self.hot_reload_path_matches(&self.shader_paths.0, path)
                || self.hot_reload_path_matches(&self.shader_paths.1, path)
                || self.hot_reload_path_matches(&self.shader_paths.2, path)
        });
        if shader_changed {
            if let Some(cycler) = &self.profiles {
                let profile = cycler.current().clone();
                let drawable_size = ctx.drawable_size();
                match self.apply_profile(gl, drawable_size, &profile) {
                    Ok(()) => log::info!("hot reload: reapplied shader profile '{}'", profile.name),
                    Err(err) => log::error!("hot reload: failed to reapply shader profile: {err}"),
                }
            }
        }

        let scripted_entities: Vec<(Entity, PathBuf)> = self
            .world
            .query::<&LevelObjectMeta>()
            .iter()
            .filter_map(|(entity, meta)| meta.script_path.clone().map(|path| (entity, path)))
            .collect();
        for (entity, script_path) in scripted_entities {
            if changed_paths.iter().any(|path| self.hot_reload_path_matches(&script_path, path)) {
                log::info!("hot reload: reattaching script {script_path:?}");
                self.attach_script_to_entity(entity, &script_path);
            }
        }

        let textured_entities: Vec<(Entity, PathBuf)> = self
            .world
            .query::<&LevelObjectMeta>()
            .iter()
            .filter_map(|(entity, meta)| meta.texture_path.clone().map(|path| (entity, path)))
            .collect();
        for (entity, texture_path) in textured_entities {
            if changed_paths.iter().any(|path| self.hot_reload_path_matches(&texture_path, path)) {
                match GpuTexture::load_from_file(gl, &self.asset_root.join(&texture_path), TextureFilter::Nearest) {
                    Ok(texture) => {
                        let texture = Arc::new(texture);
                        if let Ok(mut query) = self.world.query_one::<&mut MeshRenderer>(entity) {
                            if let Some(renderer) = query.get() {
                                renderer.texture = Some(texture);
                            }
                        }
                        log::info!("hot reload: reloaded texture {texture_path:?}");
                    }
                    Err(err) => log::error!("hot reload: failed to reload texture {texture_path:?}: {err}"),
                }
            }
        }
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
            MeshSource::GltfFile(path) => {
                // Same "first entry only" convention as ObjFile above — a
                // multi-node glTF is better imported as a rig (F2 panel's
                // "Import glTF as Rig...") than collapsed into one object.
                let scene = load_gltf(&self.asset_root.join(path))?;
                let entry = scene
                    .meshes
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("glTF file {path:?} has no mesh-carrying node"))?;
                Arc::new(GpuMesh::upload(gl, &entry.mesh)?)
            }
            MeshSource::GltfNode { path, node } => {
                let scene = load_gltf(&self.asset_root.join(path))?;
                let entry = scene
                    .meshes
                    .iter()
                    .find(|entry| &entry.name == node)
                    .ok_or_else(|| anyhow::anyhow!("glTF file {path:?} has no node named '{node}'"))?;
                Arc::new(GpuMesh::upload(gl, &entry.mesh)?)
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

    /// Spawns a placed character: `Transform` + physics (sphere `Collider`,
    /// matching the player's own convention, plus `RigidBody` so it's a
    /// dynamic body physics already treats like any other) + `MeshRenderer`
    /// (a solid-colored cube — the "simple color primitives" visual style)
    /// + `CharacterMeta`/`CharacterBrain`, plus `Health`/`Dialogue` if the
    /// instance was authored with combat/dialogue.
    fn spawn_character(&mut self, gl: &glow::Context, instance: &CharacterInstance) -> anyhow::Result<Entity> {
        let mesh = self.resolve_mesh(gl, &MeshSource::Primitive(PrimitiveKind::Cube))?;
        let rgba = [
            (instance.color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
            (instance.color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
            (instance.color[2].clamp(0.0, 1.0) * 255.0).round() as u8,
            255,
        ];
        let texture = match &instance.texture_path {
            Some(path) => match GpuTexture::load_from_file(gl, &self.asset_root.join(path), TextureFilter::Nearest) {
                Ok(texture) => texture,
                Err(err) => {
                    log::error!("failed to load character texture {path:?}: {err}, falling back to solid color");
                    solid_color_texture(gl, rgba)
                }
            },
            None => solid_color_texture(gl, rgba),
        };
        let position = Vec3::from(instance.position);
        let entity = self.world.spawn((
            Transform { position, rotation: Quat::IDENTITY, scale: Vec3::from(instance.scale) },
            MeshRenderer { mesh, texture: Some(Arc::new(texture)) },
            Collider { shape: ColliderShape::Sphere { radius: instance.scale[0] * 0.5 }, is_trigger: false },
            RigidBody::default(),
            CharacterMeta {
                name: instance.name.clone(),
                color: instance.color,
                texture_path: instance.texture_path.clone(),
                disposition: instance.disposition,
                move_speed: instance.move_speed,
                wander_radius: instance.wander_radius,
                sight_range: instance.sight_range,
                damage: instance.damage,
                attack_range: instance.attack_range,
                attack_cooldown_secs: instance.attack_cooldown_secs,
            },
            CharacterBrain::new(position),
        ));
        if let Some(max_health) = instance.max_health {
            let _ = self.world.insert_one(entity, Health::new(max_health));
        }
        if !instance.dialogue_nodes.is_empty() {
            let _ = self.world.insert_one(entity, Dialogue::new(instance.dialogue_nodes.clone()));
        }
        Ok(entity)
    }

    /// Spawns a visible avatar for a newly-joined remote player — same
    /// physical shape as the host's own local player (`enter_play_mode`'s
    /// spawn: sphere `Collider`, `RigidBody`, 100 HP) plus a `MeshRenderer`
    /// cube, since unlike the local player (first-person, no mesh) a
    /// remote player needs to actually be visible to everyone else.
    /// Positioned via `spawn_ring_offset` around `base_position` so 2-4
    /// players sharing one spawn point (an initial join, or everyone
    /// landing at a level transition's `spawn_position`) don't stack on
    /// top of each other. Tagged `Networked`/`RemotePlayer` so disconnect
    /// handling and the snapshot broadcast can find it by `NetId`.
    fn spawn_remote_player_at(
        &mut self,
        gl: &glow::Context,
        net_id: NetId,
        base_position: Vec3,
        base_yaw_deg: f32,
    ) -> anyhow::Result<Entity> {
        let mesh = self.resolve_mesh(gl, &MeshSource::Primitive(PrimitiveKind::Cube))?;
        let (position, _) = spawn_ring_offset(base_position, base_yaw_deg, net_id);
        let entity = self.world.spawn((
            Transform { position, rotation: Quat::IDENTITY, scale: Vec3::new(0.8, 1.6, 0.8) },
            MeshRenderer { mesh, texture: Some(Arc::new(solid_color_texture(gl, [200, 200, 200, 255]))) },
            Collider { shape: ColliderShape::Sphere { radius: 0.4 }, is_trigger: false },
            RigidBody::default(),
            PlayerController::default(),
            Health::new(100.0),
            Networked(net_id),
            RemotePlayer,
        ));
        Ok(entity)
    }

    /// Host-side: applies a remote player's latest `InputState` directly
    /// to their `RigidBody`/facing — the same "set horizontal velocity,
    /// jump if grounded" shape `update_player_input` applies to the local
    /// player, except `move_dir` arrives pre-computed (already
    /// camera-relative, in world space) from the client rather than being
    /// derived from `ctx.input` here, since the host has no camera/input
    /// state for a remote player. `physics::step`'s existing all-entities
    /// query then resolves gravity/collision for this `RigidBody` exactly
    /// like any other — no changes needed there.
    fn apply_remote_input(&mut self, entity: Entity, input: &InputState) {
        if let Ok(mut query) = self.world.query_one::<(&mut Transform, &mut RigidBody, &PlayerController)>(entity) {
            if let Some((transform, body, controller)) = query.get() {
                let move_dir = Vec3::new(input.move_dir[0], 0.0, input.move_dir[1]);
                body.velocity.x = move_dir.x * controller.move_speed;
                body.velocity.z = move_dir.z * controller.move_speed;
                transform.rotation = Quat::from_rotation_y(input.yaw_deg.to_radians());
                if input.jump && body.grounded {
                    body.velocity.y = controller.jump_speed;
                }
            }
        }
    }

    /// Client-side: matches a name carried in `ServerMessage::Welcome`'s
    /// `named_net_ids` against the entity `apply_level` just spawned for
    /// it, so `net_id_to_entity` can be built without needing `Level`'s
    /// shape itself to carry any multiplayer-specific data. Searches
    /// `LevelObjectMeta` then `CharacterMeta` — the two component kinds
    /// that carry a `name` at all.
    fn find_entity_by_name(&self, name: &str) -> Option<Entity> {
        for (entity, meta) in self.world.query::<&LevelObjectMeta>().iter() {
            if meta.name == name {
                return Some(entity);
            }
        }
        for (entity, meta) in self.world.query::<&CharacterMeta>().iter() {
            if meta.name == name {
                return Some(entity);
            }
        }
        None
    }

    /// Host-side: every currently-`Networked` character/object's name and
    /// `NetId`, for `ServerMessage::Welcome`'s `named_net_ids` — matched
    /// back up against the client's own freshly-`apply_level`'d entities
    /// via `find_entity_by_name`. Players are deliberately excluded (they
    /// have no `CharacterMeta`/`LevelObjectMeta` to match against anyway,
    /// and are announced separately via `PlayerJoined`).
    fn build_named_net_ids(&self) -> Vec<(String, NetId)> {
        let mut named = Vec::new();
        for (_entity, (meta, networked)) in self.world.query::<(&CharacterMeta, &Networked)>().iter() {
            named.push((meta.name.clone(), networked.0));
        }
        for (_entity, (meta, networked)) in self.world.query::<(&LevelObjectMeta, &Networked)>().iter() {
            named.push((meta.name.clone(), networked.0));
        }
        named
    }

    /// If hosting, allocates a fresh `NetId` and tags `entity` with it —
    /// a no-op returning `None` under `Offline`/`Client` (only the host
    /// assigns identities). `self.net_mode` is taken out for the
    /// duration, the same "take-before-mutate" shape `update_networking`
    /// uses, since inserting a component needs `&mut self.world` while
    /// `NetHost` is borrowed from `self.net_mode`.
    fn assign_net_id_if_hosting(&mut self, entity: Entity) -> Option<NetId> {
        let mut net_mode = std::mem::replace(&mut self.net_mode, NetMode::Offline);
        let assigned = if let NetMode::Host(host) = &mut net_mode {
            let net_id = host.allocate_net_id();
            let _ = self.world.insert_one(entity, Networked(net_id));
            Some(net_id)
        } else {
            None
        };
        self.net_mode = net_mode;
        assigned
    }

    /// Broadcasts `msg` if hosting; a silent no-op under `Offline`/`Client`.
    fn broadcast_if_hosting(&mut self, msg: &ServerMessage) {
        if let NetMode::Host(host) = &mut self.net_mode {
            host.broadcast(msg);
        }
    }

    /// Spawns a placed periodic spawner: `Transform` (its position is where
    /// spawned characters appear around, via `spawn_radius`) +
    /// `SpawnerConfig`/`SpawnerState`. No mesh/collider of its own — it's a
    /// logic-only marker, drawn nowhere, listed only in the F2 panel's own
    /// "Spawners" outliner (mirrors how `CharacterMeta`/characters get
    /// their own section rather than sharing "Scene Objects").
    fn spawn_spawner(&mut self, instance: &SpawnerInstance) -> Entity {
        self.world.spawn((
            Transform { position: Vec3::from(instance.position), rotation: Quat::IDENTITY, scale: Vec3::ONE },
            SpawnerConfig {
                name: instance.name.clone(),
                template: instance.template.clone(),
                spawn_interval_secs: instance.spawn_interval_secs,
                max_alive: instance.max_alive,
                total_to_spawn: instance.total_to_spawn,
                spawn_radius: instance.spawn_radius,
            },
            SpawnerState::new(),
        ))
    }

    fn apply_level(&mut self, gl: &glow::Context, level: &Level) -> anyhow::Result<()> {
        self.world.clear();
        self.selected_entity = None;
        self.selected_rig = None;
        self.selected_rig_part = None;
        self.selected_character = None;
        self.selected_spawner = None;
        self.selected_entities.clear();
        self.selected_characters.clear();
        self.despawned_since_load.clear();
        for obj in &level.objects {
            if let Err(err) = self.spawn_level_object(gl, obj) {
                log::error!("failed to spawn level object '{}': {err}", obj.name);
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
        for instance in &level.characters {
            if let Err(err) = self.spawn_character(gl, instance) {
                log::error!("failed to spawn character '{}': {err}", instance.name);
            }
        }
        for instance in &level.spawners {
            self.spawn_spawner(instance);
        }
        self.spawn_native_behavior_demo(gl);
        // Baked last so it sees the level's full static geometry —
        // characters themselves are dynamic bodies, so they're excluded
        // from the bake regardless of spawn order (see `NavGrid::bake`).
        self.nav_grid = Some(NavGrid::bake(&self.world, 0.5));
        self.current_level_name = level.name.clone();
        self.physics_params = level.physics;
        // Only (re)trigger music if the track actually changed — undo/redo
        // and other same-level `apply_level` calls (e.g. restoring the
        // pre-play snapshot) would otherwise glitch/restart music that was
        // already playing correctly.
        let previous_music_path = self.current_music_path.clone();
        self.current_music_path = level.music_path.clone();
        let music_volume = self.music_volume;
        if previous_music_path != self.current_music_path {
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
        }
        log::info!(
            "loaded level '{}' ({} objects, {} lights, {} particle emitters, {} rig instances, {} characters)",
            level.name,
            level.objects.len(),
            level.lights.len(),
            level.particle_emitters.len(),
            level.rig_instances.len(),
            level.characters.len()
        );
        Ok(())
    }

    /// A compiled-in demo object, not part of the serialized level data —
    /// it's re-added every time `apply_level` rebuilds the world (level
    /// load, or restoring the pre-Play snapshot) rather than being saved as
    /// a `LevelObject`. This is the tradeoff of a native-Rust `Behavior`
    /// versus a script: it can't be authored/persisted as level data, only
    /// compiled in. Tagged with `NativeBehaviorDemoMarker` so
    /// `build_level_from_ecs` knows to skip it — saving the level should
    /// never freeze a copy of it in as an inert, behavior-less cube.
    fn spawn_native_behavior_demo(&mut self, gl: &glow::Context) {
        let obj = LevelObject {
            name: "Native Behavior Cube".to_string(),
            mesh: MeshSource::Primitive(PrimitiveKind::Cube),
            texture_path: Some(PathBuf::from("assets/textures/test_diffuse.png")),
            position: [2.4, 1.0, -2.0],
            rotation_euler_deg: [0.0, 0.0, 0.0],
            scale: [0.6, 0.6, 0.6],
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
                // Two separate `insert_one` calls, not one call with a tuple:
                // `insert_one`'s generic `T` would happily accept a tuple as
                // a single opaque component type instead of unpacking it
                // into two real components, silently breaking the
                // `.without::<&NativeBehaviorDemoMarker>()` filter elsewhere.
                let _ = self.world.insert_one(entity, BehaviorSlot(Box::new(NativeBobBehavior::new(0.4, 2.0))));
                let _ = self.world.insert_one(entity, NativeBehaviorDemoMarker);
            }
            Err(err) => log::error!("failed to spawn native behavior demo cube: {err}"),
        }
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

    /// Reconstructs a `LevelObject` for a single entity — the same
    /// per-entity logic `build_level_from_ecs`'s main loop uses, factored
    /// out so it's also reusable by "duplicate selected"
    /// (`duplicate_selected`). Returns `None` if `entity` has no
    /// `Transform`/`LevelObjectMeta` (e.g. it's a light or particle
    /// emitter, which have their own reconstruction below).
    fn level_object_from_entity(&self, entity: Entity) -> Option<LevelObject> {
        let mut query = self.world.query_one::<(&Transform, &LevelObjectMeta)>(entity).ok()?;
        let (transform, meta) = query.get()?;

        // An animated object's live `Transform` is mid-motion — save the
        // `Animator`'s fixed base pose instead, so saving mid-animation
        // doesn't capture a random instant.
        let position = self
            .world
            .get::<&Animator>(entity)
            .map(|animator| animator.base_position)
            .unwrap_or(transform.position);

        let animation = self.world.get::<&Animator>(entity).ok().map(|animator| match animator.kind {
            AnimationKind::Orbit { axis, speed_deg_per_sec } => {
                AnimationSpec::Orbit { axis: axis.to_array(), speed_deg_per_sec }
            }
            AnimationKind::Bob { axis, amplitude, period_secs } => {
                AnimationSpec::Bob { axis: axis.to_array(), amplitude, period_secs }
            }
        });

        Some(LevelObject {
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
        })
    }

    /// Sibling to `level_object_from_entity` for a placed character.
    fn character_instance_from_entity(&self, entity: Entity) -> Option<CharacterInstance> {
        let mut query = self.world.query_one::<(&Transform, &CharacterMeta)>(entity).ok()?;
        let (transform, meta) = query.get()?;
        let max_health = self.world.get::<&Health>(entity).ok().map(|health| health.max);
        let dialogue_nodes = self.world.get::<&Dialogue>(entity).map(|d| d.nodes.clone()).unwrap_or_default();
        Some(CharacterInstance {
            name: meta.name.clone(),
            position: transform.position.to_array(),
            scale: transform.scale.to_array(),
            color: meta.color,
            texture_path: meta.texture_path.clone(),
            disposition: meta.disposition,
            move_speed: meta.move_speed,
            wander_radius: meta.wander_radius,
            sight_range: meta.sight_range,
            max_health,
            damage: meta.damage,
            attack_range: meta.attack_range,
            attack_cooldown_secs: meta.attack_cooldown_secs,
            dialogue_nodes,
        })
    }

    fn build_level_from_ecs(&self) -> Level {
        let mut objects = Vec::new();
        for (entity, _meta) in self
            .world
            .query::<&LevelObjectMeta>()
            .without::<&Light>()
            .without::<&ParticleEmitter>()
            .without::<&NativeBehaviorDemoMarker>()
            .iter()
        {
            if let Some(obj) = self.level_object_from_entity(entity) {
                objects.push(obj);
            }
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

        let mut characters = Vec::new();
        for (entity, _meta) in self.world.query::<&CharacterMeta>().iter() {
            if let Some(instance) = self.character_instance_from_entity(entity) {
                characters.push(instance);
            }
        }

        let mut spawners = Vec::new();
        for (_entity, (transform, config)) in self.world.query::<(&Transform, &SpawnerConfig)>().iter() {
            spawners.push(SpawnerInstance {
                name: config.name.clone(),
                position: transform.position.to_array(),
                template: config.template.clone(),
                spawn_interval_secs: config.spawn_interval_secs,
                max_alive: config.max_alive,
                total_to_spawn: config.total_to_spawn,
                spawn_radius: config.spawn_radius,
            });
        }

        Level {
            name: self.current_level_name.clone(),
            objects,
            lights,
            particle_emitters,
            rig_instances,
            characters,
            spawners,
            physics: self.physics_params,
            music_path: self.current_music_path.clone(),
        }
    }

    /// Snapshots the currently-loaded level's live ECS state back into
    /// `self.levels`'s cached entry for it (the same `build_level_from_ecs`
    /// round-trip `push_undo_snapshot`/`save_current_level` already use) —
    /// call this just before switching to a *different* level (F2 "Load"
    /// click). `self.levels` is otherwise only populated once at boot, so
    /// without this, any in-session edit that was never explicitly saved to
    /// disk (e.g. an assigned character texture, a moved object) is lost
    /// the moment you swap away from the level and back — `apply_level`
    /// would respawn from the stale boot-time snapshot instead of what's
    /// actually on screen.
    fn sync_current_level_into_cache(&mut self) {
        let level = self.build_level_from_ecs();
        if let Some(existing) = self.levels.iter_mut().find(|l| l.name == level.name) {
            *existing = level;
        } else {
            self.levels.push(level);
        }
    }

    fn save_current_level(&mut self) {
        let level = self.build_level_from_ecs();
        let path = self.levels_dir.join(format!("{}.ron", level.name));
        match engine::level::save_to_file(&level, &path) {
            Ok(()) => {
                log::info!("saved level '{}' to {path:?}", level.name);
                if let Some(existing) = self.levels.iter_mut().find(|l| l.name == level.name) {
                    *existing = level;
                }
            }
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

    const UNDO_STACK_LIMIT: usize = 50;

    /// Snapshots the current level state onto `undo_stack` (reusing the
    /// exact `build_level_from_ecs` round-trip Save/Load already goes
    /// through) and clears `redo_stack` — any new edit invalidates the old
    /// redo history. Call *before* the edit it should let you undo.
    fn push_undo_snapshot(&mut self) {
        self.undo_stack.push(self.build_level_from_ecs());
        if self.undo_stack.len() > Self::UNDO_STACK_LIMIT {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    fn undo(&mut self, gl: &glow::Context) {
        let Some(level) = self.undo_stack.pop() else {
            log::info!("nothing to undo");
            return;
        };
        self.redo_stack.push(self.build_level_from_ecs());
        if let Err(err) = self.apply_level(gl, &level) {
            log::error!("undo failed: {err}");
        }
        // `apply_level` just nulled every selection field — sync the
        // change-detector to match, or next frame's "selection changed"
        // check would fire spuriously and clear the `redo_stack` we just
        // populated above.
        self.last_undo_selection = (None, None, None, None);
    }

    fn redo(&mut self, gl: &glow::Context) {
        let Some(level) = self.redo_stack.pop() else {
            log::info!("nothing to redo");
            return;
        };
        self.undo_stack.push(self.build_level_from_ecs());
        if let Err(err) = self.apply_level(gl, &level) {
            log::error!("redo failed: {err}");
        }
        self.last_undo_selection = (None, None, None, None);
    }

    /// Ctrl+D — duplicates every entity in the active multi-select set(s),
    /// nudged along X so copies never sit flush on top of the original
    /// (the same offset convention `add_primitive` uses for staggering new
    /// objects). The freshly-spawned copies become the new selection.
    fn duplicate_selected(&mut self, gl: &glow::Context) {
        if self.selected_entities.is_empty() && self.selected_characters.is_empty() {
            return;
        }
        self.push_undo_snapshot();

        let object_copies: Vec<LevelObject> = self
            .selected_entities
            .iter()
            .filter_map(|&entity| self.level_object_from_entity(entity))
            .map(|obj| LevelObject {
                name: format!("{} Copy", obj.name),
                position: [obj.position[0] + 1.0, obj.position[1], obj.position[2]],
                ..obj
            })
            .collect();
        let character_copies: Vec<CharacterInstance> = self
            .selected_characters
            .iter()
            .filter_map(|&entity| self.character_instance_from_entity(entity))
            .map(|instance| CharacterInstance {
                name: format!("{} Copy", instance.name),
                position: [instance.position[0] + 1.0, instance.position[1], instance.position[2]],
                ..instance
            })
            .collect();

        self.selected_entities.clear();
        self.selected_characters.clear();
        for obj in &object_copies {
            match self.spawn_level_object(gl, obj) {
                Ok(entity) => {
                    self.selected_entity = Some(entity);
                    self.selected_entities.insert(entity);
                }
                Err(err) => log::error!("failed to duplicate object '{}': {err}", obj.name),
            }
        }
        for instance in &character_copies {
            match self.spawn_character(gl, instance) {
                Ok(entity) => {
                    self.selected_character = Some(entity);
                    self.selected_characters.insert(entity);
                }
                Err(err) => log::error!("failed to duplicate character '{}': {err}", instance.name),
            }
        }
    }

    /// Delete key — removes every entity in whichever multi-select set is
    /// non-empty (batches the existing per-object "Delete" button action).
    fn delete_selected(&mut self) {
        if self.selected_entities.is_empty() && self.selected_characters.is_empty() {
            return;
        }
        self.push_undo_snapshot();
        for entity in self.selected_entities.drain() {
            let _ = self.world.despawn(entity);
        }
        for entity in self.selected_characters.drain() {
            let _ = self.world.despawn(entity);
        }
        self.selected_entity = None;
        self.selected_character = None;
    }

    fn add_primitive(&mut self, gl: &glow::Context, kind: PrimitiveKind) {
        self.push_undo_snapshot();
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
        self.push_undo_snapshot();
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
            .add_filter("3D Models", &["obj", "gltf", "glb"])
            .set_directory(&self.asset_root)
            .pick_file()
        else {
            return;
        };
        self.push_undo_snapshot();

        let is_gltf = model_path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("gltf") || ext.eq_ignore_ascii_case("glb"));

        let texture_path = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg"])
            .set_directory(&self.asset_root)
            .pick_file();
        let mut texture_path = texture_path.map(|path| engine::level::relativize(&path, &self.asset_root));

        let name = model_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Imported".to_string());
        let count = self.world.query::<&LevelObjectMeta>().iter().count();

        let mesh = if is_gltf {
            MeshSource::GltfFile(engine::level::relativize(&model_path, &self.asset_root))
        } else {
            MeshSource::ObjFile(engine::level::relativize(&model_path, &self.asset_root))
        };

        // A glTF's own embedded/referenced base-color texture is a
        // convenience fallback, only used if the user didn't separately
        // pick one above (matches OBJ import's "texture is always its own
        // pick" behavior when one is provided).
        if is_gltf && texture_path.is_none() {
            match load_gltf(&model_path) {
                Ok(scene) => {
                    if let Some(image) = scene.meshes.first().and_then(|entry| entry.image.as_ref()) {
                        let textures_dir = self.asset_root.join("textures");
                        match std::fs::create_dir_all(&textures_dir) {
                            Ok(()) => {
                                let out_path = textures_dir.join(format!("{name}_basecolor.png"));
                                match engine::texture::save_rgba8_png(&out_path, &image.rgba, image.width, image.height) {
                                    Ok(()) => texture_path = Some(engine::level::relativize(&out_path, &self.asset_root)),
                                    Err(err) => log::error!("failed to write extracted glTF texture: {err}"),
                                }
                            }
                            Err(err) => log::error!("failed to create textures dir: {err}"),
                        }
                    }
                }
                Err(err) => log::error!("failed to load glTF for texture extraction: {err}"),
            }
        }

        let obj = LevelObject {
            name,
            mesh,
            texture_path,
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

    /// Sibling to `assign_texture_to_entity` for a placed character.
    fn assign_texture_to_character(&mut self, gl: &glow::Context, entity: Entity) {
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
        if let Ok(mut query) = self.world.query_one::<(&mut MeshRenderer, &mut CharacterMeta)>(entity) {
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
            Health::new(100.0),
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

    /// One-line status the multiplayer panel shows above its buttons.
    fn net_status_text(&self) -> String {
        if self.net_reconnect_attempts_left > 0 {
            return format!("Reconnecting... ({} attempts left)", self.net_reconnect_attempts_left);
        }
        match &self.net_mode {
            NetMode::Offline => "Offline".to_string(),
            NetMode::Host(host) => format!("Hosting ({}/{})", host.player_count(), engine::net::MAX_PLAYERS),
            NetMode::Client(client) if client.is_connected() => {
                format!("Connected to {}", self.net_join_address)
            }
            NetMode::Client(_) => "Disconnected".to_string(),
        }
    }

    /// Multiplayer panel's "Host Game" button.
    fn start_hosting(&mut self) {
        match NetHost::bind(engine::net::DEFAULT_PORT) {
            Ok(mut host) => {
                log::info!("hosting on port {}", engine::net::DEFAULT_PORT);
                // The multiplayer panel only appears while already in Play
                // mode (paused), so `player_entity` is guaranteed to exist
                // here — tag it so it shows up in the (future) snapshot
                // broadcast the same as any other replicated entity.
                if let Some(player) = self.player_entity {
                    let net_id = host.allocate_net_id();
                    let _ = self.world.insert_one(player, Networked(net_id));
                }
                // Tag every character/dynamic prop already in the level
                // too, so they show up in `named_net_ids` for clients that
                // join afterward and in the periodic `Snapshot` broadcast.
                // New ones can only appear afterward via a spawner (F2
                // editing is Edit-mode-only, mutually exclusive with this
                // Play-mode-only panel) — those get tagged individually as
                // they spawn instead, via `assign_net_id_if_hosting`.
                let characters: Vec<Entity> = self.world.query::<&CharacterMeta>().iter().map(|(e, _)| e).collect();
                for entity in characters {
                    let net_id = host.allocate_net_id();
                    let _ = self.world.insert_one(entity, Networked(net_id));
                }
                // Eligible objects are anything `interact()` itself would
                // consider interactable — a `RigidBody` (pushable) or a
                // `BehaviorSlot` (scripted `on_interact`) — mirrored here
                // so a remote client's `Interact` can ever resolve to one
                // via `find_networked_entity` (which only searches
                // `Networked` entities).
                let interactable_objects: Vec<Entity> = self
                    .world
                    .query::<&LevelObjectMeta>()
                    .iter()
                    .filter(|&(entity, _)| {
                        self.world.get::<&RigidBody>(entity).is_ok()
                            || self.world.get::<&BehaviorSlot>(entity).is_ok()
                    })
                    .map(|(e, _)| e)
                    .collect();
                for entity in interactable_objects {
                    let net_id = host.allocate_net_id();
                    let _ = self.world.insert_one(entity, Networked(net_id));
                }
                self.net_mode = NetMode::Host(host);
                self.hud.show_toast("Hosting a game", 2.0);
            }
            Err(err) => {
                log::error!("failed to start hosting: {err}");
                self.hud.show_toast(&format!("Failed to host: {err}"), 3.0);
            }
        }
    }

    /// Multiplayer panel's "Join Game" button.
    fn join_game(&mut self) {
        // A manual join always supersedes any automatic reconnect
        // sequence still counting down in the background.
        self.net_reconnect_attempts_left = 0;
        let Ok(addr) = self.net_join_address.parse() else {
            log::error!("invalid address {:?}", self.net_join_address);
            self.hud.show_toast("Invalid address", 2.0);
            return;
        };
        match NetClient::connect(addr, self.net_player_name.clone(), std::time::Duration::from_secs(3)) {
            Ok(client) => {
                log::info!("connecting to {addr}");
                self.net_mode = NetMode::Client(client);
                self.hud.show_toast(&format!("Connecting to {addr}..."), 2.0);
            }
            Err(err) => {
                log::error!("failed to connect to {addr}: {err}");
                self.hud.show_toast(&format!("Failed to connect: {err}"), 3.0);
            }
        }
    }

    /// Multiplayer panel's "Disconnect" button — drops the host/client
    /// state and clears the replication bookkeeping. Does not touch the
    /// currently-loaded level or player entity; single-player play just
    /// continues locally.
    fn disconnect_net(&mut self) {
        self.net_mode = NetMode::Offline;
        self.net_id_to_entity.clear();
        self.remote_players.clear();
        self.net_dialogue = None;
        self.net_local_target = None;
        self.remote_level_switch_permission.clear();
        // Any call to `disconnect_net` — manual or automatic — cancels a
        // pending reconnect sequence; `update_networking`'s drop-handling
        // re-arms it explicitly right afterward when that's actually
        // what's happening.
        self.net_reconnect_attempts_left = 0;
        self.hud.show_toast("Disconnected", 1.5);
    }

    /// Multiplayer panel's "Cancel" button while an automatic reconnect
    /// sequence is counting down.
    fn cancel_reconnect(&mut self) {
        self.net_reconnect_attempts_left = 0;
        self.hud.show_toast("Reconnect cancelled", 1.5);
    }

    /// Multiplayer panel's per-player "Can switch levels" checkbox
    /// (host-only — see `Sandbox.remote_level_switch_permission`).
    fn set_level_switch_permission(&mut self, net_id: NetId, allowed: bool) {
        self.remote_level_switch_permission.insert(net_id, allowed);
    }

    /// Pumps whichever side of a listen-server session is active, every
    /// frame, regardless of pause state — see `engine::net`'s "never
    /// blocks" framing contract. This pass handles the connection
    /// lifecycle (accept/handshake/disconnect) plus the join bootstrap
    /// (spawning a remote player's avatar, sending/receiving `Welcome`);
    /// snapshot-driven movement/AI/object replication is layered on in
    /// later passes as those systems come online.
    ///
    /// `self.net_mode` is taken out (replaced with a placeholder
    /// `Offline`) for the duration of this call rather than matched on
    /// directly — spawning a remote player or applying a level needs
    /// `&mut self` broadly, which would otherwise conflict with
    /// `host`/`client`'s live borrow of `self.net_mode` itself (the same
    /// "take-before-mutate" shape `self.nav_grid.take()` already uses
    /// elsewhere, extended to a whole enum instead of an `Option`).
    fn update_networking(&mut self, ctx: &mut Context, dt: f32) {
        const NET_RECONNECT_MAX_ATTEMPTS: u32 = 5;
        const NET_RECONNECT_INTERVAL_SECS: f32 = 3.0;

        let mut net_mode = std::mem::replace(&mut self.net_mode, NetMode::Offline);
        let mut go_offline = false;
        let mut start_reconnect = false;

        // Ticks down between automatic reconnect attempts after an
        // unexpected drop (see the `!client.is_connected()` branch
        // below) — reuses `net_join_address`/`net_player_name` and the
        // exact same `NetClient::connect` call `join_game` makes from a
        // button click, just fired on a timer instead. A no-op whenever
        // no reconnect sequence is active.
        if self.net_reconnect_attempts_left > 0 {
            self.net_reconnect_timer -= dt;
            if self.net_reconnect_timer <= 0.0 {
                self.net_reconnect_attempts_left -= 1;
                let attempts_left = self.net_reconnect_attempts_left;
                match self.net_join_address.parse() {
                    Ok(addr) => match NetClient::connect(
                        addr,
                        self.net_player_name.clone(),
                        std::time::Duration::from_secs(2),
                    ) {
                        Ok(client) => {
                            log::info!("reconnected to {addr}");
                            net_mode = NetMode::Client(client);
                            self.net_reconnect_attempts_left = 0;
                            self.hud.show_toast("Reconnected", 2.0);
                        }
                        Err(err) => {
                            log::warn!("reconnect attempt failed: {err}");
                            if attempts_left == 0 {
                                self.hud.show_toast("Could not reconnect", 2.5);
                            } else {
                                self.net_reconnect_timer = NET_RECONNECT_INTERVAL_SECS;
                                self.hud.show_toast(&format!("Reconnecting... ({attempts_left} left)"), 2.0);
                            }
                        }
                    },
                    Err(_) => self.net_reconnect_attempts_left = 0,
                }
            }
        }

        match &mut net_mode {
            NetMode::Offline => {}
            NetMode::Host(host) => {
                for (net_id, name) in host.poll_new_connections() {
                    let gl = ctx.gl();
                    match self.spawn_remote_player_at(gl, net_id, Vec3::new(0.0, 3.0, 4.0), 0.0) {
                        Ok(entity) => {
                            self.remote_players.insert(net_id, entity);
                            let level = self.build_level_from_ecs();
                            let named_net_ids = self.build_named_net_ids();
                            host.send_to(
                                net_id,
                                &ServerMessage::Welcome {
                                    your_net_id: net_id,
                                    level,
                                    named_net_ids,
                                    tick_rate: engine::net::SNAPSHOT_RATE_HZ,
                                },
                            );
                            // Catch the new client up on everyone already in
                            // the session (the host's own player + any
                            // other already-connected remote players) —
                            // they arrived before this client did, so no
                            // `PlayerJoined` broadcast ever reached it.
                            if let Some(local_player) = self.player_entity {
                                if let Ok(networked) = self.world.get::<&Networked>(local_player) {
                                    host.send_to(
                                        net_id,
                                        &ServerMessage::PlayerJoined {
                                            net_id: networked.0,
                                            name: self.net_player_name.clone(),
                                        },
                                    );
                                }
                            }
                            for other_id in host.connected_ids() {
                                if other_id == net_id {
                                    continue;
                                }
                                if let Some(other_name) = host.player_name(other_id) {
                                    host.send_to(
                                        net_id,
                                        &ServerMessage::PlayerJoined { net_id: other_id, name: other_name.to_string() },
                                    );
                                }
                            }
                            host.broadcast(&ServerMessage::PlayerJoined { net_id, name: name.clone() });
                            log::info!("{name} joined as {net_id:?}");
                            self.hud.show_toast(&format!("{name} joined"), 2.0);
                        }
                        Err(err) => log::error!("failed to spawn a remote player for {name}: {err}"),
                    }
                }

                for (net_id, input) in host.poll_inputs() {
                    if let Some(&entity) = self.remote_players.get(&net_id) {
                        self.apply_remote_input(entity, &input);
                    }
                }
                for (requester, target) in host.poll_interacts() {
                    let Some(entity) = self.find_networked_entity(target) else { continue };
                    let has_dialogue = self.world.get::<&Dialogue>(entity).is_ok();
                    if has_dialogue {
                        if let Ok(mut query) = self.world.query_one::<&mut Dialogue>(entity) {
                            if let Some(dialogue) = query.get() {
                                let text = dialogue.current_text().to_string();
                                let choices = dialogue.current_choices().to_vec();
                                if choices.is_empty() {
                                    // Mirrors `interact()`'s local behavior:
                                    // no choices means linear cycling, so
                                    // advance immediately rather than waiting
                                    // on a pick that will never come.
                                    dialogue.advance();
                                }
                                host.send_to(requester, &ServerMessage::Dialogue { speaker: target, text, choices });
                            }
                        }
                        continue;
                    }

                    // A `Hostile` character with `Health` is a fight, not
                    // a push — same rule `interact()` uses locally.
                    // Death handling happens next frame via
                    // `AiEvent::CharacterDied`, the same path a
                    // character's own attack uses.
                    const PLAYER_ATTACK_DAMAGE: f32 = 10.0;
                    let is_hostile = self
                        .world
                        .get::<&CharacterMeta>(entity)
                        .is_ok_and(|meta| meta.disposition == Disposition::Hostile);
                    if is_hostile {
                        let hit = {
                            let mut query = self.world.query_one::<&mut Health>(entity);
                            match query.as_mut().ok().and_then(|q| q.get()) {
                                Some(health) => {
                                    health.damage(PLAYER_ATTACK_DAMAGE);
                                    true
                                }
                                None => false,
                            }
                        };
                        if hit {
                            self.play_tone(180.0, 0.08);
                            if let Ok(position) = self.world.get::<&Transform>(entity).map(|t| t.position) {
                                self.spawn_burst_at(position, ParticleEmitterDef::default(), 8);
                            }
                            continue;
                        }
                    }

                    // A scripted/native-behavior object handles its own
                    // interaction instead of the generic push — mirrors
                    // `interact()`'s own ordering. Only reachable at all
                    // if it was tagged `Networked` while hosting (see
                    // `start_hosting`/`transition_to_level`'s
                    // `interactable_objects` tagging, which includes
                    // `BehaviorSlot` objects specifically so this can
                    // resolve).
                    if self.with_behavior(entity, |behavior, api| behavior.on_interact(api)) {
                        continue;
                    }

                    // Nothing else claimed it — treat it as the
                    // requesting client's own version of `interact()`'s
                    // "push the nearest dynamic object" action, using the
                    // requester's own remote avatar as the push origin
                    // (not the host's own local player). The resulting
                    // `RigidBody.velocity` change reaches every client
                    // for free via the next periodic `Snapshot` — no
                    // extra broadcast needed.
                    let Some(&requester_entity) = self.remote_players.get(&requester) else { continue };
                    let Some(requester_position) =
                        self.world.get::<&Transform>(requester_entity).ok().map(|t| t.position)
                    else {
                        continue;
                    };
                    let Some(controller) =
                        self.world.get::<&PlayerController>(requester_entity).ok().map(|c| *c)
                    else {
                        continue;
                    };
                    let mut pushed = false;
                    if let Ok(mut query) = self.world.query_one::<(&Transform, &mut RigidBody)>(entity) {
                        if let Some((transform, body)) = query.get() {
                            let mut push_dir = transform.position - requester_position;
                            push_dir.y = 0.0;
                            let push_dir = push_dir.normalize_or_zero();
                            let push_dir = if push_dir == Vec3::ZERO { Vec3::X } else { push_dir };
                            body.velocity += push_dir * controller.interact_impulse
                                + Vec3::Y * (controller.interact_impulse * 0.3);
                            pushed = true;
                        }
                    }
                    if pushed {
                        self.play_tone(220.0, 0.1);
                        if let Ok(position) = self.world.get::<&Transform>(entity).map(|t| t.position) {
                            self.spawn_burst_at(
                                position,
                                ParticleEmitterDef { rate_per_sec: 0.0, ..ParticleEmitterDef::default() },
                                12,
                            );
                        }
                    }
                }
                for (requester, speaker, index) in host.poll_dialogue_choices() {
                    let Some(entity) = self.find_networked_entity(speaker) else { continue };
                    if let Ok(mut query) = self.world.query_one::<&mut Dialogue>(entity) {
                        if let Some(dialogue) = query.get() {
                            if dialogue.choose(index) {
                                let text = dialogue.current_text().to_string();
                                let choices = dialogue.current_choices().to_vec();
                                host.send_to(requester, &ServerMessage::Dialogue { speaker, text, choices });
                            }
                        }
                    }
                }
                for net_id in host.take_disconnected() {
                    if let Some(entity) = self.remote_players.remove(&net_id) {
                        let _ = self.world.despawn(entity);
                    }
                    self.remote_level_switch_permission.remove(&net_id);
                    host.broadcast(&ServerMessage::PlayerLeft { net_id });
                    log::info!("{net_id:?} disconnected");
                    self.hud.show_toast("A player disconnected", 2.0);
                }

                if host.should_send_snapshot(dt) {
                    // Keep the host's own avatar's facing in sync with its
                    // camera — the local player never otherwise touches its
                    // own `Transform.rotation` (first-person, no mesh to
                    // rotate), so without this every `PlayerSnapshot` for
                    // the host would report a stale/identity yaw.
                    if let Some(local_player) = self.player_entity {
                        if let Ok(mut transform) = self.world.get::<&mut Transform>(local_player) {
                            transform.rotation = Quat::from_rotation_y(self.fp_camera.yaw);
                        }
                    }

                    let mut players = Vec::new();
                    for (_entity, (transform, networked, _controller, health)) in self
                        .world
                        .query::<(&Transform, &Networked, &PlayerController, Option<&Health>)>()
                        .iter()
                    {
                        let (yaw, _, _) = transform.rotation.to_euler(engine::glam::EulerRot::YXZ);
                        players.push(PlayerSnapshot {
                            net_id: networked.0,
                            position: transform.position.to_array(),
                            yaw_deg: yaw.to_degrees(),
                            health: health.map(|h| (h.current, h.max)),
                        });
                    }
                    let mut characters = Vec::new();
                    for (_entity, (transform, networked, _meta, health)) in self
                        .world
                        .query::<(&Transform, &Networked, &CharacterMeta, Option<&Health>)>()
                        .iter()
                    {
                        characters.push(CharacterSnapshot {
                            net_id: networked.0,
                            position: transform.position.to_array(),
                            health: health.map(|h| (h.current, h.max)),
                        });
                    }

                    let mut objects = Vec::new();
                    for (_entity, (transform, networked, _meta)) in
                        self.world.query::<(&Transform, &Networked, &LevelObjectMeta)>().iter()
                    {
                        objects.push(ObjectSnapshot {
                            net_id: networked.0,
                            position: transform.position.to_array(),
                            rotation: transform.rotation.to_array(),
                        });
                    }

                    host.broadcast(&ServerMessage::Snapshot {
                        tick: 0,
                        players,
                        characters,
                        objects,
                        removed: std::mem::take(&mut self.net_removed),
                    });
                }
            }
            NetMode::Client(client) => {
                for msg in client.poll_messages() {
                    match msg {
                        ServerMessage::Reject { reason } => {
                            log::error!("host rejected connection: {reason}");
                            self.hud.show_toast(&format!("Connection rejected: {reason}"), 3.0);
                            go_offline = true;
                        }
                        ServerMessage::Welcome { your_net_id, level, named_net_ids, .. } => {
                            let gl = ctx.gl();
                            if let Err(err) = self.apply_level(gl, &level) {
                                log::error!("failed to apply the host's level: {err}");
                            }
                            self.net_id_to_entity.clear();
                            for (name, net_id) in named_net_ids {
                                if let Some(entity) = self.find_entity_by_name(&name) {
                                    self.net_id_to_entity.insert(net_id, entity);
                                    // Without this, entities that already
                                    // existed in the level at join time
                                    // (as opposed to arriving later via
                                    // `CharacterSpawned`) are tracked in
                                    // `net_id_to_entity` but never actually
                                    // tagged `Networked` — `interact_or_send`'s
                                    // nearest-target search queries for
                                    // `&Networked`, so it silently found
                                    // nothing for any pre-placed NPC.
                                    let _ = self.world.insert_one(entity, Networked(net_id));
                                }
                            }
                            // `apply_level` just cleared the whole world,
                            // including whatever single-player
                            // `player_entity` existed before — spawn a
                            // fresh one for this client's own avatar. Its
                            // position/rotation are host-authoritative from
                            // here on (see the `Snapshot` handling below);
                            // only look direction (`fp_camera`) stays local.
                            let position = Vec3::new(0.0, 3.0, 4.0);
                            let entity = self.world.spawn((
                                Transform { position, rotation: Quat::IDENTITY, scale: Vec3::ONE },
                                RigidBody::default(),
                                Collider { shape: ColliderShape::Sphere { radius: 0.4 }, is_trigger: false },
                                PlayerController::default(),
                                Health::new(100.0),
                                Networked(your_net_id),
                            ));
                            self.player_entity = Some(entity);
                            self.net_id_to_entity.insert(your_net_id, entity);
                            self.fp_camera = FirstPersonCamera::new();
                            self.resume(ctx);
                            log::info!("joined as {your_net_id:?}");
                            self.hud.show_toast("Joined the game", 2.0);
                        }
                        ServerMessage::PlayerJoined { net_id, name } => {
                            if Some(net_id) != client.my_net_id() && !self.net_id_to_entity.contains_key(&net_id) {
                                let gl = ctx.gl();
                                match self.spawn_remote_player_at(gl, net_id, Vec3::new(0.0, 3.0, 4.0), 0.0) {
                                    Ok(entity) => {
                                        self.net_id_to_entity.insert(net_id, entity);
                                        log::info!("{name} joined ({net_id:?})");
                                    }
                                    Err(err) => log::error!("failed to spawn an avatar for {name}: {err}"),
                                }
                            }
                        }
                        ServerMessage::PlayerLeft { net_id } => {
                            if let Some(entity) = self.net_id_to_entity.remove(&net_id) {
                                let _ = self.world.despawn(entity);
                            }
                        }
                        ServerMessage::CharacterSpawned { net_id, instance } => {
                            let gl = ctx.gl();
                            match self.spawn_character(gl, &instance) {
                                Ok(entity) => {
                                    let _ = self.world.insert_one(entity, Networked(net_id));
                                    self.net_id_to_entity.insert(net_id, entity);
                                }
                                Err(err) => log::error!("failed to spawn '{}': {err}", instance.name),
                            }
                        }
                        ServerMessage::Snapshot { players, characters, objects, removed, .. } => {
                            for player in players {
                                let Some(&entity) = self.net_id_to_entity.get(&player.net_id) else {
                                    continue;
                                };
                                if Some(entity) == self.player_entity {
                                    // Stored as a smoothing target, not
                                    // applied directly — see
                                    // `net_local_target`'s doc comment
                                    // and `update_player_input`'s
                                    // per-frame blend.
                                    self.net_local_target = Some(Vec3::from(player.position));
                                } else {
                                    let _ = self.world.insert_one(
                                        entity,
                                        NetTarget {
                                            position: Vec3::from(player.position),
                                            rotation: Some(Quat::from_rotation_y(player.yaw_deg.to_radians())),
                                        },
                                    );
                                }
                                if let Some((current, max)) = player.health {
                                    if let Ok(mut health) = self.world.get::<&mut Health>(entity) {
                                        health.current = current;
                                        health.max = max;
                                    }
                                }
                            }
                            for character in characters {
                                let Some(&entity) = self.net_id_to_entity.get(&character.net_id) else {
                                    continue;
                                };
                                let _ = self.world.insert_one(
                                    entity,
                                    NetTarget { position: Vec3::from(character.position), rotation: None },
                                );
                                if let Some((current, max)) = character.health {
                                    if let Ok(mut health) = self.world.get::<&mut Health>(entity) {
                                        health.current = current;
                                        health.max = max;
                                    }
                                }
                            }
                            for object in objects {
                                let Some(&entity) = self.net_id_to_entity.get(&object.net_id) else {
                                    continue;
                                };
                                let _ = self.world.insert_one(
                                    entity,
                                    NetTarget {
                                        position: Vec3::from(object.position),
                                        rotation: Some(Quat::from_array(object.rotation)),
                                    },
                                );
                            }
                            for net_id in removed {
                                if let Some(entity) = self.net_id_to_entity.remove(&net_id) {
                                    let _ = self.world.despawn(entity);
                                }
                            }
                        }
                        ServerMessage::Dialogue { speaker, text, choices } => {
                            self.hud.show_toast(&text, 3.0);
                            self.net_dialogue = if choices.is_empty() { None } else { Some((speaker, text, choices)) };
                        }
                        ServerMessage::DialogueClosed => {
                            self.net_dialogue = None;
                        }
                        ServerMessage::LevelTransition { level, spawn_position, spawn_yaw_deg, named_net_ids } => {
                            let gl = ctx.gl();
                            if let Err(err) = self.apply_level(gl, &level) {
                                log::error!("failed to apply the host's level transition: {err}");
                            }
                            self.net_id_to_entity.clear();
                            for (name, net_id) in named_net_ids {
                                if let Some(entity) = self.find_entity_by_name(&name) {
                                    self.net_id_to_entity.insert(net_id, entity);
                                    let _ = self.world.insert_one(entity, Networked(net_id));
                                }
                            }
                            // Same "spawn a fresh local avatar, position/
                            // rotation host-authoritative from here" shape
                            // as the initial `Welcome` handling — `apply_level`
                            // just wiped the old one. Position is a
                            // deterministic offset from the shared spawn
                            // point (see `spawn_ring_offset`) computed
                            // identically host- and client-side from just
                            // this client's own `NetId`, so no extra data
                            // needs to travel over the wire for it.
                            if let Some(my_net_id) = client.my_net_id() {
                                let (position, yaw_deg) =
                                    spawn_ring_offset(Vec3::from(spawn_position), spawn_yaw_deg, my_net_id);
                                let entity = self.world.spawn((
                                    Transform { position, rotation: Quat::IDENTITY, scale: Vec3::ONE },
                                    RigidBody::default(),
                                    Collider { shape: ColliderShape::Sphere { radius: 0.4 }, is_trigger: false },
                                    PlayerController::default(),
                                    Health::new(100.0),
                                    Networked(my_net_id),
                                ));
                                self.player_entity = Some(entity);
                                self.net_id_to_entity.insert(my_net_id, entity);
                                self.fp_camera.pitch = 0.0;
                                self.fp_camera.yaw = yaw_deg.to_radians();
                            }
                            self.hud.show_toast(&format!("Entering {}", level.name), 1.5);
                        }
                        ServerMessage::ParticleBurst { position, def, count } => {
                            self.spawn_burst_at(Vec3::from(position), def, count);
                        }
                    }
                }
                if !client.is_connected() {
                    log::warn!("disconnected from host — will attempt to reconnect");
                    go_offline = true;
                    start_reconnect = true;
                }
            }
        }

        self.net_mode = net_mode;
        if go_offline {
            self.disconnect_net();
            if start_reconnect {
                self.net_reconnect_attempts_left = NET_RECONNECT_MAX_ATTEMPTS;
                self.net_reconnect_timer = NET_RECONNECT_INTERVAL_SECS;
                self.hud.show_toast("Connection lost — reconnecting...", 2.0);
            }
            // Surface the disconnect immediately rather than leaving the
            // player wondering why nothing else is moving anymore —
            // pausing (if not already) brings up the pause menu, and with
            // it the multiplayer panel showing the current status
            // ("Reconnecting..." or "Offline"). No host migration: if
            // the host quits, the session still ends for everyone.
            if !self.paused {
                self.pause(ctx);
            }
        }
    }

    /// Client-side: nudges every `NetTarget`-tagged entity's rendered
    /// `Transform` toward its latest `Snapshot` value instead of snapping
    /// to it instantly — see `NetTarget`'s doc comment for why. A no-op
    /// in single-player/hosting, since nothing there is ever tagged
    /// `NetTarget` (only client-side `Snapshot` handling inserts one).
    fn smooth_networked_transforms(&mut self, dt: f32) {
        const NET_REMOTE_SMOOTH_RATE: f32 = 15.0;
        let alpha = (NET_REMOTE_SMOOTH_RATE * dt).min(1.0);
        for (_entity, (transform, target)) in self.world.query::<(&mut Transform, &NetTarget)>().iter() {
            transform.position = transform.position.lerp(target.position, alpha);
            if let Some(rotation) = target.rotation {
                transform.rotation = transform.rotation.slerp(rotation, alpha);
            }
        }
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
        let player_health = self.world.get::<&Health>(player).ok().map(|health| (health.current, health.max));
        let data = SaveData {
            level_name: self.current_level_name.clone(),
            player_position: position.to_array(),
            player_yaw: self.fp_camera.yaw,
            player_pitch: self.fp_camera.pitch,
            saved_at_elapsed: self.elapsed_time,
            player_health,
            despawned_names: self.despawned_since_load.clone(),
            extra: std::collections::HashMap::new(),
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
            if let Some((current, max)) = data.player_health {
                let _ = self.world.insert_one(player, Health { current, max });
            }
        }
        self.fp_camera.yaw = data.player_yaw;
        self.fp_camera.pitch = data.player_pitch;

        // Re-despawn whatever was already removed at save time —
        // `apply_level` just freshly respawned it.
        for name in &data.despawned_names {
            let mut to_despawn = None;
            for (entity, meta) in self.world.query::<&LevelObjectMeta>().iter() {
                if &meta.name == name {
                    to_despawn = Some(entity);
                    break;
                }
            }
            if to_despawn.is_none() {
                for (entity, meta) in self.world.query::<&CharacterMeta>().iter() {
                    if &meta.name == name {
                        to_despawn = Some(entity);
                        break;
                    }
                }
            }
            if let Some(entity) = to_despawn {
                self.despawned_since_load.push(name.clone());
                let _ = self.world.despawn(entity);
            }
        }

        log::info!("checkpoint loaded from {path:?}");
        self.hud.show_toast("Checkpoint loaded", 1.5);
    }

    /// This frame's camera-relative horizontal move direction (WASD +
    /// left-stick, blended, normalized, Y always 0) — factored out of
    /// `update_player_input` so a `NetMode::Client` can send the exact
    /// same world-space direction to the host via `ClientMessage::Input`
    /// instead of re-deriving movement from raw key state a second way.
    fn compute_move_dir(&self, ctx: &Context) -> Vec3 {
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
        move_dir.normalize_or_zero()
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

        let right = self.fp_camera.right();
        let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
        let move_dir = self.compute_move_dir(ctx);

        // How much of this frame's movement is sideways, -1 (left) to 1
        // (right) — reused as-is for the strafe-tilt target rather than
        // tracking A/D separately, so diagonal movement tilts proportionally.
        let strafe_input = move_dir.dot(right_flat);
        self.strafe_tilt.update(strafe_input, dt);
        self.fp_camera.roll = self.strafe_tilt.roll_radians();
        self.camera_shake.tick(dt);
        self.landing_dip.update(dt);

        let mut jumped = false;
        if let Ok(mut query) = self.world.query_one::<(&mut Transform, &mut RigidBody, &PlayerController)>(player) {
            if let Some((transform, body, controller)) = query.get() {
                let horizontal_velocity = move_dir * controller.move_speed;
                body.velocity.x = horizontal_velocity.x;
                body.velocity.z = horizontal_velocity.z;
                // Under `NetMode::Client`, `physics::step` never runs
                // locally (see `Game::update`'s `is_client` gate), so
                // without this the camera's eye position (`player_eye_position`
                // reads `Transform.position` directly) would only ever
                // advance once per incoming `Snapshot` — 20Hz — making
                // movement look stepped/frozen every frame in between.
                // Dead reckoning smooths that out for X/Z; a gentle blend
                // toward the latest `Snapshot` (all three axes, since Y
                // has no local prediction of its own) corrects any drift
                // every frame instead of either freezing between
                // snapshots or hard-snapping on arrival — a hard snap
                // was tried first and reproduced a visible wobble every
                // ~50ms, since even tiny natural client/host timing
                // differences show up as a full pop that way. See
                // `net_local_target`'s doc comment.
                if matches!(self.net_mode, NetMode::Client(_)) {
                    transform.position.x += horizontal_velocity.x * dt;
                    transform.position.z += horizontal_velocity.z * dt;
                    if let Some(target) = self.net_local_target {
                        const NET_LOCAL_SMOOTH_RATE: f32 = 10.0;
                        let alpha = (NET_LOCAL_SMOOTH_RATE * dt).min(1.0);
                        // X/Z: dead reckoning above already tracks the
                        // host closely in the common case — continuously
                        // blending toward a target that's merely a few
                        // frames stale (ordinary network latency, not a
                        // real desync) fights the player's own forward
                        // momentum every frame and reads as a persistent
                        // wobble/drag instead of smooth motion. Only
                        // reconcile once the gap exceeds a small
                        // tolerance, which only happens on a genuine
                        // desync (e.g. the host stopped the player at a
                        // wall the local prediction doesn't know about).
                        const NET_LOCAL_XZ_DEADZONE: f32 = 0.3;
                        let xz_error = Vec3::new(target.x - transform.position.x, 0.0, target.z - transform.position.z);
                        if xz_error.length() > NET_LOCAL_XZ_DEADZONE {
                            transform.position.x += xz_error.x * alpha;
                            transform.position.z += xz_error.z * alpha;
                        }
                        // Y (jump/fall) has no local prediction to fight
                        // — always blended, not gated behind a
                        // tolerance, or it'd reintroduce the choppy
                        // step-then-catch-up motion this replaced.
                        transform.position.y += (target.y - transform.position.y) * alpha;
                    }
                }
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

    /// `E`/gamepad-X dispatcher — offline or hosting, runs the normal local
    /// `interact()` below unchanged (the host is always authoritative for
    /// its own actions). Under `NetMode::Client`, every flavor of
    /// interaction replicates — dialogue, hitting a hostile character,
    /// a scripted object's own `on_interact`, and pushing a dynamic
    /// object: it searches the client's own locally-mirrored `Networked`
    /// characters/objects for the nearest one in range — the same
    /// combined search `interact()` does locally, just against
    /// replicated positions — and sends `ClientMessage::Interact` instead
    /// of touching any component directly, since the actual state (a
    /// `Dialogue`'s current node, a `Health`, a pushed object's
    /// `RigidBody.velocity`) lives host-side only. The host resolves
    /// which flavor applies (see `update_networking`'s `poll_interacts`
    /// handling) using the exact same dispatch order `interact()` uses.
    fn interact_or_send(&mut self) {
        if self.net_dialogue.is_some() {
            return;
        }
        let NetMode::Client(client) = &mut self.net_mode else {
            self.interact();
            return;
        };
        let Some(player) = self.player_entity else { return };
        let Some(player_position) = self.world.get::<&Transform>(player).ok().map(|t| t.position) else {
            return;
        };
        let Some(controller) = self.world.get::<&PlayerController>(player).ok().map(|c| *c) else {
            return;
        };

        let mut nearest: Option<(NetId, f32)> = None;
        for (_entity, (transform, networked, _meta)) in
            self.world.query::<(&Transform, &Networked, &CharacterMeta)>().iter()
        {
            let distance = transform.position.distance(player_position);
            if distance <= controller.interact_radius && nearest.is_none_or(|(_, best)| distance < best) {
                nearest = Some((networked.0, distance));
            }
        }
        for (_entity, (transform, networked, _meta)) in
            self.world.query::<(&Transform, &Networked, &LevelObjectMeta)>().iter()
        {
            let distance = transform.position.distance(player_position);
            if distance <= controller.interact_radius && nearest.is_none_or(|(_, best)| distance < best) {
                nearest = Some((networked.0, distance));
            }
        }
        if let Some((target, _)) = nearest {
            client.send_interact(target);
        }
    }

    /// Host-side: finds the entity currently tagged with `net_id`, if any
    /// — a linear scan, fine at this engine's entity-count scale (see the
    /// plan's stated simplifications for `NetId` lookups in general).
    fn find_networked_entity(&self, net_id: NetId) -> Option<Entity> {
        self.world.query::<&Networked>().iter().find(|(_, networked)| networked.0 == net_id).map(|(e, _)| e)
    }

    /// Pushes the nearest dynamic level object within reach with an outward
    /// impulse — the example "interact with the environment" action, tying
    /// the physics engine and the input hook together. Replace this with
    /// your own game's interaction (pickup, dialogue, open a door, ...).
    fn interact(&mut self) {
        // A choice prompt is already waiting on a number-key pick — resolve
        // that first rather than letting E re-trigger the same dialogue (or
        // push/attack whatever's now nearest) out from under it.
        if self.active_dialogue.is_some() {
            return;
        }
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

        // Characters aren't `LevelObjectMeta` objects, so they need their
        // own scan folded into the same `nearest` search.
        for (entity, transform) in self.world.query::<&Transform>().with::<&CharacterMeta>().iter() {
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

        if let Ok(mut query) = self.world.query_one::<&mut Dialogue>(entity) {
            if let Some(dialogue) = query.get() {
                let line = dialogue.current_text().to_string();
                let has_choices = !dialogue.current_choices().is_empty();
                self.hud.show_toast(&line, 3.0);
                if has_choices {
                    // Wait for a number-key pick (see `handle_event`'s
                    // `Keycode::Num1..=Num9` arm) instead of advancing —
                    // `Dialogue::advance` is a no-op on a choice node
                    // anyway, but staying explicit here documents why.
                    self.active_dialogue = Some(entity);
                } else {
                    dialogue.advance();
                }
                log::info!("talked to a character {distance:.2}m away");
                return;
            }
        }

        // A `Hostile` character with `Health` is a fight, not a push — the
        // only player-facing damage source in this vertical slice, reusing
        // the interact key rather than adding a dedicated attack input.
        // Actual death handling (despawn/particles/tone) happens next frame
        // via `engine::ai::step`'s `AiEvent::CharacterDied`, the same path
        // a character's own attacks use to notice the player died.
        const PLAYER_ATTACK_DAMAGE: f32 = 10.0;
        let is_hostile = self
            .world
            .get::<&CharacterMeta>(entity)
            .is_ok_and(|meta| meta.disposition == Disposition::Hostile);
        if is_hostile {
            let hit = {
                let mut query = self.world.query_one::<&mut Health>(entity);
                match query.as_mut().ok().and_then(|q| q.get()) {
                    Some(health) => {
                        health.damage(PLAYER_ATTACK_DAMAGE);
                        true
                    }
                    None => false,
                }
            };
            if hit {
                log::info!("hit a hostile character {distance:.2}m away for {PLAYER_ATTACK_DAMAGE}");
                self.play_tone(180.0, 0.08);
                if let Ok(position) = self.world.get::<&Transform>(entity).map(|t| t.position) {
                    self.spawn_burst_at(position, ParticleEmitterDef::default(), 8);
                }
                return;
            }
        }

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

    /// Number-key handler for the dialogue choice-picker (see
    /// `handle_event`'s `Keycode::Num1..=Num9` arm) — `index` is 0-based.
    /// Clears `active_dialogue` regardless of whether the pick was valid,
    /// since the overlay's choices were only ever for that one prompt.
    fn choose_dialogue_option(&mut self, index: usize) {
        let Some(entity) = self.active_dialogue.take() else { return };
        if let Ok(mut query) = self.world.query_one::<&mut Dialogue>(entity) {
            if let Some(dialogue) = query.get() {
                if dialogue.choose(index) {
                    let line = dialogue.current_text().to_string();
                    self.hud.show_toast(&line, 3.0);
                }
            }
        }
    }

    /// Client-side sibling to `choose_dialogue_option` — sends the pick to
    /// the host instead of resolving it locally, since under
    /// `NetMode::Client` the `Dialogue` component itself lives host-side
    /// only. Clears `net_dialogue` regardless of connection state, same
    /// "the prompt was only ever for this one pick" reasoning.
    fn choose_net_dialogue_option(&mut self, index: usize) {
        let Some((speaker, _, _)) = self.net_dialogue.take() else { return };
        if let NetMode::Client(client) = &mut self.net_mode {
            client.send_dialogue_choice(speaker, index);
        }
    }

    /// Draws the choice-picker overlay while `active_dialogue` (offline/
    /// host) or `net_dialogue` (client) is set — bottom-center, numbered to
    /// match the 1-9 key hints. A no-op (and self-healing) if the character
    /// despawned or its current node no longer has choices; `active_dialogue`
    /// itself is only cleared by `choose_dialogue_option`, so a stale
    /// reference just draws nothing here until the next successful/failed
    /// pick clears it.
    fn draw_dialogue_choices(&self, egui_ctx: &egui::Context) {
        let choices: Vec<(String, usize)> = if let Some(entity) = self.active_dialogue {
            let Ok(dialogue) = self.world.get::<&Dialogue>(entity) else { return };
            dialogue.current_choices().to_vec()
        } else if let Some((_, _, choices)) = &self.net_dialogue {
            choices.clone()
        } else {
            return;
        };
        if choices.is_empty() {
            return;
        }
        egui::Area::new("dialogue_choices".into())
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -60.0))
            .show(egui_ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    for (i, (label, _)) in choices.iter().enumerate() {
                        ui.label(format!("{}. {label}", i + 1));
                    }
                });
            });
    }

    /// Spawns a transient, self-cleaning burst of particles at `position` —
    /// no `LevelObjectMeta` (so it never shows up in Scene Objects or gets
    /// saved as level data) and no mesh/texture of its own, just a
    /// `ParticleEmitter` that `engine::particles::step` despawns once every
    /// particle it made has aged out (see `Game::update`). Ties particles +
    /// physics + the `interact` hook together, the same demo spirit as the
    /// jump/push tones.
    ///
    /// If hosting, also broadcasts the burst to every connected client —
    /// a purely cosmetic effect like this has no `Networked` entity of
    /// its own to ride along on a `Snapshot`, so without this, particle
    /// feedback for a network-triggered action (a client pushing an
    /// object, a hostile character dying) would only ever show up on the
    /// *host's* screen. Every call site gets this for free, host-only
    /// (`broadcast_if_hosting` is a silent no-op offline/as a client);
    /// the client-side `ServerMessage::ParticleBurst` handler calls this
    /// same function to actually spawn the effect, so there's no
    /// separate client-side particle-spawning path to keep in sync.
    fn spawn_burst_at(&mut self, position: Vec3, def: ParticleEmitterDef, count: u32) {
        let mut emitter = ParticleEmitter::new(def);
        emitter.spawn_burst(position, count);
        self.world.spawn((Transform { position, rotation: Quat::IDENTITY, scale: Vec3::ONE }, emitter));
        self.broadcast_if_hosting(&ServerMessage::ParticleBurst { position: position.to_array(), def, count });
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
            Health::new(100.0),
        ));
        self.player_entity = Some(entity);
        self.fp_camera.pitch = 0.0;
        self.fp_camera.yaw = transition.spawn_yaw_deg.to_radians();
        // The old trigger entities this set refers to no longer exist —
        // leaving it stale risks a dead `Entity` aliasing a freshly-spawned
        // one after `world.clear()` resets id generations.
        self.trigger_overlaps.clear();
        self.hud.show_toast(&format!("Entering {}", level.name), 1.5);

        // Only the host's own local player can trigger a transition at all
        // (a remote player physically overlapping the same trigger in the
        // host's simulation is never checked — see the plan's stated
        // simplifications), so if hosting, this is always host-initiated:
        // `apply_level` above just wiped every remote player's avatar and
        // every character/object's `Networked` tag along with everything
        // else — rebuild both and tell every connected client to follow.
        let mut net_mode = std::mem::replace(&mut self.net_mode, NetMode::Offline);
        if let NetMode::Host(host) = &mut net_mode {
            let net_id = host.allocate_net_id();
            let _ = self.world.insert_one(entity, Networked(net_id));

            let characters: Vec<Entity> = self.world.query::<&CharacterMeta>().iter().map(|(e, _)| e).collect();
            for char_entity in characters {
                let net_id = host.allocate_net_id();
                let _ = self.world.insert_one(char_entity, Networked(net_id));
            }
            let interactable_objects: Vec<Entity> = self
                .world
                .query::<&LevelObjectMeta>()
                .iter()
                .filter(|&(entity, _)| {
                    self.world.get::<&RigidBody>(entity).is_ok() || self.world.get::<&BehaviorSlot>(entity).is_ok()
                })
                .map(|(e, _)| e)
                .collect();
            for obj_entity in interactable_objects {
                let net_id = host.allocate_net_id();
                let _ = self.world.insert_one(obj_entity, Networked(net_id));
            }

            let remote_ids = host.connected_ids();
            self.remote_players.clear();
            for remote_id in remote_ids {
                match self.spawn_remote_player_at(
                    gl,
                    remote_id,
                    Vec3::from(transition.spawn_position),
                    transition.spawn_yaw_deg,
                ) {
                    Ok(remote_entity) => {
                        self.remote_players.insert(remote_id, remote_entity);
                    }
                    Err(err) => log::error!("failed to respawn a remote player after transition: {err}"),
                }
            }

            let named_net_ids = self.build_named_net_ids();
            host.broadcast(&ServerMessage::LevelTransition {
                level,
                spawn_position: transition.spawn_position,
                spawn_yaw_deg: transition.spawn_yaw_deg,
                named_net_ids,
            });
        }
        self.net_mode = net_mode;
    }

    /// Host-side: permitted remote players (see
    /// `Sandbox.remote_level_switch_permission` and the multiplayer
    /// panel's per-player "Can switch levels" checkbox) can also trigger
    /// a level transition — checked separately from the host's own
    /// trigger-overlap bookkeeping in `Game::update`
    /// (`trigger_overlaps`/`on_trigger_entered`), since firing the full
    /// generic trigger dispatch for a remote player's overlap would
    /// wrongly flash/shake the *host's* own screen and log/play a sound
    /// for something only the remote player is near. No enter/exit
    /// diffing here (unlike `trigger_overlaps`) — mirrors the same
    /// acceptable one-frame re-trigger risk the host's own transition
    /// already has (`trigger_overlaps` is cleared by
    /// `transition_to_level` too), since a transition wipes the whole
    /// world anyway. Returns `true` if a transition fired.
    fn check_remote_level_switch_triggers(&mut self, gl: &glow::Context, overlaps: &[(Entity, Entity)]) -> bool {
        if !matches!(self.net_mode, NetMode::Host(_)) {
            return false;
        }
        let permitted_entered: Vec<Entity> = overlaps
            .iter()
            .filter_map(|&(dynamic, trigger)| {
                self.remote_players
                    .iter()
                    .find(|&(_, &entity)| entity == dynamic)
                    .map(|(&net_id, _)| (net_id, trigger))
            })
            .filter(|(net_id, _)| self.remote_level_switch_permission.get(net_id).copied().unwrap_or(false))
            .map(|(_, trigger)| trigger)
            .collect();
        for trigger in permitted_entered {
            let transition =
                self.world.get::<&LevelObjectMeta>(trigger).ok().and_then(|meta| meta.level_transition.clone());
            if let Some(transition) = transition {
                self.transition_to_level(gl, &transition);
                return true;
            }
        }
        false
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
        // Bundles a whole slider-drag editing session on one object into a
        // single undo step: the moment the selection changes to something
        // else, snapshot the state as it stood *before* that new object's
        // edits begin.
        let current_selection = (self.selected_entity, self.selected_rig, self.selected_character, self.selected_class);
        if current_selection != self.last_undo_selection {
            self.push_undo_snapshot();
            self.last_undo_selection = current_selection;
        }

        ui.heading("Level");
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.undo_stack.is_empty(), egui::Button::new("Undo (Ctrl+Z)")).clicked() {
                self.undo(gl);
            }
            if ui.add_enabled(!self.redo_stack.is_empty(), egui::Button::new("Redo (Ctrl+Shift+Z)")).clicked() {
                self.redo(gl);
            }
        });
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
                self.sync_current_level_into_cache();
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
            self.push_undo_snapshot();
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
            self.push_undo_snapshot();
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
        if self.selected_entities.len() > 1 {
            ui.small(format!("{} selected (Ctrl+click to multi-select, Ctrl+D to duplicate, Del to delete)", self.selected_entities.len()));
        }
        let mut clicked_entity = None;
        for (entity, meta) in self.world.query::<&LevelObjectMeta>().iter() {
            let selected = self.selected_entities.contains(&entity);
            if ui.selectable_label(selected, &meta.name).clicked() {
                clicked_entity = Some(entity);
            }
        }
        if let Some(entity) = clicked_entity {
            self.selected_entity = Some(entity);
            if ui.input(|i| i.modifiers.ctrl) {
                if !self.selected_entities.remove(&entity) {
                    self.selected_entities.insert(entity);
                }
            } else {
                self.selected_entities.clear();
                self.selected_entities.insert(entity);
            }
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
        if ui.button("Import glTF as Rig...").clicked() {
            self.import_gltf_as_rig(gl);
        }
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
        ui.heading("Characters");
        if ui.button("Add Character").clicked() {
            self.push_undo_snapshot();
            let count = self.world.query::<&CharacterMeta>().iter().count();
            let instance = CharacterInstance {
                name: format!("Character_{}", count + 1),
                position: [0.0, 1.0, 0.0],
                scale: [0.8, 1.6, 0.8],
                color: [0.8, 0.2, 0.2],
                texture_path: None,
                disposition: Disposition::Passive,
                move_speed: 2.0,
                wander_radius: 3.0,
                sight_range: 6.0,
                max_health: None,
                damage: None,
                attack_range: 1.2,
                attack_cooldown_secs: 1.0,
                dialogue_nodes: Vec::new(),
            };
            match self.spawn_character(gl, &instance) {
                Ok(entity) => self.selected_character = Some(entity),
                Err(err) => log::error!("failed to add character: {err}"),
            }
        }
        if self.selected_characters.len() > 1 {
            ui.small(format!("{} selected (Ctrl+click to multi-select, Ctrl+D to duplicate, Del to delete)", self.selected_characters.len()));
        }
        let mut clicked_character = None;
        for (entity, meta) in self.world.query::<&CharacterMeta>().iter() {
            let selected = self.selected_characters.contains(&entity);
            if ui.selectable_label(selected, &meta.name).clicked() {
                clicked_character = Some(entity);
            }
        }
        if let Some(entity) = clicked_character {
            self.selected_character = Some(entity);
            if ui.input(|i| i.modifiers.ctrl) {
                if !self.selected_characters.remove(&entity) {
                    self.selected_characters.insert(entity);
                }
            } else {
                self.selected_characters.clear();
                self.selected_characters.insert(entity);
            }
        }

        ui.separator();
        if let Some(entity) = self.selected_character {
            if self.world.contains(entity) {
                self.draw_selected_character_ui(ui, gl, entity);
            } else {
                self.selected_character = None;
            }
        } else {
            ui.label("No character selected.");
        }

        ui.separator();
        ui.heading("Spawners");
        if ui.button("Add Spawner").clicked() {
            self.push_undo_snapshot();
            let count = self.world.query::<&SpawnerConfig>().iter().count();
            let instance = SpawnerInstance {
                name: format!("Spawner_{}", count + 1),
                position: [0.0, 1.0, 0.0],
                template: CharacterInstance {
                    name: "Spawned".to_string(),
                    position: [0.0, 1.0, 0.0],
                    scale: [0.8, 1.6, 0.8],
                    color: [0.8, 0.2, 0.2],
                    texture_path: None,
                    disposition: Disposition::Hostile,
                    move_speed: 2.0,
                    wander_radius: 2.0,
                    sight_range: 6.0,
                    max_health: Some(20.0),
                    damage: Some(5.0),
                    attack_range: 1.2,
                    attack_cooldown_secs: 1.0,
                    dialogue_nodes: Vec::new(),
                },
                spawn_interval_secs: 5.0,
                max_alive: 3,
                total_to_spawn: None,
                spawn_radius: 2.0,
            };
            let entity = self.spawn_spawner(&instance);
            self.selected_spawner = Some(entity);
        }
        let mut clicked_spawner = None;
        for (entity, config) in self.world.query::<&SpawnerConfig>().iter() {
            let selected = self.selected_spawner == Some(entity);
            if ui.selectable_label(selected, &config.name).clicked() {
                clicked_spawner = Some(entity);
            }
        }
        if let Some(entity) = clicked_spawner {
            self.selected_spawner = Some(entity);
        }

        ui.separator();
        if let Some(entity) = self.selected_spawner {
            if self.world.contains(entity) {
                self.draw_selected_spawner_ui(ui, entity);
            } else {
                self.selected_spawner = None;
            }
        } else {
            ui.label("No spawner selected.");
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
                        if ui.button("Browse Texture...").clicked() {
                            self.pending_texture_pick = Some(PendingTexturePick::ForEntity(entity));
                            self.texture_asset_browser.open(&self.asset_root);
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
            self.push_undo_snapshot();
            let _ = self.world.despawn(entity);
            self.selected_entity = None;
        }
    }

    /// Sibling to `draw_selected_object_ui`/`draw_selected_light_ui` for a
    /// placed character — name/position/AI tuning, plus the same optional-
    /// field Add/Clear pattern `screen_effect`/`camera_shake` use, applied
    /// here to combat (`CharacterMeta::damage`) and to `Health`/`Dialogue`.
    fn draw_selected_character_ui(&mut self, ui: &mut egui::Ui, gl: &glow::Context, entity: Entity) {
        let mut delete = false;
        let mut preview_line: Option<String> = None;
        let mut assign_texture = false;
        let mut browse_texture = false;
        let mut clear_texture = false;

        if let Ok(mut query) = self.world.query_one::<(&mut Transform, &mut CharacterMeta)>(entity) {
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

                ui.horizontal(|ui| {
                    ui.label("Color:");
                    ui.color_edit_button_rgb(&mut meta.color);
                });
                ui.label(match &meta.texture_path {
                    Some(path) => format!("Texture: {}", path.display()),
                    None => "Texture: (solid color)".to_string(),
                });
                ui.horizontal(|ui| {
                    if ui.button("Assign Texture...").clicked() {
                        assign_texture = true;
                    }
                    if ui.button("Browse Texture...").clicked() {
                        browse_texture = true;
                    }
                    if meta.texture_path.is_some() && ui.button("Clear Texture").clicked() {
                        clear_texture = true;
                    }
                });

                ui.label("Disposition");
                egui::ComboBox::from_id_salt("character_disposition")
                    .selected_text(match meta.disposition {
                        Disposition::Passive => "Passive",
                        Disposition::Hostile => "Hostile",
                        Disposition::Friendly => "Friendly",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut meta.disposition, Disposition::Passive, "Passive");
                        ui.selectable_value(&mut meta.disposition, Disposition::Hostile, "Hostile");
                        ui.selectable_value(&mut meta.disposition, Disposition::Friendly, "Friendly");
                    });

                ui.add(egui::DragValue::new(&mut meta.move_speed).speed(0.1).range(0.0..=20.0).prefix("Move speed: "));
                ui.add(egui::DragValue::new(&mut meta.wander_radius).speed(0.1).range(0.0..=50.0).prefix("Wander radius: "));
                ui.add(egui::DragValue::new(&mut meta.sight_range).speed(0.1).range(0.0..=50.0).prefix("Sight range: "));

                ui.separator();
                ui.label("Combat");
                if let Some(damage) = &mut meta.damage {
                    ui.add(egui::DragValue::new(damage).speed(0.5).range(0.0..=100.0).prefix("Damage: "));
                    ui.add(egui::DragValue::new(&mut meta.attack_range).speed(0.1).range(0.0..=20.0).prefix("Attack range: "));
                    ui.add(
                        egui::DragValue::new(&mut meta.attack_cooldown_secs)
                            .speed(0.05)
                            .range(0.1..=10.0)
                            .prefix("Attack cooldown (s): "),
                    );
                    if ui.button("Clear Combat").clicked() {
                        meta.damage = None;
                    }
                } else if ui.button("Add Combat").clicked() {
                    meta.damage = Some(10.0);
                    meta.attack_range = 1.2;
                    meta.attack_cooldown_secs = 1.0;
                }
            }
        }

        ui.separator();
        ui.label("Health");
        if self.world.get::<&Health>(entity).is_ok() {
            let mut max = self.world.get::<&Health>(entity).map(|health| health.max).unwrap_or(20.0);
            if ui.add(egui::DragValue::new(&mut max).speed(1.0).range(1.0..=1000.0).prefix("Max health: ")).changed() {
                if let Ok(mut query) = self.world.query_one::<&mut Health>(entity) {
                    if let Some(health) = query.get() {
                        health.max = max;
                        health.current = max;
                    }
                }
            }
            if ui.button("Clear Health").clicked() {
                let _ = self.world.remove_one::<Health>(entity);
            }
        } else if ui.button("Add Health").clicked() {
            let _ = self.world.insert_one(entity, Health::new(20.0));
        }

        ui.separator();
        ui.label("Dialogue");
        if self.world.get::<&Dialogue>(entity).is_ok() {
            let mut nodes = self.world.get::<&Dialogue>(entity).map(|d| d.nodes.clone()).unwrap_or_default();
            let mut changed = false;
            let mut remove_node = None;
            for (i, node) in nodes.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Node {i}:"));
                        changed |= ui.text_edit_singleline(&mut node.text).changed();
                        if ui.small_button("x").clicked() {
                            remove_node = Some(i);
                        }
                    });
                    ui.small("Choices (empty = linear, auto-advances to the next node):");
                    let mut remove_choice = None;
                    for (ci, (label, target)) in node.choices.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            changed |= ui.text_edit_singleline(label).changed();
                            let mut target_i32 = *target as i32;
                            if ui.add(egui::DragValue::new(&mut target_i32).range(0..=999).prefix("-> node ")).changed() {
                                *target = target_i32.max(0) as usize;
                                changed = true;
                            }
                            if ui.small_button("x").clicked() {
                                remove_choice = Some(ci);
                            }
                        });
                    }
                    if let Some(ci) = remove_choice {
                        node.choices.remove(ci);
                        changed = true;
                    }
                    if ui.small_button("+ Choice").clicked() {
                        node.choices.push((String::new(), 0));
                        changed = true;
                    }
                });
            }
            if let Some(i) = remove_node {
                nodes.remove(i);
                changed = true;
            }
            if ui.button("+ Node").clicked() {
                nodes.push(DialogueNode { text: String::new(), choices: Vec::new() });
                changed = true;
            }
            if changed {
                if let Ok(mut query) = self.world.query_one::<&mut Dialogue>(entity) {
                    if let Some(dialogue) = query.get() {
                        dialogue.nodes = nodes;
                    }
                }
            }
            if ui.button("Preview").clicked() {
                if let Ok(mut query) = self.world.query_one::<&mut Dialogue>(entity) {
                    if let Some(dialogue) = query.get() {
                        preview_line = Some(dialogue.current_text().to_string());
                        dialogue.advance();
                    }
                }
            }
            if ui.button("Clear Dialogue").clicked() {
                let _ = self.world.remove_one::<Dialogue>(entity);
            }
        } else if ui.button("Add Dialogue").clicked() {
            let _ = self.world.insert_one(entity, Dialogue::new(vec![DialogueNode { text: "...".to_string(), choices: Vec::new() }]));
        }

        ui.separator();
        if ui.button("Delete").clicked() {
            delete = true;
        }

        if let Some(line) = preview_line {
            self.hud.show_toast(&line, 3.0);
        }
        if assign_texture {
            self.assign_texture_to_character(gl, entity);
        }
        if browse_texture {
            self.pending_texture_pick = Some(PendingTexturePick::ForCharacter(entity));
            self.texture_asset_browser.open(&self.asset_root);
        }
        if clear_texture {
            let rgba = if let Ok(meta) = self.world.get::<&CharacterMeta>(entity) {
                [
                    (meta.color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (meta.color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (meta.color[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                    255,
                ]
            } else {
                [255, 255, 255, 255]
            };
            let texture = Arc::new(solid_color_texture(gl, rgba));
            if let Ok(mut query) = self.world.query_one::<(&mut MeshRenderer, &mut CharacterMeta)>(entity) {
                if let Some((renderer, meta)) = query.get() {
                    renderer.texture = Some(texture);
                    meta.texture_path = None;
                }
            }
        }
        if delete {
            self.push_undo_snapshot();
            let _ = self.world.despawn(entity);
            self.selected_character = None;
        }
    }

    /// Sibling to `draw_selected_character_ui` for a placed spawner —
    /// timing/limits, plus a compact editor for the `CharacterInstance`
    /// template it spawns (a smaller field set than the full character
    /// inspector; scale/attack-range/cooldown/dialogue are left at their
    /// "Add Spawner" defaults, editable by selecting a spawned instance
    /// directly once one exists).
    fn draw_selected_spawner_ui(&mut self, ui: &mut egui::Ui, entity: Entity) {
        let mut delete = false;
        if let Ok(mut query) = self.world.query_one::<(&mut Transform, &mut SpawnerConfig)>(entity) {
            if let Some((transform, config)) = query.get() {
                let mut name = config.name.clone();
                if ui.text_edit_singleline(&mut name).changed() {
                    config.name = name;
                }

                ui.label("Position");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut transform.position.x).speed(0.05).prefix("x: "));
                    ui.add(egui::DragValue::new(&mut transform.position.y).speed(0.05).prefix("y: "));
                    ui.add(egui::DragValue::new(&mut transform.position.z).speed(0.05).prefix("z: "));
                });

                ui.add(
                    egui::DragValue::new(&mut config.spawn_interval_secs)
                        .speed(0.1)
                        .range(0.1..=120.0)
                        .prefix("Interval (s): "),
                );
                let mut max_alive_f = config.max_alive as f32;
                if ui
                    .add(egui::DragValue::new(&mut max_alive_f).speed(1.0).range(0.0..=50.0).prefix("Max alive: "))
                    .changed()
                {
                    config.max_alive = max_alive_f.max(0.0) as u32;
                }
                ui.add(egui::DragValue::new(&mut config.spawn_radius).speed(0.1).range(0.0..=50.0).prefix("Spawn radius: "));

                let mut limited = config.total_to_spawn.is_some();
                if ui.checkbox(&mut limited, "Limit total spawned").changed() {
                    config.total_to_spawn = if limited { Some(10) } else { None };
                }
                if let Some(total) = &mut config.total_to_spawn {
                    let mut total_f = *total as f32;
                    if ui
                        .add(egui::DragValue::new(&mut total_f).speed(1.0).range(1.0..=999.0).prefix("Total to spawn: "))
                        .changed()
                    {
                        *total = total_f.max(1.0) as u32;
                    }
                }

                ui.separator();
                ui.label("Template character");
                let mut template_name = config.template.name.clone();
                if ui.text_edit_singleline(&mut template_name).changed() {
                    config.template.name = template_name;
                }
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    ui.color_edit_button_rgb(&mut config.template.color);
                });
                egui::ComboBox::from_id_salt("spawner_template_disposition")
                    .selected_text(match config.template.disposition {
                        Disposition::Passive => "Passive",
                        Disposition::Hostile => "Hostile",
                        Disposition::Friendly => "Friendly",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut config.template.disposition, Disposition::Passive, "Passive");
                        ui.selectable_value(&mut config.template.disposition, Disposition::Hostile, "Hostile");
                        ui.selectable_value(&mut config.template.disposition, Disposition::Friendly, "Friendly");
                    });
                ui.add(egui::DragValue::new(&mut config.template.move_speed).speed(0.1).range(0.0..=20.0).prefix("Move speed: "));
                ui.add(egui::DragValue::new(&mut config.template.sight_range).speed(0.1).range(0.0..=50.0).prefix("Sight range: "));

                let mut has_combat = config.template.damage.is_some();
                if ui.checkbox(&mut has_combat, "Can attack").changed() {
                    config.template.damage = if has_combat { Some(5.0) } else { None };
                }
                if let Some(damage) = &mut config.template.damage {
                    ui.add(egui::DragValue::new(damage).speed(0.5).range(0.0..=100.0).prefix("Damage: "));
                }

                let mut has_health = config.template.max_health.is_some();
                if ui.checkbox(&mut has_health, "Has health").changed() {
                    config.template.max_health = if has_health { Some(20.0) } else { None };
                }
                if let Some(max_health) = &mut config.template.max_health {
                    ui.add(egui::DragValue::new(max_health).speed(1.0).range(1.0..=1000.0).prefix("Max health: "));
                }

                ui.separator();
                if ui.button("Delete").clicked() {
                    delete = true;
                }
            }
        }
        if delete {
            self.push_undo_snapshot();
            let _ = self.world.despawn(entity);
            self.selected_spawner = None;
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
            self.push_undo_snapshot();
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
            self.push_undo_snapshot();
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

    /// F2 "Import glTF as Rig..." — maps a multi-node glTF/GLB file's node
    /// hierarchy onto a `RigAsset` (one `RigPartDef` per mesh-carrying
    /// node, `MeshSource::GltfNode` addressing that specific node so
    /// parts don't collide the way `MeshSource::GltfFile`'s "first node
    /// only" convention would). No clips are generated — see
    /// `engine::mesh::load_gltf`'s doc comment for why animation import is
    /// out of scope. The new rig is saved to `rigs/` and immediately
    /// placeable via the existing "Add instance of" list, same as any
    /// hand-authored rig.
    fn import_gltf_as_rig(&mut self, gl: &glow::Context) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("glTF", &["gltf", "glb"])
            .set_directory(&self.asset_root)
            .pick_file()
        else {
            return;
        };
        let scene = match load_gltf(&path) {
            Ok(scene) => scene,
            Err(err) => {
                log::error!("failed to load glTF {path:?}: {err}");
                return;
            }
        };
        if scene.meshes.is_empty() {
            log::warn!("glTF {path:?} has no mesh-carrying nodes; nothing to import");
            return;
        }

        let rig_name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "imported_rig".to_string());
        if self.rigs.iter().any(|rig| rig.name == rig_name) {
            log::warn!("a rig named '{rig_name}' already exists; pick a differently-named glTF file or delete the existing rig first");
            return;
        }
        let relative_gltf_path = engine::level::relativize(&path, &self.asset_root);
        let textures_dir = self.asset_root.join("textures");
        if let Err(err) = std::fs::create_dir_all(&textures_dir) {
            log::error!("failed to create textures dir: {err}");
        }

        let parts: Vec<RigPartDef> = scene
            .meshes
            .iter()
            .map(|entry| {
                let texture_path = entry.image.as_ref().and_then(|image| {
                    let out_path = textures_dir.join(format!("{rig_name}_{}.png", entry.name));
                    match engine::texture::save_rgba8_png(&out_path, &image.rgba, image.width, image.height) {
                        Ok(()) => Some(engine::level::relativize(&out_path, &self.asset_root)),
                        Err(err) => {
                            log::error!("failed to write extracted glTF texture for part '{}': {err}", entry.name);
                            None
                        }
                    }
                });
                RigPartDef {
                    name: entry.name.clone(),
                    parent: entry.parent_name.clone(),
                    mesh: MeshSource::GltfNode { path: relative_gltf_path.clone(), node: entry.name.clone() },
                    texture_path,
                    local_position: entry.local_position,
                    local_rotation_euler_deg: entry.local_rotation_euler_deg,
                    scale: entry.scale,
                }
            })
            .collect();

        let asset = RigAsset { name: rig_name.clone(), parts, clips: Vec::new() };
        let rig_path = self.rigs_dir.join(format!("{rig_name}.ron"));
        if let Err(err) = engine::rig::save_to_file(&asset, &rig_path) {
            log::error!("failed to save imported rig '{rig_name}': {err}");
            return;
        }
        log::info!("imported glTF {path:?} as rig '{rig_name}' ({} parts)", asset.parts.len());
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
            if ui.button("Browse Texture...").clicked() {
                self.pending_texture_pick = Some(PendingTexturePick::ForClass(index));
                self.texture_asset_browser.open(&self.asset_root);
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
            self.push_undo_snapshot();
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
        ui.heading("HUD");
        hud_style_editor(ui, &mut self.render_params.hud);
        ui.small("Saved/loaded with the shader profile, like Render Params above.");

        ui.separator();
        ui.checkbox(&mut self.profiler_visible, "Show Profiler");

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
        if classes.is_empty() {
            log::info!("no object classes found in {:?}; writing default demo class", self.classes_dir);
            let class = default_barrel_class();
            let path = self.classes_dir.join(format!("{}.ron", class.name));
            engine::class::save_to_file(&class, &path)?;
            classes = vec![class];
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
                "no levels found in {:?}; writing default level",
                self.levels_dir
            );
            let level = default_level();
            let path = self.levels_dir.join(format!("{}.ron", level.name));
            engine::level::save_to_file(&level, &path)?;
            levels = vec![level];
        }
        if !levels.iter().any(|l| l.name == "second_room") {
            log::info!("no 'second_room' level found in {:?}; writing it", self.levels_dir);
            let level = second_room_level();
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
        } else {
            // Hot reload is a dev-only convenience — no watcher (and no
            // per-frame poll overhead) in a shipped build.
            self.hot_reload = match HotReloadWatcher::new(&self.asset_root) {
                Ok(watcher) => Some(watcher),
                Err(err) => {
                    log::warn!("hot reload disabled: failed to start file watcher: {err}");
                    None
                }
            };
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
                keycode: Some(Keycode::Z),
                repeat: false,
                keymod,
                ..
            } if editing
                && !ui_wants_keyboard
                && (keymod.contains(sdl2::keyboard::Mod::LCTRLMOD) || keymod.contains(sdl2::keyboard::Mod::RCTRLMOD)) =>
            {
                let gl = ctx.gl();
                if keymod.contains(sdl2::keyboard::Mod::LSHIFTMOD) || keymod.contains(sdl2::keyboard::Mod::RSHIFTMOD) {
                    self.redo(gl);
                } else {
                    self.undo(gl);
                }
            }
            Event::KeyDown {
                keycode: Some(Keycode::D),
                repeat: false,
                keymod,
                ..
            } if editing
                && !ui_wants_keyboard
                && (keymod.contains(sdl2::keyboard::Mod::LCTRLMOD) || keymod.contains(sdl2::keyboard::Mod::RCTRLMOD)) =>
            {
                let gl = ctx.gl();
                self.duplicate_selected(gl);
            }
            Event::KeyDown {
                keycode: Some(Keycode::Delete),
                repeat: false,
                ..
            } if editing && !ui_wants_keyboard => {
                self.delete_selected();
            }
            Event::KeyDown {
                keycode: Some(Keycode::E),
                repeat: false,
                ..
            } if self.mode == EditorMode::Play && !self.paused => {
                self.interact_or_send();
            }
            Event::KeyDown {
                keycode: Some(keycode),
                repeat: false,
                ..
            } if self.mode == EditorMode::Play
                && !self.paused
                && (self.active_dialogue.is_some() || self.net_dialogue.is_some()) =>
            {
                if let Some(choice_index) = number_key_index(keycode) {
                    if self.net_dialogue.is_some() {
                        self.choose_net_dialogue_option(choice_index);
                    } else {
                        self.choose_dialogue_option(choice_index);
                    }
                }
            }
            Event::ControllerButtonDown { button: ControllerButton::X, .. }
                if self.mode == EditorMode::Play && !self.paused =>
            {
                self.interact_or_send();
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) -> anyhow::Result<()> {
        self.elapsed_time = ctx.time.elapsed;

        // Drained before touching anything else in `self` — borrowing
        // `self.hot_reload` only for the duration of `poll_events` (which
        // returns an owned `Vec`) means `handle_hot_reload` below is free
        // to take `&mut self` without fighting a still-live borrow.
        let hot_reload_changes = self.hot_reload.as_mut().map(|watcher| watcher.poll_events()).unwrap_or_default();
        if !hot_reload_changes.is_empty() {
            self.handle_hot_reload(ctx, &hot_reload_changes);
        }

        // Network I/O is polled unconditionally, every frame, regardless of
        // `paused`/Edit-vs-Play — sockets never block (see `engine::net`),
        // and a paused game shouldn't stop noticing a dropped connection or
        // a newly-joined player.
        self.update_networking(ctx, dt);

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
            self.smooth_networked_transforms(dt);
        }

        if self.mode == EditorMode::Edit {
            let ui_wants_keyboard = self.ui.as_ref().map(|ui| ui.wants_keyboard_input()).unwrap_or(false);
            if !ui_wants_keyboard {
                self.update_editor_camera_input(ctx, dt);
            }
        }

        if self.mode == EditorMode::Play && !self.paused {
            self.update_player_input(ctx, dt);

            // `update_player_input` above still runs unconditionally — it
            // drives local camera feel (look, head bob, strafe tilt, FOV
            // kick) and sets `RigidBody.velocity` on `player_entity`, but
            // that velocity is simply never consumed client-side (physics
            // isn't stepped there, see below), so it's harmless. A client
            // additionally reports its intent to the host instead of
            // simulating locally.
            let is_client = matches!(self.net_mode, NetMode::Client(_));
            if is_client {
                let move_dir = self.compute_move_dir(ctx);
                let jump = ctx.input.just_pressed(Keycode::Space)
                    || ctx.input.controller_just_pressed(ControllerButton::A);
                let input = InputState {
                    move_dir: [move_dir.x, move_dir.z],
                    yaw_deg: self.fp_camera.yaw.to_degrees(),
                    pitch_deg: self.fp_camera.pitch.to_degrees(),
                    jump,
                };
                if let NetMode::Client(client) = &mut self.net_mode {
                    client.send_input(&input);
                }
            }

        if !is_client {
            // Collected up front (mirrors the physics-collider pattern
            // above) so dispatching `on_update` can freely use `&mut self`
            // per entity without holding this query's borrow of `world`.
            let scripted: Vec<Entity> = self.world.query::<&BehaviorSlot>().iter().map(|(entity, _)| entity).collect();
            for entity in scripted {
                self.with_behavior(entity, |behavior, api| behavior.on_update(api, dt));
            }

            // `nav_grid` is taken out (rather than borrowed) for the
            // duration of `ai::step` so handling its returned events below
            // — which can call `restart_level` (needs the whole `&mut
            // self`) — never fights a live borrow of `self.nav_grid`.
            if let Some(nav_grid) = self.nav_grid.take() {
                // Every player currently in the session — the host's own
                // local player plus every connected remote player's
                // avatar (see `engine::net`) — so hostile/passive AI
                // reacts to whichever one is actually nearest instead of
                // being blind to everyone but the host.
                let player_positions: Vec<(Entity, Vec3)> = self
                    .player_entity
                    .iter()
                    .chain(self.remote_players.values())
                    .filter_map(|&entity| self.world.get::<&Transform>(entity).ok().map(|t| (entity, t.position)))
                    .collect();
                let events = engine::ai::step(&mut self.world, dt, &player_positions, &nav_grid);
                self.nav_grid = Some(nav_grid);

                for event in events {
                    match event {
                        AiEvent::AttackedPlayer { player, damage, .. } => {
                            let mut died = false;
                            if let Ok(mut query) = self.world.query_one::<&mut Health>(player) {
                                if let Some(health) = query.get() {
                                    health.damage(damage);
                                    died = health.is_dead();
                                }
                            }
                            // Only flash/shake the HOST's own screen when
                            // the host's own local player was the one
                            // hit — a remote player taking damage has no
                            // bearing on what the host is looking at
                            // (mirrors `check_remote_level_switch_triggers`'s
                            // same reasoning).
                            if Some(player) != self.player_entity {
                                if died {
                                    // No shared death/respawn flow exists
                                    // yet for anyone but the host's own
                                    // local player (see the multiplayer
                                    // plan's stated simplifications) — a
                                    // full heal in place is the simplest
                                    // reasonable outcome, rather than
                                    // leaving them stuck at 0 HP forever
                                    // or restarting the level for
                                    // everyone over one player's death.
                                    if let Ok(mut health) = self.world.get::<&mut Health>(player) {
                                        health.current = health.max;
                                    }
                                    self.hud.show_toast("A player was knocked out and recovered", 2.0);
                                }
                                continue;
                            }
                            self.screen_effects.trigger(ScreenEffectSpec {
                                color: [0.9, 0.15, 0.1],
                                strength: 0.6,
                                fade_in_secs: 0.02,
                                hold_secs: 0.05,
                                fade_out_secs: 0.25,
                            });
                            self.camera_shake.trigger(CameraShakeSpec { intensity: 0.12, duration_secs: 0.3 });
                            if died {
                                self.hud.show_toast("You died...", 2.0);
                                self.restart_level(ctx);
                                break;
                            }
                        }
                        AiEvent::CharacterDied { entity, position } => {
                            if let Ok(meta) = self.world.get::<&CharacterMeta>(entity) {
                                self.despawned_since_load.push(meta.name.clone());
                            }
                            // Captured before despawn so the next `Snapshot`
                            // broadcast tells clients to despawn their copy
                            // too, instead of leaving a dead character's
                            // cube standing forever (`Networked` only
                            // exists at all while hosting).
                            if let Ok(networked) = self.world.get::<&Networked>(entity) {
                                self.net_removed.push(networked.0);
                            }
                            let _ = self.world.despawn(entity);
                            self.spawn_burst_at(position, ParticleEmitterDef::default(), 16);
                            self.play_tone(220.0, 0.15);
                        }
                    }
                }

                if let Some(player) = self.player_entity {
                    let fraction = self.world.get::<&Health>(player).map(|health| health.fraction()).unwrap_or(1.0);
                    self.hud.set_bar("Health", fraction);
                }
            }

            // Spawners don't need `nav_grid`, so they tick independently of
            // the block above — a request only carries a `CharacterInstance`
            // (no `gl` access inside `engine::ai`, mirroring why `ai::step`
            // itself returns events instead of spawning/despawning directly).
            let spawn_requests = engine::ai::step_spawners(&mut self.world, dt);
            if !spawn_requests.is_empty() {
                let gl = ctx.gl();
                for request in spawn_requests {
                    match self.spawn_character(gl, &request.instance) {
                        Ok(entity) => {
                            if let Ok(mut query) = self.world.query_one::<&mut SpawnerState>(request.spawner) {
                                if let Some(state) = query.get() {
                                    state.track_spawned(entity);
                                }
                            }
                            // If hosting, this new character needs an
                            // identity and an announcement — a joined
                            // client only ever learns about entities that
                            // existed at `Welcome` time (via
                            // `named_net_ids`) or arrive afterward through
                            // an explicit event like this one.
                            if let Some(net_id) = self.assign_net_id_if_hosting(entity) {
                                self.broadcast_if_hosting(&ServerMessage::CharacterSpawned {
                                    net_id,
                                    instance: request.instance.clone(),
                                });
                            }
                        }
                        Err(err) => log::error!("spawner failed to spawn character: {err}"),
                    }
                }
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
                .iter()
                .copied()
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
                transitioned = self.check_remote_level_switch_triggers(gl, &overlaps);
            }
            if !transitioned {
                for trigger in exited {
                    self.on_trigger_exited(trigger);
                }
                self.trigger_overlaps = current;
            }
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

        let mut draw_calls: u32 = 0;
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
                draw_calls += 1;
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

        // Debug profiler: average frame time over a short rolling window
        // (less jittery than a single-frame instantaneous FPS reading).
        self.frame_times.push_back(ctx.time.delta);
        while self.frame_times.len() > 30 {
            self.frame_times.pop_front();
        }
        let avg_frame_time = self.frame_times.iter().sum::<f32>() / self.frame_times.len().max(1) as f32;
        let profiler_stats = engine::ui::ProfilerStats {
            fps: if avg_frame_time > 0.0 { 1.0 / avg_frame_time } else { 0.0 },
            frame_time_ms: avg_frame_time * 1000.0,
            entity_count: self.world.len() as usize,
            draw_calls,
        };
        let profiler_visible = self.profiler_visible;

        // Must happen before `self.ui.take()` below — texture registration
        // needs `&mut EguiState`, which `ui_state.run`'s closure can't
        // provide (it's already exclusively borrowed for the closure's
        // duration). See `AssetBrowserState`'s doc comment.
        if self.texture_asset_browser.is_open() {
            if let Some(ui_state) = self.ui.as_mut() {
                self.texture_asset_browser.ensure_thumbnails_loaded(gl, ui_state);
            }
        }

        // Editor overlay, drawn on top of the final (already-pixelated) image
        // at full window resolution so the UI itself stays crisp.
        let mut ui_state = self.ui.take().expect("ui set up in init");
        let editing = editor_available() && self.mode == EditorMode::Edit;
        let ui_visible = self.ui_visible && editing;
        let level_ui_visible = self.level_ui_visible && editing;
        let playing = self.mode == EditorMode::Play;
        let mut pause_menu_action: Option<PauseMenuAction> = None;
        let mut multiplayer_action: Option<engine::ui::MultiplayerAction> = None;
        let net_status_text = self.net_status_text();
        let net_connected = matches!(self.net_mode, NetMode::Host(_))
            || matches!(&self.net_mode, NetMode::Client(client) if client.is_connected());
        // Host-only roster for the multiplayer panel's player list — a
        // client sees `&[]` (see `draw_multiplayer_panel`'s doc comment).
        let net_players: Vec<(NetId, String, bool)> = if let NetMode::Host(host) = &self.net_mode {
            host.connected_ids()
                .into_iter()
                .filter_map(|net_id| {
                    host.player_name(net_id).map(|name| {
                        let allowed = self.remote_level_switch_permission.get(&net_id).copied().unwrap_or(false);
                        (net_id, name.to_string(), allowed)
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        let mut picked_texture: Option<PathBuf> = None;
        let full_output = ui_state.run(drawable_size, |egui_ctx| {
            if profiler_visible {
                engine::ui::draw_profiler_overlay(egui_ctx, &profiler_stats);
            }
            if editing {
                if let Some(path) = self.texture_asset_browser.show(egui_ctx, &self.asset_root) {
                    picked_texture = Some(path);
                }
            }
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
                draw_hud(egui_ctx, &self.hud, &self.render_params.hud);
                self.draw_dialogue_choices(egui_ctx);
                if self.paused {
                    pause_menu_action = draw_pause_menu(egui_ctx, editor_available());
                    multiplayer_action = engine::ui::draw_multiplayer_panel(
                        egui_ctx,
                        &net_status_text,
                        net_connected,
                        self.net_reconnect_attempts_left > 0,
                        &mut self.net_join_address,
                        &mut self.net_player_name,
                        &net_players,
                    );
                }
            }
        });
        ui_state.paint(drawable_size, full_output);
        self.ui = Some(ui_state);

        if let Some(path) = picked_texture {
            match self.pending_texture_pick.take() {
                Some(PendingTexturePick::ForEntity(entity)) => {
                    match GpuTexture::load_from_file(gl, &self.asset_root.join(&path), TextureFilter::Nearest) {
                        Ok(texture) => {
                            let texture = Arc::new(texture);
                            if let Ok(mut query) =
                                self.world.query_one::<(&mut MeshRenderer, &mut LevelObjectMeta)>(entity)
                            {
                                if let Some((renderer, meta)) = query.get() {
                                    renderer.texture = Some(texture);
                                    meta.texture_path = Some(path.clone());
                                }
                            }
                            log::info!("assigned texture {path:?} via asset browser");
                        }
                        Err(err) => log::error!("failed to load texture {path:?}: {err}"),
                    }
                }
                Some(PendingTexturePick::ForClass(index)) => {
                    if let Some(class) = self.classes.get_mut(index) {
                        class.texture_path = Some(path);
                    }
                }
                Some(PendingTexturePick::ForCharacter(entity)) => {
                    match GpuTexture::load_from_file(gl, &self.asset_root.join(&path), TextureFilter::Nearest) {
                        Ok(texture) => {
                            let texture = Arc::new(texture);
                            if let Ok(mut query) =
                                self.world.query_one::<(&mut MeshRenderer, &mut CharacterMeta)>(entity)
                            {
                                if let Some((renderer, meta)) = query.get() {
                                    renderer.texture = Some(texture);
                                    meta.texture_path = Some(path.clone());
                                }
                            }
                            log::info!("assigned character texture {path:?} via asset browser");
                        }
                        Err(err) => log::error!("failed to load texture {path:?}: {err}"),
                    }
                }
                None => {}
            }
        }

        match pause_menu_action {
            Some(PauseMenuAction::Resume) => self.resume(ctx),
            Some(PauseMenuAction::ExitToEditor) => self.exit_play_mode(ctx),
            Some(PauseMenuAction::RestartLevel) => self.restart_level(ctx),
            Some(PauseMenuAction::QuitGame) => ctx.should_quit = true,
            None => {}
        }

        match multiplayer_action {
            Some(engine::ui::MultiplayerAction::Host) => self.start_hosting(),
            Some(engine::ui::MultiplayerAction::Join) => self.join_game(),
            Some(engine::ui::MultiplayerAction::Disconnect) => self.disconnect_net(),
            Some(engine::ui::MultiplayerAction::SetLevelSwitchPermission { net_id, allowed }) => {
                self.set_level_switch_permission(net_id, allowed);
            }
            Some(engine::ui::MultiplayerAction::CancelReconnect) => self.cancel_reconnect(),
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
            .set_title("Jame Engine walkaround_demo - Error")
            .set_description(message)
            .set_level(rfd::MessageLevel::Error)
            .show();
    }
    #[cfg(debug_assertions)]
    {
        let _ = message;
    }
}

/// `--smoke-test <path>` runs a compiled `.pss` script driving the game
/// instead of a human (see `engine::app::App::run_scripted`), exiting with
/// the script's pass/fail code — CI-friendly (recommend
/// `SDL_VIDEODRIVER=dummy`, since there's no headless mode). Any other
/// argument shape falls through to the normal interactive `App::run`.
fn smoke_test_path_from_args() -> Option<PathBuf> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--smoke-test" {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

fn main() -> anyhow::Result<()> {
    init_logging();

    std::panic::set_hook(Box::new(|info| {
        log::error!("panic: {info}");
        show_fatal_error_dialog(&info.to_string());
    }));

    if let Some(script_path) = smoke_test_path_from_args() {
        let source = std::fs::read_to_string(&script_path)?;
        let script = match engine::script::Interpreter::compile(&source) {
            Ok(script) => script,
            Err(errors) => {
                for err in &errors {
                    log::error!("smoke test script parse error: {err}");
                }
                std::process::exit(1);
            }
        };
        let code = App::run_scripted("Jame Engine walkaround_demo", 1280, 720, Sandbox::new(), script)?;
        std::process::exit(code);
    }

    let result = App::run("Jame Engine walkaround_demo", 1280, 720, Sandbox::new());
    if let Err(err) = &result {
        log::error!("fatal error: {err:?}");
        show_fatal_error_dialog(&format!("{err:?}"));
    }
    result
}
