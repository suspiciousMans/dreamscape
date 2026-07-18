// A shipped release build has no console attached; a debug build keeps one
// so `cargo run` still shows log output and panic messages as usual.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engine::animation::{AnimationKind, Animator};
use engine::app::{App, Context, Game};
use engine::audio::AudioContext;
use engine::camera::{FirstPersonCamera, OrbitCamera};
use engine::ecs::{Entity, Light, LevelObjectMeta, LightKind, MeshRenderer, PlayerController, Transform, World};
use engine::glam::{EulerRot, Quat, Vec3};
use engine::glow::{self, HasContext};
use engine::level::{AnimationSpec, Level, LevelLight, LevelObject, MeshSource, PrimitiveKind};
use engine::mesh::{load_obj, primitives, GpuMesh};
use engine::physics::{Collider, ColliderShape, PhysicsParams, RigidBody};
use engine::profile::{LightingMode, ProfileCycler, RenderParams, ShaderProfile, TextureFilterMode};
use engine::renderer::{PostParams, Renderer};
use engine::sdl2::controller::Button as ControllerButton;
use engine::sdl2::event::Event;
use engine::sdl2::keyboard::Keycode;
use engine::sdl2::mouse::MouseButton;
use engine::shader::{ShaderVariantCache, AFFINE_UV_BIT};
use engine::texture::{GpuTexture, TextureFilter};
use engine::ui::{render_params_editor, EguiState};

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
    };

    let floor = LevelObject {
        name: "Floor".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Plane),
        texture_path: None,
        position: [0.0, 0.0, 0.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [8.0, 1.0, 8.0],
        is_dynamic: false,
        is_trigger: false,
        animation: None,
    };

    // Static (no RigidBody) but animated — demonstrates that an animated
    // object with no physics still moves under its own motion, purely
    // decorative here since nothing touches it.
    let orbiting_cube = LevelObject {
        name: "Orbiting Cube".to_string(),
        mesh: MeshSource::Primitive(PrimitiveKind::Cube),
        texture_path: None,
        position: [0.0, 3.0, -2.0],
        rotation_euler_deg: [0.0, 0.0, 0.0],
        scale: [0.6, 0.6, 0.6],
        is_dynamic: false,
        is_trigger: false,
        animation: Some(AnimationSpec::Orbit {
            axis: [0.0, 1.0, 0.0],
            speed_deg_per_sec: 90.0,
        }),
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
    };

    Level {
        name: "default".to_string(),
        objects: vec![
            floor,
            make_cube("Cube 1", -1.6),
            make_cube("Cube 2", 0.0),
            make_cube("Cube 3", 1.6),
            trigger,
            orbiting_cube,
        ],
        lights: vec![demo_light],
        physics: PhysicsParams::default(),
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

fn euler_deg_to_quat(euler_deg: Vec3) -> Quat {
    Quat::from_euler(
        EulerRot::XYZ,
        euler_deg.x.to_radians(),
        euler_deg.y.to_radians(),
        euler_deg.z.to_radians(),
    )
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
    light_dir: Vec3,
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
    physics_params: PhysicsParams,
    mode: EditorMode,
    fp_camera: FirstPersonCamera,
    player_entity: Option<Entity>,
    pre_play_snapshot: Option<Level>,
    // `None` if no audio output device was available — degrades silently
    // rather than failing the whole game.
    audio: Option<AudioContext>,
    /// Trigger-vs-entity pairs overlapping as of last frame, diffed each
    /// frame against `physics::step`'s return value to fire enter/exit.
    trigger_overlaps: HashSet<(Entity, Entity)>,
}

impl Sandbox {
    fn new() -> Self {
        let asset_root = resolve_asset_root();
        let profiles_dir = asset_root.join("profiles");
        let levels_dir = asset_root.join("levels");
        Self {
            asset_root,
            profiles_dir,
            levels_dir,
            shader_cache: None,
            shader_paths: (PathBuf::new(), PathBuf::new(), PathBuf::new()),
            render_params: RenderParams::default(),
            world: World::new(),
            camera: OrbitCamera::new(Vec3::ZERO, 4.0),
            renderer: None,
            light_dir: Vec3::new(0.4, 0.8, 0.5).normalize(),
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
            plane_mesh: None,
            physics_params: PhysicsParams::default(),
            mode: EditorMode::Edit,
            fp_camera: FirstPersonCamera::new(),
            player_entity: None,
            pre_play_snapshot: None,
            trigger_overlaps: HashSet::new(),
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
    fn spawn_level_object(&mut self, gl: &glow::Context, obj: &LevelObject) -> anyhow::Result<Entity> {
        let mesh: Arc<GpuMesh> = match &obj.mesh {
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
        };

        let texture = if obj.is_trigger {
            Arc::new(solid_color_texture(gl, TRIGGER_COLOR))
        } else {
            match &obj.texture_path {
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
        };

        // Approximate collision bounds from scale, not an exact mesh-fitted
        // bound — fine for boxy low-poly levels, called out in the guide.
        let half_extents = match &obj.mesh {
            MeshSource::Primitive(PrimitiveKind::Plane) => {
                Vec3::new(obj.scale[0], 0.1, obj.scale[2])
            }
            _ => Vec3::from(obj.scale) * 0.5,
        };

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
            },
            Collider {
                shape: ColliderShape::Aabb { half_extents },
                is_trigger: obj.is_trigger,
            },
        ));

        if obj.is_dynamic {
            let _ = self.world.insert_one(entity, RigidBody::default());
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
            },
        ))
    }

    fn apply_level(&mut self, gl: &glow::Context, level: &Level) -> anyhow::Result<()> {
        self.world.clear();
        self.selected_entity = None;
        for obj in &level.objects {
            if let Err(err) = self.spawn_level_object(gl, obj) {
                log::error!("failed to spawn level object '{}': {err}", obj.name);
            }
        }
        for light in &level.lights {
            self.spawn_level_light(light);
        }
        self.current_level_name = level.name.clone();
        self.physics_params = level.physics;
        log::info!(
            "loaded level '{}' ({} objects, {} lights)",
            level.name,
            level.objects.len(),
            level.lights.len()
        );
        Ok(())
    }

    fn build_level_from_ecs(&self) -> Level {
        let mut objects = Vec::new();
        for (entity, (transform, meta)) in self
            .world
            .query::<(&Transform, &LevelObjectMeta)>()
            .without::<&Light>()
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

        Level {
            name: self.current_level_name.clone(),
            objects,
            lights,
            physics: self.physics_params,
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
        };

        match self.spawn_level_object(gl, &obj) {
            Ok(entity) => {
                self.selected_entity = Some(entity);
                log::info!("imported {model_path:?} as a new level object");
            }
            Err(err) => log::error!("failed to import model {model_path:?}: {err}"),
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
            .map(|transform| transform.position + Vec3::new(0.0, Self::PLAYER_EYE_HEIGHT, 0.0))
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
        log::info!("exited play mode, restored edit-time state");
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

        let mut jumped = false;
        if let Ok(mut query) = self.world.query_one::<(&mut RigidBody, &PlayerController)>(player) {
            if let Some((body, controller)) = query.get() {
                let horizontal_velocity = move_dir * controller.move_speed;
                body.velocity.x = horizontal_velocity.x;
                body.velocity.z = horizontal_velocity.z;

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

        let mut nearest: Option<(Entity, f32)> = None;
        for (entity, (transform, _meta, _body)) in self
            .world
            .query::<(&Transform, &LevelObjectMeta, &RigidBody)>()
            .iter()
        {
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
        }
    }

    /// Sibling to `interact` for the other kind of "things that happen when
    /// the player does something": entering/exiting a non-solid trigger
    /// zone. Replace this with your own game's trigger logic (checkpoints,
    /// level transitions, damage zones, ...).
    fn on_trigger_entered(&self, trigger: Entity) {
        let name = self
            .world
            .get::<&LevelObjectMeta>(trigger)
            .map(|meta| meta.name.clone())
            .unwrap_or_else(|_| "Trigger".to_string());
        log::info!("entered trigger '{name}'");
        self.play_tone(880.0, 0.08);
    }

    fn on_trigger_exited(&self, trigger: Entity) {
        let name = self
            .world
            .get::<&LevelObjectMeta>(trigger)
            .map(|meta| meta.name.clone())
            .unwrap_or_else(|_| "Trigger".to_string());
        log::info!("exited trigger '{name}'");
        self.play_tone(440.0, 0.08);
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
        ui.small("F2 toggle level editor \u{b7} F3 play/stop \u{b7} click an object below to select it");
    }

    fn draw_selected_object_ui(&mut self, ui: &mut egui::Ui, gl: &glow::Context, entity: Entity) {
        if self.world.get::<&Light>(entity).is_ok() {
            self.draw_selected_light_ui(ui, entity);
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
        ui.small(
            "F1 toggle UI \u{b7} F2 level editor \u{b7} F3 play/stop \u{b7} Tab cycle profile \u{b7} F5 save \u{b7} Esc quit",
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
        let first_level = levels[0].clone();
        {
            let gl = ctx.gl();
            self.apply_level(gl, &first_level)?;
        }
        self.levels = levels;

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

        let editing = self.mode == EditorMode::Edit;

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
            } => {
                if self.mode == EditorMode::Play {
                    self.exit_play_mode(ctx);
                } else {
                    ctx.should_quit = true;
                }
            }
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
            } => match self.mode {
                EditorMode::Edit => self.enter_play_mode(ctx),
                EditorMode::Play => self.exit_play_mode(ctx),
            },
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
            } if self.mode == EditorMode::Play => {
                self.interact();
            }
            Event::ControllerButtonDown { button: ControllerButton::X, .. }
                if self.mode == EditorMode::Play =>
            {
                self.interact();
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) -> anyhow::Result<()> {
        // Runs in both Edit and Play mode — a live preview while editing
        // costs nothing extra and is a nice default.
        engine::animation::step(&mut self.world, dt);

        if self.mode == EditorMode::Play {
            self.update_player_input(ctx, dt);
            let overlaps = engine::physics::step(&mut self.world, dt, &self.physics_params);

            let Some(player) = self.player_entity else {
                return Ok(());
            };
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

            for trigger in entered {
                self.on_trigger_entered(trigger);
            }
            for trigger in exited {
                self.on_trigger_exited(trigger);
            }
            self.trigger_overlaps = current;
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

            gl.use_program(Some(program));
            gl.uniform_matrix_4_f32_slice(uniforms.view.as_ref(), false, &view.to_cols_array());
            gl.uniform_matrix_4_f32_slice(uniforms.proj.as_ref(), false, &proj.to_cols_array());
            gl.uniform_3_f32(
                uniforms.light_dir.as_ref(),
                self.light_dir.x,
                self.light_dir.y,
                self.light_dir.z,
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
        }

        renderer.present(
            gl,
            drawable_size,
            &PostParams {
                color_levels: params.color_levels as f32,
                dither_strength: params.dither_strength,
            },
        );

        // Editor overlay, drawn on top of the final (already-pixelated) image
        // at full window resolution so the UI itself stays crisp.
        let mut ui_state = self.ui.take().expect("ui set up in init");
        let editing = self.mode == EditorMode::Edit;
        let ui_visible = self.ui_visible && editing;
        let level_ui_visible = self.level_ui_visible && editing;
        let playing = self.mode == EditorMode::Play;
        let full_output = ui_state.run(drawable_size, |egui_ctx| {
            if level_ui_visible {
                egui::SidePanel::left("level_editor")
                    .default_width(280.0)
                    .show(egui_ctx, |ui| {
                        self.draw_level_editor_ui(ui, gl);
                    });
            }
            if ui_visible {
                egui::SidePanel::right("editor")
                    .default_width(300.0)
                    .show(egui_ctx, |ui| {
                        self.draw_shader_editor_ui(ui, gl, drawable_size);
                    });
            }
            if playing {
                egui::Area::new("play_mode_indicator".into())
                    .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 12.0))
                    .show(egui_ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.label("Play Mode \u{2014} F3 or Esc to stop \u{b7} WASD + mouse \u{b7} Space jump \u{b7} E interact");
                        });
                    });
            }
        });
        ui_state.paint(drawable_size, full_output);
        self.ui = Some(ui_state);

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
            .set_title("PS2 Engine Sandbox - Error")
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

    let result = App::run("PS2 Engine Sandbox", 1280, 720, Sandbox::new());
    if let Err(err) = &result {
        log::error!("fatal error: {err:?}");
        show_fatal_error_dialog(&format!("{err:?}"));
    }
    result
}
