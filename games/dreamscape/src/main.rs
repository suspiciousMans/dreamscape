#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context as _;
use engine::app::{App, Context, Game};
use engine::audio::AudioContext;
use engine::ecs::{Entity, MeshRenderer, Transform, World};
use engine::glam::{Mat4, Quat, Vec3};
use engine::glow::HasContext;
use engine::mesh::GpuMesh;
use engine::physics::{Collider, ColliderShape, PhysicsParams, RigidBody};
use engine::profile::{ProfileCycler, RenderParams};
use engine::renderer::{PostParams, Renderer};
use engine::sdl2::event::Event;
use engine::sdl2::keyboard::Keycode;
use engine::shader::{ShaderVariantCache, AFFINE_UV_BIT};
use engine::texture::{GpuTexture, TextureFilter};

mod dream;
mod enemy_ai;
mod gameplay;

use dream::{Atmosphere, BlockKind, Dream, DreamDirector, DreamTheme, PropKind, DREAMS_PER_RUN};
use enemy_ai::EnemyAI;
use gameplay::{PlayerInputState, PortalMarker};

const PROFILES_DIR: &str = "games/dreamscape/profiles";
const PLAYER_COLOR: [u8; 4] = [255, 255, 255, 255];
const ENEMY_COLOR: [u8; 4] = [200, 40, 40, 255];
const ENEMY_SPEED: f32 = 3.0;

fn init_logging() {
    let _ = env_logger::builder()
        .is_test(false)
        .format_timestamp_millis()
        .try_init();
}

/// DREAMSCAPE_SEED=<u64> replays a run exactly; otherwise seed from the clock.
fn run_seed() -> u64 {
    std::env::var("DREAMSCAPE_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs())
        })
}

pub struct DreamscapeGame {
    renderer: Option<Renderer>,
    shader_cache: Option<ShaderVariantCache>,
    profiles: Option<ProfileCycler>,
    render_params: Option<RenderParams>,
    cube: Option<Arc<GpuMesh>>,
    textures: HashMap<[u8; 4], Arc<GpuTexture>>,
    audio: Option<AudioContext>,

    world: World,
    director: DreamDirector,
    dream: Option<Dream>,
    /// Prop kind carried from the previous dream into the next one.
    motif: Option<PropKind>,
    enemies: Vec<(Entity, EnemyAI)>,

    input: PlayerInputState,
    player: Option<Entity>,
    player_position: Vec3,
    camera_pos: Vec3,
    /// F1: fixed overview camera.
    debug_camera: bool,
    /// DREAMSCAPE_AUTOPILOT=1: follow the generated route (end-to-end test).
    autopilot: bool,
    route_index: usize,
}

impl DreamscapeGame {
    pub fn new(run_seed: u64) -> Self {
        Self {
            renderer: None,
            shader_cache: None,
            profiles: None,
            render_params: None,
            cube: None,
            textures: HashMap::new(),
            audio: None,
            world: World::new(),
            director: DreamDirector::new(run_seed),
            dream: None,
            motif: None,
            enemies: Vec::new(),
            input: PlayerInputState::default(),
            player: None,
            player_position: Vec3::ZERO,
            camera_pos: gameplay::CAMERA_OFFSET,
            debug_camera: false,
            autopilot: std::env::var("DREAMSCAPE_AUTOPILOT").is_ok(),
            route_index: 0,
        }
    }

    /// One cached 1x1 texture per colour.
    fn texture(&mut self, gl: &engine::glow::Context, color: [u8; 4]) -> Arc<GpuTexture> {
        self.textures
            .entry(color)
            .or_insert_with(|| {
                Arc::new(
                    GpuTexture::from_rgba8(gl, &color, 1, 1, TextureFilter::Nearest)
                        .expect("1x1 texture upload"),
                )
            })
            .clone()
    }

