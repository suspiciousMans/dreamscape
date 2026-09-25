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

use engine::ui::egui;

mod booklet_ui;
mod cards;
mod dream;
mod enemy_ai;
mod gameplay;
mod hud;
mod hunter;
mod pixels;
mod records;
mod reveal_ui;
mod shop_ui;
mod store;
mod summary_ui;
mod title_ui;
mod transition;
mod tunnel;
mod upgrade_ui;
mod upgrades;

use dream::{
    Atmosphere, BlockKind, Dream, DreamDirector, DreamTheme, Pressure, PropKind, RunLength, Shape,
    TexSpec, Twists, Variant, TEX_SIZE,
};
use enemy_ai::EnemyAI;
use gameplay::{PlayerInputState, PortalMarker};
use upgrades::{Ability, RunUpgrades, Upgrade};

const PROFILES_DIR: &str = "games/dreamscape/profiles";
const SHARD_SIZE: f32 = 0.8;
const SHARD_SPIN: f32 = 2.0;
const BEACON_HEIGHT: f32 = 8.0;
/// The dreamer's dark outline shell, relative to its body.
const OUTLINE_SCALE: f32 = 1.22;

/// Collect to become lucid.
#[derive(Clone, Copy)]
struct ShardMarker;
/// Nightmare sigil: gather them all to open the portal.
#[derive(Clone, Copy)]
struct SigilMarker;
/// Only appears once lucid: step through to wake up.
#[derive(Clone, Copy)]
struct WakeMarker;
/// Spins in place (shards, wake door).
#[derive(Clone, Copy)]
struct Spin(f32);
/// Texture repeats per world unit (0 = use mesh UVs).
#[derive(Clone, Copy)]
struct SurfaceUv(f32);

/// Drawn with uMelt = 0: the dreamer stays solid while the dream melts.
#[derive(Clone, Copy)]
struct NoMelt;

/// Glows through the tunnel-vision darkness (you, shards, beacons, portal).
#[derive(Clone, Copy)]
struct Lit;

/// The dreamer: never vertex-snapped or fogged, and drawn again as a
/// silhouette wherever something hides it.
#[derive(Clone, Copy)]
struct Hero;

/// The glowing ring on the floor under the dreamer.
#[derive(Clone, Copy)]
struct Halo;

/// Drawn in the translucent pass (FLOODED water).
#[derive(Clone, Copy)]
struct Water;

/// Floats up and down around `base` y.
#[derive(Clone, Copy)]
struct Bob {
    base: f32,
    amp: f32,
    speed: f32,
    phase: f32,
}

/// Tallies for the end-of-run summary.
#[derive(Clone, Debug, Default)]
struct RunStats {
    caught: u32,
    fell: u32,
    nightmares: u32,
    twists: Vec<Variant>,
    /// Seconds spent dreaming (not in menus or melts).
    time: f32,
    skipped: u32,
}

