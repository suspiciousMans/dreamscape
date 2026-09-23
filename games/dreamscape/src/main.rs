#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;
use engine::app::{App, Context, Game};
use engine::audio::AudioContext;
use engine::ecs::{MeshRenderer, Transform, World};
use engine::glam::{Mat4, Quat, Vec3};
use engine::glow::HasContext;
use engine::mesh::GpuMesh;
use engine::profile::ProfileCycler;
use engine::renderer::Renderer;
use engine::shader::{ShaderVariantCache, AFFINE_UV_BIT};
use engine::sdl2::event::Event;
use engine::texture::{GpuTexture, TextureFilter};

mod world_generator;
mod world_transitions;
mod enemy_ai;
mod gameplay;

use world_generator::{WorldGenerator, WorldVariant};
use world_transitions::TransitionManager;
use enemy_ai::EnemyAI;

fn init_logging() {
    let _ = env_logger::builder()
        .is_test(false)
        .format_timestamp_millis()
        .try_init();
}

/// Player input state
#[derive(Debug, Clone, Default)]
pub struct PlayerInputState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
}

/// The Dreamscape game
pub struct DreamscapeGame {
    // Rendering
    renderer: Option<Renderer>,
    shader_cache: Option<ShaderVariantCache>,
    profiles: Option<ProfileCycler>,
    profiles_dir: PathBuf,

    // ECS world
    world: World,

    // Game state
    world_index: usize,
    transition_manager: TransitionManager,
    world_generator: WorldGenerator,
    current_variant: WorldVariant,
    enemies: Vec<(engine::ecs::Entity, EnemyAI)>,

    // Audio
    audio_context: Option<AudioContext>,
    
    // Input
    player_input: PlayerInputState,
    
    // Player
    player_entity: Option<engine::ecs::Entity>,
    player_position: Vec3,
    player_velocity: Vec3,

    // Fallback white texture so meshes aren't rendered pure black
    white_texture: Option<Arc<GpuTexture>>,
    // Solid cyan texture for trigger/portal objects — visually distinct from
    // white/grey walls so the exit portal reads as a distinct thing.
    portal_texture: Option<Arc<GpuTexture>>,
    // Per-world wall/floor tint textures, keyed by WorldType Debug string
    world_tint_textures: std::collections::HashMap<String, Arc<GpuTexture>>,
}

impl DreamscapeGame {
    pub fn new() -> Self {
        Self {
            renderer: None,
            shader_cache: None,
            profiles: None,
            profiles_dir: PathBuf::from("games/dreamscape/profiles"),

            world: World::new(),

            world_index: 0,
            transition_manager: TransitionManager::new(),
            world_generator: WorldGenerator::new(42),
            current_variant: WorldVariant {
                seed: 42,
                difficulty: 0.0,
            },
            enemies: Vec::new(),
            audio_context: None,
            player_input: PlayerInputState::default(),
            player_entity: None,
            player_position: Vec3::new(0.0, 2.0, 0.0),
            player_velocity: Vec3::ZERO,
            white_texture: None,
            portal_texture: None,
            world_tint_textures: std::collections::HashMap::new(),
        }
    }