    fn apply_atmosphere(
        &mut self,
        theme: DreamTheme,
        atmosphere: &Atmosphere,
    ) -> anyhow::Result<()> {
        let profiles = self.profiles.as_mut().context("profiles not loaded")?;
        let name = theme.spec().base_profile;
        let index = gameplay::profile_index(profiles.all(), name)
            .with_context(|| format!("no shader profile named '{name}' in {PROFILES_DIR}"))?;
        let mut params = profiles.select(index).render;
        params.fog_color = atmosphere.fog_color;
        params.ambient_color = atmosphere.ambient;
        params.fog_start = atmosphere.fog_start;
        params.fog_end = atmosphere.fog_end;
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.set_resolution_scale(params.resolution_scale);
        }
        self.render_params = Some(params);
        log::info!(
            "Atmosphere: profile '{name}', fog {:?}",
            atmosphere.fog_color
        );
        Ok(())
    }

    fn play_music(&mut self, path: &str) {
        let Some(audio) = self.audio.as_mut() else {
            return;
        };
        match audio.play_music_file(Path::new(path), true) {
            Ok(()) => log::info!("Music: {path}"),
            Err(e) => log::error!("Could not play music {path}: {e}"),
        }
    }

    fn load_dream(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        let theme = self.director.theme;
        let spec = theme.spec();
        let dream = dream::generate(
            theme,
            self.director.dream_seed(),
            self.director.depth,
            self.motif,
        );
        log::info!(
            "Dream {}/{}: {:?} seed={} blocks={} enemies={} route={} portal={:?} motif={:?}",
            dream.depth + 1,
            DREAMS_PER_RUN,
            dream.theme,
            dream.seed,
            dream.blocks.len(),
            dream.patrols.len(),
            dream.route.len(),
            dream.portal,
            dream.motif_at,
        );

        self.world.clear();
        self.enemies.clear();
        self.player = None;
        self.route_index = 0;
        self.apply_atmosphere(theme, &dream.atmosphere)?;
        self.play_music(spec.music);

        let gl = ctx.gl();
        let cube = self.cube.clone().context("cube mesh not uploaded")?;
        for block in &dream.blocks {
            let texture = self.texture(gl, block.color);
            let entity = self.world.spawn((
                Transform {
                    position: block.pos,
                    rotation: block.rotation,
                    scale: block.size,
                },
                MeshRenderer {
                    mesh: cube.clone(),
                    texture: Some(texture),
                },
            ));
            let half_extents = block.size * 0.5;
            match block.kind {
                BlockKind::Decor => {}
                BlockKind::Portal => {
                    self.world
                        .insert(
                            entity,
                            (
                                Collider {
                                    shape: ColliderShape::Aabb { half_extents },
                                    is_trigger: true,
                                },
                                PortalMarker,
                            ),
                        )
                        .expect("entity was just spawned");
                }
                BlockKind::Floor | BlockKind::Wall | BlockKind::Prop => {
                    self.world
                        .insert_one(
                            entity,
                            Collider {
                                shape: ColliderShape::Aabb { half_extents },
                                is_trigger: false,
                            },
                        )
                        .expect("entity was just spawned");
                }
            }
        }

        let spawn = dream.spawn + Vec3::Y;
        self.player_position = spawn;
        self.camera_pos = spawn + gameplay::CAMERA_OFFSET;
        let player_texture = self.texture(gl, PLAYER_COLOR);
        self.player = Some(self.world.spawn((
            Transform {
                position: spawn,
                rotation: Quat::IDENTITY,
                scale: Vec3::splat(gameplay::PLAYER_RADIUS * 2.0),
            },
            MeshRenderer {
                mesh: cube.clone(),
                texture: Some(player_texture),
            },
            RigidBody::default(),
            Collider {
                shape: ColliderShape::Sphere {
                    radius: gameplay::PLAYER_RADIUS,
                },
                is_trigger: false,
            },
        )));

        let enemy_texture = self.texture(gl, ENEMY_COLOR);
        for &(a, b) in &dream.patrols {
            let entity = self.world.spawn((
                Transform {
                    position: a,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::splat(0.9),
                },
                MeshRenderer {
                    mesh: cube.clone(),
                    texture: Some(enemy_texture.clone()),
                },
            ));
            self.enemies
                .push((entity, EnemyAI::new_patrol(a, b, ENEMY_SPEED)));
        }

        // The next dream inherits one of this dream's prop kinds as its motif.
        self.motif = spec
            .props
            .get((dream.seed % spec.props.len() as u64) as usize)
            .copied();
        self.dream = Some(dream);
        Ok(())
    }

    fn respawn_player(&mut self) {
        let (Some(player), Some(dream)) = (self.player, self.dream.as_ref()) else {
            return;
        };
        let spawn = dream.spawn + Vec3::Y;
        if let Ok(mut t) = self.world.get::<&mut Transform>(player) {
            t.position = spawn;
        }
        if let Ok(mut body) = self.world.get::<&mut RigidBody>(player) {
            body.velocity = Vec3::ZERO;
        }
        self.player_position = spawn;
        self.camera_pos = spawn + gameplay::CAMERA_OFFSET;
        self.route_index = 0;
    }

    /// Autopilot: walk to the next route waypoint; jump when it's a jump hop.
    fn autopilot_step(&mut self) -> (Vec3, bool) {
        let Some(dream) = &self.dream else {
            return (Vec3::ZERO, false);
        };
        while let Some(wp) = dream.route.get(self.route_index) {
            let flat = Vec3::new(
                wp.pos.x - self.player_position.x,
                0.0,
                wp.pos.z - self.player_position.z,
            );
            if flat.length() < 0.3 {
                self.route_index += 1;
            } else {
                break;
            }
        }
        match dream.route.get(self.route_index) {
            Some(wp) => (
                gameplay::autopilot_velocity(self.player_position, wp.pos),
                wp.jump,
            ),
            None => (Vec3::ZERO, false),
        }
    }
}

