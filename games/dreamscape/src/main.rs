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
use engine::ui::EguiState;

mod dream;
mod enemy_ai;
mod gameplay;
mod hud;
mod pixels;
mod records;

use dream::{
    Atmosphere, BlockKind, Dream, DreamDirector, DreamTheme, PropKind, Shape, TexSpec,
    LUCIDITY_TO_WAKE, TEX_SIZE,
};
use enemy_ai::EnemyAI;
use gameplay::{PlayerInputState, PortalMarker};

const PROFILES_DIR: &str = "games/dreamscape/profiles";
const PLAYER_COLOR: [u8; 4] = [255, 255, 255, 255];
const ENEMY_SPEED: f32 = 3.0;
const SHARD_SIZE: f32 = 0.8;
const SHARD_SPIN: f32 = 2.0;
const BEACON_HEIGHT: f32 = 8.0;

/// Collect to become lucid.
#[derive(Clone, Copy)]
struct ShardMarker;
/// Only appears once lucid: step through to wake up.
#[derive(Clone, Copy)]
struct WakeMarker;
/// Spins in place (shards, wake door).
#[derive(Clone, Copy)]
struct Spin(f32);
/// Texture repeats per world unit (0 = use mesh UVs).
#[derive(Clone, Copy)]
struct SurfaceUv(f32);

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
    meshes: HashMap<Shape, Arc<GpuMesh>>,
    textures: HashMap<[u8; 4], Arc<GpuTexture>>,
    /// Per-dream procedural textures; destroyed when the dream changes.
    dream_textures: Vec<Arc<GpuTexture>>,
    time: f32,
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
    flash: gameplay::Flash,
    /// Seconds of enemy immunity left after a respawn.
    grace: f32,
    /// The shard (or wake door) entity and its beacon.
    shard_entities: Vec<Entity>,
    shard_tex: Option<Arc<GpuTexture>>,
    wake_tex: Option<Arc<GpuTexture>>,
    best_depth: u32,
    ui: Option<EguiState>,
    mode: hud::Mode,
    /// Seconds since the current dream (or the journal) began.
    title_age: f32,
    dream_name: String,
    dream_whisper: String,
    journal: Vec<String>,
    run_seed: u64,
    restart_requested: bool,
    fonts_installed: bool,
}