    fn load_world(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        log::info!("Loading world {}", self.world_index);

        self.world.clear();
        self.enemies.clear();
        self.player_entity = None;

        let world_type = self.transition_manager.current_world();
        let difficulty = world_type.difficulty();

        self.current_variant = WorldVariant {
            seed: (self.world_index as u64) ^ 0xdeadbeef,
            difficulty,
        };
        self.world_generator = WorldGenerator::new(self.current_variant.seed);

        // Load level from RON
        let level_path = world_type.level_path();
        let level =
            engine::level::load_from_file(&PathBuf::from(level_path)).unwrap_or_else(|err| {
                log::warn!(
                    "Could not load level {}, creating empty: {}",
                    level_path,
                    err
                );
                engine::level::Level {
                    name: format!("{:?}", world_type),
                    objects: Vec::new(),
                    lights: Vec::new(),
                    particle_emitters: Vec::new(),
                    rig_instances: Vec::new(),
                    characters: Vec::new(),
                    spawners: Vec::new(),
                    physics: engine::physics::PhysicsParams::default(),
                    music_path: None,
                }
            });

        // Spawn level objects
        let gl = ctx.gl();

        // Lazy-create per-world tint texture for non-trigger meshes
        let tint_key = format!("{:?}", world_type);
        let tint_texture = self
            .world_tint_textures
            .entry(tint_key)
            .or_insert_with(|| {
                let rgba = world_type.tint_color();
                Arc::new(
                    GpuTexture::from_rgba8(gl, &rgba, 1, 1, TextureFilter::Nearest)
                        .expect("1x1 tint texture upload cannot fail"),
                )
            })
            .clone();

        for obj in &level.objects {
            let mesh_data = match &obj.mesh {
                engine::level::MeshSource::Primitive(kind) => match kind {
                    engine::level::PrimitiveKind::Cube => engine::mesh::primitives::cube(),
                    engine::level::PrimitiveKind::Plane => engine::mesh::primitives::plane(),
                },
                engine::level::MeshSource::ObjFile(path) => engine::mesh::load_obj(path)
                    .ok()
                    .and_then(|mut meshes| meshes.pop())
                    .unwrap_or_else(|| engine::mesh::primitives::cube()),
                _ => engine::mesh::primitives::cube(),
            };

            let gpu_mesh = Arc::new(GpuMesh::upload(gl, &mesh_data)?);

            let pos: Vec3 = obj.position.into();
            let rot = Quat::from_euler(
                glam::EulerRot::XYZ,
                obj.rotation_euler_deg[0].to_radians(),
                obj.rotation_euler_deg[1].to_radians(),
                obj.rotation_euler_deg[2].to_radians(),
            );

            let (mutated_pos, mutated_rot) =
                self.world_generator
                    .mutate_object(pos, rot, &self.current_variant);

            self.world.spawn((
                Transform {
                    position: mutated_pos,
                    rotation: mutated_rot,
                    scale: obj.scale.into(),
                },
                MeshRenderer {
                    mesh: gpu_mesh,
                    texture: if obj.is_trigger {
                        self.portal_texture.clone()
                    } else {
                        Some(tint_texture.clone())
                    },
                },
            ));
        }

        // Spawn the player
        self.player_position = Vec3::new(0.0, 2.0, -5.0);
        self.player_velocity = Vec3::ZERO;
        
        let player = self.world.spawn((
            Transform::from_position(self.player_position),
            MeshRenderer {
                mesh: Arc::new(GpuMesh::upload(gl, &engine::mesh::primitives::cube())?),
                texture: self.white_texture.clone(),
            },
        ));
        self.player_entity = Some(player);

        // Spawn enemies
        if difficulty > 0.5 {
            for i in 0..2 {
                let offset = (i as f32) * 3.0;
                let enemy_pos = Vec3::new(-5.0 + offset, 1.0, 0.0);
                let enemy_mesh = Arc::new(GpuMesh::upload(gl, &engine::mesh::primitives::cube())?);

                let _enemy_entity = self.world.spawn((
                    Transform::from_position(enemy_pos),
                    MeshRenderer {
                        mesh: enemy_mesh,
                        texture: self.white_texture.clone(),
                    },
                ));

                let ai = EnemyAI::new_patrol(
                    Vec3::new(-5.0 + offset, 1.0, -5.0),
                    Vec3::new(-5.0 + offset, 1.0, 5.0),
                    4.0,
                );

                self.enemies.push((_enemy_entity, ai));
            }
        }

        log::info!(
            "Loaded world {} (difficulty: {:.2})",
            self.world_index,
            difficulty
        );
        Ok(())
    }
}

impl Game for DreamscapeGame {
    fn init(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        log::info!("Initializing Dreamscape game");

        unsafe {
            ctx.gl().enable(engine::glow::DEPTH_TEST);
        }

        let gl = ctx.gl();
        let drawable_size = ctx.drawable_size();

        // Load shader profiles
        std::fs::create_dir_all(&self.profiles_dir)?;
        let profiles = engine::profile::load_dir(&self.profiles_dir)?;

        if profiles.is_empty() {
            log::warn!("No profiles found");
        }

        let cycler = ProfileCycler::new(profiles.clone())?;
        let first_profile = cycler.current().clone();

        self.profiles = Some(cycler);

        // Load shader sources
        let vertex_src =
            std::fs::read_to_string(&first_profile.vertex_shader).unwrap_or_else(|_| {
                log::warn!("Could not load vertex shader");
                "void main() { }".to_string()
            });
        let fragment_src =
            std::fs::read_to_string(&first_profile.fragment_shader).unwrap_or_else(|_| {
                log::warn!("Could not load fragment shader");
                "void main() { }".to_string()
            });

        self.shader_cache = Some(ShaderVariantCache::new(vertex_src, fragment_src));

        // Initialize renderer
        let post_frag_src = std::fs::read_to_string(&first_profile.post_fragment_shader)
            .unwrap_or_else(|_| {
                log::warn!("Could not load post-process shader");
                engine::renderer::DEFAULT_FRAGMENT_SRC.to_string()
            });

        self.renderer = Some(Renderer::new(
            gl,
            drawable_size,
            first_profile.render.resolution_scale,
            &post_frag_src,
        )?);

        // Initialize audio
        match AudioContext::new() {
            Ok(audio) => self.audio_context = Some(audio),
            Err(e) => log::warn!("Failed to initialize audio: {}", e),
        }

        // Fallback white texture (1x1) so meshes aren't rendered pure black
        self.white_texture = Some(Arc::new(
            GpuTexture::from_rgba8(gl, &[255, 255, 255, 255], 1, 1, TextureFilter::Nearest)
                .expect("1x1 white texture upload cannot fail"),
        ));

        // Solid cyan texture for trigger/portal objects
        self.portal_texture = Some(Arc::new(
            GpuTexture::from_rgba8(gl, &[40, 220, 220, 255], 1, 1, TextureFilter::Nearest)
                .expect("1x1 cyan texture upload cannot fail"),
        ));

        // Load the first world
        self.load_world(ctx)?;

        log::info!("Dreamscape initialized");
        Ok(())
    }