impl Game for DreamscapeGame {
    fn init(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        log::info!("Initializing Dreamscape");
        let gl = ctx.gl();
        unsafe {
            gl.enable(engine::glow::DEPTH_TEST);
        }
        let cycler = ProfileCycler::new(engine::profile::load_dir(Path::new(PROFILES_DIR))?)?;
        let first = cycler.current().clone();
        let read =
            |p: &Path| std::fs::read_to_string(p).with_context(|| format!("reading shader {p:?}"));
        let vertex_src = read(&first.vertex_shader)?;
        let fragment_src = read(&first.fragment_shader)?;
        let post_src = read(&first.post_fragment_shader)?;
        self.shader_cache = Some(ShaderVariantCache::new(vertex_src, fragment_src));
        self.renderer = Some(Renderer::new(
            gl,
            ctx.drawable_size(),
            first.render.resolution_scale,
            &post_src,
        )?);
        self.profiles = Some(cycler);
        self.cube = Some(Arc::new(GpuMesh::upload(
            gl,
            &engine::mesh::primitives::cube(),
        )?));
        match AudioContext::new() {
            Ok(audio) => self.audio = Some(audio),
            Err(e) => log::warn!("Failed to initialize audio: {e}"),
        }
        self.load_dream(ctx)?;
        log::info!("Dreamscape initialized");
        Ok(())
    }