/// What the choice screen is choosing.
#[derive(Clone, Debug, PartialEq)]
enum ChoiceKind {
    Upgrade(Vec<Upgrade>),
    /// At the wake door: 0 = wake, 1 = go deeper.
    WakeDoor,
}

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
    run_log: Vec<cards::DreamRecord>,
    booklet: cards::Booklet,
    booklet_path: std::path::PathBuf,
    booklet_page: usize,
    /// Mode to return to when the booklet closes.
    booklet_return: hud::Mode,
    journal_status: Option<String>,
    card_art: booklet_ui::CardArt,
    pack: reveal_ui::PackView,
    shop_shelf: usize,
    shop_col: usize,
    /// Title menu selection.
    title_sel: usize,
    store_status: Option<String>,
    /// Perks consumed for the current run.
    perks: Vec<store::Perk>,
    shards_this_run: u32,
    /// SECOND WIND: the first catch this dream has been forgiven.
    second_wind_used: bool,
    transition: transition::Transition,
    run_seed: u64,
    restart_requested: bool,
    fonts_installed: bool,
    /// Film grain, uploaded to egui on the first frame.
    grain: Option<egui::TextureHandle>,
    /// F2 toggles tunnel vision + grain (for screenshots / accessibility).
    tunnel_on: bool,
    /// Chosen on the title screen for the next run.
    run_length: RunLength,
    /// Upgrades and abilities picked up this run.
    run: RunUpgrades,
    /// This dream's variant twists.
    twists: Twists,
    /// Seconds spent playing this dream (HURRIED timer, FROZEN rhythm).
    dream_age: f32,
    /// Dust earned this run on top of the pack (HURRIED, GILDED).
    bonus_dust: u32,
    /// Highest difficulty survived this run (scales the dust you wake with).
    peak_difficulty: f32,
    /// Offer an upgrade once the next dream has finished reforming.
    offer_upgrade: bool,
    choice: Option<ChoiceKind>,
    choice_view: upgrade_ui::ChoiceView,
    /// The pending pick is a beaten nightmare's reward.
    boss_reward: bool,
    /// Nightmare arenas: the hunter, its start, sigils left, and the
    /// portal waiting to open.
    hunter: Option<(Entity, hunter::Hunter, Vec3)>,
    sigils_left: usize,
    sealed_portal: Option<(dream::Block, Arc<GpuTexture>)>,
    stats: RunStats,
    /// Mid-air jumps used since last touching the ground.
    air_jumps_used: u32,
    jump_was_down: bool,
    /// Last direction the dreamer moved in (dash / blink aim).
    facing: Vec3,
    /// Ability keys pressed since the last update.
    ability_requests: [bool; upgrades::ABILITY_SLOTS],
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
            mode: if std::env::var("DREAMSCAPE_AUTOPILOT").is_ok() {
                hud::Mode::Playing
            } else {
                hud::Mode::Title
            },
            title_age: 0.0,
            dream_name: String::new(),
            dream_whisper: String::new(),
            run_log: Vec::new(),
            booklet: {
                let mut b = cards::load(&cards::booklet_path());
                b.stash.migrate();
                b
            },
            booklet_path: cards::booklet_path(),
            booklet_page: 0,
            booklet_return: hud::Mode::Journal,
            journal_status: None,
            card_art: booklet_ui::CardArt::default(),
            pack: reveal_ui::PackView::default(),
            shop_shelf: 0,
            shop_col: 0,
            title_sel: 0,
            store_status: None,
            perks: Vec::new(),
            shards_this_run: 0,
            second_wind_used: false,
            transition: transition::Transition::new(
                std::env::var("DREAMSCAPE_MELT_SCALE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1.0),
            ),
            run_seed,
            restart_requested: false,
            fonts_installed: false,
            grain: None,
            tunnel_on: std::env::var("DREAMSCAPE_NO_TUNNEL").is_err(),
            run_length: match std::env::var("DREAMSCAPE_RUN").as_deref() {
                Ok("long") => RunLength::Long,
                _ => RunLength::Short,
            },
            run: RunUpgrades::default(),
            twists: Twists::default(),
            dream_age: 0.0,
            bonus_dust: 0,
            peak_difficulty: 1.0,
            offer_upgrade: false,
            choice: None,
            choice_view: upgrade_ui::ChoiceView::default(),
            boss_reward: false,
            hunter: None,
            sigils_left: 0,
            sealed_portal: None,
            stats: RunStats::default(),
            air_jumps_used: 0,
            jump_was_down: false,
            facing: Vec3::Z,
            ability_requests: [false; upgrades::ABILITY_SLOTS],
        }
    }

    fn difficulty(&self) -> f32 {
        gameplay::difficulty(self.director.hardness())
    }

    fn pressure(&self) -> Pressure {
        let hard = self.director.hardness().is_some();
        Pressure {
            enemies: gameplay::pressured_enemy_count(self.difficulty()) * self.twists.enemy_count(),
            growth_cap: if hard { 24 } else { 12 },
        }
    }

    /// Screen-relative controls: in an UPSIDE DOWN dream the camera is
    /// rolled 180 degrees, so the world directions flip with it.
    fn inverted(&self) -> bool {
        self.twists.has(Variant::Inverted)
    }

    fn open_upgrade_choice(&mut self) {
        let seed = self.director.dream_seed()
            ^ 0xC401_CE00
            ^ (self.run.rerolls_used as u64).wrapping_mul(0x9E37_79B9);
        let roll = upgrades::Roll {
            depth: self.director.depth,
            boss: self.boss_reward,
        };
        let options = upgrades::roll_choices(seed, &self.run, roll);
        if options.is_empty() {
            return;
        }
        let card = |u: Upgrade| {
            let info = u.info();
            let have = self.run.count(u);
            // Would taking this complete a combo?
            let mut after = self.run.clone();
            after.take(u);
            let combo = after
                .synergies()
                .into_iter()
                .find(|s| !self.run.has(*s))
                .map(|s| format!("completes {}", s.name()));
            let tag = combo.unwrap_or_else(|| {
                if u.is_ability() {
                    let slot = self.run.abilities.len().min(upgrades::ABILITY_SLOTS - 1);
                    if self.run.abilities.len() >= upgrades::ABILITY_SLOTS {
                        format!(
                            "ability · replaces {}",
                            self.run.abilities[0].ability.name()
                        )
                    } else {
                        format!("ability · [{}]", upgrades::SLOT_KEYS[slot])
                    }
                } else if have > 0 {
                    format!("{} · {} / {}", info.tier.label(), have + 1, info.max)
                } else {
                    info.tier.label().into()
                }
            });
            upgrade_ui::ChoiceCard {
                name: info.name.into(),
                desc: info.desc.into(),
                icon: info.icon,
                color: info.color,
                frame: info.tier.frame(),
                tag,
            }
        };
        self.choice_view = upgrade_ui::ChoiceView {
            title: if self.boss_reward {
                "THE NIGHTMARE LEAVES SOMETHING BEHIND".into()
            } else {
                "THE DREAM OFFERS YOU SOMETHING".into()
            },
            subtitle: format!(
                "depth {} · keep one for the rest of the run",
                self.director.depth
            ),
            cards: options.iter().map(|&u| card(u)).collect(),
            selected: 0,
            age: 0.0,
            hint: format!(
                "[a/d] choose   [enter] take it   [r] reroll ({} left)   [x] skip (+{} dust)",
                self.run.rerolls_left(),
                upgrades::SKIP_DUST
            ),
        };
        self.choice = Some(ChoiceKind::Upgrade(options));
        self.input = PlayerInputState::default();
        self.mode = hud::Mode::Choice;
    }

    fn reroll_choice(&mut self) {
        if !matches!(self.choice, Some(ChoiceKind::Upgrade(_))) || self.run.rerolls_left() == 0 {
            return;
        }
        self.run.rerolls_used += 1;
        log::info!("Rerolled the pick");
        self.tone(330.0, 0.15);
        self.open_upgrade_choice();
    }

    fn skip_choice(&mut self) {
        if !matches!(self.choice, Some(ChoiceKind::Upgrade(_))) {
            return;
        }
        self.choice = None;
        self.boss_reward = false;
        self.mode = hud::Mode::Playing;
        self.bonus_dust += upgrades::SKIP_DUST;
        self.stats.skipped += 1;
        log::info!("Skipped the pick (+{} dust)", upgrades::SKIP_DUST);
    }

    fn open_wake_choice(&mut self) {
        let next = gameplay::difficulty(Some(
            self.director.hardness().map_or((0, 1), |(n, o)| (n, o + 1)),
        ));
        self.choice_view = upgrade_ui::ChoiceView {
            title: "THE WAKE DOOR".into(),
            subtitle: "you could wake up now. or you could stay.".into(),
            cards: vec![
                upgrade_ui::ChoiceCard {
                    name: "WAKE".into(),
                    desc: "open your eyes and keep what you remember".into(),
                    icon: upgrades::Icon::Eye,
                    color: [255, 230, 160],
                    frame: [255, 230, 160],
                    tag: format!("{} dreams deep", self.director.depth),
                },
                upgrade_ui::ChoiceCard {
                    name: "GO DEEPER".into(),
                    desc: format!(
                        "{} more shards to wake. every dream harder than the last, and worth more.",
                        dream::DEEPER_SHARDS
                    ),
                    icon: upgrades::Icon::Shard,
                    color: [255, 90, 140],
                    frame: [255, 90, 140],
                    tag: format!("nightmare x{next:.1}"),
                },
            ],
            selected: 0,
            age: 0.0,
            hint: "[a/d] choose   [enter] decide".into(),
        };
        self.choice = Some(ChoiceKind::WakeDoor);
        self.input = PlayerInputState::default();
        self.mode = hud::Mode::Choice;
    }

    fn choose(&mut self, index: usize) {
        let Some(kind) = self.choice.take() else {
            return;
        };
        self.mode = hud::Mode::Playing;
        match kind {
            ChoiceKind::Upgrade(options) => {
                let Some(&u) = options.get(index) else {
                    self.choice = Some(ChoiceKind::Upgrade(options));
                    self.mode = hud::Mode::Choice;
                    return;
                };
                let before = self.run.synergies();
                self.run.take(u);
                self.boss_reward = false;
                self.director.shard_bonus = self.run.shard_bonus();
                if u == Upgrade::LucidHeart {
                    // Banked straight away: it can't be dropped.
                    self.director.lucidity += 1;
                    self.shards_this_run += 1;
                }
                for s in self.run.synergies() {
                    if !before.contains(&s) {
                        log::info!("Synergy: {} ({})", s.name(), s.desc());
                    }
                }
                log::info!("Upgrade taken: {u:?} (run: {:?})", self.run.taken);
                self.flash
                    .trigger(u.info().color.map(|c| c as f32 / 255.0), 0.5);
                self.tone(523.0, 0.25);
            }
            ChoiceKind::WakeDoor if index == 0 => {
                log::info!("Wake door taken at depth {}", self.director.depth);
                self.flash.trigger([1.0, 1.0, 1.0], 1.0);
                self.tone(660.0, 0.6);
                self.begin_melt(transition::Pending::Wake);
            }
            ChoiceKind::WakeDoor => {
                self.director.go_deeper();
                log::info!(
                    "Refused to wake at depth {}: {} shards to wake, difficulty now x{:.2}",
                    self.director.depth,
                    self.director.shards_to_wake,
                    self.difficulty()
                );
                self.despawn_shard_slot();
                self.flash.trigger([1.0, 0.2, 0.4], 0.8);
                self.tone(98.0, 0.8);
                self.begin_melt(transition::Pending::Descend);
            }
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
                self.director.depth,
                self.director.lucidity,
                self.director.shards_to_wake,
                self.best_depth
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
            lucid_target: self.director.shards_to_wake,
            unbanked: self.director.shard_this_dream,
            strangeness: self.visual_strangeness(),
            title: self.dream_name.clone(),
            whisper: self.dream_whisper.clone(),
            title_age: self.title_age,
            shard_dir: target
                .and_then(|t| hud::compass(self.player_position.to_array(), t.to_array(), 7.0))
                .map(|d| if self.inverted() { [-d[0], -d[1]] } else { d }),
            shard_dist: target.map(|t| {
                Vec3::new(
                    t.x - self.player_position.x,
                    0.0,
                    t.z - self.player_position.z,
                )
                .length()
            }),
            journal: if self.mode == hud::Mode::Journal {
                self.run_log
                    .iter()
                    .map(|r| {
                        (
                            format!("{:>3}  {}", r.depth, r.name),
                            cards::card_from(r).rarity,
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            },
            journal_status: self.journal_status.clone(),
            booklet_cards: if self.mode == hud::Mode::Booklet {
                let all: Vec<&cards::Card> = self.booklet.cards().collect();
                all[cards::page_range(all.len(), self.booklet_page)]
                    .iter()
                    .map(|c| (*c).clone())
                    .collect()
            } else {
                Vec::new()
            },
            booklet_page: self.booklet_page,
            booklet_pages: cards::page_count(self.booklet.card_count()),
            booklet_total: self.booklet.card_count(),
            seed: self.run_seed,
            dust: self.booklet.stash.dust,
            eye: self.booklet.stash.eye,
            hud: self.booklet.stash.hud,
            card_style: self.booklet.stash.card,
            shop_shelf: self.shop_shelf,
            shop_col: if self.mode == hud::Mode::Title {
                self.title_sel
            } else {
                self.shop_col
            },
            dust_earned: self.booklet.stash.dust_earned,
            best_depth: self.best_depth,
            cards_owned: self.booklet.card_count(),
            store_states: store::CATALOG
                .iter()
                .map(|&i| self.booklet.stash.state(i))
                .collect(),
            store_status: self.store_status.clone(),
            perks: if self.mode == hud::Mode::Title {
                self.booklet
                    .stash
                    .charges
                    .iter()
                    .map(|&(p, _)| store::Item::Perk(p).info().name)
                    .collect()
            } else {
                self.perks
                    .iter()
                    .map(|&p| store::Item::Perk(p).info().name)
                    .collect()
            },
            upgrades: self
                .run
                .summary()
                .into_iter()
                .chain(
                    self.run
                        .synergies()
                        .iter()
                        .map(|s| format!("= {}", s.name())),
                )
                .collect(),
            abilities: self
                .run
                .abilities
                .iter()
                .enumerate()
                .map(|(k, a)| {
                    (
                        a.ability.name(),
                        upgrades::SLOT_KEYS[k],
                        a.charge(self.run.cooldown_for(a.ability)),
                        a.is_active(),
                    )
                })
                .collect(),
            twist: self.twists.label(),
            twist_blurb: self
                .twists
                .0
                .iter()
                .map(|v| v.blurb())
                .collect::<Vec<_>>()
                .join("; "),
            timer: self.twists.timer().map(|t| t - self.dream_age),
            difficulty: self.director.hardness().map(|_| self.difficulty()),
            run_length: self.run_length.label(),
            summary: if self.mode == hud::Mode::Summary {
                self.summary_view()
            } else {
                summary_ui::SummaryView::default()
            },
            objective: self.hunter.as_ref().map(|_| {
                if self.sigils_left > 0 {
                    format!(
                        "SIGILS {}/{}",
                        dream::SIGILS - self.sigils_left,
                        dream::SIGILS
                    )
                } else {
                    "THE PORTAL IS OPEN".into()
                }
            }),
            choice: if self.mode == hud::Mode::Choice {
                self.choice_view.clone()
            } else {
                upgrade_ui::ChoiceView::default()
            },
        }
    }

    /// Every sigil taken: the nightmare's portal opens.
    fn open_portal(&mut self) -> anyhow::Result<()> {
        let Some((block, texture)) = self.sealed_portal.take() else {
            return Ok(());
        };
        let mesh = self.mesh(block.shape)?;
        self.world.spawn((
            Transform {
                position: block.pos,
                rotation: block.rotation,
                scale: block.size,
            },
            MeshRenderer {
                mesh,
                texture: Some(texture),
            },
            SurfaceUv(0.5),
            Collider {
                shape: ColliderShape::Aabb {
                    half_extents: block.size * 0.5,
                },
                is_trigger: true,
            },
            PortalMarker,
            Spin(0.5),
            Lit,
            Hero,
        ));
        log::info!("The nightmare's portal opens");
        self.flash.trigger([0.3, 1.0, 1.0], 0.8);
        self.tone(990.0, 0.5);
        Ok(())
    }

    /// Fire the ability in `slot`, if held and charged.
    fn use_ability(&mut self, slot: usize) {
        let Some(state) = self.run.abilities.get(slot).copied() else {
            return;
        };
        let mult = self.run.cooldown_for(state.ability);
        let stretch = self.run.duration_for(state.ability);
        if state.cooldown > 0.0 {
            self.tone(140.0, 0.05);
            return;
        }
        if state.ability == Ability::Blink {
            let Some(at) = self.dream.as_ref().and_then(|d| {
                gameplay::blink_target(
                    &d.blocks,
                    self.player_position,
                    self.facing,
                    upgrades::BLINK_CELLS,
                )
            }) else {
                // Nowhere safe to land: don't spend the charge.
                self.tone(140.0, 0.05);
                return;
            };
            if let Some(p) = self.player {
                if let Ok(mut t) = self.world.get::<&mut Transform>(p) {
                    t.position = at;
                }
                if let Ok(mut b) = self.world.get::<&mut RigidBody>(p) {
                    b.velocity = Vec3::ZERO;
                }
            }
            self.player_position = at;
        }
        if self.run.abilities[slot].try_use(mult, stretch) {
            log::info!("Ability: {:?}", state.ability);
            match state.ability {
                Ability::Dash if self.run.has(upgrades::Synergy::GhostStep) => {
                    self.grace = self
                        .grace
                        .max(Ability::Dash.duration() + upgrades::GHOST_STEP_SECONDS);
                }
                Ability::Blink if self.run.has(upgrades::Synergy::Skywalk) => {
                    self.air_jumps_used = 0;
                }
                _ => {}
            }
            let color = match state.ability {
                Ability::Dash => [0.5, 0.9, 1.0],
                Ability::Blink => [0.9, 0.5, 1.0],
                Ability::Stillness => [0.6, 0.7, 1.0],
                Ability::Phase => [1.0, 1.0, 1.0],
                Ability::ShardCall => [0.3, 1.0, 1.0],
            };
            self.flash.trigger(color, 0.3);
            self.tone(880.0, 0.1);
        }
    }

    /// SHARD CALL: the shard (and its beacon) drift toward the dreamer.
    fn pull_shard(&mut self, dt: f32) {
        let to = self.player_position;
        for &e in &self.shard_entities {
            if let Ok(mut t) = self.world.get::<&mut Transform>(e) {
                let d = Vec3::new(to.x - t.position.x, 0.0, to.z - t.position.z);
                let step = (upgrades::SHARD_CALL_SPEED * dt).min(d.length());
                t.position += d.normalize_or_zero() * step;
            }
        }
    }

    /// Leaving a dream through the portal: HURRIED and GILDED pay out.
    fn dream_bonus(&mut self) {
        let mut bonus = 0;
        if self.twists.timer().is_some_and(|t| self.dream_age <= t) {
            bonus += dream::HASTY_DUST;
            log::info!("Beat the clock in {:.1}s", self.dream_age);
        }
        bonus += ((self.twists.dust() - 1.0) * 15.0).round() as u32;
        if bonus > 0 {
            self.bonus_dust += bonus;
            self.flash.trigger([1.0, 0.85, 0.3], 0.5);
            log::info!("Dream bonus: +{bonus} dust (run bonus {})", self.bonus_dust);
        }
    }

    /// The dreamer's walking speed right now.
    fn player_speed(&self) -> f32 {
        gameplay::MOVE_SPEED
            * self.run.speed()
            * self.twists.walk()
            * if self.has_perk(store::Perk::LongStride) {
                store::LONG_STRIDE
            } else {
                1.0
            }
    }

    /// How strange the dream *looks*: CALM MIND takes the edge off.
    fn visual_strangeness(&self) -> f32 {
        self.dream.as_ref().map_or(0.0, |d| d.strangeness) * self.run.calm()
    }

    fn has_perk(&self, p: store::Perk) -> bool {
        self.perks.contains(&p)
    }

    fn persist_booklet(&mut self) -> bool {
        match cards::save(&self.booklet_path, &self.booklet) {
            Ok(()) => true,
            Err(e) => {
                log::error!("could not save booklet {:?}: {e}", self.booklet_path);
                false
            }
        }
    }

    /// Wake up: roll which dreams stayed with you and start the pack opening.
    fn open_pack(&mut self) {
        let boost = cards::MemoryBoost {
            lucid_wake: true,
            deep_memory: self.has_perk(store::Perk::DeepMemory),
            extra: self.run.memory_bonus(),
        };
        self.pack = reveal_ui::PackView {
            pack: cards::recall(&self.run_log, boost),
            ..Default::default()
        };
        let kept = self.pack.pack.iter().filter(|r| r.remembered).count();
        log::info!(
            "Pack: {kept}/{} dreams remembered: {}",
            self.pack.pack.len(),
            self.pack
                .pack
                .iter()
                .map(|r| format!(
                    "{} [{}{}]",
                    r.card.name,
                    r.card.rarity.label(),
                    if r.remembered { "" } else { ", faded" }
                ))
                .collect::<Vec<_>>()
                .join(" | ")
        );
        self.mode = hud::Mode::Reveal;
    }

    fn save_journal(&mut self) {
        if self.pack.pressed || self.booklet.has_run(self.run_seed) {
            log::info!("Booklet: run {} already saved", self.run_seed);
            return;
        }
        let before = self.booklet.clone();
        let (added, dust) = self.booklet.press(
            self.run_seed,
            &self.pack.pack,
            self.shards_this_run,
            self.has_perk(store::Perk::DustMagnet),
        );
        // Run upgrades and hard runs pay out on top of the pack.
        let mult = self.dust_multiplier();
        let extra = (dust as f32 * (mult - 1.0)).round() as u32 + self.bonus_dust;
        self.booklet.stash.earn(extra);
        let dust = dust + extra;
        if self.persist_booklet() {
            log::info!(
                "Booklet: pressed {added} cards (total {}), +{dust} dust (x{mult:.2}, +{} bonus; now {}) -> {:?}",
                self.booklet.card_count(),
                self.bonus_dust,
                self.booklet.stash.dust,
                self.booklet_path
            );
            self.pack.pressed = true;
            self.pack.cards_added = added;
            self.pack.dust_earned = dust;
        } else {
            self.booklet = before;
        }
    }

    fn open_store(&mut self) {
        self.booklet_return = self.mode;
        self.store_status = None;
        self.mode = hud::Mode::Store;
    }

    fn store_select(&mut self) {
        let item = shop_ui::cursor_item(self.shop_shelf, self.shop_col);
        let before = self.booklet.stash.clone();
        let outcome = self.booklet.stash.select(item);
        let name = item.info().name;
        self.store_status = Some(match outcome {
            store::Outcome::Bought => format!("{name} is yours"),
            store::Outcome::Stocked { runs } => format!("{name}: {runs} runs stocked"),
            store::Outcome::Full => format!("{name} is fully stocked"),
            store::Outcome::Equipped => format!("{name} equipped"),
            store::Outcome::AlreadyEquipped => format!("already wearing {name}"),
            store::Outcome::TooPoor { need } => format!("you need {need} more dust"),
        });
        if self.booklet.stash != before {
            log::info!(
                "Store: {outcome:?} {item:?}, dust now {}",
                self.booklet.stash.dust
            );
            if !self.persist_booklet() {
                self.booklet.stash = before;
                self.store_status = Some("the store couldn't write it down (see log)".into());
            }
        }
    }

    fn open_booklet(&mut self) {
        self.booklet_return = self.mode;
        self.booklet_page = cards::page_count(self.booklet.card_count()) - 1;
        self.mode = hud::Mode::Booklet;
    }

    /// Enter on the title: the dream already loaded behind it becomes the run.
    fn start_from_title(&mut self) {
        // The lobby behind the title is the same whatever the length.
        self.director = DreamDirector::with_length(self.run_seed, self.run_length);
        self.begin_run();
        self.title_age = 0.0;
        self.mode = hud::Mode::Playing;
        log::info!("Fell asleep (perks: {:?})", self.perks);
    }

    fn restart(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        self.run_seed = gameplay::next_run_seed(self.run_seed);
        log::info!(
            "Dream run seed = {} (replay with DREAMSCAPE_SEED={})",
            self.run_seed,
            self.run_seed
        );
        self.director = DreamDirector::with_length(self.run_seed, self.run_length);
        self.motif = None;
        self.run_log.clear();
        self.journal_status = None;
        self.mode = hud::Mode::Playing;
        self.transition.cancel();
        self.begin_run();
        self.load_dream(ctx)
    }

    /// Consume armed perks for the run that is starting.
    fn begin_run(&mut self) {
        self.shards_this_run = 0;
        self.run = RunUpgrades::default();
        self.stats = RunStats::default();
        self.boss_reward = false;
        self.bonus_dust = 0;
        self.peak_difficulty = 1.0;
        self.offer_upgrade = false;
        self.choice = None;
        log::info!(
            "Run length: {:?} ({} shards to wake)",
            self.run_length,
            self.director.shards_to_wake
        );
        self.pack = reveal_ui::PackView::default();
        self.perks = if self.autopilot {
            Vec::new()
        } else {
            self.booklet.stash.take_for_run()
        };
        if !self.perks.is_empty() {
            log::info!("Perks this run: {:?}", self.perks);
            self.persist_booklet();
        }
        if self.has_perk(store::Perk::FirstLight) {
            self.director.collect_shard();
            self.director.shard_this_dream = false;
            self.shards_this_run += 1;
        }
    }

    /// Awake: roll the pack (and, for autopilot E2E runs, autosave).
    fn complete_run(&mut self) {
        log::info!("Game complete! You woke up.");
        self.title_age = 0.0;
        if self.autopilot && std::env::var("DREAMSCAPE_AUTOSAVE").is_ok() {
            self.open_pack();
            self.pack.age = f32::MAX;
            self.save_journal();
            match std::env::var("DREAMSCAPE_AUTOSAVE").as_deref() {
                Ok("booklet") => self.open_booklet(),
                Ok("store") => {
                    self.open_store();
                    self.shop_shelf = 2;
                    self.shop_col = 3;
                }
                _ => {}
            }
        } else {
            self.mode = hud::Mode::Summary;
            log::info!("Summary: {:?}", self.summary_view());
        }
    }

    /// Dust multiplier on waking: DUST HOARDER etc. times the nightmare bonus.
    fn dust_multiplier(&self) -> f32 {
        self.run.dust() * gameplay::difficulty_reward(self.peak_difficulty)
    }

    fn summary_view(&self) -> summary_ui::SummaryView {
        let t = self.stats.time as u32;
        let mut rows = vec![
            (
                "deepest dream".to_string(),
                format!("{}", self.director.depth),
            ),
            (
                "run".into(),
                format!(
                    "{} · {} shards",
                    self.run_length.label(),
                    self.shards_this_run
                ),
            ),
            ("time dreaming".into(), format!("{}:{:02}", t / 60, t % 60)),
            (
                "nightmares beaten".into(),
                self.stats.nightmares.to_string(),
            ),
            (
                "caught / fell".into(),
                format!("{} / {}", self.stats.caught, self.stats.fell),
            ),
        ];
        if self.director.overdrive > 0 {
            rows.push((
                "refused to wake".into(),
                format!("{}x", self.director.overdrive),
            ));
        }
        if self.peak_difficulty > 1.0 {
            rows.push((
                "peak nightmare".into(),
                format!("x{:.1}", self.peak_difficulty),
            ));
        }
        if !self.stats.twists.is_empty() {
            let mut names: Vec<&str> = self.stats.twists.iter().map(|v| v.label()).collect();
            names.sort_unstable();
            names.dedup();
            rows.push(("twists survived".into(), names.join(", ")));
        }
        summary_ui::SummaryView {
            rows,
            upgrades: self.run.summary(),
            synergies: self
                .run
                .synergies()
                .iter()
                .map(|s| s.name().to_string())
                .collect(),
            dust: format!(
                "dust x{:.2}  (hoarding x{:.2} · nightmare x{:.2})  + {} bonus",
                self.dust_multiplier(),
                self.run.dust(),
                gameplay::difficulty_reward(self.peak_difficulty),
                self.bonus_dust
            ),
        }
    }

    /// Begin melting toward `pending`. Ignored if a melt is already running
    /// (so touching the portal for several frames can't queue two descents).
    fn begin_melt(&mut self, pending: transition::Pending) {
        let fog = match pending {
            transition::Pending::Descend => self.director.next.spec().fog_color,
            transition::Pending::Wake => DreamTheme::Awakening.spec().fog_color,
            // The pack reveal's backdrop, rgb(6, 3, 16).
            transition::Pending::Finish => [0.024, 0.012, 0.063],
        };
        if self.transition.start(pending, fog) {
            log::info!("Melt: {pending:?} begins at depth {}", self.director.depth);
            self.tone(196.0, 0.9);
            self.input = PlayerInputState::default();
        }
    }

    /// The bottom of the melt: the screen is fog, so change the dream now.
    fn finish_melt(
        &mut self,
        ctx: &mut Context,
        pending: transition::Pending,
    ) -> anyhow::Result<()> {
        log::info!("Melt: swap ({pending:?})");
        match pending {
            transition::Pending::Descend => {
                self.director.descend();
                self.offer_upgrade = true;
                self.load_dream(ctx)
            }
            transition::Pending::Wake => {
                self.director.wake();
                self.load_dream(ctx)
            }
            transition::Pending::Finish => {
                self.complete_run();
                Ok(())
            }
        }
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
            Lit,
            Collider {
                shape: ColliderShape::Aabb {
                    half_extents: size * 0.5,
                },
                is_trigger: true,
            },
        ));
        // THIRD EYE: the shard shows through walls, like the dreamer does.
        if self.run.has(upgrades::Synergy::ThirdEye) {
            self.world.insert_one(entity, Hero).expect("just spawned");
        }
        if wake_door {
            self.world
                .insert_one(entity, WakeMarker)
                .expect("just spawned");
        } else {
            self.world
                .insert_one(entity, ShardMarker)
                .expect("just spawned");
        }
        // No Collider: purely visual, physics never sees it. In a BLACKOUT
        // the beacon burns twice as wide.
        let girth = if self.twists.has(Variant::Blackout) {
            0.35
        } else {
            0.15
        };
        let beacon = self.world.spawn((
            Transform {
                position: at + Vec3::Y * (size.y + 0.4 + BEACON_HEIGHT * 0.5),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(girth, BEACON_HEIGHT, girth),
            },
            MeshRenderer {
                mesh: beacon_mesh,
                texture: Some(tex),
            },
            SurfaceUv(0.8),
            Spin(-spin),
            Lit,
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
        self.twists = if self.director.nightmare {
            Twists::default()
        } else {
            dream::roll_twists(
                theme,
                self.director.dream_seed(),
                self.director.depth,
                self.director.hardness().is_some(),
            )
        };
        if self.twists.has(Variant::Gilded) {
            self.director.has_shard = true;
        }
        self.peak_difficulty = self.peak_difficulty.max(self.difficulty());
        self.dream_age = 0.0;
        self.air_jumps_used = 0;
        let dream = if self.director.nightmare {
            dream::generate_nightmare(theme, self.director.dream_seed(), self.director.depth)
        } else {
            dream::generate_with(
                theme,
                self.director.dream_seed(),
                self.director.depth,
                self.motif,
                self.director.has_shard,
                self.pressure(),
            )
        };
        self.hunter = None;
        self.sealed_portal = None;
        self.sigils_left = dream.sigils.len();
        if !self.twists.0.is_empty() {
            log::info!("Dream twist: {}", self.twists.label());
        }
        let wake_door = self.director.lucid() && dream.shard.is_some();
        self.dream_name = dream::dream_name(theme, dream.seed);
        self.dream_whisper = dream::whisper(theme, dream.seed);
        self.title_age = 0.0;
        self.second_wind_used = false;
        self.run_log.push(cards::DreamRecord {
            theme,
            seed: dream.seed,
            depth: dream.depth,
            name: self.dream_name.clone(),
            whisper: self.dream_whisper.clone(),
            strangeness: dream.strangeness,
            enemies: dream.patrols.len() as u32,
            shard_taken: false,
            art: dream.surfaces.floor.clone(),
        });
        log::info!("Dream name: {} — {}", self.dream_name, self.dream_whisper);
        log::info!(
            "Dream depth={} {:?} seed={} strangeness={:.2} lucidity={}/{} blocks={} enemies={} route={} shard={:?} portal={:?} motif={:?} wake_door={} next={:?} patterns=({:?},{:?},{:?})",
            dream.depth,
            dream.theme,
            dream.seed,
            dream.strangeness,
            self.director.lucidity,
            self.director.shards_to_wake,
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
            if block.kind == BlockKind::Portal && !dream.sigils.is_empty() {
                // Sealed until every sigil is gathered.
                self.sealed_portal = Some((block.clone(), portal_tex.clone()));
                continue;
            }
            let (texture, uv) = match block.kind {
                BlockKind::Floor => (floor_tex.clone(), per_cell),
                BlockKind::Wall => (wall_tex.clone(), per_cell),
                BlockKind::Prop | BlockKind::Decor | BlockKind::Sky => (prop_tex.clone(), 0.5),
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
            // Scenery drifts: a slow spin and a bob, desynchronised by position.
            let h = (block.pos.x * 12.9898 + block.pos.z * 78.233).sin() * 43_758.547;
            let h = h - h.floor();
            let bob = |amp: f32| Bob {
                base: block.pos.y,
                amp,
                speed: 0.4 + 0.6 * h,
                phase: h * std::f32::consts::TAU,
            };
            match block.kind {
                BlockKind::Decor => {
                    let island_under_floor = block.shape == Shape::Island && block.pos.y > -3.0;
                    if !island_under_floor {
                        self.world
                            .insert(entity, (Spin((h - 0.5) * 0.3), bob(0.25 + 0.35 * h)))
                            .expect("entity was just spawned");
                    }
                }
                BlockKind::Sky => {
                    let spin = if block.shape == Shape::Crescent {
                        0.0
                    } else {
                        0.5 + h
                    };
                    self.world
                        .insert(entity, (Spin(spin), bob(0.15 + 0.2 * h), Lit))
                        .expect("entity was just spawned");
                }
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
                                Lit,
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

        // FLOODED: a sheet of translucent water over the whole floor plan.
        if self.twists.has(Variant::Flooded) {
            let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for b in dream.blocks.iter().filter(|b| b.kind == BlockKind::Floor) {
                lo = lo.min(b.pos - b.size * 0.5);
                hi = hi.max(b.pos + b.size * 0.5);
            }
            if lo.x < hi.x {
                let water_tex = self.texture(gl, [60, 150, 230, 255]);
                let cube = self.mesh(Shape::Cube)?;
                self.world.spawn((
                    Transform {
                        position: Vec3::new((lo.x + hi.x) * 0.5, 0.3, (lo.z + hi.z) * 0.5),
                        rotation: Quat::IDENTITY,
                        scale: Vec3::new(hi.x - lo.x, 0.05, hi.z - lo.z),
                    },
                    MeshRenderer {
                        mesh: cube,
                        texture: Some(water_tex),
                    },
                    Water,
                ));
            }
        }

        let spawn = dream.spawn + Vec3::Y;
        self.player_position = spawn;
        self.camera_pos = spawn + gameplay::CAMERA_OFFSET;
        let crystal = self.booklet.stash.crystal.rgba();
        let player_texture = self.texture(gl, crystal);
        let halo_mesh = self.mesh(Shape::Torus)?;
        self.world.spawn((
            Transform {
                position: spawn,
                // A hoop laid flat on the floor.
                rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                scale: Vec3::new(1.3, 1.3, 0.04),
            },
            MeshRenderer {
                mesh: halo_mesh,
                texture: Some(player_texture.clone()),
            },
            Halo,
            NoMelt,
            Lit,
        ));
        self.player = Some(self.world.spawn((
            Transform {
                position: spawn,
                rotation: Quat::IDENTITY,
                scale: Vec3::new(0.9, 1.25, 0.9),
            },
            MeshRenderer {
                mesh: player_mesh,
                texture: Some(player_texture),
            },
            Spin(1.2),
            NoMelt,
            Lit,
            Hero,
            RigidBody::default(),
            Collider {
                shape: ColliderShape::Sphere {
                    radius: gameplay::PLAYER_RADIUS,
                },
                is_trigger: false,
            },
        )));

        let slow_heart = self.has_perk(store::Perk::SlowHeart);
        let eyelids = self.has_perk(store::Perk::HeavyEyelids);
        let enemy_texture = self.upload_surface(gl, &dream.surfaces.enemy)?;
        let difficulty = self.difficulty();
        let player_speed = self.player_speed();
        let speed = (gameplay::pressured_enemy_speed(dream.depth, difficulty, player_speed)
            * self.twists.enemy_speed()
            * self.run.enemy_speed()
            * if slow_heart {
                store::SLOW_HEART_ENEMY_SPEED
            } else {
                1.0
            })
        .min(0.92 * player_speed / enemy_ai::CHASE_BOOST);
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
            self.enemies.push((
                entity,
                EnemyAI::new(
                    a,
                    b,
                    speed,
                    if eyelids {
                        0.0
                    } else {
                        gameplay::pressured_chase_radius(dream.depth, difficulty) * self.run.alert()
                    },
                    gameplay::CHASE_LEASH,
                ),
            ));
        }

        // Nightmare: sigils round the edge, and the hunter in the middle.
        let sigil_tex = self
            .shard_tex
            .clone()
            .context("shard textures not uploaded")?;
        let sigil_mesh = self.mesh(Shape::Octahedron)?;
        for (k, &at) in dream.sigils.iter().enumerate() {
            let size = Vec3::new(0.7, 1.3, 0.7);
            self.world.spawn((
                Transform {
                    position: at + Vec3::Y * 1.0,
                    rotation: Quat::IDENTITY,
                    scale: size,
                },
                MeshRenderer {
                    mesh: sigil_mesh.clone(),
                    texture: Some(sigil_tex.clone()),
                },
                SurfaceUv(0.8),
                Spin(2.5),
                Bob {
                    base: 1.0,
                    amp: 0.25,
                    speed: 2.0,
                    phase: k as f32 * 2.1,
                },
                Lit,
                Hero,
                SigilMarker,
                Collider {
                    shape: ColliderShape::Aabb {
                        half_extents: size * 0.5,
                    },
                    is_trigger: true,
                },
            ));
        }
        if let Some(at) = dream.hunter {
            let h = hunter::Hunter::new(dream.depth, self.player_speed());
            let entity = self.world.spawn((
                Transform {
                    position: at,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::splat(h.size()),
                },
                MeshRenderer {
                    mesh: enemy_mesh.clone(),
                    texture: Some(enemy_texture.clone()),
                },
                Spin(0.6),
                Lit,
            ));
            self.hunter = Some((entity, h, at));
            self.dream_whisper = "gather the sigils. it is hunting you.".into();
            self.dream_name = format!("NIGHTMARE: {}", self.dream_name);
            log::info!(
                "Nightmare at depth {}: {} sigils",
                dream.depth,
                dream.sigils.len()
            );
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
        self.grace = gameplay::RESPAWN_GRACE
            * self.run.grace()
            * if self.has_perk(store::Perk::SlowHeart) {
                store::SLOW_HEART_GRACE
            } else {
                1.0
            };
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
        if self.mode == hud::Mode::Playing {
            self.begin_run();
        }
        // Dev switch: DREAMSCAPE_THEME=SkyStairs starts one dream deep in it.
        if let Ok(name) = std::env::var("DREAMSCAPE_THEME") {
            match dream::ALL_THEMES.iter().find(|t| format!("{t:?}") == name) {
                Some(&t) => {
                    self.director.theme = t;
                    self.director.depth = 1;
                }
                None => log::warn!("DREAMSCAPE_THEME={name}: no such dream"),
            }
        }
        // Dev switch: DREAMSCAPE_NIGHTMARE=1 starts in a nightmare arena.
        if std::env::var("DREAMSCAPE_NIGHTMARE").is_ok() {
            if self.director.theme == DreamTheme::Lobby {
                self.director.theme = DreamTheme::NightmareFactory;
            }
            self.director.depth = dream::NIGHTMARE_EVERY;
            self.director.nightmare = true;
            self.director.has_shard = false;
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
            let revealed = reveal_ui::reveal_done(self.pack.pack.len(), self.pack.age);
            if self.mode == hud::Mode::Summary {
                if matches!(key, Keycode::Return | Keycode::Space) && self.title_age > 0.5 {
                    self.title_age = 0.0;
                    self.open_pack();
                }
                return;
            }
            if self.mode == hud::Mode::Choice {
                let n = self.choice_view.cards.len().max(1);
                match key {
                    Keycode::A | Keycode::Left => {
                        self.choice_view.selected = (self.choice_view.selected + n - 1) % n;
                    }
                    Keycode::D | Keycode::Right => {
                        self.choice_view.selected = (self.choice_view.selected + 1) % n;
                    }
                    Keycode::Num1 | Keycode::Num2 | Keycode::Num3 => {
                        let i = [Keycode::Num1, Keycode::Num2, Keycode::Num3]
                            .iter()
                            .position(|&k| k == key)
                            .expect("matched above");
                        if i < n {
                            self.choose(i);
                        }
                    }
                    Keycode::R => self.reroll_choice(),
                    Keycode::X => self.skip_choice(),
                    // Cards deal in first, so a held key can't pick blind.
                    Keycode::Return | Keycode::Space if self.choice_view.age > 0.4 => {
                        self.choose(self.choice_view.selected);
                    }
                    _ => {}
                }
                return;
            }
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
                (hud::Mode::Paused, Keycode::Q) | (hud::Mode::Reveal, Keycode::Escape)
                    if self.mode != hud::Mode::Reveal || revealed =>
                {
                    ctx.should_quit = true;
                    return;
                }
                (hud::Mode::Reveal, Keycode::Space | Keycode::Return) if !revealed => {
                    self.pack.age = f32::MAX;
                    return;
                }
                (hud::Mode::Reveal, Keycode::R) if revealed => {
                    self.restart_requested = true;
                    return;
                }
                (hud::Mode::Reveal, Keycode::S) if revealed => {
                    self.save_journal();
                    return;
                }
                (hud::Mode::Reveal, Keycode::B) if revealed => {
                    self.open_booklet();
                    return;
                }
                (hud::Mode::Paused, Keycode::B) => {
                    self.open_booklet();
                    return;
                }
                (hud::Mode::Reveal, Keycode::L) if revealed => {
                    self.open_store();
                    return;
                }
                (hud::Mode::Paused, Keycode::L) => {
                    self.open_store();
                    return;
                }
                (hud::Mode::Booklet | hud::Mode::Store, Keycode::Escape) => {
                    self.mode = self.booklet_return;
                    return;
                }
                (hud::Mode::Booklet, Keycode::A | Keycode::Left) => {
                    self.booklet_page = self.booklet_page.saturating_sub(1);
                    return;
                }
                (hud::Mode::Booklet, Keycode::D | Keycode::Right) => {
                    let last = cards::page_count(self.booklet.card_count()) - 1;
                    self.booklet_page = (self.booklet_page + 1).min(last);
                    return;
                }
                (hud::Mode::Store, Keycode::W | Keycode::Up) => {
                    let n = store::Shelf::ALL.len();
                    self.shop_shelf = (self.shop_shelf + n - 1) % n;
                    self.shop_col = shop_ui::clamp_col(self.shop_shelf, self.shop_col);
                    return;
                }
                (hud::Mode::Store, Keycode::S | Keycode::Down) => {
                    self.shop_shelf = (self.shop_shelf + 1) % store::Shelf::ALL.len();
                    self.shop_col = shop_ui::clamp_col(self.shop_shelf, self.shop_col);
                    return;
                }
                (hud::Mode::Store, Keycode::A | Keycode::Left) => {
                    let n = store::Shelf::ALL[self.shop_shelf].items().len();
                    self.shop_col = (self.shop_col + n - 1) % n;
                    return;
                }
                (hud::Mode::Store, Keycode::D | Keycode::Right) => {
                    let n = store::Shelf::ALL[self.shop_shelf].items().len();
                    self.shop_col = (self.shop_col + 1) % n;
                    return;
                }
                (hud::Mode::Title, Keycode::W | Keycode::Up) => {
                    let n = title_ui::MENU.len();
                    self.title_sel = (self.title_sel + n - 1) % n;
                    return;
                }
                (hud::Mode::Title, Keycode::S | Keycode::Down) => {
                    self.title_sel = (self.title_sel + 1) % title_ui::MENU.len();
                    return;
                }
                (hud::Mode::Title, Keycode::Return | Keycode::Space) => {
                    match self.title_sel {
                        0 => self.start_from_title(),
                        title_ui::RUN_ROW => self.run_length = self.run_length.toggled(),
                        2 => self.open_store(),
                        3 => self.open_booklet(),
                        _ => ctx.should_quit = true,
                    }
                    return;
                }
                (hud::Mode::Title, Keycode::R) => {
                    self.run_length = self.run_length.toggled();
                    self.title_sel = title_ui::RUN_ROW;
                    return;
                }
                (hud::Mode::Title, Keycode::L) => {
                    self.open_store();
                    return;
                }
                (hud::Mode::Title, Keycode::B) => {
                    self.open_booklet();
                    return;
                }
                (hud::Mode::Title, Keycode::Q | Keycode::Escape) => {
                    ctx.should_quit = true;
                    return;
                }
                (hud::Mode::Store, Keycode::Return | Keycode::Space) => {
                    self.store_select();
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
            Keycode::LShift | Keycode::RShift if down && !repeat => self.ability_requests[0] = true,
            Keycode::E if down && !repeat => self.ability_requests[1] = true,
            Keycode::F2 if down && !repeat => {
                self.tunnel_on = !self.tunnel_on;
                log::info!("tunnel vision: {}", self.tunnel_on);
            }
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
        let time = self.time;
        for (_e, (t, bob)) in self.world.query_mut::<(&mut Transform, &Bob)>() {
            t.position.y = bob.base + bob.amp * (time * bob.speed + bob.phase).sin();
        }
        self.title_age += dt;
        if self.mode == hud::Mode::Summary && self.autopilot && self.title_age > 1.5 {
            self.title_age = 0.0;
            self.open_pack();
        }
        if self.mode == hud::Mode::Choice {
            self.choice_view.age += dt;
            // E2E runs: take the first card (WAKE at the door) after a beat.
            if self.autopilot && self.choice_view.age > 0.6 {
                self.choose(0);
            }
            return Ok(());
        }
        if self.mode == hud::Mode::Reveal && self.pack.age < f32::MAX {
            self.pack.age += dt;
        }
        if std::mem::take(&mut self.restart_requested) {
            return self.restart(ctx);
        }
        // E2E runs: show the journal briefly (for screenshots), then exit.
        if self.autopilot
            && matches!(
                self.mode,
                hud::Mode::Journal | hud::Mode::Booklet | hud::Mode::Reveal | hud::Mode::Store
            )
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
        // 0. Dream transition: the world is frozen while it melts and reforms.
        if let Some(ev) = self.transition.tick(dt) {
            match ev {
                transition::Event::Swap(p) => {
                    self.finish_melt(ctx, p)?;
                    // The world just changed under us (or the run ended):
                    // don't run this frame's gameplay against it, or a
                    // player still standing in the old portal re-triggers.
                    return Ok(());
                }
                transition::Event::Done => {
                    log::info!("Melt: reformed at depth {}", self.director.depth);
                    if std::mem::take(&mut self.offer_upgrade) {
                        self.open_upgrade_choice();
                        return Ok(());
                    }
                }
            }
        }
        if self.transition.active() {
            self.camera_pos = gameplay::follow_camera(self.camera_pos, self.player_position, dt);
            return Ok(());
        }
        let Some(player) = self.player else {
            return Ok(());
        };
        self.dream_age += dt;
        self.stats.time += dt;
        self.run.tick(dt);

        // 0b. Abilities
        let requests = std::mem::take(&mut self.ability_requests);
        for (slot, _) in requests.iter().enumerate().filter(|(_, &r)| r) {
            self.use_ability(slot);
        }
        if self.run.active(Ability::ShardCall) {
            self.pull_shard(dt);
        }

        // 1. Input (or autopilot) -> player body
        let (desired, wants_jump) = if self.autopilot {
            self.autopilot_step()
        } else {
            let mut v = gameplay::horizontal_velocity(&self.input)
                * (self.player_speed() / gameplay::MOVE_SPEED);
            if self.inverted() {
                v = Vec3::new(-v.x, v.y, -v.z);
            }
            (v, self.input.jump)
        };
        if desired.length_squared() > 1e-4 {
            self.facing = desired.normalize();
        }
        let jump_pressed = wants_jump && !self.jump_was_down;
        self.jump_was_down = wants_jump;
        let jump_speed = gameplay::JUMP_SPEED * self.run.jump() * self.twists.jump();
        let dashing = self.run.active(Ability::Dash);
        if let Ok(mut body) = self.world.get::<&mut RigidBody>(player) {
            let v = if dashing {
                self.facing * gameplay::MOVE_SPEED * upgrades::DASH_SPEED_MULT
            } else {
                desired
            };
            body.velocity.x = v.x;
            body.velocity.z = v.z;
            if body.grounded {
                self.air_jumps_used = 0;
            }
            if wants_jump && body.grounded {
                body.velocity.y = jump_speed;
            } else if jump_pressed && !body.grounded && self.air_jumps_used < self.run.air_jumps() {
                self.air_jumps_used += 1;
                body.velocity.y = jump_speed;
                self.tone(740.0, 0.08);
            }
        }

        // 2. Enemies (STILLNESS and STUTTERING dreams hold them in place)
        let target = self.player_position;
        let held =
            self.run.active(Ability::Stillness) || self.twists.enemies_frozen(self.dream_age);
        for (entity, ai) in self.enemies.iter_mut() {
            if held {
                continue;
            }
            if let Ok(mut t) = self.world.get::<&mut Transform>(*entity) {
                let was = ai.chasing;
                ai.update(&mut t.position, target, dt);
                // Swell when they notice you: the tell that you've been seen.
                t.scale = Vec3::splat(if ai.chasing { 1.25 } else { 0.9 });
                if ai.chasing && !was && !self.autopilot {
                    log::info!("An enemy noticed you");
                }
            }
        }
        if let Some((entity, h, _)) = self.hunter.as_mut() {
            if !held {
                if let Ok(mut t) = self.world.get::<&mut Transform>(*entity) {
                    h.update(&mut t.position, target, dt);
                    t.scale = Vec3::splat(h.size());
                }
            }
        }

        // 3. Physics: gravity, collision, trigger overlaps
        let physics = PhysicsParams {
            gravity: PhysicsParams::default().gravity * self.twists.gravity(),
            ..PhysicsParams::default()
        };
        let overlaps = engine::physics::step(&mut self.world, dt, &physics);
        if let Ok(t) = self.world.get::<&Transform>(player) {
            self.player_position = t.position;
        }
        let under = self.player_position;
        for (_e, (t, _)) in self.world.query_mut::<(&mut Transform, &Halo)>() {
            t.position = Vec3::new(under.x, 0.04, under.z);
        }
        self.camera_pos = gameplay::follow_camera(self.camera_pos, self.player_position, dt);

        // 4. Fail states (autopilot is immune to enemies so E2E runs are deterministic)
        let untouchable = self.run.active(Ability::Dash) || self.run.active(Ability::Phase);
        let caught = !self.autopilot
            && self.grace <= 0.0
            && !untouchable
            && self
                .enemies
                .iter()
                .map(|(e, _)| (*e, false))
                .chain(self.hunter.iter().map(|(e, _, _)| (*e, true)))
                .any(|(e, is_hunter)| {
                    self.world
                        .get::<&Transform>(e)
                        .map(|t| {
                            if is_hunter {
                                // Flat distance: it's too big to jump over.
                                let d = t.position - self.player_position;
                                Vec3::new(d.x, 0.0, d.z).length() < hunter::HUNTER_TOUCH
                            } else {
                                gameplay::touches(
                                    t.position,
                                    self.player_position,
                                    gameplay::ENEMY_TOUCH_RADIUS,
                                )
                            }
                        })
                        .unwrap_or(false)
                });
        if caught {
            self.flash.trigger([1.0, 0.1, 0.15], 0.7);
            self.tone(110.0, 0.35);
            let forgiven = self.has_perk(store::Perk::SecondWind)
                && !self.second_wind_used
                && self.director.shard_this_dream;
            let anchored =
                !forgiven && self.director.shard_this_dream && self.run.anchors_left() > 0;
            if forgiven {
                self.second_wind_used = true;
                log::info!("Second wind: the dream lets you keep your shard");
            }
            if anchored {
                self.run.anchors_used += 1;
                log::info!("Dream anchor: the shard stays with you");
            }
            let forgiven = forgiven || anchored;
            if !forgiven && self.director.caught() {
                self.shards_this_run = self.shards_this_run.saturating_sub(1);
                if let Some(r) = self.run_log.last_mut() {
                    r.shard_taken = false;
                }
                log::info!(
                    "Shard dropped ({}/{}) — it's back where you found it",
                    self.director.lucidity,
                    self.director.shards_to_wake
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
            if caught {
                self.stats.caught += 1;
            } else {
                self.stats.fell += 1;
            }
            // The hunter goes back to its corner to catch its breath.
            if let Some((entity, h, home)) = self.hunter.as_mut() {
                h.rest();
                if let Ok(mut t) = self.world.get::<&mut Transform>(*entity) {
                    t.position = *home;
                }
            }
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
                self.shards_this_run += 1;
                if let Some(r) = self.run_log.last_mut() {
                    r.shard_taken = true;
                }
                log::info!(
                    "Lucidity shard collected at depth {} ({}/{})",
                    self.director.depth,
                    self.director.lucidity,
                    self.director.shards_to_wake
                );
                self.update_title(ctx);
                break;
            }
        }
        let sigils: Vec<Entity> = touched
            .iter()
            .copied()
            .filter(|&e| self.world.get::<&SigilMarker>(e).is_ok())
            .collect();
        for e in sigils {
            let _ = self.world.despawn(e);
            self.sigils_left = self.sigils_left.saturating_sub(1);
            self.flash.trigger([1.0, 0.4, 1.0], 0.5);
            self.tone(
                660.0 + 110.0 * (dream::SIGILS - self.sigils_left) as f32,
                0.2,
            );
            log::info!("Sigil taken ({} left)", self.sigils_left);
            if self.sigils_left == 0 {
                self.open_portal()?;
            }
        }
        if touched
            .iter()
            .any(|&e| self.world.get::<&WakeMarker>(e).is_ok())
        {
            self.open_wake_choice();
            return Ok(());
        }
        if touched
            .iter()
            .any(|&e| self.world.get::<&PortalMarker>(e).is_ok())
        {
            log::info!("Portal entered at depth {}", self.director.depth);
            self.dream_bonus();
            self.stats.twists.extend(self.twists.0.iter().copied());
            if self.director.nightmare {
                self.stats.nightmares += 1;
                self.boss_reward = true;
                log::info!("Nightmare beaten at depth {}", self.director.depth);
            }
            self.begin_melt(if self.director.theme == DreamTheme::Awakening {
                transition::Pending::Finish
            } else {
                transition::Pending::Descend
            });
        }
        Ok(())
    }

    fn render(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        let hud_view = self.hud_view();
        let drawable_size = ctx.drawable_size();
        let time = self.time;
        let flash = self.flash;
        let melt = self.transition.melt();
        let transition = self.transition;
        let strangeness = self.visual_strangeness();
        let player = self.player_position;
        let up = if self.inverted() && !self.debug_camera {
            -Vec3::Y
        } else {
            Vec3::Y
        };
        let sight_scale = self.twists.sight() * self.run.sight();
        // Nightmares: you get to see the whole fight coming.
        let sight_bonus = self.run.sight_bonus()
            + if self.hunter.is_some() {
                1.5 * gameplay::CELL
            } else {
                0.0
            };
        // Off on the title/menus and in the F1 overview; on while dreaming.
        let tunnel = self.tunnel_on && !self.debug_camera && self.mode != hud::Mode::Title;
        let sight = if tunnel {
            tunnel::sight_radius(strangeness) * sight_scale + sight_bonus
        } else {
            0.0
        };
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
        let fog = transition.fog(params.fog_color);
        let dark = tunnel::darkness(fog);
        let clear = if tunnel { dark } else { fog };
        renderer.resize_if_needed(gl, drawable_size)?;
        renderer.begin_scene(gl);
        unsafe {
            // egui (drawn last frame) leaves depth test off and blending/scissor on.
            gl.enable(engine::glow::DEPTH_TEST);
            gl.disable(engine::glow::BLEND);
            gl.disable(engine::glow::SCISSOR_TEST);
        }
        unsafe {
            gl.clear_color(clear[0], clear[1], clear[2], 1.0);
            gl.clear(engine::glow::COLOR_BUFFER_BIT | engine::glow::DEPTH_BUFFER_BIT);
        }

        let aspect = drawable_size.0 as f32 / drawable_size.1.max(1) as f32;
        let aspect_for_overlay = aspect;
        let proj = Mat4::perspective_rh(60.0_f32.to_radians(), aspect, 0.1, 1000.0);
        let view = Mat4::look_at_rh(eye, target, up);

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
                gl.uniform_3_f32(Some(&l), fog[0], fog[1], fog[2]);
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
            if let Some(l) = loc("uPlayer") {
                gl.uniform_3_f32(Some(&l), player.x, player.y, player.z);
            }
            if let Some(l) = loc("uSight") {
                gl.uniform_1_f32(Some(&l), sight);
            }
            if let Some(l) = loc("uSightFade") {
                gl.uniform_1_f32(Some(&l), tunnel::SIGHT_FADE);
            }
            if let Some(l) = loc("uDark") {
                gl.uniform_3_f32(Some(&l), dark[0], dark[1], dark[2]);
            }
        }

        let model_loc = unsafe { gl.get_uniform_location(program, "uModel") };
        let uv_loc = unsafe { gl.get_uniform_location(program, "uUVScale") };
        let melt_loc = unsafe { gl.get_uniform_location(program, "uMelt") };
        let lit_loc = unsafe { gl.get_uniform_location(program, "uLit") };
        let hero_loc = unsafe { gl.get_uniform_location(program, "uHero") };
        let ghost_loc = unsafe { gl.get_uniform_location(program, "uGhost") };
        let outline_loc = unsafe { gl.get_uniform_location(program, "uOutline") };
        unsafe {
            if let Some(l) = &ghost_loc {
                gl.uniform_1_f32(Some(l), 0.0);
            }
        }
        // 0: everything opaque. 1: a dark shell around the dreamer (inflated
        // back faces), so it reads on bright floors as well as dark ones.
        // 2 (translucent): water. 3: the dreamer's silhouette wherever
        // something stands in front of it.
        for pass in 0..4 {
            unsafe {
                match pass {
                    0 => {}
                    1 => {
                        gl.enable(engine::glow::CULL_FACE);
                        gl.cull_face(engine::glow::FRONT);
                        if let Some(l) = &outline_loc {
                            gl.uniform_1_f32(Some(l), 1.0);
                        }
                    }
                    2 => {
                        if params.backface_culling {
                            gl.cull_face(engine::glow::BACK);
                        } else {
                            gl.disable(engine::glow::CULL_FACE);
                        }
                        if let Some(l) = &outline_loc {
                            gl.uniform_1_f32(Some(l), 0.0);
                        }
                        gl.enable(engine::glow::BLEND);
                        gl.blend_func(engine::glow::SRC_ALPHA, engine::glow::ONE_MINUS_SRC_ALPHA);
                        gl.depth_mask(false);
                        if let Some(l) = &ghost_loc {
                            gl.uniform_1_f32(Some(l), 0.45);
                        }
                    }
                    _ => {
                        gl.depth_func(engine::glow::GREATER);
                        if let Some(l) = &ghost_loc {
                            gl.uniform_1_f32(Some(l), 0.4);
                        }
                    }
                }
            }
            for (_entity, (transform, mesh_renderer, uv, solid, lit, hero, water)) in self
                .world
                .query::<(
                    &Transform,
                    &MeshRenderer,
                    Option<&SurfaceUv>,
                    Option<&NoMelt>,
                    Option<&Lit>,
                    Option<&Hero>,
                    Option<&Water>,
                )>()
                .iter()
            {
                let wanted = match pass {
                    0 => water.is_none(),
                    2 => water.is_some(),
                    _ => hero.is_some(),
                };
                if !wanted {
                    continue;
                }
                unsafe {
                    if let Some(l) = &lit_loc {
                        gl.uniform_1_f32(Some(l), if lit.is_some() { 1.0 } else { 0.0 });
                    }
                    if let Some(l) = &hero_loc {
                        gl.uniform_1_f32(Some(l), if hero.is_some() { 1.0 } else { 0.0 });
                    }
                    if let Some(l) = &melt_loc {
                        gl.uniform_1_f32(Some(l), if solid.is_some() { 0.0 } else { melt });
                    }
                    if let Some(l) = &uv_loc {
                        gl.uniform_1_f32(Some(l), uv.map_or(0.0, |u| u.0));
                    }
                    if let Some(l) = &model_loc {
                        gl.uniform_matrix_4_f32_slice(
                            Some(l),
                            false,
                            &if pass == 1 {
                                Transform {
                                    scale: transform.scale * OUTLINE_SCALE,
                                    ..*transform
                                }
                                .matrix()
                            } else {
                                transform.matrix()
                            }
                            .to_cols_array(),
                        );
                    }
                }
                if let Some(texture) = &mesh_renderer.texture {
                    texture.bind(gl, 0);
                }
                mesh_renderer.mesh.draw(gl);
            }
        }
        unsafe {
            gl.depth_func(engine::glow::LESS);
            gl.depth_mask(true);
            gl.disable(engine::glow::BLEND);
            if let Some(l) = &ghost_loc {
                gl.uniform_1_f32(Some(l), 0.0);
            }
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
        let card_art = &mut self.card_art;
        let pack = &self.pack;
        let grain = &mut self.grain;
        // Where the player is on screen (NDC -> 0..1), for the vignette centre.
        let clip = Mat4::perspective_rh(60.0_f32.to_radians(), aspect_for_overlay, 0.1, 1000.0)
            * Mat4::look_at_rh(eye, target, up)
            * player.extend(1.0);
        let player_uv = if clip.w > 0.0 {
            [0.5 + 0.5 * clip.x / clip.w, 0.5 - 0.5 * clip.y / clip.w]
        } else {
            [0.5, 0.5]
        };
        let overlay = tunnel && hud_view.mode == hud::Mode::Playing
            || tunnel && hud_view.mode == hud::Mode::Paused;
        if let Some(ui) = self.ui.as_mut() {
            let output = ui.run(drawable_size, |egui_ctx| {
                if install_fonts {
                    hud::install_font(egui_ctx);
                }
                if overlay {
                    let tex = grain
                        .get_or_insert_with(|| {
                            egui_ctx.load_texture(
                                "film-grain",
                                tunnel::grain_image(7),
                                egui::TextureOptions::NEAREST_REPEAT,
                            )
                        })
                        .id();
                    let screen = egui_ctx.screen_rect();
                    let p = egui_ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Background,
                        egui::Id::new("tunnel_vision"),
                    ));
                    let at = egui::Pos2::new(
                        screen.left() + player_uv[0] * screen.width(),
                        screen.top() + player_uv[1] * screen.height(),
                    );
                    tunnel::draw_overlay(&p, screen, at, strangeness, time, Some(tex));
                }
                hud::draw(egui_ctx, &hud_view, card_art, pack);
            });
            ui.paint(drawable_size, output);
        }
        self.maybe_screenshot(ctx);
        Ok(())
    }
}

impl DreamscapeGame {
    /// DREAMSCAPE_SHOT=out.png: after DREAMSCAPE_SHOT_AT seconds (default 6)
    /// of the run, save the frame and quit. Used by tools/screenshots.sh.
    fn maybe_screenshot(&mut self, ctx: &mut Context) {
        let Ok(path) = std::env::var("DREAMSCAPE_SHOT") else {
            return;
        };
        let at: f32 = std::env::var("DREAMSCAPE_SHOT_AT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(6.0);
        if self.time < at || ctx.should_quit {
            return;
        }
        let (w, h) = ctx.drawable_size();
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        unsafe {
            let gl = ctx.gl();
            gl.bind_framebuffer(engine::glow::READ_FRAMEBUFFER, None);
            gl.read_pixels(
                0,
                0,
                w as i32,
                h as i32,
                engine::glow::RGBA,
                engine::glow::UNSIGNED_BYTE,
                engine::glow::PixelPackData::Slice(&mut rgba),
            );
        }
        // GL rows run bottom-up.
        let row = (w * 4) as usize;
        let flipped: Vec<u8> = rgba.chunks(row).rev().flatten().copied().collect();
        match image::RgbaImage::from_raw(w, h, flipped).map(|img| img.save(&path)) {
            Some(Ok(())) => log::info!("Screenshot saved to {path}"),
            other => log::error!("Screenshot {path} failed: {other:?}"),
        }
        ctx.should_quit = true;
    }
}

fn main() -> anyhow::Result<()> {
    init_logging();
    std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));
    let seed = run_seed();
    log::info!("Dream run seed = {seed} (replay with DREAMSCAPE_SEED={seed})");
    App::run("Dreamscape", 1280, 720, DreamscapeGame::new(seed))
}