impl DreamscapeGame {
    pub fn new(run_seed: u64) -> Self {
        Self {
            renderer: None,
            shader_cache: None,
            profiles: None,
            render_params: None,
            meshes: HashMap::new(),
            textures: HashMap::new(),
            dream_textures: Vec::new(),
            time: 0.0,
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
            flash: gameplay::Flash::default(),
            grace: 0.0,
            shard_entities: Vec::new(),
            shard_tex: None,
            wake_tex: None,
            best_depth: records::load(Path::new(records::RECORD_PATH)),
            ui: None,
            mode: hud::Mode::Playing,
            title_age: 0.0,
            dream_name: String::new(),
            dream_whisper: String::new(),
            journal: Vec::new(),
            run_seed,
            restart_requested: false,
            fonts_installed: false,
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

    fn upload_surface(
        &mut self,
        gl: &engine::glow::Context,
        spec: &TexSpec,
    ) -> anyhow::Result<Arc<GpuTexture>> {
        let tex = Arc::new(GpuTexture::from_rgba8(
            gl,
            &spec.rgba(),
            TEX_SIZE,
            TEX_SIZE,
            // Bilinear: nearest-filtered 64px patterns shimmer into static at range.
            TextureFilter::Bilinear,
        )?);
        self.dream_textures.push(tex.clone());
        Ok(tex)
    }

    fn update_title(&self, ctx: &mut Context) {
        let theme = self.director.theme;
        let title = if theme == DreamTheme::Awakening {
            "Dreamscape — waking up...".to_string()
        } else if self.director.lucid() {
            format!(
                "Dreamscape — depth {} — LUCID: find the white door to wake",
                self.director.depth
            )
        } else {
            format!(
                "Dreamscape — depth {} — lucidity {}/{} — best {}",
                self.director.depth, self.director.lucidity, LUCIDITY_TO_WAKE, self.best_depth
            )
        };
        let _ = ctx.platform.window.set_title(&title);
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

    fn mesh(&self, shape: Shape) -> anyhow::Result<Arc<GpuMesh>> {
        self.meshes
            .get(&shape)
            .cloned()
            .context("mesh not uploaded")
    }

    fn hud_view(&self) -> hud::HudView {
        let target = self
            .shard_entities
            .first()
            .and_then(|&e| self.world.get::<&Transform>(e).ok().map(|t| t.position));
        hud::HudView {
            mode: self.mode,
            time: self.time,
            depth: self.director.depth,
            best: self.best_depth,
            lucidity: self.director.lucidity,
            lucid_target: LUCIDITY_TO_WAKE,
            unbanked: self.director.shard_this_dream,
            strangeness: self.dream.as_ref().map_or(0.0, |d| d.strangeness),
            title: self.dream_name.clone(),
            whisper: self.dream_whisper.clone(),
            title_age: self.title_age,
            shard_dir: target
                .and_then(|t| hud::compass(self.player_position.to_array(), t.to_array(), 7.0)),
            shard_dist: target.map(|t| {
                Vec3::new(
                    t.x - self.player_position.x,
                    0.0,
                    t.z - self.player_position.z,
                )
                .length()
            }),
            journal: self.journal.clone(),
            seed: self.run_seed,
        }
    }

    fn restart(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        self.run_seed = gameplay::next_run_seed(self.run_seed);
        log::info!(
            "Dream run seed = {} (replay with DREAMSCAPE_SEED={})",
            self.run_seed,
            self.run_seed
        );
        self.director = DreamDirector::new(self.run_seed);
        self.motif = None;
        self.journal.clear();
        self.mode = hud::Mode::Playing;
        self.load_dream(ctx)
    }

    fn tone(&self, hz: f32, secs: f32) {
        if let Some(audio) = &self.audio {
            audio.play_tone(hz, secs);
        }
    }

    /// The shard (or, once lucid, the wake door) plus a tall beacon above it
    /// that can be seen over maze walls.
    fn spawn_shard_slot(&mut self, at: Vec3, wake_door: bool) -> anyhow::Result<()> {
        let body_mesh = self.mesh(if wake_door {
            Shape::Cylinder
        } else {
            Shape::Octahedron
        })?;
        let beacon_mesh = self.mesh(Shape::Cylinder)?;
        let tex = if wake_door {
            self.wake_tex.clone()
        } else {
            self.shard_tex.clone()
        }
        .context("shard textures not uploaded")?;
        let (size, spin) = if wake_door {
            (Vec3::new(1.2, 2.4, 1.2), 0.6)
        } else {
            (Vec3::splat(SHARD_SIZE), SHARD_SPIN)
        };
        let entity = self.world.spawn((
            Transform {
                position: at + Vec3::Y * (size.y * 0.5 + 0.2),
                rotation: Quat::IDENTITY,
                scale: size,
            },
            MeshRenderer {
                mesh: body_mesh,
                texture: Some(tex.clone()),
            },
            SurfaceUv(0.8),
            Spin(spin),
            Collider {
                shape: ColliderShape::Aabb {
                    half_extents: size * 0.5,
                },
                is_trigger: true,
            },
        ));
        if wake_door {
            self.world
                .insert_one(entity, WakeMarker)
                .expect("just spawned");
        } else {
            self.world
                .insert_one(entity, ShardMarker)
                .expect("just spawned");
        }
        // No Collider: purely visual, physics never sees it.
        let beacon = self.world.spawn((
            Transform {
                position: at + Vec3::Y * (size.y + 0.4 + BEACON_HEIGHT * 0.5),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(0.15, BEACON_HEIGHT, 0.15),
            },
            MeshRenderer {
                mesh: beacon_mesh,
                texture: Some(tex),
            },
            SurfaceUv(0.8),
            Spin(-spin),
        ));
        self.shard_entities = vec![entity, beacon];
        Ok(())
    }

    fn despawn_shard_slot(&mut self) {
        for e in self.shard_entities.drain(..) {
            let _ = self.world.despawn(e);
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
            self.director.has_shard,
        );
        let wake_door = self.director.lucid() && dream.shard.is_some();
        self.dream_name = dream::dream_name(theme, dream.seed);
        self.dream_whisper = dream::whisper(theme, dream.seed);
        self.title_age = 0.0;
        self.journal
            .push(format!("{:>3}  {}", dream.depth, self.dream_name));
        log::info!("Dream name: {} — {}", self.dream_name, self.dream_whisper);
        log::info!(
            "Dream depth={} {:?} seed={} strangeness={:.2} lucidity={}/{} blocks={} enemies={} route={} shard={:?} portal={:?} motif={:?} wake_door={} next={:?} patterns=({:?},{:?},{:?})",
            dream.depth,
            dream.theme,
            dream.seed,
            dream.strangeness,
            self.director.lucidity,
            LUCIDITY_TO_WAKE,
            dream.blocks.len(),
            dream.patrols.len(),
            dream.route.len(),
            dream.shard,
            dream.portal,
            dream.motif_at,
            wake_door,
            self.director.next,
            dream.surfaces.floor.pattern,
            dream.surfaces.wall.pattern,
            dream.surfaces.prop.pattern,
        );

        self.world.clear();
        {
            let gl = ctx.gl();
            for tex in self.dream_textures.drain(..) {
                // Safe: world.clear() dropped every entity holding these.
                unsafe { tex.destroy(gl) };
            }
        }
        self.enemies.clear();
        self.player = None;
        self.route_index = 0;
        self.apply_atmosphere(theme, &dream.atmosphere)?;
        self.play_music(spec.music);

        let gl = ctx.gl();
        let player_mesh = self.mesh(Shape::Octahedron)?;
        let enemy_mesh = self.mesh(Shape::Orb)?;
        let floor_tex = self.upload_surface(gl, &dream.surfaces.floor)?;
        let wall_tex = self.upload_surface(gl, &dream.surfaces.wall)?;
        let prop_tex = self.upload_surface(gl, &dream.surfaces.prop)?;
        self.shard_tex = Some(self.upload_surface(gl, &dream.surfaces.shard)?);
        let preview = if theme == DreamTheme::Awakening {
            DreamTheme::Awakening
        } else {
            self.director.next
        };
        let portal_tex =
            self.upload_surface(gl, &dream::portal_surface(preview, dream.seed ^ 0x5EED))?;
        self.wake_tex = Some(self.upload_surface(
            gl,
            &dream::portal_surface(DreamTheme::Awakening, dream.seed),
        )?);
        // One texture repeat per two cells: big, readable swirls instead of noise.
        let per_cell = 0.5 / gameplay::CELL;
        for block in &dream.blocks {
            let (texture, uv) = match block.kind {
                BlockKind::Floor => (floor_tex.clone(), per_cell),
                BlockKind::Wall => (wall_tex.clone(), per_cell),
                BlockKind::Prop | BlockKind::Decor => (prop_tex.clone(), 0.5),
                BlockKind::Portal => (portal_tex.clone(), 0.5),
            };
            let mesh = self.mesh(block.shape)?;
            let entity = self.world.spawn((
                Transform {
                    position: block.pos,
                    rotation: block.rotation,
                    scale: block.size,
                },
                MeshRenderer {
                    mesh,
                    texture: Some(texture),
                },
                SurfaceUv(uv),
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
                                Spin(0.5),
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

        // world.clear() above already removed the old ones.
        self.shard_entities.clear();
        if let Some(at) = dream.shard {
            self.spawn_shard_slot(at, wake_door)?;
        }

        let spawn = dream.spawn + Vec3::Y;
        self.player_position = spawn;
        self.camera_pos = spawn + gameplay::CAMERA_OFFSET;
        let player_texture = self.texture(gl, PLAYER_COLOR);
        self.player = Some(self.world.spawn((
            Transform {
                position: spawn,
                rotation: Quat::IDENTITY,
                scale: Vec3::new(0.8, 1.1, 0.8),
            },
            MeshRenderer {
                mesh: player_mesh,
                texture: Some(player_texture),
            },
            Spin(1.2),
            RigidBody::default(),
            Collider {
                shape: ColliderShape::Sphere {
                    radius: gameplay::PLAYER_RADIUS,
                },
                is_trigger: false,
            },
        )));

        let enemy_texture = self.upload_surface(gl, &dream.surfaces.enemy)?;
        for &(a, b) in &dream.patrols {
            let entity = self.world.spawn((
                Transform {
                    position: a,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::splat(0.9),
                },
                MeshRenderer {
                    mesh: enemy_mesh.clone(),
                    texture: Some(enemy_texture.clone()),
                },
                Spin(0.9),
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
        if !self.autopilot && self.director.depth > self.best_depth {
            self.best_depth = self.director.depth;
            if let Err(e) = records::save(Path::new(records::RECORD_PATH), self.best_depth) {
                log::warn!("could not save best depth: {e}");
            }
        }
        self.update_title(ctx);
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
        self.grace = gameplay::RESPAWN_GRACE;
    }

    /// Autopilot: walk to the next route waypoint; jump when it's a jump hop.
    fn autopilot_step(&mut self) -> (Vec3, bool) {
        let Some(dream) = &self.dream else {
            return (Vec3::ZERO, false);
        };
        let route = &dream.lucid_route;
        let grounded = self
            .player
            .and_then(|p| self.world.get::<&RigidBody>(p).ok().map(|b| b.grounded))
            .unwrap_or(false);
        self.route_index = gameplay::advance_waypoint(
            route.iter().map(|w| w.pos),
            self.route_index,
            self.player_position,
            grounded,
        );
        match route.get(self.route_index) {
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
        for shape in dream::ALL_SHAPES {
            self.meshes.insert(
                shape,
                Arc::new(GpuMesh::upload(gl, &dream::build_mesh(shape))?),
            );
        }
        self.ui = Some(EguiState::new(ctx.gl_arc())?);
        match AudioContext::new() {
            Ok(audio) => self.audio = Some(audio),
            Err(e) => log::warn!("Failed to initialize audio: {e}"),
        }
        self.load_dream(ctx)?;
        log::info!("Dreamscape initialized");
        Ok(())
    }

    fn handle_event(&mut self, ctx: &mut Context, event: &Event) {
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
        if down && !repeat {
            match (self.mode, key) {
                (hud::Mode::Playing, Keycode::Escape) => {
                    self.mode = hud::Mode::Paused;
                    self.input = PlayerInputState::default();
                    return;
                }
                (hud::Mode::Paused, Keycode::Escape) => {
                    self.mode = hud::Mode::Playing;
                    return;
                }
                (hud::Mode::Paused, Keycode::Q) | (hud::Mode::Journal, Keycode::Escape) => {
                    ctx.should_quit = true;
                    return;
                }
                (hud::Mode::Journal, Keycode::R) => {
                    self.restart_requested = true;
                    return;
                }
                _ => {}
            }
        }
        if self.mode != hud::Mode::Playing {
            return;
        }
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
        self.time += dt;
        self.flash.tick(dt);
        self.grace = (self.grace - dt).max(0.0);
        for (_e, (t, spin)) in self.world.query_mut::<(&mut Transform, &Spin)>() {
            t.rotation = Quat::from_rotation_y(spin.0 * dt) * t.rotation;
        }
        self.title_age += dt;
        if std::mem::take(&mut self.restart_requested) {
            return self.restart(ctx);
        }
        // E2E runs: show the journal briefly (for screenshots), then exit.
        if self.autopilot
            && self.mode == hud::Mode::Journal
            && self.title_age
                > std::env::var("DREAMSCAPE_JOURNAL_HOLD")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(3.0)
        {
            ctx.should_quit = true;
        }
        if self.mode != hud::Mode::Playing {
            return Ok(());
        }
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
            && self.grace <= 0.0
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
        if caught {
            self.flash.trigger([1.0, 0.1, 0.15], 0.7);
            self.tone(110.0, 0.35);
            if self.director.caught() {
                log::info!(
                    "Shard dropped ({}/{}) — it's back where you found it",
                    self.director.lucidity,
                    LUCIDITY_TO_WAKE
                );
                if let Some(at) = self.dream.as_ref().and_then(|d| d.shard) {
                    self.spawn_shard_slot(at, false)?;
                }
            }
            self.update_title(ctx);
        }
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

        // 5. Triggers: shards, the wake door, and the portal deeper
        let touched: Vec<Entity> = overlaps
            .iter()
            .filter(|&&(a, _)| a == player)
            .map(|&(_, b)| b)
            .collect();
        for &e in &touched {
            if self.world.get::<&ShardMarker>(e).is_ok() {
                self.despawn_shard_slot();
                self.flash.trigger([0.3, 1.0, 1.0], 0.6);
                self.tone(880.0, 0.2);
                self.director.collect_shard();
                log::info!(
                    "Lucidity shard collected at depth {} ({}/{})",
                    self.director.depth,
                    self.director.lucidity,
                    LUCIDITY_TO_WAKE
                );
                self.update_title(ctx);
                break;
            }
        }
        if touched
            .iter()
            .any(|&e| self.world.get::<&WakeMarker>(e).is_ok())
        {
            log::info!("Wake door taken at depth {}", self.director.depth);
            self.flash.trigger([1.0, 1.0, 1.0], 1.0);
            self.tone(660.0, 0.6);
            self.director.wake();
            return self.load_dream(ctx);
        }
        if touched
            .iter()
            .any(|&e| self.world.get::<&PortalMarker>(e).is_ok())
        {
            log::info!("Portal entered at depth {}", self.director.depth);
            if self.director.descend().is_some() {
                self.load_dream(ctx)?;
            } else {
                log::info!("Game complete! You woke up.");
                self.mode = hud::Mode::Journal;
                self.title_age = 0.0;
            }
        }
        Ok(())
    }

    fn render(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        let hud_view = self.hud_view();
        let drawable_size = ctx.drawable_size();
        let time = self.time;
        let flash = self.flash;
        let strangeness = self.dream.as_ref().map_or(0.0, |d| d.strangeness);
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
            // egui (drawn last frame) leaves depth test off and blending/scissor on.
            gl.enable(engine::glow::DEPTH_TEST);
            gl.disable(engine::glow::BLEND);
            gl.disable(engine::glow::SCISSOR_TEST);
        }
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
            if let Some(l) = loc("uTime") {
                gl.uniform_1_f32(Some(&l), time);
            }
            if let Some(l) = loc("uStrangeness") {
                gl.uniform_1_f32(Some(&l), strangeness);
            }
        }

        let model_loc = unsafe { gl.get_uniform_location(program, "uModel") };
        let uv_loc = unsafe { gl.get_uniform_location(program, "uUVScale") };
        for (_entity, (transform, mesh_renderer, uv)) in self
            .world
            .query::<(&Transform, &MeshRenderer, Option<&SurfaceUv>)>()
            .iter()
        {
            unsafe {
                if let Some(l) = &uv_loc {
                    gl.uniform_1_f32(Some(l), uv.map_or(0.0, |u| u.0));
                }
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
                tint_color: flash.color,
                tint_strength: flash.strength,
            },
        );
        let install_fonts = !std::mem::replace(&mut self.fonts_installed, true);
        if let Some(ui) = self.ui.as_mut() {
            let output = ui.run(drawable_size, |egui_ctx| {
                if install_fonts {
                    hud::install_font(egui_ctx);
                }
                hud::draw(egui_ctx, &hud_view);
            });
            ui.paint(drawable_size, output);
        }
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