    fn handle_event(&mut self, _ctx: &mut Context, event: &Event) {
        let (key, down, repeat) = match event {
            Event::KeyDown {
                keycode: Some(k),
                repeat,
                ..
            } => (*k, true, *repeat),
            Event::KeyUp {
                keycode: Some(k), ..
            } => (*k, false, false),
            _ => return,
        };
        match key {
            Keycode::W => self.input.forward = down,
            Keycode::S => self.input.backward = down,
            Keycode::A => self.input.left = down,
            Keycode::D => self.input.right = down,
            Keycode::Space => self.input.jump = down,
            Keycode::F1 if down && !repeat => {
                self.debug_camera = !self.debug_camera;
                log::info!("debug camera: {}", self.debug_camera);
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) -> anyhow::Result<()> {
        let dt = dt.min(gameplay::MAX_DT);
        let Some(player) = self.player else {
            return Ok(());
        };

        // 1. Input (or autopilot) -> player body
        let (desired, wants_jump) = if self.autopilot {
            self.autopilot_step()
        } else {
            (gameplay::horizontal_velocity(&self.input), self.input.jump)
        };
        if let Ok(mut body) = self.world.get::<&mut RigidBody>(player) {
            body.velocity.x = desired.x;
            body.velocity.z = desired.z;
            if wants_jump && body.grounded {
                body.velocity.y = gameplay::JUMP_SPEED;
            }
        }

        // 2. Enemies
        for (entity, ai) in self.enemies.iter_mut() {
            if let Ok(mut t) = self.world.get::<&mut Transform>(*entity) {
                ai.update(&mut t.position, dt);
            }
        }

        // 3. Physics: gravity, collision, trigger overlaps
        let overlaps = engine::physics::step(&mut self.world, dt, &PhysicsParams::default());
        if let Ok(t) = self.world.get::<&Transform>(player) {
            self.player_position = t.position;
        }
        self.camera_pos = gameplay::follow_camera(self.camera_pos, self.player_position, dt);

        // 4. Fail states (autopilot is immune to enemies so E2E runs are deterministic)
        let caught = !self.autopilot
            && self.enemies.iter().any(|(e, _)| {
                self.world
                    .get::<&Transform>(*e)
                    .map(|t| {
                        gameplay::touches(
                            t.position,
                            self.player_position,
                            gameplay::ENEMY_TOUCH_RADIUS,
                        )
                    })
                    .unwrap_or(false)
            });
        if caught || gameplay::fell_out(self.player_position) {
            log::info!(
                "Player {} — respawning",
                if caught {
                    "caught by the dream"
                } else {
                    "fell out of the dream"
                }
            );
            self.respawn_player();
            return Ok(());
        }

        // 5. Portal → shift to the next dream
        let hit_portal = overlaps
            .iter()
            .any(|&(a, b)| a == player && self.world.get::<&PortalMarker>(b).is_ok());
        if hit_portal {
            log::info!("Portal entered in dream {}", self.director.depth + 1);
            if self.director.advance().is_some() {
                self.load_dream(ctx)?;
            } else {
                log::info!("Game complete! You woke up.");
                ctx.should_quit = true;
            }
        }
        Ok(())
    }

    fn render(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        let drawable_size = ctx.drawable_size();
        let (eye, target) = if self.debug_camera {
            (Vec3::new(0.0, 45.0, -35.0), Vec3::ZERO)
        } else {
            (self.camera_pos, self.player_position)
        };
        let (Some(renderer), Some(shader_cache), Some(params)) = (
            self.renderer.as_mut(),
            self.shader_cache.as_mut(),
            self.render_params,
        ) else {
            return Ok(());
        };
        let gl = ctx.gl();
        renderer.resize_if_needed(gl, drawable_size)?;
        renderer.begin_scene(gl);
        unsafe {
            gl.clear_color(
                params.fog_color[0],
                params.fog_color[1],
                params.fog_color[2],
                1.0,
            );
            gl.clear(engine::glow::COLOR_BUFFER_BIT | engine::glow::DEPTH_BUFFER_BIT);
        }

        let aspect = drawable_size.0 as f32 / drawable_size.1.max(1) as f32;
        let proj = Mat4::perspective_rh(60.0_f32.to_radians(), aspect, 0.1, 1000.0);
        let view = Mat4::look_at_rh(eye, target, Vec3::Y);

        let flags = if params.affine_texture_mapping {
            AFFINE_UV_BIT
        } else {
            0
        };
        let program = shader_cache.get_or_compile(gl, flags)?;
        unsafe {
            gl.use_program(Some(program));
            let loc = |name: &str| gl.get_uniform_location(program, name);
            if let Some(l) = loc("uView") {
                gl.uniform_matrix_4_f32_slice(Some(&l), false, &view.to_cols_array());
            }
            if let Some(l) = loc("uProj") {
                gl.uniform_matrix_4_f32_slice(Some(&l), false, &proj.to_cols_array());
            }
            if let Some(l) = loc("uLightDir") {
                let d = Vec3::from(params.light_dir).normalize_or_zero();
                gl.uniform_3_f32(Some(&l), d.x, d.y, d.z);
            }
            if let Some(l) = loc("uAmbientColor") {
                gl.uniform_3_f32(
                    Some(&l),
                    params.ambient_color[0],
                    params.ambient_color[1],
                    params.ambient_color[2],
                );
            }
            if let Some(l) = loc("uLightingMode") {
                let mode = match params.lighting_mode {
                    engine::profile::LightingMode::Unlit => 0,
                    engine::profile::LightingMode::VertexLit => 1,
                };
                gl.uniform_1_i32(Some(&l), mode);
            }
            if let Some(l) = loc("uVertexSnapAmount") {
                gl.uniform_1_f32(Some(&l), params.vertex_snap_amount);
            }
            if let Some(l) = loc("uFogStart") {
                gl.uniform_1_f32(Some(&l), params.fog_start);
            }
            if let Some(l) = loc("uFogEnd") {
                gl.uniform_1_f32(Some(&l), params.fog_end);
            }
            if let Some(l) = loc("uFogColor") {
                gl.uniform_3_f32(
                    Some(&l),
                    params.fog_color[0],
                    params.fog_color[1],
                    params.fog_color[2],
                );
            }
            if let Some(l) = loc("uPointLightCount") {
                gl.uniform_1_i32(Some(&l), 0);
            }
            if params.backface_culling {
                gl.enable(engine::glow::CULL_FACE);
                gl.cull_face(engine::glow::BACK);
            } else {
                gl.disable(engine::glow::CULL_FACE);
            }
            if let Some(l) = loc("uTex") {
                gl.uniform_1_i32(Some(&l), 0);
            }
        }

        let model_loc = unsafe { gl.get_uniform_location(program, "uModel") };
        for (_entity, (transform, mesh_renderer)) in
            self.world.query::<(&Transform, &MeshRenderer)>().iter()
        {
            unsafe {
                if let Some(l) = &model_loc {
                    gl.uniform_matrix_4_f32_slice(
                        Some(l),
                        false,
                        &transform.matrix().to_cols_array(),
                    );
                }
            }
            if let Some(texture) = &mesh_renderer.texture {
                texture.bind(gl, 0);
            }
            mesh_renderer.mesh.draw(gl);
        }

        renderer.present(
            gl,
            drawable_size,
            &PostParams {
                color_levels: 256.0,
                dither_strength: 0.0,
                tint_color: [1.0, 1.0, 1.0],
                tint_strength: 0.0,
            },
        );
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    init_logging();
    std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));
    let seed = run_seed();
    log::info!("Dream run seed = {seed} (replay with DREAMSCAPE_SEED={seed})");
    App::run("Dreamscape", 1280, 720, DreamscapeGame::new(seed))
}