    fn handle_event(&mut self, _ctx: &mut Context, event: &Event) {
        match event {
            Event::KeyDown { keycode: Some(k), .. } => {
                use engine::sdl2::keyboard::Keycode;
                match *k {
                    Keycode::W => self.player_input.forward = true,
                    Keycode::S => self.player_input.backward = true,
                    Keycode::A => self.player_input.left = true,
                    Keycode::D => self.player_input.right = true,
                    Keycode::Space => self.player_input.jump = true,
                    _ => {}
                }
            }
            Event::KeyUp { keycode: Some(k), .. } => {
                use engine::sdl2::keyboard::Keycode;
                match *k {
                    Keycode::W => self.player_input.forward = false,
                    Keycode::S => self.player_input.backward = false,
                    Keycode::A => self.player_input.left = false,
                    Keycode::D => self.player_input.right = false,
                    Keycode::Space => self.player_input.jump = false,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) -> anyhow::Result<()> {
        // Update player position
        let move_speed = 8.0;
        let mut movement = Vec3::ZERO;

        if self.player_input.forward {
            movement.z += move_speed;
        }
        if self.player_input.backward {
            movement.z -= move_speed;
        }
        if self.player_input.left {
            movement.x -= move_speed;
        }
        if self.player_input.right {
            movement.x += move_speed;
        }

        // Apply gravity
        let gravity = -9.8;
        self.player_velocity.y += gravity * dt;
        self.player_velocity.y = self.player_velocity.y.max(-20.0);

        // Jump
        if self.player_input.jump && self.player_position.y <= 0.5 {
            self.player_velocity.y = 10.0;
        }

        // Apply movement
        self.player_position += movement * dt;
        self.player_position.y += self.player_velocity.y * dt;

        // Ground collision
        if self.player_position.y < 0.5 {
            self.player_position.y = 0.5;
            self.player_velocity.y = 0.0;
        }

        // Update player transform in ECS
        if let Some(player_entity) = self.player_entity {
            if let Ok(mut transform) = self.world.get::<&mut Transform>(player_entity) {
                transform.position = self.player_position;
            }
        }

        // Debug: log position periodically so movement can be verified
        // from console output during manual testing.
        if self.player_input.forward
            || self.player_input.backward
            || self.player_input.left
            || self.player_input.right
        {
            log::info!("player_position = {:?}", self.player_position);
        }

        // Check for portal (proximity > 6.0 on X)
        if self.player_position.x > 6.0 {
            log::info!("Portal triggered!");
            if let Some(_next) = self.transition_manager.advance() {
                self.world_index = self.transition_manager.current_index();
                self.load_world(ctx)?;
            } else {
                log::info!("Game complete!");
                ctx.should_quit = true;
            }
        }

        // Update enemies
        for (entity, ai) in self.enemies.iter_mut() {
            if let Ok(mut transform) = self.world.get::<&mut Transform>(*entity) {
                ai.update(&mut transform.position, dt);
            }
        }

        Ok(())
    }

    fn render(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        let drawable_size = ctx.drawable_size();

        if let Some(ref mut renderer) = &mut self.renderer {
            let gl = ctx.gl();
            renderer.resize_if_needed(gl, drawable_size)?;
            renderer.begin_scene(gl);

            if let Some(ref profiles) = &self.profiles {
                let params = profiles.current().render;

                // Clear
                unsafe {
                    gl.clear_color(
                        params.fog_color[0],
                        params.fog_color[1],
                        params.fog_color[2],
                        1.0,
                    );
                    gl.clear(engine::glow::COLOR_BUFFER_BIT | engine::glow::DEPTH_BUFFER_BIT);
                }

                // Fixed overview camera (not player-locked) so movement
                // is visibly detectable against static level geometry —
                // a camera that tracks the player 1:1 makes them appear
                // frozen in the viewport even while genuinely moving.
                let fov = 60.0_f32.to_radians();
                let aspect = drawable_size.0 as f32 / drawable_size.1.max(1) as f32;
                let proj = Mat4::perspective_rh(fov, aspect, 0.1, 1000.0);

                let camera_pos = Vec3::new(0.0, 14.0, -14.0);
                let view = Mat4::look_at_rh(camera_pos, Vec3::new(0.0, 0.0, 0.0), Vec3::Y);

                // Shader
                if let Some(ref mut shader_cache) = &mut self.shader_cache {
                    let flags = if params.affine_texture_mapping { AFFINE_UV_BIT } else { 0 };
                    let program = shader_cache.get_or_compile(gl, flags)?;

                    unsafe {
                        gl.use_program(Some(program));

                        if let Some(loc) = gl.get_uniform_location(program, "uView") {
                            gl.uniform_matrix_4_f32_slice(Some(&loc), false, &view.to_cols_array());
                        }
                        if let Some(loc) = gl.get_uniform_location(program, "uProj") {
                            gl.uniform_matrix_4_f32_slice(Some(&loc), false, &proj.to_cols_array());
                        }

                        // Lighting + fog uniforms — without these every
                        // mesh renders pure black regardless of texture.
                        if let Some(loc) = gl.get_uniform_location(program, "uLightDir") {
                            let dir = Vec3::from(params.light_dir).normalize_or_zero();
                            gl.uniform_3_f32(Some(&loc), dir.x, dir.y, dir.z);
                        }
                        if let Some(loc) = gl.get_uniform_location(program, "uAmbientColor") {
                            gl.uniform_3_f32(
                                Some(&loc),
                                params.ambient_color[0],
                                params.ambient_color[1],
                                params.ambient_color[2],
                            );
                        }
                        if let Some(loc) = gl.get_uniform_location(program, "uLightingMode") {
                            let mode = match params.lighting_mode {
                                engine::profile::LightingMode::Unlit => 0,
                                engine::profile::LightingMode::VertexLit => 1,
                            };
                            gl.uniform_1_i32(Some(&loc), mode);
                        }
                        if let Some(loc) = gl.get_uniform_location(program, "uVertexSnapAmount") {
                            gl.uniform_1_f32(Some(&loc), params.vertex_snap_amount);
                        }
                        if let Some(loc) = gl.get_uniform_location(program, "uFogStart") {
                            gl.uniform_1_f32(Some(&loc), params.fog_start);
                        }
                        if let Some(loc) = gl.get_uniform_location(program, "uFogEnd") {
                            gl.uniform_1_f32(Some(&loc), params.fog_end);
                        }
                        if let Some(loc) = gl.get_uniform_location(program, "uFogColor") {
                            gl.uniform_3_f32(
                                Some(&loc),
                                params.fog_color[0],
                                params.fog_color[1],
                                params.fog_color[2],
                            );
                        }
                        if let Some(loc) = gl.get_uniform_location(program, "uPointLightCount") {
                            gl.uniform_1_i32(Some(&loc), 0);
                        }

                        if params.backface_culling {
                            gl.enable(engine::glow::CULL_FACE);
                            gl.cull_face(engine::glow::BACK);
                        } else {
                            gl.disable(engine::glow::CULL_FACE);
                        }
                    }

                    // Draw meshes
                    for (_id, (transform, mesh_renderer)) in
                        self.world.query::<(&Transform, &MeshRenderer)>().iter()
                    {
                        let model = Mat4::from_translation(transform.position)
                            * Mat4::from_quat(transform.rotation)
                            * Mat4::from_scale(transform.scale);

                        unsafe {
                            if let Some(loc) = gl.get_uniform_location(program, "uModel") {
                                gl.uniform_matrix_4_f32_slice(Some(&loc), false, &model.to_cols_array());
                            }

                            if let Some(texture) = &mesh_renderer.texture {
                                texture.bind(gl, 0);
                                if let Some(loc) = gl.get_uniform_location(program, "uTex") {
                                    gl.uniform_1_i32(Some(&loc), 0);
                                }
                            }
                        }

                        mesh_renderer.mesh.draw(gl);
                    }
                }
            }

            // Present
            let post_params = engine::renderer::PostParams {
                color_levels: 256.0,
                dither_strength: 0.0,
                tint_color: [1.0, 1.0, 1.0],
                tint_strength: 0.0,
            };
            renderer.present(gl, drawable_size, &post_params);
        }

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    init_logging();

    std::panic::set_hook(Box::new(|info| {
        log::error!("panic: {info}");
    }));

    App::run("Dreamscape", 1280, 720, DreamscapeGame::new())
}
