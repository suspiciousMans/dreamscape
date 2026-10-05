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

mod achievements;
mod booklet_ui;
mod boss;
mod cards;
mod codex_ui;
mod companion;
mod coop;
mod coop_game;
mod crash;
mod dev;
mod dissolve;
mod dream;
mod ending;
mod enemy_ai;
mod eye;
mod eye_motion;
mod lottery;
mod lottery_ui;
mod retention;
mod streaks;
mod worth;
mod fpv;
mod fusion;
mod gameplay;
mod hud;
mod hunter;
mod loadout_ui;
mod lore;
mod memories_ui;
mod merge_ui;
mod music;
mod objective;
mod pad;
mod paths;
mod pixels;
mod powers;
mod progress;
mod records;
mod reveal_ui;
mod settings;
mod settings_ui;
mod shop_ui;
mod sounds;
mod specials;
mod steam;
mod store;
mod summary_ui;
mod title_ui;
mod transition;
mod tunnel;
mod tutorial;
mod upgrade_ui;
mod upgrades;

use dream::{
    Atmosphere, BlockKind, Dream, DreamDirector, DreamTheme, EnemyKind, Pressure, PropKind,
    RunLength, Shape, TexSpec, Twists, Variant, TEX_SIZE,
};
use enemy_ai::EnemyAI;
use gameplay::{PlayerInputState, PortalMarker};
use sounds::Sound;
use specials::Elite;
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
/// A note lying in the dream: touch to read it.
#[derive(Clone, Copy)]
struct NoteMarker;
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

/// Segments the boss's shockwave ring is drawn with.
const RING_SEGMENTS: usize = 40;

/// The dreamer's own body (hidden in first-person dreams).
#[derive(Clone, Copy)]
struct PlayerBody;

/// The glowing ring on the floor under the dreamer.
#[derive(Clone, Copy)]
struct Halo;

/// Drawn in the translucent pass (FLOODED water, fog, sentry beams, mimics).
#[derive(Clone, Copy)]
struct Translucent;

/// Floats up and down around `base` y.
#[derive(Clone, Copy)]
struct Bob {
    base: f32,
    amp: f32,
    speed: f32,
    phase: f32,
}

/// A pacing enemy, maybe an elite.
struct Pacer {
    entity: Entity,
    ai: EnemyAI,
    elite: Option<Elite>,
    /// Patrol ends (a splitter's child walks them the other way round).
    a: Vec3,
    b: Vec3,
    /// A splitter that has already split.
    split: bool,
}

enum SpecialAI {
    Stalker(specials::Stalker),
    Mimic,
    /// The sentry and its beam entity.
    Sentry(specials::Sentry, Entity),
    Drifter(EnemyAI),
    Jester(EnemyAI),
}

struct SpecialEnemy {
    entity: Entity,
    kind: EnemyKind,
    ai: SpecialAI,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Crumble {
    Solid,
    /// Seconds until it falls.
    Shaking(f32),
    /// Seconds until it comes back.
    Gone(f32),
}

/// Seconds a crumble tile shakes before falling, and stays gone.
const CRUMBLE_SHAKE: f32 = 1.0;
const CRUMBLE_GONE: f32 = 4.0;

struct CrumbleTile {
    entity: Option<Entity>,
    block: dream::Block,
    texture: Arc<GpuTexture>,
    state: Crumble,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum StartKind {
    Daily,
    Continue,
    Prologue,
}

/// Title menu entries.
#[derive(Clone, Copy, Debug, PartialEq)]
enum TitleItem {
    Continue,
    Start,
    Daily,
    RunLength,
    Ascension,
    Store,
    Booklet,
    Memories,
    Prologue,
    Codex,
    Settings,
    HostCoop,
    JoinCoop,
    Quit,
}

fn today() -> u64 {
    progress::day_number(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    )
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
    /// Abilities fired this run.
    abilities: u32,
}

/// The point in `among` nearest `from` (`among` must not be empty).
fn nearest(among: &[Vec3], from: Vec3) -> Vec3 {
    *among
        .iter()
        .min_by(|a, b| {
            a.distance_squared(from)
                .total_cmp(&b.distance_squared(from))
        })
        .expect("someone to hunt")
}

/// Seconds an achievement's name stays up.
const ACHIEVEMENT_TOAST: f32 = 4.0;

/// What the choice screen is choosing.
#[derive(Clone, Debug, PartialEq)]
enum ChoiceKind {
    Upgrade(Vec<Upgrade>),
    /// At the wake door: 0 = wake, 1 = go deeper.
    WakeDoor,
}

fn init_logging() {
    let mut b = env_logger::Builder::from_default_env();
    b.format_timestamp_millis();
    // Release: no console, so log to a file players can send us.
    if !cfg!(debug_assertions) {
        if let Some(file) = crash::open_log() {
            b.target(env_logger::Target::Pipe(Box::new(file)));
            if std::env::var_os("RUST_LOG").is_none() {
                b.filter_level(log::LevelFilter::Info);
            }
        }
    }
    let _ = b.try_init();
}

/// DREAMSCAPE_SEED=<u64> replays a run exactly; otherwise seed from the clock.
fn run_seed() -> u64 {
    crate::dev::var("DREAMSCAPE_SEED")
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
    enemies: Vec<Pacer>,
    specials: Vec<SpecialEnemy>,
    /// Where the dreamer has been (the mimic walks it).
    trail: specials::Trail,
    mimic_awake_at: f32,
    jester_cooldown: f32,
    crumbles: Vec<CrumbleTile>,
    fog: Vec<(Vec3, f32)>,
    /// Mycelium veins (segments you walk faster on).
    veins: Vec<(Vec3, Vec3)>,
    /// Tunnel gates: (membrane entity, centre, direction, phase).
    gates: Vec<(Entity, Vec3, Vec3, f32)>,
    /// Elfworks shifting tiles: (entity, level position, phase).
    shifters: Vec<(Entity, Vec3, f32)>,
    /// Synesthesia Hall: (plate, cell centre) and the dream's tempo.
    beat_tiles: Vec<(Entity, Vec3)>,
    bpm: f32,
    /// Melting Clockworks: loops completed in this dream.
    loops: u32,
    /// Afterimage Fields: your ghosts (entity, born at dream_age) and the spawn timer.
    echoes: Vec<(Entity, f32)>,
    /// Each loadout ability and the dream colours of the card that taught it.
    ability_tints: Vec<(Ability, [u8; 3], [u8; 3])>,
    /// An ability just fired: burst here, in these colours (spawned next update).
    burst_pending: Option<(Vec3, [u8; 3], [u8; 3])>,
    /// Burst cubes: (entity, dream age at birth, index in the ring, centre).
    bursts: Vec<(Entity, f32, usize, Vec3)>,
    /// This dream's music, being composed on a worker thread.
    music_job: Option<std::sync::mpsc::Receiver<Vec<f32>>>,
    /// Length of the current music loop, seconds.
    music_loop: f32,
    /// DECOY: where it stands and its marker.
    decoy: Option<(Vec3, Entity)>,
    /// REWIND: where you've stood lately.
    rewind_trail: upgrades::Trail,
    echo_timer: f32,
    player_tex: Option<Arc<GpuTexture>>,
    /// Watching Wallpaper: eyes in the walls.
    watchers: Vec<Entity>,
    /// Entrance animation: the eye sinks in from above the screen on run start.
    entrance: eye_motion::Entrance,
    /// Seconds of play since the run began; drives the 20s objective glance.
    objective_clock: f32,
    /// The lid as drawn: eases toward the lucidity-driven opening so the eye
    /// never snaps shut.
    eye_lid: f32,
    /// Summed bonuses of this run's loadout cards (quirks + mastery).
    card_fx: worth::QuirkEffect,
    /// Card numbers that went along this run (they earn mastery xp).
    loadout_numbers: Vec<u32>,
    /// What the eye should say first next run (a pack, a streak, a level-up).
    pending_eye: Option<eye::Moment>,
    /// The run was loaded from a save: its first dream isn't a fresh one.
    just_continued: bool,
    /// Lucid Store tab: 0 = the shelves, 1 = the lottery.
    store_tab: usize,
    lottery_outcome: Option<lottery_ui::Outcome>,
    /// Seconds since the last pull (drives the shuffle and reveal).
    lottery_age: f32,
    lottery_status: Option<String>,
    /// Index into `lottery::pullable_themes()` for the spark pick.
    lottery_pick: usize,
    /// What the last pressed run did for its cards (shown on the pack screen).
    last_report: Option<retention::RunReport>,
    /// White Dissolve: the path state and its tiles (entity, full size).
    dissolve: Option<(dissolve::Dissolve, Vec<(Entity, Vec3)>)>,
    /// Special kinds met so far this run (each gets a hint the first time).
    seen_kinds: Vec<EnemyKind>,
    /// Shown under the title card.
    dream_hint: String,
    /// Every sound effect, synthesized at start-up.
    sounds: HashMap<Sound, Vec<f32>>,
    /// Booklet [p]: save this page's cards as images after the next frame.
    export_requested: bool,
    settings: settings::Settings,
    settings_path: std::path::PathBuf,
    settings_sel: usize,
    /// Waiting for a key to bind on the selected row.
    settings_capture: bool,
    /// Where the settings screen returns to.
    settings_return: hud::Mode,
    /// Ascension level picked on the title screen for the next run.
    ascension: u32,
    /// The rules of the run in progress.
    active_asc: progress::Ascension,
    /// The run in progress is today's daily dream (day number).
    daily: Option<u64>,
    /// A run is in progress (saved at the start of each dream).
    run_active: bool,
    save_path: std::path::PathBuf,
    /// A start requested from the title (handled next update, with a GL context).
    pending_start: Option<StartKind>,
    /// Where the codex screen returns to.
    codex_return: hud::Mode,

    input: PlayerInputState,
    player: Option<Entity>,
    player_position: Vec3,
    camera_pos: Vec3,
    /// DREAMSCAPE_FPS=1: (accumulated ms, frames) for the average frame-time log.
    frame_ms: (f32, u32),
    /// First-person look (radians); only used when `first_person_active()`.
    yaw: f32,
    pitch: f32,
    mouse_captured: bool,
    /// Dev switch: DREAMSCAPE_FP=1 forces first person in every dream.
    force_fp: bool,
    player_grounded: bool,
    /// Last input came from a controller: show button prompts, not keys.
    using_pad: bool,
    /// Controller button -> the key its press sent (released as the same key).
    pad_held: HashMap<i32, Keycode>,
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
    /// Seconds the photosensitivity warning has been up.
    warning_age: f32,
    /// The eye's current line and its age.
    eye_line: Option<(String, f32)>,
    /// The note this dream holds: (dreamer, note index).
    pending_note: Option<(u32, u32)>,
    /// The note being read (Mode::Note).
    note_view: Option<lore::Note>,
    /// What this dream asks of you (None: nightmares, lucid dreams, no shard).
    objective: Option<objective::Active>,
    /// Rolled before the dream is built (fragments need cells).
    objective_kind: Option<objective::Kind>,
    memories_sel: usize,
    loadout_sel: usize,
    /// The merge screen: cursor, first card picked, last result, where to go back.
    merge_sel: usize,
    merge_first: Option<u32>,
    merge_status: Option<String>,
    merge_return: hud::Mode,
    /// Seconds into the ending (`Mode::Ending`).
    ending_age: f32,
    /// This run's difficulty factor from its starting kit.
    loadout_scale: f32,
    /// A shared dream, if this is one.
    coop: Option<coop_game::CoopState>,
    /// Co-op: the run to start next frame (seed, long, dreamers).
    pending_coop_start: Option<(u64, bool, usize)>,
    /// Times caught in the current dream (untouched nightmares).
    caught_this_dream: u32,
    /// Seconds spent holding a stalker still with your gaze.
    stare: f32,
    /// A just-earned achievement's name, and how long it has shown.
    achievement_toast: Option<(String, f32)>,
    /// This run's portal leads to the bottom (the ending plays on arrival).
    to_bottom: bool,
    /// The first-time guided dreams, while they run.
    prologue: Option<tutorial::Prologue>,
    /// The eye's line is a tutorial prompt: stays up until the step changes.
    eye_sticky: bool,
    /// Lines the eye says next, one after another.
    eye_queue: Vec<String>,
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
    hunter: Option<(Entity, boss::Boss, Vec3)>,
    sigils_left: usize,
    /// Sigils in this nightmare (3, or 4 from the fourth tier).
    sigils_total: usize,
    /// Where taken sigils stood, newest last (a tier-3 catch puts one back).
    sigils_taken: Vec<Vec3>,
    /// The shockwave ring and the wisp bodies, mirrored from the boss.
    boss_fx: Option<(Vec<Entity>, [Entity; boss::MAX_WISPS])>,
    sigil_tex: Option<Arc<GpuTexture>>,
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
            specials: Vec::new(),
            trail: specials::Trail::default(),
            mimic_awake_at: 0.0,
            jester_cooldown: 0.0,
            crumbles: Vec::new(),
            fog: Vec::new(),
            veins: Vec::new(),
            gates: Vec::new(),
            shifters: Vec::new(),
            beat_tiles: Vec::new(),
            bpm: 0.0,
            loops: 0,
            echoes: Vec::new(),
            ability_tints: Vec::new(),
            burst_pending: None,
            bursts: Vec::new(),
            music_job: None,
            music_loop: 1.0,
            decoy: None,
            rewind_trail: upgrades::Trail::default(),
            echo_timer: 0.0,
            player_tex: None,
            watchers: Vec::new(),
            dissolve: None,
            seen_kinds: Vec::new(),
            dream_hint: String::new(),
            export_requested: false,
            sounds: sounds::ALL_SOUNDS
                .iter()
                .map(|&s| (s, sounds::synth(s)))
                .collect(),
            settings: settings::load(&settings::settings_path()),
            settings_path: settings::settings_path(),
            settings_sel: 0,
            settings_capture: false,
            settings_return: hud::Mode::Title,
            ascension: 0,
            active_asc: progress::Ascension(0),
            daily: None,
            run_active: false,
            save_path: progress::save_path(),
            pending_start: None,
            codex_return: hud::Mode::Title,
            input: PlayerInputState::default(),
            player: None,
            player_position: Vec3::ZERO,
            frame_ms: (0.0, 0),
            yaw: 0.0,
            pitch: 0.0,
            mouse_captured: false,
            force_fp: crate::dev::var_os("DREAMSCAPE_FP").is_some(),
            player_grounded: false,
            // Dev switch: DREAMSCAPE_PAD=1 starts with controller prompts.
            using_pad: crate::dev::var_os("DREAMSCAPE_PAD").is_some(),
            pad_held: HashMap::new(),
            camera_pos: gameplay::CAMERA_OFFSET,
            debug_camera: false,
            autopilot: crate::dev::var("DREAMSCAPE_AUTOPILOT").is_ok(),
            route_index: 0,
            flash: gameplay::Flash::default(),
            grace: 0.0,
            shard_entities: Vec::new(),
            shard_tex: None,
            wake_tex: None,
            best_depth: records::load(&records::record_path()),
            ui: None,
            mode: if crate::dev::var("DREAMSCAPE_AUTOPILOT").is_ok() {
                hud::Mode::Playing
            } else if crate::dev::var("DREAMSCAPE_SKIP_WARNING").is_ok() {
                hud::Mode::Title
            } else {
                hud::Mode::Warning
            },
            title_age: 0.0,
            warning_age: 0.0,
            eye_line: None,
            pending_note: None,
            note_view: None,
            objective: None,
            objective_kind: None,
            memories_sel: 0,
            loadout_sel: 0,
            merge_sel: 0,
            merge_first: None,
            merge_status: None,
            merge_return: hud::Mode::Title,
            ending_age: 0.0,
            loadout_scale: 1.0,
            coop: None,
            pending_coop_start: None,
            caught_this_dream: 0,
            stare: 0.0,
            achievement_toast: None,
            to_bottom: false,
            prologue: None,
            eye_sticky: false,
            eye_queue: Vec::new(),
            dream_name: String::new(),
            dream_whisper: String::new(),
            run_log: Vec::new(),
            booklet: {
                let mut b = cards::load(&cards::booklet_path());
                b.stash.migrate();
                if b.lore.save_seed == 0 {
                    b.lore.save_seed = run_seed ^ 0x5EED_0F_D2EA;
                }
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
            entrance: eye_motion::Entrance::new(),
            objective_clock: 0.0,
            eye_lid: eye_motion::LID_SHUT,
            card_fx: worth::QuirkEffect::default(),
            loadout_numbers: Vec::new(),
            pending_eye: None,
            just_continued: false,
            store_tab: 0,
            lottery_outcome: None,
            lottery_age: 0.0,
            lottery_status: None,
            lottery_pick: 0,
            last_report: None,
            shards_this_run: 0,
            second_wind_used: false,
            transition: transition::Transition::new(
                crate::dev::var("DREAMSCAPE_MELT_SCALE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1.0),
            ),
            run_seed,
            restart_requested: false,
            fonts_installed: false,
            grain: None,
            tunnel_on: crate::dev::var("DREAMSCAPE_NO_TUNNEL").is_err(),
            run_length: match crate::dev::var("DREAMSCAPE_RUN").as_deref() {
                Ok("long") => RunLength::Long,
                _ => RunLength::Short,
            },
            run: RunUpgrades::default(),
            twists: Twists::default(),
            perks: Vec::new(),
            dream_age: 0.0,
            bonus_dust: 0,
            peak_difficulty: 1.0,
            offer_upgrade: false,
            choice: None,
            choice_view: upgrade_ui::ChoiceView::default(),
            boss_reward: false,
            hunter: None,
            sigils_left: 0,
            sigils_total: 0,
            sigils_taken: Vec::new(),
            boss_fx: None,
            sigil_tex: None,
            sealed_portal: None,
            stats: RunStats::default(),
            air_jumps_used: 0,
            jump_was_down: false,
            facing: Vec3::Z,
            ability_requests: [false; upgrades::ABILITY_SLOTS],
        }
    }

    fn difficulty(&self) -> f32 {
        gameplay::difficulty(self.director.hardness()) * self.loadout_scale
    }

    fn pressure(&self) -> Pressure {
        self.coop_pressure(self.pressure_base())
    }

    fn pressure_base(&self) -> Pressure {
        let hard = self.director.hardness().is_some();
        Pressure {
            enemies: gameplay::pressured_enemy_count(self.difficulty()) * self.twists.enemy_count(),
            growth_cap: if hard { 24 } else { 12 },
            // Dev switch: DREAMSCAPE_SPECIAL=Stalker puts that kind in every dream.
            force_special: crate::dev::var("DREAMSCAPE_SPECIAL").ok().and_then(|name| {
                dream::ALL_ENEMY_KINDS
                    .iter()
                    .copied()
                    .find(|k| format!("{k:?}") == name)
            }),
        }
    }

    /// Screen-relative controls: in an UPSIDE DOWN dream the camera is
    /// rolled 180 degrees, so the world directions flip with it.
    fn inverted(&self) -> bool {
        self.twists.has(Variant::Inverted)
    }

    /// First-person dreams (or DREAMSCAPE_FP=1); the F1 overview camera wins.
    fn first_person_active(&self) -> bool {
        !self.debug_camera
            && self
                .dream
                .as_ref()
                .is_some_and(|d| d.theme.first_person() || self.force_fp)
    }

    /// The look direction, when seeing through the dreamer's eyes.
    fn fp_yaw(&self) -> Option<f32> {
        self.first_person_active().then_some(self.yaw)
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
                } else if self.run.would_max(u) {
                    format!("{} · maxes out", info.tier.label())
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
        self.sfx(Sound::Reroll);
        self.open_upgrade_choice();
    }

    /// Every one of your own notes found, and the ending not yet seen.
    fn ending_due(&self) -> bool {
        let l = &self.booklet.lore;
        self.prologue.is_none()
            && self.coop.is_none()
            && !l.ending_seen
            && l.count(0) >= lore::notes_for(0)
            && (!self.autopilot || crate::dev::var("DREAMSCAPE_ENDING").is_ok())
    }

    /// [enter] after the credits: the story is told; the run goes on awake.
    fn finish_ending(&mut self) {
        if !ending::can_continue(self.ending_age) {
            return;
        }
        self.booklet.lore.ending_seen = true;
        self.persist_booklet();
        log::info!("The ending is seen");
        self.mode = hud::Mode::Playing;
    }

    /// After a beaten nightmare's reward, the moth offers to merge cards.
    fn moth_after_nightmare(&mut self) {
        let cards: Vec<cards::Card> = self.booklet.cards().cloned().collect();
        if self.boss_reward
            && !self.autopilot
            && self.prologue.is_none()
            && self.coop.is_none()
            && merge_ui::any_pair(&cards)
        {
            self.open_merge(hud::Mode::Playing);
        }
    }

    fn skip_choice(&mut self) {
        if !matches!(self.choice, Some(ChoiceKind::Upgrade(_))) {
            return;
        }
        self.choice = None;
        self.mode = hud::Mode::Playing;
        self.moth_after_nightmare();
        self.boss_reward = false;
        self.bonus_dust += upgrades::SKIP_DUST;
        self.stats.skipped += 1;
        self.coop_ready();
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
        self.think(eye::Moment::WakeDoor, false);
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
                self.moth_after_nightmare();
                self.boss_reward = false;
                self.director.shard_bonus = self.shard_bonus();
                if u == Upgrade::LucidHeart {
                    // Banked straight away: it can't be dropped.
                    self.director.lucidity += 1;
                    self.shards_this_run += 1;
                }
                if self.run_active {
                    let n = self.run.synergies().len();
                    self.achieve(Some(achievements::Event::Synergies(n)));
                }
                for s in self.run.synergies() {
                    if !before.contains(&s) {
                        log::info!("Synergy: {} ({})", s.name(), s.desc());
                    }
                }
                log::info!("Upgrade taken: {u:?} (run: {:?})", self.run.taken);
                self.coop_ready();
                self.flash
                    .trigger(u.info().color.map(|c| c as f32 / 255.0), 0.5);
                self.sfx(Sound::Pick);
            }
            ChoiceKind::WakeDoor if index == 0 => {
                log::info!("Wake door taken at depth {}", self.director.depth);
                self.flash.trigger([1.0, 1.0, 1.0], 1.0);
                self.sfx(Sound::WakeDoor);
                self.begin_melt(transition::Pending::Wake);
            }
            ChoiceKind::WakeDoor => {
                self.director.go_deeper();
                if self.run_active {
                    self.achieve(Some(achievements::Event::WentDeeper));
                }
                log::info!(
                    "Refused to wake at depth {}: {} shards to wake, difficulty now x{:.2}",
                    self.director.depth,
                    self.director.shards_to_wake,
                    self.difficulty()
                );
                self.despawn_shard_slot();
                self.flash.trigger([1.0, 0.2, 0.4], 0.8);
                self.sfx(Sound::Deeper);
                self.eye_line = None;
                self.think(eye::Moment::WentDeeper, false);
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

    /// Composes this dream's loop off the main thread (inline on the web,
    /// which has no threads). `poll_music` starts it in time with the
    /// dream's clock, so beat tiles burn on the beat.
    fn start_music(&mut self, theme: DreamTheme, strangeness: f32, seed: u64) {
        let st = music::style(theme);
        self.music_loop = music::loop_secs(&st, music::BARS);
        log::info!("Music: {theme:?} in {:?} at {} bpm", st.scale, st.bpm);
        let compose = move || {
            let notes = music::score(&st, strangeness, seed, music::BARS);
            music::render(&st, &notes, strangeness, music::BARS)
        };
        let (tx, rx) = std::sync::mpsc::channel();
        #[cfg(not(target_os = "emscripten"))]
        std::thread::spawn(move || {
            let _ = tx.send(compose());
        });
        #[cfg(target_os = "emscripten")]
        let _ = tx.send(compose());
        // A newer dream replaces the job; the old thread's send just fails.
        self.music_job = Some(rx);
    }

    fn poll_music(&mut self) {
        let Some(rx) = &self.music_job else {
            return;
        };
        let Ok(samples) = rx.try_recv() else {
            return;
        };
        self.music_job = None;
        let offset = self.dream_age.rem_euclid(self.music_loop.max(0.01));
        if let Some(audio) = self.audio.as_mut() {
            log::info!(
                "Music playing: {:.1}s loop, joined {offset:.2}s in",
                samples.len() as f32 / sounds::RATE as f32
            );
            audio.play_music_samples(samples, sounds::RATE, offset);
        }
    }

    fn mesh(&self, shape: Shape) -> anyhow::Result<Arc<GpuMesh>> {
        self.meshes
            .get(&shape)
            .cloned()
            .context("mesh not uploaded")
    }

    fn hud_view(&self) -> hud::HudView {
        let target = self.objective_target();
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
            eye_line: self.eye_line.clone().map(|(t, age)| {
                (
                    t,
                    if self.eye_sticky {
                        age.min(eye::LINE_SECS)
                    } else {
                        age
                    },
                )
            }),
            note: self
                .note_view
                .as_ref()
                .map(|n| (n.object.to_string(), n.title.clone(), n.body.clone())),
            memories: if self.mode == hud::Mode::Memories {
                self.memories_view()
            } else {
                memories_ui::MemoriesView::default()
            },
            loadout: if self.mode == hud::Mode::Loadout {
                loadout_ui::LoadoutView {
                    cards: self.booklet.cards().cloned().collect(),
                    chosen: self.booklet.lore.loadout.clone(),
                    slots: self.booklet.lore.slots(),
                    selected: self.loadout_sel,
                    companion: (!self.booklet.lore.dreamers.is_empty()).then(|| {
                        let l = &self.booklet.lore;
                        match l.companion() {
                            Some(id) => {
                                let d = lore::dreamer(l.save_seed, id);
                                format!(
                                    "[c] companion: {}  {}",
                                    d.name,
                                    companion::describe(d.weight)
                                )
                            }
                            None => "[c] companion: nobody".to_string(),
                        }
                    }),
                }
            } else {
                loadout_ui::LoadoutView::default()
            },
            merge: if self.mode == hud::Mode::Merge {
                merge_ui::MergeView {
                    cards: self.booklet.cards().cloned().collect(),
                    selected: self.merge_sel,
                    first: self.merge_first,
                    status: self.merge_status.clone(),
                }
            } else {
                merge_ui::MergeView::default()
            },
            ending_age: self.ending_age,
            achievement: self.achievement_toast.clone(),
            coop_line: self.coop_line(),
            lobby: if self.mode == hud::Mode::Lobby {
                self.lobby_view()
            } else {
                coop_game::LobbyView::default()
            },
            title_age: self.title_age,
            warning_age: self.warning_age,
            shard_dir: target.and_then(|t| {
                if self.first_person_active() {
                    fpv::compass(self.player_position, self.yaw, t, 4.0)
                } else {
                    hud::compass(self.player_position.to_array(), t.to_array(), 7.0).map(|d| {
                        if self.inverted() {
                            [-d[0], -d[1]]
                        } else {
                            d
                        }
                    })
                }
            }),
            first_person: self.first_person_active(),
            pad: self.using_pad,
            settings_snapshot: self.settings.clone(),
            dream_age: self.dream_age,
            glance: eye_motion::glance(self.objective_clock),
            lid: self.eye_lid,
            entrance_age: if !self.entrance.done {
                self.entrance.t
            } else {
                f32::MAX
            },
            boss: self.hunter.as_ref().map(|(_, b, _)| {
                (
                    format!(
                        "{} — {}",
                        boss::title(b.tier),
                        eye::nightmare_subtitle(
                            &lore::dreamer(self.booklet.lore.save_seed, 0),
                            b.tier.saturating_sub(1)
                        )
                    ),
                    self.sigils_total - self.sigils_left,
                    self.sigils_total,
                    b.warning(),
                )
            }),
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
            store_tab: self.store_tab,
            streak: self.booklet.streak.streak,
            goal: if matches!(
                self.mode,
                hud::Mode::Title | hud::Mode::Summary | hud::Mode::Reveal
            ) {
                let loadout: Vec<u32> = self.booklet.loadout_cards().iter().map(|c| c.number).collect();
                streaks::teaser(&retention::progress_inputs(&self.booklet, &loadout, self.best_depth))
            } else {
                String::new()
            },
            report_line: self.last_report.as_ref().map_or_else(String::new, |r| {
                let mut bits = Vec::new();
                if r.xp_each > 0 && !self.loadout_numbers.is_empty() {
                    bits.push(format!("+{} MASTERY XP", r.xp_each));
                }
                if r.echoes > 0 {
                    bits.push(format!("{} FADED DREAMS LEFT ECHOES", r.echoes));
                }
                bits.join("  ·  ")
            }),
            lottery: if self.mode == hud::Mode::Store && self.store_tab == 1 {
                self.lottery_view()
            } else {
                lottery_ui::LotteryView::default()
            },
            mastery_levels: self
                .booklet
                .cards()
                .map(|c| (c.number, self.booklet.mastery.level_of(c.number)))
                .collect(),
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
            summary: if self.mode == hud::Mode::Summary {
                self.summary_view()
            } else {
                summary_ui::SummaryView::default()
            },
            hint: self.dream_hint.clone(),
            menu: if self.mode == hud::Mode::Title {
                self.title_items()
                    .into_iter()
                    .map(|i| {
                        let (k, l, n) = self.title_label(i);
                        (k.to_string(), l, n)
                    })
                    .collect()
            } else {
                Vec::new()
            },
            codex: if self.mode == hud::Mode::Codex {
                let c = &self.booklet.codex;
                codex_ui::CodexView {
                    dreams: dream::ALL_THEMES
                        .iter()
                        .map(|&t| (codex_ui::spaced(&format!("{t:?}")), c.best_in(t)))
                        .collect(),
                    enemies: dream::ALL_ENEMY_KINDS
                        .iter()
                        .map(|&k| {
                            (
                                codex_ui::spaced(&format!("{k:?}")),
                                c.enemies.contains(&k).then(|| k.hint().to_string()),
                            )
                        })
                        .collect(),
                    footer: format!(
                        "{} of {} dreams · {} nightmares beaten · ascension unlocked: {}",
                        c.dreams.len(),
                        dream::ALL_THEMES.len(),
                        c.nightmares_beaten,
                        c.ascension_unlocked
                    ),
                }
            } else {
                codex_ui::CodexView::default()
            },
            settings: if self.mode == hud::Mode::Settings {
                settings_ui::SettingsView {
                    rows: settings::rows()
                        .into_iter()
                        .map(|r| (r.label(), settings::value(&self.settings, r)))
                        .collect(),
                    selected: self.settings_sel,
                    capturing: self.settings_capture,
                }
            } else {
                settings_ui::SettingsView::default()
            },
            objective: self
                .hunter
                .as_ref()
                .map(|_| {
                    if self.sigils_left > 0 {
                        format!(
                            "SIGILS {}/{}",
                            self.sigils_total - self.sigils_left,
                            self.sigils_total
                        )
                    } else {
                        "THE PORTAL IS OPEN".into()
                    }
                })
                .or_else(|| {
                    let looping = self
                        .dream
                        .as_ref()
                        .is_some_and(|d| d.theme.spec().feature == dream::Feature::TimeLoop);
                    looping.then(|| {
                        let left = specials::LOOP_SECS - self.dream_age % specials::LOOP_SECS;
                        format!("LOOP {}  ·  {:>2}s", self.loops + 1, left.ceil() as i32)
                    })
                })
                .or_else(|| {
                    // A plain shard dream says nothing (the beacon says it all).
                    self.objective
                        .as_ref()
                        .filter(|o| !matches!(o, objective::Active::Shard))
                        .map(|o| o.label())
                }),
            choice: if self.mode == hud::Mode::Choice {
                self.choice_view.clone()
            } else {
                upgrade_ui::ChoiceView::default()
            },
        }
    }

    /// A crumbling floor tile (solid, with its collider).
    /// Afterimage Fields: leave a fading ghost of yourself every ECHO_EVERY.
    /// An ability's ring of cubes in its card's colours, fading out.
    fn update_bursts(&mut self, gl: &engine::glow::Context) -> anyhow::Result<()> {
        let now = self.dream_age;
        if let Some((at, a, b)) = self.burst_pending.take() {
            let cube = self.mesh(Shape::Cube)?;
            for i in 0..gameplay::BURST_CUBES {
                let c = if i % 2 == 0 { a } else { b };
                let tex = self.texture(gl, [c[0], c[1], c[2], 255]);
                let (off, s) = gameplay::burst_cube(i, 0.0);
                let e = self.world.spawn((
                    Transform {
                        position: at + off,
                        rotation: Quat::IDENTITY,
                        scale: Vec3::splat(s),
                    },
                    MeshRenderer {
                        mesh: cube.clone(),
                        texture: Some(tex),
                    },
                    Spin(4.0),
                    Lit,
                    Hero,
                ));
                self.bursts.push((e, now, i, at));
            }
        }
        let world = &mut self.world;
        self.bursts.retain(|&(e, born, i, at)| {
            let age = now - born;
            if age >= gameplay::BURST_SECONDS {
                let _ = world.despawn(e);
                return false;
            }
            if let Ok(mut t) = world.get::<&mut Transform>(e) {
                let (off, s) = gameplay::burst_cube(i, age);
                t.position = at + off;
                t.scale = Vec3::splat(s);
            }
            true
        });
        Ok(())
    }

    fn update_echoes(&mut self, player: Entity, dt: f32) -> anyhow::Result<()> {
        let echoing = self
            .dream
            .as_ref()
            .is_some_and(|d| d.theme.spec().feature == dream::Feature::Echoes);
        if !echoing {
            return Ok(());
        }
        let now = self.dream_age;
        let moving = self
            .world
            .get::<&RigidBody>(player)
            .is_ok_and(|b| Vec3::new(b.velocity.x, 0.0, b.velocity.z).length() > 0.5);
        self.echo_timer -= dt;
        if moving && self.echo_timer <= 0.0 {
            self.echo_timer = specials::ECHO_EVERY;
            if let Some(tex) = self.player_tex.clone() {
                let e = self.world.spawn((
                    Transform {
                        position: self.player_position,
                        rotation: Quat::IDENTITY,
                        scale: Vec3::new(0.9, 1.25, 0.9),
                    },
                    MeshRenderer {
                        mesh: self.mesh(Shape::Octahedron)?,
                        texture: Some(tex),
                    },
                    Translucent,
                    Lit,
                ));
                self.echoes.push((e, now));
            }
        }
        while self.echoes.len() > specials::MAX_ECHOES {
            let (e, _) = self.echoes.remove(0);
            let _ = self.world.despawn(e);
        }
        let mut gone = Vec::new();
        for (k, &(e, born)) in self.echoes.iter().enumerate() {
            let s = specials::echo_scale(now - born);
            if s <= 0.0 {
                gone.push(k);
            } else if let Ok(mut t) = self.world.get::<&mut Transform>(e) {
                t.scale = Vec3::new(0.9, 1.25, 0.9) * s;
            }
        }
        for k in gone.into_iter().rev() {
            let (e, _) = self.echoes.remove(k);
            let _ = self.world.despawn(e);
        }
        Ok(())
    }

    /// Mirrors the boss's ring and wisps onto their entities.
    fn sync_boss_fx(&mut self) {
        let (Some((_, b, _)), Some((ring, wisps))) = (self.hunter.as_ref(), self.boss_fx.as_ref())
        else {
            return;
        };
        for (k, e) in ring.iter().enumerate() {
            if let Ok(mut t) = self.world.get::<&mut Transform>(*e) {
                match b.attack {
                    Some(boss::Attack::Ring { centre, radius }) => {
                        let a = std::f32::consts::TAU * k as f32 / RING_SEGMENTS as f32;
                        let out = Vec3::new(a.cos(), 0.0, a.sin());
                        let arc = std::f32::consts::TAU * radius / RING_SEGMENTS as f32;
                        t.position = centre + out * radius + Vec3::Y * boss::RING_H * 0.5;
                        // Local X runs along the ring, local Z across the band.
                        t.rotation = Quat::from_rotation_y(-a + std::f32::consts::FRAC_PI_2);
                        t.scale = Vec3::new(arc * 1.05, boss::RING_H, boss::RING_T);
                    }
                    _ => t.scale = Vec3::ZERO,
                }
            }
        }
        for (k, e) in wisps.iter().enumerate() {
            if let Ok(mut t) = self.world.get::<&mut Transform>(*e) {
                match b.wisps.get(k) {
                    Some(w) => {
                        t.position = w.pos + Vec3::Y * 0.7;
                        t.scale = Vec3::splat(0.6);
                    }
                    None => t.scale = Vec3::ZERO,
                }
            }
        }
    }

    /// A nightmare sigil (also used to put one back after a tier-3 catch).
    fn spawn_sigil(&mut self, at: Vec3, k: usize) {
        let (Some(sigil_tex), Ok(sigil_mesh)) =
            (self.sigil_tex.clone(), self.mesh(Shape::Octahedron))
        else {
            return;
        };
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

    fn spawn_tile(&mut self, b: &dream::Block, texture: Arc<GpuTexture>) -> anyhow::Result<Entity> {
        let mesh = self.mesh(b.shape)?;
        Ok(self.world.spawn((
            Transform {
                position: b.pos,
                rotation: b.rotation,
                scale: b.size,
            },
            MeshRenderer {
                mesh,
                texture: Some(texture),
            },
            SurfaceUv(0.5 / gameplay::CELL),
            Collider {
                shape: ColliderShape::Aabb {
                    half_extents: b.size * 0.5,
                },
                is_trigger: false,
            },
        )))
    }

    /// Veins along the floor and gates across the tunnel.
    fn spawn_features(&mut self, gl: &engine::glow::Context, dream: &Dream) -> anyhow::Result<()> {
        if !dream.veins.is_empty() {
            let spec = dream.theme.spec();
            let tex = self.upload_surface(
                gl,
                &TexSpec {
                    pattern: dream::Pattern::Veins,
                    palette: spec.accents.to_vec(),
                    bands: 1.0,
                    seed: dream.seed ^ 0x7E1,
                },
            )?;
            let cube = self.mesh(Shape::Cube)?;
            for &(a, b) in &dream.veins {
                let d = b - a;
                let size = if d.x.abs() > d.z.abs() {
                    Vec3::new(d.x.abs() + 0.4, 0.03, 0.4)
                } else {
                    Vec3::new(0.4, 0.03, d.z.abs() + 0.4)
                };
                self.world.spawn((
                    Transform {
                        position: (a + b) * 0.5 + Vec3::Y * 0.02,
                        rotation: Quat::IDENTITY,
                        scale: size,
                    },
                    MeshRenderer {
                        mesh: cube.clone(),
                        texture: Some(tex.clone()),
                    },
                    SurfaceUv(0.6),
                    Lit,
                    NoMelt,
                ));
            }
        }
        if !dream.gates.is_empty() {
            let skin = self.texture(gl, [200, 150, 255, 255]);
            let frame_tex = self.texture(gl, [255, 245, 210, 255]);
            let slab = self.mesh(Shape::Cube)?;
            let ring = self.mesh(Shape::Torus)?;
            for &(at, dir, phase) in &dream.gates {
                let facing = Quat::from_rotation_arc(Vec3::Z, dir);
                self.world.spawn((
                    Transform {
                        position: at + Vec3::Y * 1.3,
                        rotation: facing,
                        scale: Vec3::new(2.9, 2.6, 0.3),
                    },
                    MeshRenderer {
                        mesh: ring.clone(),
                        texture: Some(frame_tex.clone()),
                    },
                    Lit,
                ));
                let membrane = self.world.spawn((
                    Transform {
                        position: at + Vec3::Y * 1.3,
                        rotation: facing,
                        scale: Vec3::ZERO,
                    },
                    MeshRenderer {
                        mesh: slab.clone(),
                        texture: Some(skin.clone()),
                    },
                    Translucent,
                    Lit,
                ));
                self.gates.push((membrane, at, dir, phase));
            }
        }
        Ok(())
    }

    /// Gates breathe open and shut; shifting tiles sink and rise.
    fn update_features(&mut self) {
        let t = self.dream_age;
        for &(e, _, _, phase) in &self.gates {
            if let Ok(mut tr) = self.world.get::<&mut Transform>(e) {
                let shut = specials::gate_closed(t, phase);
                // Shut, it fills the corridor wall to wall: exactly what hits you.
                let target = if shut {
                    Vec3::new(gameplay::CELL, 2.6, specials::GATE_DEPTH * 2.0)
                } else {
                    Vec3::ZERO
                };
                tr.scale = tr.scale.lerp(target, 0.25);
            }
        }
        // Beat tiles: full and burning on the downbeat, a small tell otherwise.
        let hot = specials::beat_hot(t, self.bpm);
        for &(e, _) in &self.beat_tiles {
            if let Ok(mut tr) = self.world.get::<&mut Transform>(e) {
                let (xz, y) = if hot { (0.9, 0.04) } else { (0.5, 0.005) };
                tr.scale = Vec3::new(gameplay::CELL * xz, y, gameplay::CELL * xz);
            }
        }
        // Watchers look at the shard (or the portal once it's gone).
        let target = self
            .shard_entities
            .first()
            .and_then(|&e| self.world.get::<&Transform>(e).ok().map(|t| t.position))
            .or(self.dream.as_ref().map(|d| d.portal));
        if let Some(target) = target {
            for &e in &self.watchers {
                if let Ok(mut tr) = self.world.get::<&mut Transform>(e) {
                    tr.rotation = Quat::from_rotation_y(specials::watcher_yaw(tr.position, target));
                }
            }
        }
        // Melting Clockworks: every LOOP_SECS the dream rewinds.
        let looping = self
            .dream
            .as_ref()
            .is_some_and(|d| d.theme.spec().feature == dream::Feature::TimeLoop);
        if looping {
            let n = specials::loop_index(t);
            if n != self.loops {
                self.loops = n;
                for p in &mut self.enemies {
                    p.ai.reset();
                    if let Ok(mut tr) = self.world.get::<&mut Transform>(p.entity) {
                        tr.position = p.a;
                    }
                }
                for sp in &self.specials {
                    if let SpecialAI::Stalker(st) = &sp.ai {
                        if let Ok(mut tr) = self.world.get::<&mut Transform>(sp.entity) {
                            tr.position = st.lair;
                        }
                    }
                }
                self.flash.trigger([1.0, 0.8, 0.4], 0.4);
                self.sfx(Sound::Deeper);
                // Enemies just snapped back to their posts, which sit on the
                // route, possibly on top of you. Same safety as a respawn,
                // so a rewind can never catch you before you can react.
                self.grace = self.grace.max(gameplay::RESPAWN_GRACE);
                log::info!("Time loop {n}");
                self.rewind_trail.clear(); // the past you'd rewind to no longer happened
            }
        }
        if self.autopilot {
            return; // E2E runs walk a level floor
        }
        for &(e, base, phase) in &self.shifters {
            if let Ok(mut tr) = self.world.get::<&mut Transform>(e) {
                tr.position =
                    base - Vec3::Y * specials::SHIFT_DROP * specials::shift_depth(t, phase);
            }
        }
    }

    /// Wading through a fog pocket?
    fn in_fog(&self) -> bool {
        self.fog.iter().any(|&(at, r)| {
            let d = at - self.player_position;
            Vec3::new(d.x, 0.0, d.z).length() < r
        })
    }

    /// Standing on a mycelium vein?
    fn on_vein(&self) -> bool {
        self.veins
            .iter()
            .any(|&(a, b)| specials::dist_to_segment(self.player_position, a, b) < 0.6)
    }

    /// Stalkers, mimics, sentries, drifters and jesters, plus fog pockets.
    fn spawn_specials(
        &mut self,
        gl: &engine::glow::Context,
        dream: &Dream,
        pacer_speed: f32,
        player_speed: f32,
        enemy_texture: &Arc<GpuTexture>,
    ) -> anyhow::Result<()> {
        let mut hints = Vec::new();
        for (k, sp) in dream.specials.iter().enumerate() {
            if !self.seen_kinds.contains(&sp.kind) {
                self.seen_kinds.push(sp.kind);
                hints.push(sp.kind.hint());
            }
            let (shape, scale, color) = match sp.kind {
                EnemyKind::Stalker => (
                    Shape::Octahedron,
                    Vec3::new(0.6, 2.2, 0.6),
                    [40, 25, 55, 255],
                ),
                EnemyKind::Mimic => (
                    Shape::Octahedron,
                    Vec3::new(0.9, 1.25, 0.9),
                    [255, 40, 70, 255],
                ),
                EnemyKind::Sentry => (Shape::Cone, Vec3::new(0.9, 1.6, 0.9), [255, 220, 120, 255]),
                EnemyKind::Drifter => (Shape::Orb, Vec3::new(1.0, 0.7, 1.0), [90, 230, 255, 255]),
                EnemyKind::Jester => (
                    Shape::Crescent,
                    Vec3::new(0.9, 0.9, 0.3),
                    [255, 120, 255, 255],
                ),
            };
            let mesh = self.mesh(shape)?;
            let texture = match sp.kind {
                EnemyKind::Drifter => enemy_texture.clone(),
                _ => self.texture(gl, color),
            };
            let position = match sp.kind {
                EnemyKind::Sentry => sp.at + Vec3::Y * scale.y * 0.5,
                _ => sp.at,
            };
            let entity = self.world.spawn((
                Transform {
                    position,
                    rotation: Quat::IDENTITY,
                    scale: if sp.kind == EnemyKind::Mimic {
                        Vec3::ZERO
                    } else {
                        scale
                    },
                },
                MeshRenderer {
                    mesh,
                    texture: Some(texture),
                },
            ));
            match sp.kind {
                EnemyKind::Stalker => {}
                EnemyKind::Mimic => self
                    .world
                    .insert(entity, (Translucent, Lit, Spin(1.2)))
                    .expect("just spawned"),
                EnemyKind::Jester => self
                    .world
                    .insert(entity, (Lit, Spin(4.0)))
                    .expect("just spawned"),
                _ => self.world.insert_one(entity, Lit).expect("just spawned"),
            }
            let ai = match sp.kind {
                EnemyKind::Stalker => {
                    SpecialAI::Stalker(specials::Stalker::new(sp.at, player_speed))
                }
                EnemyKind::Mimic => SpecialAI::Mimic,
                EnemyKind::Sentry => {
                    let beam_tex = self.texture(gl, [255, 240, 150, 255]);
                    let cone = self.mesh(Shape::Cone)?;
                    let beam = self.world.spawn((
                        Transform {
                            position: sp.at,
                            rotation: Quat::IDENTITY,
                            scale: Vec3::ZERO,
                        },
                        MeshRenderer {
                            mesh: cone,
                            texture: Some(beam_tex),
                        },
                        Translucent,
                        Lit,
                    ));
                    SpecialAI::Sentry(specials::Sentry::new(k as f32 * 2.3), beam)
                }
                EnemyKind::Drifter => {
                    SpecialAI::Drifter(EnemyAI::new(sp.a, sp.b, pacer_speed * 0.8, 0.0, 0.0))
                }
                EnemyKind::Jester => {
                    SpecialAI::Jester(EnemyAI::new(sp.a, sp.b, pacer_speed * 0.6, 0.0, 0.0))
                }
            };
            log::info!("Special enemy: {:?} at {:?}", sp.kind, sp.at);
            self.specials.push(SpecialEnemy {
                entity,
                kind: sp.kind,
                ai,
            });
        }
        // Fog pockets: low purple puffs you wade through slowly.
        if !dream.fog_pockets.is_empty() {
            let fog_tex = self.texture(gl, [150, 90, 220, 255]);
            let disc = self.mesh(Shape::Cylinder)?;
            for &(at, r) in &dream.fog_pockets {
                self.world.spawn((
                    Transform {
                        position: at + Vec3::Y * 0.3,
                        rotation: Quat::IDENTITY,
                        scale: Vec3::new(2.0 * r, 0.6, 2.0 * r),
                    },
                    MeshRenderer {
                        mesh: disc.clone(),
                        texture: Some(fog_tex.clone()),
                    },
                    Translucent,
                    Lit,
                ));
            }
            hints.push("purple fog slows you down");
        }
        self.dream_hint = hints.join(" · ");
        Ok(())
    }

    /// Specials, crumbling floor and the jester, once per frame.
    fn update_specials(&mut self, dt: f32, held: bool) -> anyhow::Result<()> {
        let player = self.player_position;
        self.trail.push(self.dream_age, player);
        self.jester_cooldown = (self.jester_cooldown - dt).max(0.0);
        let strangeness = self.visual_strangeness();
        let eclipse = self
            .hunter
            .as_ref()
            .map_or(1.0, |(_, b, _)| b.sight_scale());
        let clear =
            (tunnel::sight_radius(strangeness) * self.twists.sight() * self.run.sight() * eclipse
                + self.run.sight_bonus())
                * (1.0 - tunnel::SIGHT_FADE);
        let yaw = self.fp_yaw();
        let watched = !held
            && self.first_person_active()
            && self
                .specials
                .iter()
                .any(|sp| matches!(&sp.ai, SpecialAI::Stalker(s) if !s.moving));
        self.stare = if watched { self.stare + dt } else { 0.0 };
        if self.stare >= achievements::STARE_SECONDS && self.run_active {
            self.achieve(Some(achievements::Event::Stare(self.stare)));
        }
        let mut calls = Vec::new();
        for sp in &mut self.specials {
            let Ok(mut t) = self.world.get::<&mut Transform>(sp.entity) else {
                continue;
            };
            match &mut sp.ai {
                SpecialAI::Stalker(s) => {
                    if !held {
                        s.update(&mut t.position, player, clear, yaw, dt);
                    } else {
                        s.moving = false;
                    }
                }
                SpecialAI::Mimic => {
                    match specials::mimic_pos(&self.trail, self.dream_age, self.mimic_awake_at) {
                        Some(p) if !held => {
                            t.position = p;
                            t.scale = Vec3::new(0.9, 1.25, 0.9);
                        }
                        Some(_) => {}
                        None => t.scale = Vec3::ZERO,
                    }
                }
                SpecialAI::Sentry(s, beam) => {
                    let at = Vec3::new(t.position.x, 0.0, t.position.z);
                    if !held && s.update(at, player, dt) {
                        calls.push(at);
                    }
                    let facing = s.facing();
                    let beam = *beam;
                    drop(t);
                    if let Ok(mut bt) = self.world.get::<&mut Transform>(beam) {
                        let half_w = specials::SENTRY_RANGE * specials::SENTRY_HALF_ANGLE.tan();
                        // The cone's tip (+Y) sits on the sentry, its base out along the beam.
                        bt.rotation = Quat::from_rotation_arc(-Vec3::Y, facing);
                        bt.scale = Vec3::new(2.0 * half_w, specials::SENTRY_RANGE, 0.08);
                        bt.position = at + facing * specials::SENTRY_RANGE * 0.5 + Vec3::Y * 0.15;
                    }
                }
                SpecialAI::Drifter(ai) | SpecialAI::Jester(ai) => {
                    if !held {
                        ai.update(&mut t.position, player, dt);
                    }
                }
            }
        }
        for at in calls {
            log::info!("A sentry saw you");
            self.flash.trigger([1.0, 0.9, 0.4], 0.35);
            self.sfx(Sound::SentrySaw);
            for p in &mut self.enemies {
                let near = self
                    .world
                    .get::<&Transform>(p.entity)
                    .is_ok_and(|t| (t.position - at).length() < specials::SENTRY_CALL);
                if near {
                    p.ai.alert();
                }
            }
        }
        // Frozen stalkers you walk into shatter back to their lairs.
        for sp in &self.specials {
            if let SpecialAI::Stalker(s) = &sp.ai {
                if !s.moving {
                    if let Ok(mut t) = self.world.get::<&mut Transform>(sp.entity) {
                        let d = t.position - player;
                        if Vec3::new(d.x, 0.0, d.z).length() < gameplay::ENEMY_TOUCH_RADIUS {
                            t.position = s.lair;
                            self.sfx(Sound::StalkerShatter);
                        }
                    }
                }
            }
        }
        if !self.autopilot {
            self.jester_touch();
            self.update_crumbles(dt)?;
        }
        Ok(())
    }

    /// A jester's touch throws you onto safe floor a couple of cells away.
    fn jester_touch(&mut self) {
        if self.jester_cooldown > 0.0 {
            return;
        }
        let player = self.player_position;
        let touched = self.specials.iter().any(|sp| {
            sp.kind == EnemyKind::Jester
                && self.world.get::<&Transform>(sp.entity).is_ok_and(|t| {
                    gameplay::touches(t.position, player, gameplay::ENEMY_TOUCH_RADIUS)
                })
        });
        if !touched {
            return;
        }
        let seed = (self.dream_age * 1000.0) as u32;
        let landing = self.dream.as_ref().and_then(|d| {
            specials::throw_directions(seed)
                .into_iter()
                .find_map(|dir| {
                    gameplay::blink_target(&d.blocks, player, dir, specials::JESTER_THROW)
                })
        });
        self.jester_cooldown = specials::JESTER_COOLDOWN;
        if let (Some(at), Some(p)) = (landing, self.player) {
            if let Ok(mut t) = self.world.get::<&mut Transform>(p) {
                t.position = at;
            }
            if let Ok(mut b) = self.world.get::<&mut RigidBody>(p) {
                b.velocity = Vec3::ZERO;
            }
            self.player_position = at;
            self.flash.trigger([1.0, 0.5, 1.0], 0.4);
            self.sfx(Sound::JesterThrow);
            log::info!("A jester threw you");
        }
    }

    /// Crumbling tiles: shake when stood on, fall, come back.
    fn update_crumbles(&mut self, dt: f32) -> anyhow::Result<()> {
        let player = self.player_position;
        let grounded = self
            .player
            .and_then(|p| self.world.get::<&RigidBody>(p).ok().map(|b| b.grounded))
            .unwrap_or(false);
        let over = |b: &dream::Block| {
            (player.x - b.pos.x).abs() < b.size.x * 0.5
                && (player.z - b.pos.z).abs() < b.size.z * 0.5
        };
        let mut respawn = Vec::new();
        let mut fell = false;
        for (i, tile) in self.crumbles.iter_mut().enumerate() {
            tile.state = match tile.state {
                Crumble::Solid if grounded && over(&tile.block) => Crumble::Shaking(CRUMBLE_SHAKE),
                Crumble::Shaking(t) if t - dt <= 0.0 => {
                    if let Some(e) = tile.entity.take() {
                        let _ = self.world.despawn(e);
                        fell = true;
                    }
                    Crumble::Gone(CRUMBLE_GONE)
                }
                Crumble::Shaking(t) => {
                    if let Some(e) = tile.entity {
                        if let Ok(mut tr) = self.world.get::<&mut Transform>(e) {
                            let wobble = 0.06 * (self.time * 40.0).sin();
                            tr.position =
                                tile.block.pos + Vec3::new(wobble, -0.05 * (1.0 - t), 0.0);
                        }
                    }
                    Crumble::Shaking(t - dt)
                }
                Crumble::Gone(t) if t - dt <= 0.0 && !over(&tile.block) => {
                    respawn.push(i);
                    Crumble::Solid
                }
                Crumble::Gone(t) => Crumble::Gone((t - dt).max(0.0)),
                s => s,
            };
        }
        if fell {
            self.sfx(Sound::Crumble);
        }
        for i in respawn {
            let (block, texture) = (
                self.crumbles[i].block.clone(),
                self.crumbles[i].texture.clone(),
            );
            let e = self.spawn_tile(&block, texture)?;
            self.crumbles[i].entity = Some(e);
        }
        Ok(())
    }

    /// Is anything that catches you touching you right now?
    /// What's touching you, if anything (logged on a catch, for balancing).
    fn threat_touching(&self) -> Option<&'static str> {
        let player = self.player_position;
        let at = |e: Entity| self.world.get::<&Transform>(e).ok().map(|t| t.position);
        let pacers = self.enemies.iter().any(|p| {
            let reach = gameplay::ENEMY_TOUCH_RADIUS
                * if p.elite == Some(Elite::Big) {
                    specials::ELITE_BIG_REACH
                } else {
                    1.0
                };
            at(p.entity).is_some_and(|pos| gameplay::touches(pos, player, reach))
        });
        let hunter = self.hunter.iter().any(|(e, _, _)| {
            at(*e).is_some_and(|pos| {
                // Flat distance: it's too big to jump over.
                let d = pos - player;
                Vec3::new(d.x, 0.0, d.z).length() < hunter::HUNTER_TOUCH
            })
        });
        let special = self.specials.iter().any(|sp| {
            let armed = match &sp.ai {
                SpecialAI::Stalker(s) => s.moving,
                SpecialAI::Mimic => self.dream_age >= self.mimic_awake_at,
                SpecialAI::Drifter(_) => true,
                SpecialAI::Sentry(..) | SpecialAI::Jester(_) => false,
            };
            armed
                && at(sp.entity)
                    .is_some_and(|pos| gameplay::touches(pos, player, gameplay::ENEMY_TOUCH_RADIUS))
        });
        let beat = self.player_grounded
            && specials::beat_hot(self.dream_age, self.bpm)
            && self.beat_tiles.iter().any(|&(_, at)| {
                (at.x - player.x).abs() < gameplay::CELL * 0.5
                    && (at.z - player.z).abs() < gameplay::CELL * 0.5
            });
        let gate = self.gates.iter().any(|&(_, at, dir, phase)| {
            specials::gate_closed(self.dream_age, phase) && specials::gate_blocks(at, dir, player)
        });
        let boss_fx = self.hunter.iter().any(|(_, b, _)| b.hits(player));
        [
            (pacers, "patrol"),
            (hunter, "hunter"),
            (boss_fx, "boss attack"),
            (special, "special enemy"),
            (gate, "gate"),
            (beat, "beat tile"),
        ]
        .into_iter()
        .find_map(|(hit, what)| hit.then_some(what))
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
        self.sfx(Sound::PortalOpen);
        Ok(())
    }

    /// Fire the ability in `slot`, if held and charged.
    fn teleport_player(&mut self, at: Vec3) {
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

    fn use_ability(&mut self, slot: usize) {
        let Some(state) = self.run.abilities.get(slot).copied() else {
            return;
        };
        let mult = self.run.cooldown_for(state.ability);
        let stretch = self.run.duration_for(state.ability);
        if state.cooldown > 0.0 {
            self.sfx(Sound::Denied);
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
                self.sfx(Sound::Denied);
                return;
            };
            self.teleport_player(at);
        }
        if state.ability == Ability::Rewind {
            let Some(at) = self.rewind_trail.rewind(self.dream_age) else {
                self.sfx(Sound::Denied); // no past yet: keep the charge
                return;
            };
            self.teleport_player(at);
            self.rewind_trail.clear(); // don't rewind into the moment you just left
        }
        if self.run.abilities[slot].try_use(mult, stretch) {
            log::info!("Ability: {:?}", state.ability);
            self.stats.abilities += 1;
            if self.run_active {
                self.achieve(Some(achievements::Event::AbilityUsed {
                    ability: state.ability,
                    nightmare: self.director.nightmare,
                    theme: self.director.theme,
                }));
            }
            match state.ability {
                Ability::Dash if self.run.has(upgrades::Synergy::GhostStep) => {
                    self.grace = self
                        .grace
                        .max(Ability::Dash.duration() + upgrades::GHOST_STEP_SECONDS);
                }
                Ability::Blink if self.run.has(upgrades::Synergy::Skywalk) => {
                    self.air_jumps_used = 0;
                }
                Ability::Decoy => {
                    if let Some((_, e)) = self.decoy.take() {
                        let _ = self.world.despawn(e);
                    }
                    if let (Some(tex), Ok(mesh)) =
                        (self.player_tex.clone(), self.mesh(Shape::Octahedron))
                    {
                        let at = self.player_position;
                        let e = self.world.spawn((
                            Transform {
                                position: at,
                                rotation: Quat::IDENTITY,
                                scale: Vec3::new(0.9, 1.25, 0.9),
                            },
                            MeshRenderer {
                                mesh,
                                texture: Some(tex),
                            },
                            Spin(3.0),
                            Lit,
                            Hero, // stays lit in the tunnel darkness, like the player
                        ));
                        self.decoy = Some((at, e));
                    }
                }
                _ => {}
            }
            let color = match state.ability {
                Ability::Dash => [0.5, 0.9, 1.0],
                Ability::Blink => [0.9, 0.5, 1.0],
                Ability::Stillness => [0.6, 0.7, 1.0],
                Ability::Phase => [1.0, 1.0, 1.0],
                Ability::ShardCall => [0.3, 1.0, 1.0],
                Ability::Decoy => [1.0, 0.6, 0.9],
                Ability::Float => [0.7, 1.0, 0.8],
                Ability::Flare => [1.0, 0.95, 0.6],
                Ability::Rewind => [0.6, 0.6, 1.0],
            };
            let tint = self
                .ability_tints
                .iter()
                .find(|t| t.0 == state.ability)
                .map(|&(_, a, b)| (a, b));
            let color = tint.map_or(color, |(a, _)| a.map(|c| c as f32 / 255.0));
            self.flash.trigger(color, 0.3);
            if let Some((a, b)) = tint {
                self.burst_pending = Some((self.player_position, a, b));
            }
            self.sfx(match state.ability {
                Ability::Dash => Sound::Dash,
                Ability::Blink => Sound::Blink,
                Ability::Stillness => Sound::Stillness,
                Ability::Phase => Sound::Phase,
                Ability::ShardCall => Sound::ShardCall,
                Ability::Decoy => Sound::JesterThrow,
                Ability::Float => Sound::AirJump,
                Ability::Flare => Sound::PortalOpen,
                Ability::Rewind => Sound::Blink,
            });
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

    /// 1 = full motion effects; lower with the reduced-motion setting.
    fn motion_scale(&self) -> f32 {
        self.settings.motion()
    }

    fn open_settings(&mut self) {
        self.settings_return = self.mode;
        self.settings_sel = 0;
        self.settings_capture = false;
        self.input = PlayerInputState::default();
        self.mode = hud::Mode::Settings;
    }

    /// Pushes volumes and the window mode out to the engine, and saves.
    fn apply_settings(&mut self, ctx: &mut Context) {
        if let Some(audio) = self.audio.as_mut() {
            audio.set_music_volume(self.settings.music);
            audio.set_sfx_volume(self.settings.sfx);
        }
        let want = if self.settings.fullscreen {
            engine::sdl2::video::FullscreenType::Desktop
        } else {
            engine::sdl2::video::FullscreenType::Off
        };
        if ctx.platform.window.fullscreen_state() != want {
            if let Err(e) = ctx.platform.window.set_fullscreen(want) {
                log::warn!("fullscreen: {e}");
            }
        }
    }

    fn save_settings(&self) {
        if let Err(e) = settings::save(&self.settings_path, &self.settings) {
            log::warn!("could not save settings: {e}");
        }
    }

    /// Keys on the settings screen.
    fn settings_key(&mut self, ctx: &mut Context, key: Keycode) {
        let rows = settings::rows();
        let row = rows[self.settings_sel.min(rows.len() - 1)];
        if self.settings_capture {
            self.settings_capture = false;
            if let (settings::Row::Key(action), false) = (row, key == Keycode::Escape) {
                self.settings.bind(action, key);
                log::info!("Bound {action:?} to {}", key.name());
                self.save_settings();
            }
            return;
        }
        match key {
            Keycode::Escape => self.mode = self.settings_return,
            Keycode::W | Keycode::Up => {
                self.settings_sel = (self.settings_sel + rows.len() - 1) % rows.len()
            }
            Keycode::S | Keycode::Down => self.settings_sel = (self.settings_sel + 1) % rows.len(),
            Keycode::A | Keycode::Left | Keycode::D | Keycode::Right => {
                let dir = if matches!(key, Keycode::A | Keycode::Left) {
                    -1
                } else {
                    1
                };
                settings::adjust(&mut self.settings, row, dir);
                self.apply_settings(ctx);
                self.save_settings();
                self.sfx(Sound::Menu);
            }
            Keycode::Return | Keycode::Space => match row {
                settings::Row::Key(_) => self.settings_capture = true,
                settings::Row::Defaults => {
                    self.settings = settings::Settings::default();
                    self.apply_settings(ctx);
                    self.save_settings();
                }
                other => {
                    settings::adjust(&mut self.settings, other, 1);
                    self.apply_settings(ctx);
                    self.save_settings();
                }
            },
            _ => {}
        }
    }

    fn shard_bonus(&self) -> f64 {
        self.run.shard_bonus() - self.active_asc.shard_penalty()
    }

    /// The title menu, top to bottom.
    fn title_items(&self) -> Vec<TitleItem> {
        let mut items = Vec::new();
        if progress::load(&self.save_path).is_some() {
            items.push(TitleItem::Continue);
        }
        if !self.booklet.lore.prologue_done {
            items.push(TitleItem::Prologue);
        }
        items.extend([TitleItem::Start, TitleItem::Daily, TitleItem::RunLength]);
        if self.booklet.codex.ascension_unlocked > 0 {
            items.push(TitleItem::Ascension);
        }
        items.extend([
            TitleItem::Store,
            TitleItem::Booklet,
            TitleItem::Memories,
            TitleItem::Codex,
            TitleItem::Settings,
            TitleItem::HostCoop,
            TitleItem::JoinCoop,
        ]);
        if self.booklet.lore.prologue_done {
            items.push(TitleItem::Prologue);
        }
        items.push(TitleItem::Quit);
        items
    }

    fn title_label(&self, item: TitleItem) -> (&'static str, String, String) {
        let codex = &self.booklet.codex;
        match item {
            TitleItem::Continue => ("c", "continue the dream".into(), {
                progress::load(&self.save_path)
                    .map(|s| format!("depth {} · {} shards", s.depth, s.lucidity))
                    .unwrap_or_default()
            }),
            TitleItem::Start if !self.booklet.lore.prologue_done => {
                ("enter", "skip it and fall asleep".into(), String::new())
            }
            TitleItem::Start => ("enter", "fall asleep".into(), String::new()),
            TitleItem::Daily => (
                "t",
                "today's dream".into(),
                match codex.daily_best_for(today()) {
                    Some(d) => format!("the same dream for everyone today · your best: depth {d}"),
                    None => "the same dream for everyone today".into(),
                },
            ),
            TitleItem::RunLength => (
                "r",
                format!("run: {}", self.run_length.label()),
                title_ui::run_blurb(self.run_length.label()).into(),
            ),
            TitleItem::Ascension => (
                "a/d",
                format!("ascension: {}", self.ascension),
                format!(
                    "{} (unlocked up to {})",
                    progress::ascension_rule(self.ascension),
                    codex.ascension_unlocked
                ),
            ),
            TitleItem::Store => ("l", "the lucid store".into(), String::new()),
            TitleItem::Booklet => ("b", "dream booklet".into(), String::new()),
            TitleItem::Prologue if !self.booklet.lore.prologue_done => (
                "p",
                "begin".into(),
                "a short first dream: the eye will show you how".into(),
            ),
            TitleItem::Prologue => ("p", "replay the prologue".into(), String::new()),
            TitleItem::Memories => (
                "m",
                "memories".into(),
                if self.booklet.lore.ending_seen {
                    format!(
                        "{} notes found · you woke up. the others still dream",
                        self.booklet.lore.notes.len()
                    )
                } else {
                    format!("{} notes found", self.booklet.lore.notes.len())
                },
            ),
            TitleItem::Codex => (
                "x",
                "codex".into(),
                format!(
                    "{} dreams · {} strange things met",
                    codex.dreams.len(),
                    codex.enemies.len()
                ),
            ),
            TitleItem::Settings => ("o", "settings".into(), String::new()),
            TitleItem::HostCoop => (
                "h",
                "host a shared dream".into(),
                format!("up to {} dreamers · the others join you", coop::MAX_PLAYERS),
            ),
            TitleItem::JoinCoop => ("j", "join a shared dream".into(), String::new()),
            TitleItem::Quit => ("q", "stay awake".into(), String::new()),
        }
    }

    fn title_select(&mut self, ctx: &mut Context, item: TitleItem, dir: i32) {
        match item {
            TitleItem::Continue => self.pending_start = Some(StartKind::Continue),
            TitleItem::Start => self.open_loadout(),
            TitleItem::Prologue => self.pending_start = Some(StartKind::Prologue),
            TitleItem::Daily => self.pending_start = Some(StartKind::Daily),
            TitleItem::RunLength => self.run_length = self.run_length.toggled(),
            TitleItem::Ascension => {
                let max = self.booklet.codex.ascension_unlocked as i32;
                self.ascension = (self.ascension as i32 + dir).rem_euclid(max + 1) as u32;
            }
            TitleItem::Store => self.open_store(),
            TitleItem::Booklet => self.open_booklet(),
            TitleItem::Memories => {
                self.memories_sel = 0;
                self.mode = hud::Mode::Memories;
            }
            TitleItem::Codex => self.open_codex(),
            TitleItem::Settings => self.open_settings(),
            TitleItem::HostCoop => self.open_host_lobby(),
            TitleItem::JoinCoop => self.open_join_lobby(),
            TitleItem::Quit => ctx.should_quit = true,
        }
    }

    fn open_codex(&mut self) {
        self.codex_return = self.mode;
        self.mode = hud::Mode::Codex;
    }

    /// Starts today's dream, or continues a saved run.
    fn start_pending(&mut self, ctx: &mut Context, kind: StartKind) -> anyhow::Result<()> {
        self.transition.cancel();
        self.journal_status = None;
        match kind {
            StartKind::Daily => {
                self.prologue = None;
                let day = today();
                self.run_seed = progress::daily_seed(day);
                self.daily = Some(day);
                self.director = DreamDirector::with_length(self.run_seed, RunLength::Short);
                self.motif = None;
                self.run_log.clear();
                self.begin_run();
                log::info!("Today's dream (day {day}, seed {})", self.run_seed);
            }
            StartKind::Continue => {
                self.prologue = None;
                let Some(s) = progress::load(&self.save_path) else {
                    return Ok(());
                };
                self.run_seed = s.run_seed;
                self.run_length = s.length.into();
                self.daily = s.daily;
                self.active_asc = progress::Ascension(s.ascension);
                self.director = s.director();
                self.director.nightmare_every = self.active_asc.nightmare_every();
                self.run = s.upgrades();
                self.loadout_scale = s.loadout_scale.unwrap_or(1.0);
                self.run.penalty_cards = self.active_asc.fewer_cards();
                self.run.no_free_reroll = !self.active_asc.free_rerolls();
                self.refresh_card_effects();
                self.just_continued = true;
                self.director.shard_bonus = self.shard_bonus();
                self.motif = s.motif.map(|m| m.kind());
                self.perks = s.perks.clone();
                self.shards_this_run = s.shards_this_run;
                self.bonus_dust = s.bonus_dust;
                self.peak_difficulty = s.peak_difficulty;
                self.run_log = s.run_log.clone();
                self.seen_kinds = s.seen_kinds.clone();
                let (caught, fell, nightmares, time, skipped) = s.stats;
                self.stats = RunStats {
                    caught,
                    fell,
                    nightmares,
                    twists: Vec::new(),
                    time,
                    skipped,
                    abilities: 0,
                };
                self.run_active = true;
                self.pack = reveal_ui::PackView::default();
                self.choice = None;
                self.offer_upgrade = false;
                // The dream is regenerated; don't log it twice.
                self.run_log.pop();
                log::info!("Continuing a run at depth {}", self.director.depth);
            }
            StartKind::Prologue => {
                self.daily = None;
                self.director = tutorial::director();
                self.motif = None;
                self.run_log.clear();
                self.prologue = Some(tutorial::Prologue::default());
                self.begin_run();
                log::info!("Prologue begins");
            }
        }
        self.title_age = 0.0;
        self.mode = hud::Mode::Playing;
        self.load_dream(ctx)?;
        if let Some(p) = self.prologue {
            self.show_prompt(p.step);
        }
        Ok(())
    }

    /// The eye's tutorial prompt for a prologue step (stays up).
    fn show_prompt(&mut self, step: tutorial::Step) {
        log::info!("Prologue step: {step:?}");
        let text = tutorial::prompt(step);
        self.eye_sticky = !text.is_empty();
        self.eye_line = (!text.is_empty()).then(|| (text.to_string(), 0.0));
    }

    /// Something happened that the prologue may be waiting for.
    fn tutorial(&mut self, e: tutorial::Event) {
        let Some(p) = &mut self.prologue else { return };
        let before = p.step;
        p.on(e);
        let now = p.step;
        if now != before {
            self.show_prompt(now);
        }
    }

    /// Written at the start of every dream while a run is on.
    fn save_run(&self) {
        if !self.run_active || self.director.theme == DreamTheme::Awakening {
            return;
        }
        let s = progress::SavedRun {
            run_seed: self.run_seed,
            length: self.run_length.into(),
            ascension: self.active_asc.0,
            daily: self.daily,
            depth: self.director.depth,
            lucidity: self.director.lucidity,
            shards_to_wake: self.director.shards_to_wake,
            hard_from: self.director.hard_from,
            overdrive: self.director.overdrive,
            has_shard: self.director.has_shard,
            nightmare: self.director.nightmare,
            motif: self.motif.map(progress::PropKindSave::from_kind),
            taken: self.run.taken.clone(),
            anchors_used: self.run.anchors_used,
            rerolls_used: self.run.rerolls_used,
            perks: self.perks.clone(),
            shards_this_run: self.shards_this_run,
            bonus_dust: self.bonus_dust,
            peak_difficulty: self.peak_difficulty,
            run_log: self.run_log.clone(),
            seen_kinds: self.seen_kinds.clone(),
            stats: (
                self.stats.caught,
                self.stats.fell,
                self.stats.nightmares,
                self.stats.time,
                self.stats.skipped,
            ),
            card_dreams: self
                .director
                .cards
                .iter()
                .map(|c| (c.theme, c.blend))
                .collect(),
            burden: self.run.burden,
            loadout_scale: Some(self.loadout_scale),
        };
        if let Err(e) = progress::save(&self.save_path, &s) {
            log::warn!("could not save the run: {e}");
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
            * (1.0 + self.card_fx.speed_pct as f32 / 100.0)
    }

    /// How strange the dream *looks*: CALM MIND takes the edge off.
    fn visual_strangeness(&self) -> f32 {
        self.dream.as_ref().map_or(0.0, |d| d.strangeness) * self.run.calm()
    }

    fn has_perk(&self, p: store::Perk) -> bool {
        self.perks.contains(&p)
    }

    /// Reports to the achievements (and whatever the booklet now proves);
    /// new unlocks are saved, toasted, and sent to Steam.
    fn achieve(&mut self, e: Option<achievements::Event>) {
        let ids = achievements::check(&self.booklet, e);
        let fresh = self.booklet.achievements.grant(ids);
        if fresh.is_empty() {
            return;
        }
        for id in &fresh {
            log::info!("Achievement: {} ({id})", achievements::name(id));
            steam::unlock(id);
        }
        self.achievement_toast =
            Some((achievements::name(fresh[fresh.len() - 1]).to_string(), 0.0));
        self.sfx(Sound::Pick);
        let _ = cards::save(&self.booklet_path, &self.booklet);
    }

    fn persist_booklet(&mut self) -> bool {
        self.achieve(None);
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
            extra: self.run.memory_bonus() + self.card_fx.memory_bonus,
        };
        // Dev: DREAMSCAPE_PACK_PREVIEW=40 repeats this run's dreams to 40.
        let mut log = self.run_log.clone();
        if let Some(n) = crate::dev::var("DREAMSCAPE_PACK_PREVIEW")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
        {
            while !log.is_empty() && log.len() < n {
                log.push(log[log.len() % self.run_log.len()].clone());
            }
        }
        let mut pack = cards::recall(&log, boost);
        let owned: Vec<cards::Card> = self.booklet.cards().cloned().collect();
        cards::shape_first_pack(&mut pack, &owned);
        lottery::apply_pack_pity(&mut self.booklet.lottery, &mut pack);
        self.pack = reveal_ui::PackView {
            pack,
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
        // A padded preview pack (DREAMSCAPE_PACK_PREVIEW) is never pressed.
        if crate::dev::var_os("DREAMSCAPE_PACK_PREVIEW").is_some() {
            log::info!("Booklet: preview pack, not pressed");
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
        let deepest = self.pack.pack.iter().map(|r| r.card.depth).max().unwrap_or(0);
        let report = retention::settle_run(
            &mut self.booklet,
            &self.loadout_numbers,
            &self.pack.pack,
            deepest,
            true,
            self.shards_this_run,
        );
        let week = streaks::week_of(today());
        self.booklet.weekly.record(week, deepest);
        let weekly = streaks::weekly(week);
        let weekly_dust = self.booklet.weekly.claim(&weekly);
        if let Some(d) = weekly_dust {
            self.booklet.stash.earn(d);
        }
        self.after_pack_feelings(&report, weekly_dust);
        self.last_report = Some(report);
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

    /// Toasts and the eye's next words after a pack is pressed.
    fn after_pack_feelings(&mut self, report: &retention::RunReport, weekly_dust: Option<u32>) {
        let remembered = || self.pack.pack.iter().filter(|r| r.remembered);
        let foil = remembered().any(|r| worth::is_foil(&r.card));
        let prophetic = remembered().any(|r| r.card.rarity >= cards::Rarity::Prophetic);
        let near_miss = self.pack.pack.iter().any(reveal_ui::near_miss);
        self.pending_eye = if foil {
            Some(eye::Moment::PackFoil)
        } else if prophetic {
            Some(eye::Moment::PackProphetic)
        } else if !report.level_ups.is_empty() {
            Some(eye::Moment::MasteryLevelUp)
        } else if near_miss {
            Some(eye::Moment::PackFaded)
        } else {
            self.pending_eye
        };
        let mut notes: Vec<String> = Vec::new();
        for &(n, level) in &report.level_ups {
            if let Some(c) = self.booklet.card(n) {
                notes.push(format!("{} GREW TO LEVEL {level}", c.name));
            }
        }
        if let Some(d) = weekly_dust {
            notes.push(format!("WEEKLY CHALLENGE DONE  +{d} DUST"));
        }
        if let Some(first) = notes.into_iter().next() {
            self.achievement_toast = Some((first, 0.0));
        }
    }

    fn lottery_view(&self) -> lottery_ui::LotteryView {
        let featured = lottery::featured_theme(today());
        let state = &self.booklet.lottery;
        let odds = lottery::odds(state, Some(featured));
        let themes = lottery::pullable_themes();
        lottery_ui::LotteryView {
            odds: lottery::display_odds(&odds),
            since_lucid: state.since_lucid,
            hard_lucid: lottery::HARD_PITY_LUCID,
            since_prophetic: state.since_prophetic,
            hard_prophetic: lottery::HARD_PITY_PROPHETIC,
            sparks: state.sparks,
            spark_goal: lottery::SPARKS_FOR_PICK,
            featured: Some(featured),
            featured_odds: lottery::FEATURED_WEIGHT as f32 / (themes.len() as f32 - 1.0 + lottery::FEATURED_WEIGHT as f32),
            pick: themes.get(self.lottery_pick % themes.len().max(1)).copied(),
            cost: lottery::PULL_COST,
            total_pulls: state.total_pulls,
            outcome: self.lottery_outcome.clone(),
            age: self.lottery_age,
            status: self.lottery_status.clone(),
        }
    }

    /// A fresh seed for each pull (the lottery must not be replayable).
    fn lottery_seed(&self) -> u64 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        nanos ^ (self.booklet.lottery.total_pulls as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
    }

    fn show_pull(&mut self, p: retention::Pulled) {
        let big = p.foil || p.pull.pity || p.card.rarity >= cards::Rarity::Prophetic;
        self.pending_eye = if p.pull.pity {
            Some(eye::Moment::LotteryPity)
        } else if p.foil {
            Some(eye::Moment::PackFoil)
        } else if big {
            Some(eye::Moment::PackProphetic)
        } else {
            self.pending_eye.or(Some(eye::Moment::LotteryPull))
        };
        self.sfx(if big { Sound::WakeDoor } else { Sound::Pick });
        self.lottery_outcome = Some(lottery_ui::Outcome {
            foil: p.foil,
            pity: p.pull.pity,
            near_miss: p.pull.near_miss,
            recurring: p.recurring,
            card: p.card,
        });
        self.lottery_age = 0.0;
        self.lottery_status = None;
    }

    /// Dust for a pull: the card goes straight into the booklet.
    fn lottery_pull(&mut self) {
        let before = self.booklet.clone();
        let seed = self.lottery_seed();
        match retention::pull_card(&mut self.booklet, seed, today()) {
            Err(retention::PullError::TooPoor { need }) => {
                self.lottery_status = Some(format!("you need {need} more dust"));
                self.sfx(Sound::Denied);
            }
            Ok(p) => {
                if self.persist_booklet() {
                    log::info!(
                        "Lottery: {:?}{} -> {} (dust now {})",
                        p.card.rarity,
                        if p.foil { " FOIL" } else { "" },
                        p.card.name,
                        self.booklet.stash.dust
                    );
                    self.show_pull(p);
                } else {
                    self.booklet = before;
                    self.lottery_status = Some("the deck couldn't write it down (see log)".into());
                }
            }
        }
    }

    /// Sparks for a card of your choosing.
    fn lottery_spark_pick(&mut self) {
        let themes = lottery::pullable_themes();
        let Some(&theme) = themes.get(self.lottery_pick % themes.len().max(1)) else {
            return;
        };
        let before = self.booklet.clone();
        let seed = self.lottery_seed();
        match retention::spark_pick(&mut self.booklet, theme, seed) {
            None => {
                let need = lottery::SPARKS_FOR_PICK.saturating_sub(self.booklet.lottery.sparks);
                self.lottery_status = Some(format!("{need} more sparks to pick a dream"));
                self.sfx(Sound::Denied);
            }
            Some(p) => {
                if self.persist_booklet() {
                    self.show_pull(p);
                } else {
                    self.booklet = before;
                    self.lottery_status = Some("the deck couldn't write it down (see log)".into());
                }
            }
        }
    }

    fn open_store(&mut self) {
        self.booklet_return = self.mode;
        self.store_status = None;
        self.store_tab = 0;
        self.lottery_status = None;
        self.mode = hud::Mode::Store;
    }

    fn store_select(&mut self) {
        let item = shop_ui::cursor_item(self.shop_shelf, self.shop_col);
        let before = self.booklet.stash.clone();
        let outcome = self.booklet.stash.select(item);
        if outcome == store::Outcome::Bought {
            self.booklet.achievements.purchases += 1;
        }
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

    /// START: choose the run's cards first, unless there are none yet.
    fn open_loadout(&mut self) {
        if self.booklet.card_count() == 0 {
            return self.start_from_title();
        }
        // Cards no longer in the booklet drop out silently.
        let b = &self.booklet;
        let kept: Vec<u32> = b
            .lore
            .loadout
            .iter()
            .copied()
            .filter(|&n| b.card(n).is_some())
            .take(b.lore.slots())
            .collect();
        self.booklet.lore.loadout = kept;
        self.loadout_sel = self.loadout_sel.min(self.booklet.card_count() - 1);
        self.mode = hud::Mode::Loadout;
    }

    /// The moth's merge screen; `back` is where escape returns to.
    fn open_merge(&mut self, back: hud::Mode) {
        self.merge_return = back;
        self.merge_first = None;
        self.merge_status = None;
        self.merge_sel = self
            .merge_sel
            .min(self.booklet.card_count().saturating_sub(1));
        self.mode = hud::Mode::Merge;
    }

    fn merge_key(&mut self, key: Keycode) {
        let n = self.booklet.card_count();
        let (dx, dy) = match key {
            Keycode::A | Keycode::Left => (-1, 0),
            Keycode::D | Keycode::Right => (1, 0),
            Keycode::W | Keycode::Up => (0, -1),
            Keycode::S | Keycode::Down => (0, 1),
            _ => (0, 0),
        };
        if (dx, dy) != (0, 0) {
            self.merge_sel = loadout_ui::step(self.merge_sel, n, dx, dy);
            return;
        }
        match key {
            Keycode::Return | Keycode::Space => {
                let Some(number) = self.booklet.cards().nth(self.merge_sel).map(|c| c.number)
                else {
                    return;
                };
                match self.merge_first {
                    None => {
                        self.merge_first = Some(number);
                        self.merge_status = None;
                        self.sfx(Sound::Pick);
                    }
                    Some(a) if a == number => self.merge_first = None,
                    Some(a) => match self.booklet.merge(a, number) {
                        Some(c) => {
                            log::info!("Merged #{a} and #{number}: {} ({:?})", c.name, c.rarity);
                            self.merge_status = Some(format!("{}: {}", c.rarity.label(), c.name));
                            self.merge_first = None;
                            self.merge_sel = self
                                .booklet
                                .cards()
                                .position(|x| x.number == a)
                                .unwrap_or(0);
                            self.flash.trigger([1.0, 0.8, 1.0], 0.5);
                            self.sfx(Sound::Pick);
                            self.persist_booklet();
                        }
                        None => self.sfx(Sound::Denied),
                    },
                }
            }
            Keycode::Escape if self.merge_first.is_some() => self.merge_first = None,
            Keycode::Escape => self.mode = self.merge_return,
            _ => {}
        }
    }

    fn loadout_key(&mut self, key: Keycode) {
        let n = self.booklet.card_count();
        let (dx, dy) = match key {
            Keycode::A | Keycode::Left => (-1, 0),
            Keycode::D | Keycode::Right => (1, 0),
            Keycode::W | Keycode::Up => (0, -1),
            Keycode::S | Keycode::Down => (0, 1),
            _ => (0, 0),
        };
        if (dx, dy) != (0, 0) {
            self.loadout_sel = loadout_ui::step(self.loadout_sel, n, dx, dy);
            return;
        }
        match key {
            Keycode::Return => {
                let Some(number) = self.booklet.cards().nth(self.loadout_sel).map(|c| c.number)
                else {
                    return;
                };
                let slots = self.booklet.lore.slots();
                if loadout_ui::toggle(&mut self.booklet.lore.loadout, number, slots) {
                    self.persist_booklet();
                }
            }
            Keycode::C if !self.booklet.lore.dreamers.is_empty() => {
                self.booklet.lore.companion = self.booklet.lore.next_companion();
                self.persist_booklet();
            }
            Keycode::Space => self.start_from_title(),
            Keycode::Escape => self.mode = hud::Mode::Title,
            _ => {}
        }
    }

    /// Enter on the title: the dream already loaded behind it becomes the run.
    fn start_from_title(&mut self) {
        self.prologue = None;
        // The lobby behind the title is the same whatever the length.
        self.daily = None;
        self.director = DreamDirector::with_length(self.run_seed, self.run_length);
        self.begin_run();
        self.title_age = 0.0;
        self.mode = hud::Mode::Playing;
        log::info!("Fell asleep (perks: {:?})", self.perks);
    }

    fn restart(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        self.prologue = None;
        self.run_seed = gameplay::next_run_seed(self.run_seed);
        crash::SEED.store(self.run_seed, std::sync::atomic::Ordering::Relaxed);
        log::info!(
            "Dream run seed = {} (replay with DREAMSCAPE_SEED={})",
            self.run_seed,
            self.run_seed
        );
        self.daily = None;
        self.director = DreamDirector::with_length(self.run_seed, self.run_length);
        self.motif = None;
        self.run_log.clear();
        self.journal_status = None;
        self.mode = hud::Mode::Playing;
        self.transition.cancel();
        self.begin_run();
        self.load_dream(ctx)?;
        Ok(())
    }

    /// The eye says something. `once`: only the first time ever (saved).
    fn think(&mut self, m: eye::Moment, once: bool) {
        if once && !self.booklet.lore.seen_once(&format!("{m:?}")) {
            return;
        }
        if self.eye_sticky {
            return;
        }
        // Don't talk over itself: a newer line waits unless the last is nearly done.
        if self.eye_line.as_ref().is_some_and(|(_, age)| *age < 2.0) {
            return;
        }
        let you = lore::dreamer(self.booklet.lore.save_seed, 0);
        let seed = self.director.dream_seed() ^ m as u64;
        let text = eye::line(m, &you, seed);
        log::info!("Eye: {text}");
        self.eye_line = Some((text, 0.0));
    }

    /// Reads this run's loadout cards: their quirks and mastery become the
    /// run's bonuses (speed, calm, grace, dust, memory, rerolls).
    fn refresh_card_effects(&mut self) {
        let cards = if self.prologue.is_some() {
            Vec::new()
        } else {
            self.booklet.loadout_cards()
        };
        self.card_fx = retention::card_effects(&self.booklet, &cards);
        self.loadout_numbers = cards.iter().map(|c| c.number).collect();
        self.run.bonus_rerolls = self.card_fx.free_rerolls;
        if !self.card_fx.is_zero() {
            log::info!("Card bonuses: {:?}", self.card_fx);
        }
    }

    /// Opening the game: today's visit counts toward the streak.
    fn on_launch(&mut self) {
        let Some(v) = retention::visit_today(&mut self.booklet, today()) else {
            return;
        };
        log::info!("Visit: {v:?}");
        let (moment, text) = match v.event {
            streaks::StreakEvent::Rested { .. } => (
                eye::Moment::StreakRested,
                format!("A REST SAVED YOUR STREAK  DAY {}  +{} DUST", v.streak, v.dust),
            ),
            streaks::StreakEvent::Reset { .. } => (
                eye::Moment::StreakReset,
                format!("A NEW STREAK BEGINS  +{} DUST", v.dust),
            ),
            streaks::StreakEvent::Started => (
                eye::Moment::Returning,
                format!("WELCOME BACK  +{} DUST", v.dust),
            ),
            _ => (
                eye::Moment::StreakContinued,
                match v.milestone {
                    Some(name) => format!("{name}  DAY {}  +{} DUST", v.streak, v.dust),
                    None => format!("DREAM STREAK  DAY {}  +{} DUST", v.streak, v.dust),
                },
            ),
        };
        self.pending_eye = Some(moment);
        self.achievement_toast = Some((text, 0.0));
        self.persist_booklet();
    }

    /// The first line of a run: whatever is pending, else the opening line,
    /// warmer the longer the two of you have known each other.
    fn opening_line(&mut self) -> String {
        let runs = self.booklet.runs.len() as u32;
        let milestone = matches!(runs, 10 | 25 | 50 | 100) && self.pending_eye.is_none();
        let moment = if milestone {
            eye::Moment::RunMilestone
        } else {
            self.pending_eye.take().unwrap_or(eye::Moment::FirstDream)
        };
        let you = lore::dreamer(self.booklet.lore.save_seed, 0);
        let seed = self.director.dream_seed() ^ (moment as u64);
        let stage = eye::relationship(self.booklet.runs.len() as u32);
        eye::line_for_stage(moment, &you, seed, stage)
    }

    /// Consume armed perks for the run that is starting.
    fn begin_run(&mut self) {
        self.eye_sticky = false;
        self.entrance.reset();
        self.objective_clock = 0.0;
        self.shards_this_run = 0;

        // The eye says FirstDream at the start of every run, during the entrance animation
        let text = self.opening_line();
        log::info!("Eye: {text}");
        self.eye_line = Some((text, -eye_motion::SPEECH_DELAY));
        self.booklet.clean.on_run_start();

        // Abilities come from the loadout's cards (none in the prologue),
        // and each card's dream is planned into the run.
        let cards = if self.prologue.is_some() {
            Vec::new()
        } else {
            self.booklet.loadout_cards()
        };
        if !cards.is_empty() {
            log::info!(
                "Loadout: {:?}",
                cards.iter().map(|c| c.name.as_str()).collect::<Vec<_>>()
            );
        }
        self.run = RunUpgrades::from_loadout(&cards);
        if let Some(id) = self
            .booklet
            .lore
            .companion()
            .filter(|_| self.prologue.is_none())
        {
            let d = lore::dreamer(self.booklet.lore.save_seed, id);
            log::info!("Companion: {} ({})", d.name, d.weight.label());
            self.run = std::mem::take(&mut self.run).with_companion(d.weight);
        }
        self.refresh_card_effects();
        // In co-op the host's dreams are everyone's: no per-kit pressure.
        self.loadout_scale = if self.coop.is_some() {
            1.0
        } else {
            gameplay::loadout_difficulty(self.run.power())
        };
        log::info!(
            "Starting kit power {:.1}: difficulty x{:.2}",
            self.run.power(),
            self.loadout_scale
        );
        self.ability_tints = cards
            .iter()
            .map(|c| {
                let accents = c.fused.unwrap_or(c.theme).spec().accents;
                (
                    powers::power(c.theme).active,
                    powers::power(c.theme).tint,
                    accents[accents.len().min(2) - 1],
                )
            })
            .collect();
        let asc = progress::Ascension(
            if self.daily.is_some()
                || self.autopilot
                || self.prologue.is_some()
                || self.coop.is_some()
            {
                0
            } else {
                self.ascension
            },
        );
        self.active_asc = asc;
        self.director.shards_to_wake += asc.extra_shards();
        self.director.nightmare_every = asc.nightmare_every();
        // Card dreams are personal: a shared dream has none.
        if self.coop.is_none() {
            self.director
                .plan_cards(&cards.iter().map(|c| (c.theme, c.fused)).collect::<Vec<_>>());
        }
        if asc.always_hard() {
            self.director.hard_from.get_or_insert(0);
        }
        self.run.penalty_cards = asc.fewer_cards();
        self.run.no_free_reroll = !asc.free_rerolls();
        self.director.shard_bonus = self.shard_bonus();
        // Dev: DREAMSCAPE_SHARDS_NEEDED=40 previews a deep run's shard counter.
        if let Some(n) = crate::dev::var("DREAMSCAPE_SHARDS_NEEDED")
            .ok()
            .and_then(|s| s.parse().ok())
        {
            self.director.shards_to_wake = n;
        }
        // Dev: DREAMSCAPE_GIVE=Decoy,Rewind starts the run holding those.
        if let Ok(list) = crate::dev::var("DREAMSCAPE_GIVE") {
            for name in list.split(',') {
                if let Some(&a) = upgrades::ALL_ABILITIES
                    .iter()
                    .find(|a| format!("{a:?}").eq_ignore_ascii_case(name.trim()))
                {
                    self.run.take(Upgrade::Learn(a));
                }
            }
        }
        self.run_active = !self.autopilot;
        progress::clear(&self.save_path);
        if asc.0 > 0 {
            log::info!("Ascension {}: {}", asc.0, progress::ascension_rule(asc.0));
        }
        self.stats = RunStats::default();
        self.seen_kinds.clear();
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
        self.perks = if self.autopilot || self.prologue.is_some() {
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
        // GLIMMER quirks: a card that starts you with a spark of its own.
        for _ in 0..self.card_fx.start_shards {
            self.director.collect_shard();
            self.director.shard_this_dream = false;
            self.shards_this_run += 1;
        }
    }

    /// Awake: roll the pack (and, for autopilot E2E runs, autosave).
    fn complete_run(&mut self) {
        log::info!("Game complete! You woke up.");
        if self.prologue.is_some() {
            self.tutorial(tutorial::Event::Woke);
            self.prologue = None;
            self.eye_sticky = false;
            self.eye_line = None;
            self.booklet.lore.prologue_done = true;
            self.persist_booklet();
            log::info!("Prologue complete");
            // Told at the start of the next dream.
            self.eye_queue = [
                "You've been asleep a long time.",
                "Every dream down there belongs to someone. Some of them are you.",
                "Find the notes. Remember who you are. Something at the bottom is calling.",
            ]
            .map(String::from)
            .to_vec();
        }
        self.title_age = 0.0;
        if self.run_active {
            self.achieve(Some(achievements::Event::Woke {
                depth: self.director.depth,
                caught: self.stats.caught,
                fell: self.stats.fell,
                abilities_used: self.stats.abilities,
                daily: self.daily.is_some(),
                long_at: (self.run_length == RunLength::Long && self.daily.is_none())
                    .then_some(self.active_asc.0),
            }));
            let depth = self.director.depth;
            if let Some(day) = self.daily {
                self.booklet.codex.record_daily(day, depth);
            }
            if self.run_length == RunLength::Long
                && self.daily.is_none()
                && self.booklet.codex.beat_long_run(self.active_asc.0)
            {
                log::info!(
                    "Ascension {} unlocked",
                    self.booklet.codex.ascension_unlocked
                );
            }
            self.persist_booklet();
            self.run_active = false;
            progress::clear(&self.save_path);
        }
        if self.autopilot && crate::dev::var("DREAMSCAPE_AUTOSAVE").is_ok() {
            self.open_pack();
            self.pack.age = f32::MAX;
            self.save_journal();
            match crate::dev::var("DREAMSCAPE_AUTOSAVE").as_deref() {
                Ok("booklet") => {
                    self.open_booklet();
                    self.export_requested = crate::dev::var("DREAMSCAPE_CARDS").is_ok();
                }
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
        self.run.dust()
            * gameplay::difficulty_reward(self.peak_difficulty)
            * (1.0 + self.card_fx.dust_bonus_pct as f32 / 100.0)
            * streaks::multiplier(self.booklet.clean.current)
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
        if self.active_asc.0 > 0 {
            rows.push(("ascension".into(), self.active_asc.0.to_string()));
        }
        if self.daily.is_some() {
            rows.push((
                "today's dream".into(),
                "the same one everyone dreamt".into(),
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
            self.sfx(Sound::MeltStart);
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
                if self.coop_client() {
                    // The host already rolled it.
                    if let Some(p) = self.coop.as_ref().and_then(|c| c.plan.clone()) {
                        self.apply_plan(&p);
                    }
                } else {
                    self.director.descend();
                    self.coop_broadcast_plan();
                }
                self.offer_upgrade = self.prologue.is_none();
                self.load_dream(ctx)
            }
            transition::Pending::Wake if self.coop_active() => {
                if self.coop_client() {
                    if let Some(p) = self.coop.as_ref().and_then(|c| c.plan.clone()) {
                        self.apply_plan(&p);
                    }
                } else {
                    self.director.wake();
                    self.coop_broadcast_plan();
                }
                self.load_dream(ctx)
            }
            transition::Pending::Wake => {
                self.director.wake();
                if std::mem::take(&mut self.to_bottom) {
                    // The bottom: waking, blended with the first dream you kept.
                    self.director.blend = Some(DreamTheme::Garden);
                    self.load_dream(ctx)?;
                    self.eye_line = None;
                    self.eye_queue.clear();
                    self.ending_age = 0.0;
                    self.mode = hud::Mode::Ending;
                    log::info!("The ending begins");
                    return Ok(());
                }
                self.load_dream(ctx)
            }
            transition::Pending::Finish => {
                self.complete_run();
                if self.coop.is_some() {
                    log::info!("Co-op: the shared dream ends");
                    self.coop = None;
                }
                Ok(())
            }
        }
    }

    /// A synthesized sound effect (built once, at start-up).
    fn sfx(&self, sound: Sound) {
        if let (Some(audio), Some(samples)) = (&self.audio, self.sounds.get(&sound)) {
            audio.play_samples(samples, sounds::RATE);
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
        self.shard_entities.extend([entity, beacon]);
        Ok(())
    }

    /// Before a dream is built: does it hold a note, and what does it ask?
    fn plan_goals(&mut self) -> dream::Goals {
        self.pending_note = None;
        self.objective_kind = None;
        if self.prologue.is_some() {
            // The Garden always holds the shard; the note is placed by hand.
            if self.director.theme == DreamTheme::Garden {
                self.director.has_shard = true;
            }
            if self.director.has_shard && !self.director.lucid() {
                self.objective_kind = Some(objective::Kind::Shard);
            }
            return dream::Goals::default();
        }
        if self.director.nightmare {
            return dream::Goals::default();
        }
        // A shared dream has no story: just the shard.
        if self.coop.is_some() {
            if self.director.has_shard && !self.director.lucid() {
                self.objective_kind = Some(objective::Kind::Shard);
            }
            return dream::Goals::default();
        }
        let seed = self.director.dream_seed();
        let lore = &self.booklet.lore;
        let owner = lore::owner(lore.save_seed, seed);
        let chance = if owner == 0 { 0.45 } else { 0.30 };
        let roll = {
            use rand::{rngs::StdRng, Rng, SeedableRng};
            StdRng::seed_from_u64(seed ^ 0x0A7E_5EED).gen::<f32>()
        };
        self.pending_note = lore
            .next_note(owner, self.director.depth)
            .filter(|_| roll < chance)
            .map(|k| (owner, k));
        if self.director.has_shard && !self.director.lucid() {
            let rolled = objective::roll(seed, self.director.depth, self.pending_note.is_some());
            // Dev switch: DREAMSCAPE_OBJECTIVE=Chase forces one (FindMemory needs a note).
            let forced = crate::dev::var("DREAMSCAPE_OBJECTIVE").ok().and_then(|n| {
                [
                    objective::Kind::Shard,
                    objective::Kind::Fragments,
                    objective::Kind::Chase,
                    objective::Kind::HoldOn,
                    objective::Kind::FindMemory,
                ]
                .into_iter()
                .find(|k| format!("{k:?}") == n)
            });
            if forced == Some(objective::Kind::FindMemory) && self.pending_note.is_none() {
                let o = lore::owner(lore.save_seed, seed);
                self.pending_note = Some((o, lore.next_note(o, 99).unwrap_or(0)));
            }
            self.objective_kind = Some(forced.unwrap_or(rolled));
        }
        dream::Goals {
            note: self.pending_note.is_some()
                && self.objective_kind != Some(objective::Kind::FindMemory),
            fragments: self.objective_kind == Some(objective::Kind::Fragments),
        }
    }

    fn spawn_note(&mut self, at: Vec3) -> anyhow::Result<()> {
        let tex = self.shard_tex.clone().context("textures not uploaded")?;
        let size = Vec3::new(0.55, 0.06, 0.4);
        let y = at.y + 0.9;
        self.world.spawn((
            Transform {
                position: Vec3::new(at.x, y, at.z),
                rotation: Quat::IDENTITY,
                scale: size,
            },
            MeshRenderer {
                mesh: self.mesh(Shape::Cube)?,
                texture: Some(tex),
            },
            Spin(0.6),
            Bob {
                base: y,
                amp: 0.12,
                speed: 1.4,
                phase: 0.0,
            },
            Lit,
            Hero,
            NoteMarker,
            Collider {
                shape: ColliderShape::Aabb {
                    half_extents: Vec3::splat(0.6),
                },
                is_trigger: true,
            },
        ));
        Ok(())
    }

    /// Where the compass points: the nearest shard piece, or the fleeing memory.
    fn objective_target(&self) -> Option<Vec3> {
        if matches!(
            self.objective,
            Some(objective::Active::HoldOn(_)) | Some(objective::Active::FindMemory)
        ) {
            return None;
        }
        self.shard_entities
            .iter()
            .step_by(2)
            .filter_map(|&e| self.world.get::<&Transform>(e).ok().map(|t| t.position))
            .min_by(|a, b| {
                a.distance_squared(self.player_position)
                    .total_cmp(&b.distance_squared(self.player_position))
            })
    }

    /// The dream's objective is done: the shard is yours.
    fn complete_objective(&mut self, ctx: &mut Context) {
        // The autopilot may have left its route (chasing, holding on): pick it
        // up again from wherever it stands.
        if let Some(d) = &self.dream {
            let p = self.player_position;
            if let Some((i, _)) = d.lucid_route.iter().enumerate().min_by(|a, b| {
                a.1.pos
                    .distance_squared(p)
                    .total_cmp(&b.1.pos.distance_squared(p))
            }) {
                self.route_index = self.route_index.max(i);
            }
        }
        self.despawn_shard_slot();
        self.objective = None;
        self.flash.trigger([0.3, 1.0, 1.0], 0.6);
        self.sfx(Sound::Shard);
        self.director.collect_shard();
        self.shards_this_run += 1;
        if self.run_active {
            self.achieve(Some(achievements::Event::ShardTaken));
        }
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
        self.tutorial(tutorial::Event::ShardTaken);
        self.think(eye::Moment::ShardTaken, false);
        if self.director.lucid() {
            self.eye_line = None;
            self.think(eye::Moment::Lucid, false);
        }
    }

    /// Picked a note up: keep it (saved at once), and read it.
    fn read_note(&mut self, id: u32, k: u32) {
        let before = self.booklet.lore.slots();
        self.booklet.lore.found(id, k);
        let lore = &self.booklet.lore;
        let d = lore::dreamer(lore.save_seed, id);
        let n = lore::note(&d, k, self.director.theme);
        log::info!("Note found: {} ({})", n.title, n.object);
        let complete = id != 0 && lore.dreamer_complete(id) && !lore.dreamers.contains(&id);
        if complete {
            self.booklet.lore.dreamers.push(id);
            log::info!("Dreamer collected: {}", d.name);
        }
        let slot_up = self.booklet.lore.slots() > before;
        self.note_view = Some(n);
        self.sfx(Sound::Shard);
        if !self.autopilot {
            self.mode = hud::Mode::Note;
            self.input = PlayerInputState::default();
        }
        self.persist_booklet(); // notes are kept even if the run is abandoned
        self.eye_line = None;
        if complete {
            self.think(eye::Moment::DreamerComplete, false);
        } else if slot_up {
            self.think(eye::Moment::SlotUnlocked, false);
        } else {
            self.think(eye::Moment::NoteFound, true);
        }
        self.tutorial(tutorial::Event::NoteRead);
    }

    /// Per frame: the fleeing memory and the hold-on timer.
    fn update_objective(&mut self, ctx: &mut Context, dt: f32) {
        match &mut self.objective {
            Some(objective::Active::Chase(c)) => {
                let Some(dream) = &self.dream else { return };
                let route: Vec<Vec3> = dream.lucid_route.iter().map(|w| w.pos).collect();
                let before = c.pos;
                let speed = if self.autopilot {
                    gameplay::MOVE_SPEED
                } else {
                    self.run.speed() * gameplay::MOVE_SPEED
                };
                c.step(&route, self.player_position, speed, dt);
                let moved = c.pos - before;
                let caught = c.caught(self.player_position);
                for &e in &self.shard_entities {
                    if let Ok(mut t) = self.world.get::<&mut Transform>(e) {
                        t.position += moved;
                    }
                }
                if caught && !self.shard_entities.is_empty() {
                    log::info!("Caught the memory");
                    self.complete_objective(ctx);
                }
            }
            Some(objective::Active::HoldOn(h)) => {
                let call = h.tick(dt);
                let done = h.done();
                if call {
                    for p in &mut self.enemies {
                        p.ai.alert();
                    }
                }
                if done {
                    log::info!("Held on");
                    self.complete_objective(ctx);
                }
            }
            _ => {}
        }
    }

    fn memories_view(&self) -> memories_ui::MemoriesView {
        let lore = &self.booklet.lore;
        let rows: Vec<memories_ui::DreamerRow> = (0..lore::DREAMERS)
            .map(|id| {
                let d = lore::dreamer(lore.save_seed, id);
                let found = lore.count(id);
                memories_ui::DreamerRow {
                    name: if id == 0 {
                        "YOU".into()
                    } else if found > 0 {
                        d.name.to_uppercase()
                    } else {
                        "???".into()
                    },
                    found,
                    total: lore::notes_for(id),
                    weight: (id != 0 && lore.dreamer_complete(id))
                        .then(|| format!("carrying {}", d.weight.label())),
                }
            })
            .collect();
        let sel = self.memories_sel.min(rows.len() - 1) as u32;
        let d = lore::dreamer(lore.save_seed, sel);
        let notes = (0..lore::notes_for(sel))
            .map(|k| {
                lore.has(sel, k)
                    .then(|| lore::note(&d, k, DreamTheme::Lobby).body)
            })
            .collect();
        memories_ui::MemoriesView {
            rows,
            selected: sel as usize,
            notes,
        }
    }

    fn despawn_shard_slot(&mut self) {
        for e in self.shard_entities.drain(..) {
            let _ = self.world.despawn(e);
        }
    }

    fn load_dream(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        let theme = self.director.theme;
        let spec = theme.spec();
        self.twists = if self.director.nightmare || self.prologue.is_some() {
            Twists::default()
        } else {
            dream::roll_twists(
                theme,
                self.director.dream_seed(),
                self.director.depth,
                self.director.hardness().is_some(),
                self.active_asc.twist_bonus(),
            )
        };
        if self.twists.has(Variant::Gilded) {
            self.director.has_shard = true;
        }
        self.peak_difficulty = self.peak_difficulty.max(self.difficulty());
        self.dream_age = 0.0;
        // The dream just left counts toward the clean streak (a run loaded
        // from a save has no finished dream to count).
        if !self.run_log.is_empty() && !std::mem::take(&mut self.just_continued) {
            if let streaks::CleanEvent::Clean {
                label: Some(label), ..
            } = self.booklet.clean.on_dream_end(self.caught_this_dream > 0)
            {
                self.achievement_toast = Some((format!("{label}  UNSEEN {}", self.booklet.clean.current), 0.0));
                self.think(eye::Moment::CleanStreak, false);
            }
        }
        self.just_continued = false;
        self.caught_this_dream = 0;
        self.stare = 0.0;
        if self.run_active {
            self.achieve(Some(achievements::Event::Depth(self.director.depth)));
        }
        self.air_jumps_used = 0;
        let goals = self.plan_goals();
        let mut dream = if self.director.nightmare {
            dream::generate_nightmare(theme, self.director.dream_seed(), self.director.depth)
        } else {
            dream::generate_with(
                theme,
                self.director.dream_seed(),
                self.director.depth,
                self.motif,
                self.director.has_shard,
                self.pressure(),
                goals,
            )
        };
        if let Some(b) = self.director.blend {
            dream::blend_surfaces(&mut dream.surfaces, b);
            log::info!("Fused dream: {theme:?} with {b:?}");
        }
        // No shard cell (a pure corridor): nothing to ask for.
        let kind = self.objective_kind.filter(|_| dream.shard.is_some());
        self.objective = kind.map(|k| match k {
            objective::Kind::Shard => objective::Active::Shard,
            objective::Kind::Fragments if dream.fragments.len() > 1 => {
                objective::Active::Fragments(
                    objective::Fragments::new(dream.fragments.len() as u32),
                )
            }
            objective::Kind::Fragments => objective::Active::Shard,
            objective::Kind::Chase => {
                let route: Vec<Vec3> = dream.lucid_route.iter().map(|w| w.pos).collect();
                objective::Active::Chase(objective::Chase::new(&route, route.len() / 3))
            }
            objective::Kind::HoldOn => {
                objective::Active::HoldOn(objective::HoldOn::new(self.director.depth))
            }
            objective::Kind::FindMemory => objective::Active::FindMemory,
        });
        if let Some(o) = &self.objective {
            log::info!("Objective: {}", o.label());
        }
        self.hunter = None;
        self.sealed_portal = None;
        self.sigils_left = dream.sigils.len();
        self.sigils_total = dream.sigils.len();
        self.sigils_taken.clear();
        self.boss_fx = None;
        if !self.twists.0.is_empty() {
            log::info!("Dream twist: {}", self.twists.label());
        }
        let wake_door = self.director.lucid() && dream.shard.is_some();
        self.yaw = dream
            .route
            .get(1)
            .and_then(|w| fpv::yaw_toward(w.pos - dream.spawn))
            .unwrap_or(0.0);
        self.pitch = 0.0;
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
            blend: self.director.blend,
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
        self.specials.clear();
        self.crumbles.clear();
        self.trail.clear();
        self.mimic_awake_at = specials::MIMIC_DELAY;
        self.jester_cooldown = 0.0;
        self.fog = dream.fog_pockets.clone();
        self.veins = dream.veins.clone();
        self.gates.clear();
        self.shifters.clear();
        self.beat_tiles.clear();
        self.bpm = spec.mood.bpm;
        self.loops = 0;
        self.echoes.clear();
        self.bursts.clear();
        self.burst_pending = None;
        self.decoy = None; // the old dream's world (and its marker) is gone
        self.rewind_trail.clear();
        self.echo_timer = 0.0;
        self.watchers.clear();
        self.dissolve = None;
        self.player = None;
        self.route_index = 0;
        self.apply_atmosphere(theme, &dream.atmosphere)?;
        self.start_music(theme, dream.strangeness, self.director.dream_seed());

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
        let mut dissolve_tiles: Vec<(Vec3, Entity, Vec3)> = Vec::new();
        for block in &dream.blocks {
            if block.kind == BlockKind::Portal && !dream.sigils.is_empty() {
                // Sealed until every sigil is gathered.
                self.sealed_portal = Some((block.clone(), portal_tex.clone()));
                continue;
            }
            let shifter = (block.kind == BlockKind::Floor)
                .then(|| {
                    dream.shifters.iter().find(|(c, _)| {
                        (c.x - block.pos.x).abs() < 1e-3 && (c.z - block.pos.z).abs() < 1e-3
                    })
                })
                .flatten();
            if let Some(&(_, phase)) = shifter {
                let e = self.spawn_tile(block, floor_tex.clone())?;
                self.shifters.push((e, block.pos, phase));
                continue;
            }
            // White Dissolve: each walkable cell is its own tile so it can fade.
            let dissolving = block.kind == BlockKind::Floor
                && dream
                    .dissolve
                    .iter()
                    .any(|c| (c.x - block.pos.x).abs() < 1e-3 && (c.z - block.pos.z).abs() < 1e-3);
            if dissolving {
                let e = self.spawn_tile(block, floor_tex.clone())?;
                dissolve_tiles.push((block.pos, e, block.size));
                continue;
            }
            let crumbly = block.kind == BlockKind::Floor
                && dream
                    .crumbles
                    .iter()
                    .any(|c| (c.x - block.pos.x).abs() < 1e-3 && (c.z - block.pos.z).abs() < 1e-3);
            if crumbly {
                // Crumbling tiles wear the prop pattern: the tell.
                let mut tile = CrumbleTile {
                    entity: None,
                    block: block.clone(),
                    texture: prop_tex.clone(),
                    state: Crumble::Solid,
                };
                tile.entity = Some(self.spawn_tile(block, prop_tex.clone())?);
                self.crumbles.push(tile);
                continue;
            }
            let (texture, uv) = match block.kind {
                BlockKind::Floor => (floor_tex.clone(), per_cell),
                BlockKind::Wall => (wall_tex.clone(), per_cell),
                BlockKind::Prop | BlockKind::Decor | BlockKind::Sky => (prop_tex.clone(), 0.5),
                BlockKind::Portal => (portal_tex.clone(), 0.5),
                BlockKind::Trim => (wall_tex.clone(), per_cell),
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
                // Dressing: static and never solid.
                BlockKind::Trim => {}
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

        log::info!("Dream spawned {} entities", self.world.len());
        // world.clear() above already removed the old ones.
        self.shard_entities.clear();
        let find_memory = matches!(self.objective, Some(objective::Active::FindMemory));
        match (&self.objective, dream.shard) {
            (Some(objective::Active::Fragments(_)), _) => {
                for &at in &dream.fragments {
                    self.spawn_shard_slot(at, false)?;
                }
            }
            (Some(objective::Active::Chase(c)), _) => {
                let at = c.pos;
                self.spawn_shard_slot(at, false)?;
            }
            (Some(objective::Active::HoldOn(_)), _) => {}
            // No beacon: the memory is the thing to find.
            (Some(objective::Active::FindMemory), Some(at)) => self.spawn_note(at)?,
            (_, Some(at)) => self.spawn_shard_slot(at, wake_door)?,
            _ => {}
        }
        if let (Some(at), false) = (dream.note, find_memory) {
            self.spawn_note(at)?;
        }
        if self.prologue.is_some() && self.director.depth == 0 {
            if let Some(w) = dream.route.get(2.min(dream.route.len() - 1)) {
                self.pending_note = Some((0, 0));
                self.spawn_note(w.pos)?;
            }
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
                    Translucent,
                ));
            }
        }

        let spawn = dream.spawn + Vec3::Y;
        self.player_position = spawn;
        self.camera_pos = spawn + gameplay::CAMERA_OFFSET;
        let crystal = self.booklet.stash.crystal.rgba();
        let player_texture = self.texture(gl, crystal);
        self.player_tex = Some(player_texture.clone());
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
            PlayerBody,
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
            * self.active_asc.enemy_speed()
            * (1.0 - self.card_fx.calm_pct.min(50) as f32 / 100.0)
            * if slow_heart {
                store::SLOW_HEART_ENEMY_SPEED
            } else {
                1.0
            })
        .min(0.92 * player_speed / enemy_ai::CHASE_BOOST);
        let alert = if eyelids {
            0.0
        } else {
            gameplay::pressured_chase_radius(dream.depth, difficulty)
                * self.run.alert()
                * self.active_asc.alert()
        };
        let cap = 0.92 * player_speed / enemy_ai::CHASE_BOOST;
        let mut elite_tex: HashMap<Elite, Arc<GpuTexture>> = HashMap::new();
        for (k, &(a, b)) in dream.patrols.iter().enumerate() {
            let elite = specials::roll_elite(dream.seed, k, difficulty);
            let texture = match elite {
                None => enemy_texture.clone(),
                Some(e) => match elite_tex.get(&e) {
                    Some(t) => t.clone(),
                    None => {
                        let mut spec = dream.surfaces.enemy.clone();
                        spec.palette[0] = e.color();
                        let t = self.upload_surface(gl, &spec)?;
                        elite_tex.insert(e, t.clone());
                        t
                    }
                },
            };
            let pace = if elite == Some(Elite::Fast) {
                (speed * specials::ELITE_FAST).min(cap)
            } else {
                speed
            };
            let entity = self.world.spawn((
                Transform {
                    position: a,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::splat(0.9),
                },
                MeshRenderer {
                    mesh: enemy_mesh.clone(),
                    texture: Some(texture),
                },
                Spin(0.9),
            ));
            if let Some(e) = elite {
                log::info!("Elite pacer: {}", e.label());
            }
            self.enemies.push(Pacer {
                entity,
                ai: EnemyAI::new(a, b, pace, alert, gameplay::CHASE_LEASH),
                elite,
                a,
                b,
                split: false,
            });
        }
        self.spawn_specials(gl, &dream, speed, player_speed, &enemy_texture)?;
        self.spawn_features(gl, &dream)?;
        if !dream.dissolve.is_empty() {
            // Tiles in the same order as the dissolve state's cells.
            let tiles = dream
                .dissolve
                .iter()
                .filter_map(|c| {
                    dissolve_tiles
                        .iter()
                        .find(|(p, _, _)| (p.x - c.x).abs() < 1e-3 && (p.z - c.z).abs() < 1e-3)
                        .map(|&(_, e, size)| (e, size))
                })
                .collect::<Vec<_>>();
            if tiles.len() == dream.dissolve.len() {
                self.dissolve = Some((dissolve::Dissolve::new(dream.dissolve.clone()), tiles));
            } else {
                log::warn!(
                    "dissolve: {} cells but {} tiles",
                    dream.dissolve.len(),
                    tiles.len()
                );
            }
        }
        if !dream.beat_tiles.is_empty() {
            let beat_tex = self.texture(gl, [255, 80, 200, 255]);
            let plate = self.mesh(Shape::Cube)?;
            for &at in &dream.beat_tiles {
                let e = self.world.spawn((
                    Transform {
                        position: at + Vec3::Y * 0.03,
                        rotation: Quat::IDENTITY,
                        scale: Vec3::new(gameplay::CELL * 0.9, 0.04, gameplay::CELL * 0.9),
                    },
                    MeshRenderer {
                        mesh: plate.clone(),
                        texture: Some(beat_tex.clone()),
                    },
                    Lit,
                ));
                self.beat_tiles.push((e, at));
            }
        }
        if !dream.watchers.is_empty() {
            let eye = self.mesh(Shape::Orb)?;
            let mut spec = dream.surfaces.prop.clone();
            spec.pattern = dream::Pattern::Eyes;
            let eye_tex = self.upload_surface(gl, &spec)?;
            for &(at, _) in &dream.watchers {
                let e = self.world.spawn((
                    Transform {
                        position: at,
                        rotation: Quat::IDENTITY,
                        scale: Vec3::new(0.8, 0.8, 0.25),
                    },
                    MeshRenderer {
                        mesh: eye.clone(),
                        texture: Some(eye_tex.clone()),
                    },
                    Lit,
                    NoMelt,
                ));
                self.watchers.push(e);
            }
        }

        // Nightmare: sigils round the edge, and the hunter in the middle.
        let sigil_tex = self
            .shard_tex
            .clone()
            .context("shard textures not uploaded")?;
        self.sigil_tex = Some(sigil_tex);
        for (k, &at) in dream.sigils.iter().enumerate() {
            self.spawn_sigil(at, k);
        }
        if let Some(at) = dream.hunter {
            let h = boss::Boss::new(dream.depth, self.player_speed());
            let entity = self.world.spawn((
                Transform {
                    position: at,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::splat(h.hunter.size()),
                },
                MeshRenderer {
                    mesh: enemy_mesh.clone(),
                    texture: Some(enemy_texture.clone()),
                },
                Spin(0.6),
                Lit,
            ));
            // The shockwave (a flat hoop) and the wisps, hidden until used.
            // The ring is drawn as segments exactly as wide as the band that
            // hits you (a scaled torus would look far fatter than it is).
            let seg_mesh = self.mesh(Shape::Cube)?;
            let ring_tex = self.texture(gl, [255, 90, 160, 255]);
            let ring: Vec<Entity> = (0..RING_SEGMENTS)
                .map(|_| {
                    self.world.spawn((
                        Transform {
                            position: at,
                            rotation: Quat::IDENTITY,
                            scale: Vec3::ZERO,
                        },
                        MeshRenderer {
                            mesh: seg_mesh.clone(),
                            texture: Some(ring_tex.clone()),
                        },
                        Lit,
                        NoMelt,
                    ))
                })
                .collect();
            let wisp_mesh = self.mesh(Shape::Orb)?;
            let wisps = [(); boss::MAX_WISPS].map(|_| {
                self.world.spawn((
                    Transform {
                        position: at,
                        rotation: Quat::IDENTITY,
                        scale: Vec3::ZERO,
                    },
                    MeshRenderer {
                        mesh: wisp_mesh.clone(),
                        texture: Some(enemy_texture.clone()),
                    },
                    Lit,
                    Spin(3.0),
                ))
            });
            self.boss_fx = Some((ring, wisps));
            self.dream_name = format!("{}: {}", boss::title(h.tier), self.dream_name);
            self.hunter = Some((entity, h, at));
            self.dream_whisper = "gather the sigils. it is hunting you.".into();
            log::info!(
                "Nightmare at depth {}: {} sigils",
                dream.depth,
                dream.sigils.len()
            );
            self.eye_line = None;
            self.think(eye::Moment::NightmareAhead, false);
        }
        if self.director.blend.is_some() {
            self.think(eye::Moment::FusedDream, false);
        }
        if self.director.card_dream {
            log::info!("Card dream: {theme:?} at depth {}", self.director.depth);
        }

        // The next dream inherits one of this dream's prop kinds as its motif.
        self.motif = spec
            .props
            .get((dream.seed % spec.props.len() as u64) as usize)
            .copied();
        self.dream = Some(dream);
        if !self.autopilot && self.director.depth > self.best_depth {
            self.best_depth = self.director.depth;
            if let Err(e) = records::save(&records::record_path(), self.best_depth) {
                log::warn!("could not save best depth: {e}");
            }
        }
        // The codex remembers every dream and strange thing met.
        if self.run_active {
            let mut changed = self.booklet.codex.visit(theme, self.director.depth);
            for &k in &self.seen_kinds {
                changed |= self.booklet.codex.meet(k);
            }
            if changed {
                self.persist_booklet();
            }
            self.save_run();
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
        if let Some((d, _)) = self.dissolve.as_mut() {
            d.reset();
        }
        self.route_index = 0;
        self.grace = gameplay::RESPAWN_GRACE
            * self.run.grace()
            * self.active_asc.grace()
            * (1.0 + 0.2 * self.card_fx.grace_bonus as f32)
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
        // Objectives that aren't a place: wait it out, or run the memory down.
        match &self.objective {
            Some(objective::Active::HoldOn(_)) => return (Vec3::ZERO, false),
            Some(objective::Active::Chase(c)) if grounded => {
                // Walk the route toward the memory, one waypoint at a time.
                let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
                if flat(c.pos - self.player_position).length() < 2.5 || route.is_empty() {
                    return (
                        gameplay::autopilot_velocity(self.player_position, c.pos),
                        false,
                    );
                }
                let near = route
                    .iter()
                    .enumerate()
                    .min_by(|a, b| {
                        flat(a.1.pos - self.player_position)
                            .length()
                            .total_cmp(&flat(b.1.pos - self.player_position).length())
                    })
                    .map_or(0, |(i, _)| i);
                let next = if near < c.target {
                    near + 1
                } else {
                    near.saturating_sub(1)
                };
                let here = flat(route[near].pos - self.player_position).length();
                let wp = if here > gameplay::CELL * 0.8 {
                    &route[near]
                } else {
                    &route[next.min(route.len() - 1)]
                };
                return (
                    gameplay::autopilot_velocity(self.player_position, wp.pos),
                    wp.jump,
                );
            }
            _ => {}
        }
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
        steam::init(&self.booklet.achievements.ids);
        self.on_launch();
        self.coop_dev_start();
        log::info!("Initializing Dreamscape");
        let gl = ctx.gl();
        unsafe {
            gl.enable(engine::glow::DEPTH_TEST);
        }
        let cycler = ProfileCycler::new(engine::profile::load_dir(&paths::asset(PROFILES_DIR))?)?;
        let first = cycler.current().clone();
        let read = |p: &Path| {
            std::fs::read_to_string(paths::asset_path(p))
                .with_context(|| format!("reading shader {p:?}"))
        };
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
        self.apply_settings(ctx);
        if self.mode == hud::Mode::Playing {
            self.begin_run();
        }
        // Dev switch: DREAMSCAPE_THEME=SkyStairs starts one dream deep in it.
        if let Ok(name) = crate::dev::var("DREAMSCAPE_THEME") {
            match dream::ALL_THEMES.iter().find(|t| format!("{t:?}") == name) {
                Some(&t) => {
                    self.director.theme = t;
                    self.director.depth = 1;
                }
                None => log::warn!("DREAMSCAPE_THEME={name}: no such dream"),
            }
        }
        // Dev switch: DREAMSCAPE_BLEND=TheTunnel fuses the first dream with it.
        if let Ok(name) = crate::dev::var("DREAMSCAPE_BLEND") {
            self.director.blend = dream::ALL_THEMES
                .iter()
                .copied()
                .find(|t| format!("{t:?}") == name);
        }
        // Dev switch: DREAMSCAPE_NIGHTMARE=1 starts in a nightmare arena.
        if crate::dev::var("DREAMSCAPE_NIGHTMARE").is_ok() {
            if self.director.theme == DreamTheme::Lobby {
                self.director.theme = DreamTheme::NightmareFactory;
            }
            // DREAMSCAPE_NIGHTMARE=3 starts in the third tier's arena.
            let tier: u32 = crate::dev::var("DREAMSCAPE_NIGHTMARE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1)
                .max(1);
            self.director.depth = dream::NIGHTMARE_EVERY * tier;
            self.director.nightmare = true;
            self.director.has_shard = false;
        }
        self.load_dream(ctx)?;
        // Dev switch: DREAMSCAPE_SCREEN=codex|settings opens that screen.
        match crate::dev::var("DREAMSCAPE_SCREEN").as_deref() {
            Ok("codex") => self.open_codex(),
            Ok("settings") => self.open_settings(),
            Ok("memories") => self.mode = hud::Mode::Memories,
            Ok("loadout") => self.open_loadout(),
            Ok("merge") => self.open_merge(hud::Mode::Title),
            Ok("store") => self.open_store(),
            Ok("lottery") | Ok("lottery_pull") => {
                // Dev only: a fat purse so the screen has something to show.
                self.booklet.stash.earn(300);
                self.open_store();
                self.store_tab = 1;
                if crate::dev::var("DREAMSCAPE_SCREEN").as_deref() == Ok("lottery_pull") {
                    self.lottery_pull();
                }
            }
            Ok("ending") => {
                self.ending_age = crate::dev::var("DREAMSCAPE_ENDING_AT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                self.mode = hud::Mode::Ending;
            }
            Ok("note") => {
                let d = lore::dreamer(self.booklet.lore.save_seed, 3);
                self.note_view = Some(lore::note(&d, 2, DreamTheme::Garden));
                self.mode = hud::Mode::Note;
            }
            _ => {}
        }
        if crate::dev::var("DREAMSCAPE_PROLOGUE").is_ok() {
            self.pending_start = Some(StartKind::Prologue);
        }
        if crate::dev::var("DREAMSCAPE_CONTINUE").is_ok() {
            self.pending_start = Some(StartKind::Continue);
        }
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
            // Controller buttons stand in for the keys they map to.
            // Controller buttons stand in for keys, per screen (see pad.rs).
            Event::ControllerButtonDown { button, .. } => {
                self.using_pad = true;
                match pad::key_for(*button, self.mode, &self.settings) {
                    Some(k) => {
                        // Remember what this press did, so the release matches
                        // even if the screen changed in between.
                        self.pad_held.insert(*button as i32, k);
                        (k, true, false)
                    }
                    None => return,
                }
            }
            Event::ControllerButtonUp { button, .. } => {
                match self.pad_held.remove(&(*button as i32)) {
                    Some(k) => (k, false, false),
                    None => return,
                }
            }
            Event::ControllerAxisMotion { value, .. } => {
                if value.unsigned_abs() > 12_000 {
                    self.using_pad = true;
                }
                return;
            }
            // A real nudge of the mouse (not the synthetic event a window
            // gets when it opens under the cursor) switches back to keys.
            Event::MouseMotion { xrel, yrel, .. } => {
                if !self.mouse_captured && xrel.abs() + yrel.abs() > 6 {
                    self.using_pad = false;
                }
                return;
            }
            _ => return,
        };
        if matches!(event, Event::KeyDown { .. }) {
            self.using_pad = false;
        }
        if down && !repeat && self.mode == hud::Mode::Settings {
            self.settings_key(ctx, key);
            return;
        }
        if down && !repeat {
            let revealed = reveal_ui::reveal_done_pack(&self.pack.pack, self.pack.age);
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
                (hud::Mode::Note, Keycode::Return | Keycode::Space | Keycode::Escape) => {
                    self.mode = hud::Mode::Playing;
                    return;
                }
                (hud::Mode::Loadout, _) => {
                    self.loadout_key(key);
                    return;
                }
                (hud::Mode::Lobby, _) => {
                    self.lobby_key(key);
                    return;
                }
                (hud::Mode::Ending, Keycode::Return | Keycode::Space) => {
                    self.finish_ending();
                    return;
                }
                (hud::Mode::Ending, _) => return,
                (hud::Mode::Merge, _) => {
                    self.merge_key(key);
                    return;
                }
                (hud::Mode::Store, Keycode::M) => {
                    let back = self.booklet_return;
                    self.open_merge(hud::Mode::Store);
                    self.booklet_return = back;
                    return;
                }
                (hud::Mode::Memories, Keycode::Escape) => {
                    self.mode = hud::Mode::Title;
                    return;
                }
                (hud::Mode::Memories, Keycode::W | Keycode::Up) => {
                    self.memories_sel = self.memories_sel.saturating_sub(1);
                    return;
                }
                (hud::Mode::Memories, Keycode::S | Keycode::Down) => {
                    self.memories_sel = (self.memories_sel + 1).min(lore::DREAMERS as usize - 1);
                    return;
                }
                (hud::Mode::Title, Keycode::P) => {
                    self.pending_start = Some(StartKind::Prologue);
                    return;
                }
                (hud::Mode::Title, Keycode::M) => {
                    self.memories_sel = 0;
                    self.mode = hud::Mode::Memories;
                    return;
                }
                (hud::Mode::Warning, Keycode::Return | Keycode::Space | Keycode::Escape) => {
                    if self.warning_age >= title_ui::WARNING_MIN {
                        self.mode = hud::Mode::Title;
                    }
                    return;
                }
                (hud::Mode::Playing, Keycode::Escape) => {
                    ctx.platform.sdl.mouse().set_relative_mouse_mode(false);
                    self.mouse_captured = false;
                    self.mode = hud::Mode::Paused;
                    self.input = PlayerInputState::default();
                    return;
                }
                (hud::Mode::Paused, Keycode::Escape) => {
                    self.mode = hud::Mode::Playing;
                    return;
                }
                (hud::Mode::Paused, Keycode::Q) => {
                    self.dream = None;
                    self.mode = hud::Mode::Title;
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
                (hud::Mode::Reveal, Keycode::Escape) if revealed => {
                    self.dream = None;
                    self.mode = hud::Mode::Title;
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
                (hud::Mode::Booklet, Keycode::P) => {
                    self.export_requested = true;
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
                (hud::Mode::Store, Keycode::Tab) => {
                    self.store_tab = 1 - self.store_tab;
                    self.lottery_status = None;
                    return;
                }
                (hud::Mode::Store, Keycode::Return | Keycode::Space) if self.store_tab == 1 => {
                    self.lottery_pull();
                    return;
                }
                (hud::Mode::Store, Keycode::P) if self.store_tab == 1 => {
                    self.lottery_spark_pick();
                    return;
                }
                (hud::Mode::Store, Keycode::A | Keycode::Left) if self.store_tab == 1 => {
                    let n = lottery::pullable_themes().len().max(1);
                    self.lottery_pick = (self.lottery_pick + n - 1) % n;
                    return;
                }
                (hud::Mode::Store, Keycode::D | Keycode::Right) if self.store_tab == 1 => {
                    let n = lottery::pullable_themes().len().max(1);
                    self.lottery_pick = (self.lottery_pick + 1) % n;
                    return;
                }
                (hud::Mode::Store, Keycode::W | Keycode::Up | Keycode::S | Keycode::Down)
                    if self.store_tab == 1 =>
                {
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
                    let n = self.title_items().len();
                    self.title_sel = (self.title_sel + n - 1) % n;
                    return;
                }
                (hud::Mode::Title, Keycode::S | Keycode::Down) => {
                    self.title_sel = (self.title_sel + 1) % self.title_items().len();
                    return;
                }
                (hud::Mode::Title, Keycode::Return | Keycode::Space) => {
                    let items = self.title_items();
                    let item = items[self.title_sel.min(items.len() - 1)];
                    self.title_select(ctx, item, 1);
                    return;
                }
                (hud::Mode::Title, Keycode::A | Keycode::Left | Keycode::D | Keycode::Right) => {
                    let items = self.title_items();
                    let item = items[self.title_sel.min(items.len() - 1)];
                    if matches!(item, TitleItem::Ascension | TitleItem::RunLength) {
                        let dir = if matches!(key, Keycode::A | Keycode::Left) {
                            -1
                        } else {
                            1
                        };
                        self.title_select(ctx, item, dir);
                    }
                    return;
                }
                (hud::Mode::Title, Keycode::C) => {
                    self.title_select(ctx, TitleItem::Continue, 1);
                    return;
                }
                (hud::Mode::Title, Keycode::T) => {
                    self.title_select(ctx, TitleItem::Daily, 1);
                    return;
                }
                (hud::Mode::Title | hud::Mode::Paused, Keycode::X) => {
                    self.open_codex();
                    return;
                }
                (hud::Mode::Codex, Keycode::Escape) => {
                    self.mode = self.codex_return;
                    return;
                }
                (hud::Mode::Title | hud::Mode::Paused, Keycode::O) => {
                    self.open_settings();
                    return;
                }
                (hud::Mode::Title, Keycode::R) => {
                    self.run_length = self.run_length.toggled();
                    self.title_sel = self
                        .title_items()
                        .iter()
                        .position(|&i| i == TitleItem::RunLength)
                        .unwrap_or(0);
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
        use settings::Action;
        match self.settings.action_for(key) {
            Some(Action::Forward) => self.input.forward = down,
            Some(Action::Back) => self.input.backward = down,
            Some(Action::Left) => self.input.left = down,
            Some(Action::Right) => self.input.right = down,
            Some(Action::Jump) => self.input.jump = down,
            Some(Action::Ability1) if down && !repeat => self.ability_requests[0] = true,
            Some(Action::Ability2) if down && !repeat => self.ability_requests[1] = true,
            _ => {}
        }
        match key {
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
        steam::tick();
        self.coop_tick(ctx, dt)?;
        if let Some((seed, long, players)) = self.pending_coop_start.take() {
            self.start_coop_run(ctx, seed, long, players)?;
        }
        if let Some((_, age)) = &mut self.achievement_toast {
            *age += dt;
            if *age > ACHIEVEMENT_TOAST {
                self.achievement_toast = None;
            }
        }
        self.poll_music();
        if self.time > 2.0 && crate::dev::var("DREAMSCAPE_CRASH").is_ok() {
            panic!("DREAMSCAPE_CRASH: test crash");
        }
        if self.mode == hud::Mode::Warning {
            self.warning_age += dt;
        }
        if self.mode == hud::Mode::Ending {
            self.ending_age += dt;
            // E2E runs sit through it, then carry on.
            if self.autopilot && ending::can_continue(self.ending_age) {
                self.finish_ending();
            }
        }
        if !self.entrance.done {
            self.entrance.tick(dt);
        }
        if self.mode == hud::Mode::Store {
            self.lottery_age += dt;
        }
        if self.mode == hud::Mode::Playing {
            let dt = dt.min(gameplay::MAX_DT);
            self.objective_clock += dt;
            self.eye_lid = if self.entrance.done {
                let target = hud::eye_openness(
                    self.director.lucidity,
                    self.director.shards_to_wake,
                );
                eye_motion::ease_lid(self.eye_lid, target, dt)
            } else {
                // The entrance is its own, already-smooth lid curve.
                self.entrance.lid()
            };
        }
        if let Some((_, age)) = &mut self.eye_line {
            *age += dt;
        }
        if crate::dev::var_os("DREAMSCAPE_FPS").is_some() {
            self.frame_ms.0 += dt * 1000.0;
            self.frame_ms.1 += 1;
            if self.frame_ms.1 == 300 {
                log::info!("avg frame {:.1} ms", self.frame_ms.0 / 300.0);
                self.frame_ms = (0.0, 0);
            }
        }
        let dt = dt.min(gameplay::MAX_DT);
        // Mouse look owns the cursor only while actually dreaming in first person.
        let want_mouse =
            self.first_person_active() && self.mode == hud::Mode::Playing && !self.autopilot;
        if want_mouse != self.mouse_captured {
            ctx.platform.sdl.mouse().set_relative_mouse_mode(want_mouse);
            self.mouse_captured = want_mouse;
        }
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
        if let Some(kind) = self.pending_start.take() {
            return self.start_pending(ctx, kind);
        }
        // E2E runs: show the journal briefly (for screenshots), then exit.
        if self.autopilot
            && matches!(
                self.mode,
                hud::Mode::Journal | hud::Mode::Booklet | hud::Mode::Reveal | hud::Mode::Store
            )
            && self.title_age
                > crate::dev::var("DREAMSCAPE_JOURNAL_HOLD")
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
        let fp = self.first_person_active();
        if fp && !self.autopilot {
            let (dx, dy) = ctx.input.mouse_delta();
            (self.yaw, self.pitch) = fpv::turn(
                self.yaw,
                self.pitch,
                dx as f32,
                dy as f32,
                self.settings.mouse_sens,
                self.settings.invert_y,
            );
            let (rx, ry) = ctx.input.right_stick();
            let keys = ctx.input.is_key_down(Keycode::Right) as i32 as f32
                - ctx.input.is_key_down(Keycode::Left) as i32 as f32;
            self.yaw = (self.yaw - (rx * fpv::STICK_TURN + keys * fpv::KEY_TURN) * dt)
                .rem_euclid(std::f32::consts::TAU);
            self.pitch = (self.pitch - ry * fpv::STICK_TURN * 0.6 * dt)
                .clamp(-fpv::PITCH_LIMIT, fpv::PITCH_LIMIT);
        }
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
        self.update_objective(ctx, dt);
        if !self.eye_queue.is_empty()
            && self
                .eye_line
                .as_ref()
                .is_none_or(|(_, age)| *age > eye::LINE_SECS + 1.0)
        {
            let next = self.eye_queue.remove(0);
            log::info!("Eye: {next}");
            self.eye_line = Some((next, 0.0));
        }

        // 1. Input (or autopilot) -> player body
        let (desired, wants_jump) = if self.autopilot {
            self.autopilot_step()
        } else {
            let ground = if self.on_vein() {
                specials::VEIN_BOOST
            } else {
                1.0
            } * if self.in_fog() {
                specials::FOG_SLOW
            } else {
                1.0
            };
            let speed = self.player_speed() * ground;
            let walk = if fp {
                fpv::velocity(&self.input, self.yaw)
            } else {
                gameplay::horizontal_velocity(&self.input)
            };
            let mut v = walk * (speed / gameplay::MOVE_SPEED);
            // The left stick, if it's pushed, wins (analogue: half-push walks).
            if let Some((x, z)) = settings::stick_dir(ctx.input.left_stick()) {
                v = if fp {
                    fpv::from_world_stick(x, z, self.yaw)
                } else {
                    Vec3::new(x, 0.0, z)
                } * speed;
            }
            if self.inverted() {
                v = Vec3::new(-v.x, v.y, -v.z);
            }
            // Co-op: waiting in the portal for the others.
            if self.coop.as_ref().is_some_and(|c| c.in_portal) {
                (Vec3::ZERO, false)
            } else {
                (v, self.input.jump)
            }
        };
        if desired.length_squared() > 1e-4 {
            self.facing = desired.normalize();
        }
        if fp {
            if self.autopilot {
                if let Some(y) = fpv::yaw_toward(desired) {
                    self.yaw = y;
                    self.pitch = -0.12;
                }
            } else if desired.length_squared() <= 1e-4 {
                self.facing = fpv::forward(self.yaw); // a dash goes where you look
            }
        }
        if desired.length_squared() > 1e-4 {
            self.tutorial(tutorial::Event::Moved(desired.length() * dt));
        }
        if self.autopilot || (wants_jump && !self.jump_was_down) {
            // (the autopilot never needs to jump in the Lobby)
            if self
                .prologue
                .is_some_and(|p| p.step == tutorial::Step::Jump)
            {
                self.tutorial(tutorial::Event::Jumped);
            }
        }
        let jump_pressed = wants_jump && !self.jump_was_down;
        self.jump_was_down = wants_jump;
        let feature = self
            .dream
            .as_ref()
            .map_or(dream::Feature::None, |d| d.theme.spec().feature);
        let jump_speed =
            gameplay::JUMP_SPEED * self.run.jump() * self.twists.jump() * feature.jump();
        let dashing = self.run.active(Ability::Dash);
        if let Ok(mut body) = self.world.get::<&mut RigidBody>(player) {
            let v = if dashing {
                self.facing * gameplay::MOVE_SPEED * upgrades::DASH_SPEED_MULT
            } else {
                desired
            };
            body.velocity.x = v.x;
            body.velocity.z = v.z;
            self.player_grounded = body.grounded;
            if body.grounded {
                self.air_jumps_used = 0;
            }
            if wants_jump && body.grounded {
                body.velocity.y = jump_speed;
                if jump_pressed {
                    self.sfx(Sound::Jump);
                }
            } else if jump_pressed && !body.grounded && self.air_jumps_used < self.run.air_jumps() {
                self.air_jumps_used += 1;
                body.velocity.y = jump_speed;
                self.sfx(Sound::AirJump);
            }
        }

        // 2. Enemies (STILLNESS and STUTTERING dreams hold them in place)
        let target = upgrades::lure_target(
            self.player_position,
            self.decoy.map(|(at, _)| at),
            self.run.active(Ability::Decoy),
        );
        let held = self.run.active(Ability::Stillness)
            || self.twists.enemies_frozen(self.dream_age)
            || self.coop_hold();
        // Co-op: the host's enemies hunt whoever's nearest; clients just
        // show where the host put them.
        let hunted = self.coop_targets();
        let puppet = self.coop_client();
        if puppet {
            self.coop_apply_enemies();
        }
        let mut splits = Vec::new();
        let mut noticed = false;
        for (k, p) in self.enemies.iter_mut().enumerate() {
            if held || puppet {
                continue;
            }
            if let Ok(mut t) = self.world.get::<&mut Transform>(p.entity) {
                let was = p.ai.chasing;
                let target = if self.run.active(Ability::Decoy) || hunted.is_empty() {
                    target
                } else {
                    nearest(&hunted, t.position)
                };
                p.ai.update(&mut t.position, target, dt);
                // Swell when they notice you: the tell that you've been seen.
                let size = if p.ai.chasing { 1.25 } else { 0.9 }
                    * match p.elite {
                        Some(Elite::Big) => 1.35,
                        Some(Elite::Splitter) if p.split => 0.75,
                        _ => 1.0,
                    };
                let hidden = p.elite == Some(Elite::Shade)
                    && (t.position - self.player_position).length() > specials::SHADE_SHOWS_WITHIN;
                t.scale = Vec3::splat(if hidden { 0.0 } else { size });
                if p.ai.chasing && !was && !self.autopilot {
                    log::info!("An enemy noticed you");
                    noticed = true;
                    if p.elite == Some(Elite::Splitter) && !p.split {
                        p.split = true;
                        splits.push((k, t.position));
                    }
                }
            }
        }
        if noticed {
            self.think(eye::Moment::EnemySeen, true);
        }
        for (k, at) in splits {
            let (a, b, speed) = (
                self.enemies[k].b,
                self.enemies[k].a,
                self.enemies[k].ai.speed(),
            );
            let (mesh, texture) = {
                let r = self
                    .world
                    .get::<&MeshRenderer>(self.enemies[k].entity)
                    .expect("pacer has a mesh");
                (r.mesh.clone(), r.texture.clone())
            };
            let entity = self.world.spawn((
                Transform {
                    position: at,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::splat(0.7),
                },
                MeshRenderer { mesh, texture },
                Spin(-0.9),
            ));
            log::info!("A splitter split");
            self.sfx(Sound::Split);
            self.enemies.push(Pacer {
                entity,
                ai: EnemyAI::new(a, b, speed, 0.0, gameplay::CHASE_LEASH),
                elite: Some(Elite::Splitter),
                a,
                b,
                split: true,
            });
        }
        if !self.run.active(Ability::Decoy) {
            if let Some((_, e)) = self.decoy.take() {
                let _ = self.world.despawn(e);
            }
        }
        self.update_specials(dt, held)?;
        self.update_features();
        let mut boss_events = Vec::new();
        if let Some((entity, b, _)) = self.hunter.as_mut() {
            if !held && !puppet {
                if let Ok(mut t) = self.world.get::<&mut Transform>(*entity) {
                    let target = if hunted.is_empty() {
                        target
                    } else {
                        nearest(&hunted, t.position)
                    };
                    boss_events = b.update(&mut t.position, target, dt);
                    // It swells and throbs while it winds up an attack: the tell.
                    let pulse = if b.warning().is_some() {
                        1.0 + 0.15 * (self.time * 20.0).sin()
                    } else {
                        1.0
                    };
                    t.scale = Vec3::splat(b.hunter.size() * pulse);
                }
            }
        }
        for e in boss_events {
            match e {
                boss::Event::Telegraph(_) => {
                    self.sfx(Sound::MeltStart);
                    self.flash.trigger([1.0, 0.3, 0.5], 0.25);
                }
                boss::Event::Ring => self.sfx(Sound::Crumble),
                boss::Event::Summon => self.sfx(Sound::Split),
                boss::Event::Eclipse => self.sfx(Sound::Stillness),
            }
            log::info!("Boss: {e:?}");
        }
        self.sync_boss_fx();

        // 3. Physics: gravity, collision, trigger overlaps
        let physics = PhysicsParams {
            gravity: PhysicsParams::default().gravity
                * self.twists.gravity()
                * self.run.gravity()
                * self
                    .dream
                    .as_ref()
                    .map_or(1.0, |d| d.theme.spec().feature.gravity()),
            ..PhysicsParams::default()
        };
        let overlaps = engine::physics::step(&mut self.world, dt, &physics);
        if let Some(p) = self.player {
            if self.world.get::<&RigidBody>(p).is_ok_and(|b| b.grounded) {
                self.rewind_trail.push(self.dream_age, self.player_position);
            }
        }
        if let Ok(t) = self.world.get::<&Transform>(player) {
            self.player_position = t.position;
        }
        let under = self.player_position;
        for (_e, (t, _)) in self.world.query_mut::<(&mut Transform, &Halo)>() {
            t.position = Vec3::new(under.x, 0.04, under.z);
        }
        self.camera_pos = gameplay::follow_camera(self.camera_pos, self.player_position, dt);
        self.update_echoes(player, dt)?;
        self.update_bursts(ctx.gl())?;
        if !self.autopilot {
            // White Dissolve: the way behind you goes while you move.
            let speed = self
                .world
                .get::<&RigidBody>(player)
                .map(|b| Vec3::new(b.velocity.x, 0.0, b.velocity.z).length())
                .unwrap_or(0.0);
            if let Some((d, tiles)) = self.dissolve.as_mut() {
                d.update(self.player_position, speed, dt);
                for (i, &(e, size)) in tiles.iter().enumerate() {
                    let o = d.opacity(i);
                    if let Ok(mut t) = self.world.get::<&mut Transform>(e) {
                        t.scale = Vec3::new(size.x * o, size.y, size.z * o);
                    }
                    let has = self.world.get::<&Collider>(e).is_ok();
                    if d.solid(i) && !has {
                        let collider = Collider {
                            shape: ColliderShape::Aabb {
                                half_extents: size * 0.5,
                            },
                            is_trigger: false,
                        };
                        let _ = self.world.insert_one(e, collider);
                    } else if !d.solid(i) && has {
                        let _ = self.world.remove_one::<Collider>(e);
                    }
                }
            }
        }

        // 4. Fail states (autopilot is immune to enemies so E2E runs are deterministic)
        let untouchable = self.run.active(Ability::Dash) || self.run.active(Ability::Phase);
        let caught_by =
            if !self.autopilot && self.grace <= 0.0 && !untouchable && !self.coop_ghost() {
                self.threat_touching()
            } else {
                None
            };
        let caught = caught_by.is_some();
        if let Some(by) = caught_by {
            log::info!(
                "Caught by: {by} (depth {}, {:.1}s into the dream)",
                self.director.depth,
                self.dream_age
            );
            self.think(eye::Moment::Caught, false);
            if let Some(objective::Active::HoldOn(h)) = &mut self.objective {
                h.caught();
            }
        }
        if caught && self.coop_active() {
            // Co-op: no respawn and no dropped shard, just a ghost.
            self.flash.trigger([1.0, 0.1, 0.15], 0.7);
            self.sfx(Sound::Caught);
            self.stats.caught += 1;
            self.caught_this_dream += 1;
            self.coop_caught();
            return Ok(());
        }
        if caught {
            self.flash.trigger([1.0, 0.1, 0.15], 0.7);
            self.sfx(Sound::Caught);
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
                self.sfx(Sound::ShardLost);
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
                    self.despawn_shard_slot();
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
                self.caught_this_dream += 1;
            } else {
                self.stats.fell += 1;
            }
            // The mimic loses your trail; stalkers slink back to their lairs.
            self.trail.clear();
            self.rewind_trail.clear();
            self.mimic_awake_at = self.dream_age + specials::MIMIC_DELAY;
            for sp in &self.specials {
                if let SpecialAI::Stalker(st) = &sp.ai {
                    if let Ok(mut t) = self.world.get::<&mut Transform>(sp.entity) {
                        t.position = st.lair;
                    }
                }
            }
            // The hunter goes back to its corner to catch its breath.
            if let Some((entity, b, home)) = self.hunter.as_mut() {
                b.after_catch();
                if let Ok(mut t) = self.world.get::<&mut Transform>(*entity) {
                    t.position = *home;
                }
            }
            // From the Swarm Mother on, a catch puts your newest sigil back.
            let drops = self.hunter.as_ref().is_some_and(|(_, b, _)| b.tier >= 3);
            if drops && self.sigils_left > 0 {
                if let Some(at) = self.sigils_taken.pop() {
                    self.spawn_sigil(at, self.sigils_left);
                    self.sigils_left += 1;
                    log::info!("Sigil dropped ({} left)", self.sigils_left);
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
            if self.world.get::<&NoteMarker>(e).is_ok() {
                let _ = self.world.despawn(e);
                if let Some((id, k)) = self.pending_note.take() {
                    self.read_note(id, k);
                }
                if matches!(self.objective, Some(objective::Active::FindMemory)) {
                    self.complete_objective(ctx);
                }
                break;
            }
            if self.world.get::<&ShardMarker>(e).is_ok() {
                if let Some(objective::Active::Fragments(f)) = &mut self.objective {
                    f.take();
                    let done = f.done();
                    log::info!("Fragment {}", f.label());
                    // Remove this piece and its beacon (spawned as a pair).
                    if let Some(i) = self.shard_entities.iter().position(|&x| x == e) {
                        for x in self
                            .shard_entities
                            .drain(i..(i + 2).min(self.shard_entities.len()))
                        {
                            let _ = self.world.despawn(x);
                        }
                    }
                    self.sfx(Sound::Sigil);
                    if done {
                        self.complete_objective(ctx);
                    }
                    break;
                }
                // A chased memory is caught by reaching it, not by its trigger.
                if matches!(self.objective, Some(objective::Active::Chase(_))) {
                    break;
                }
                if self.coop_active() {
                    if !self.coop_ghost() {
                        self.coop_touch_shard(ctx);
                    }
                    break;
                }
                self.complete_objective(ctx);
                break;
            }
        }
        let sigils: Vec<Entity> = touched
            .iter()
            .copied()
            .filter(|&e| self.world.get::<&SigilMarker>(e).is_ok())
            .collect();
        for e in sigils {
            if let Ok(t) = self.world.get::<&Transform>(e) {
                let at = Vec3::new(t.position.x, 0.0, t.position.z);
                self.sigils_taken.push(at);
            }
            if let Some((_, b, _)) = self.hunter.as_mut() {
                b.hunter.enrage();
            }
            let _ = self.world.despawn(e);
            self.sigils_left = self.sigils_left.saturating_sub(1);
            self.flash.trigger([1.0, 0.4, 1.0], 0.5);
            self.sfx(Sound::Sigil);
            log::info!("Sigil taken ({} left)", self.sigils_left);
            if self.sigils_left == 0 {
                self.open_portal()?;
            }
        }
        if touched
            .iter()
            .any(|&e| self.world.get::<&WakeMarker>(e).is_ok())
            && !self.coop_client()
            && !self.coop_ghost()
        {
            self.open_wake_choice();
            return Ok(());
        }
        let waiting = self.coop.as_ref().is_some_and(|c| c.in_portal);
        if touched
            .iter()
            .any(|&e| self.world.get::<&PortalMarker>(e).is_ok())
            && !waiting
            && !self.coop_ghost()
        {
            log::info!("Portal entered at depth {}", self.director.depth);
            self.tutorial(tutorial::Event::Portal);
            self.dream_bonus();
            self.stats.twists.extend(self.twists.0.iter().copied());
            if self.director.nightmare {
                self.stats.nightmares += 1;
                self.bonus_dust += 10 * self.hunter.as_ref().map_or(1, |(_, b, _)| b.tier);
                self.boss_reward = true;
                if self.run_active {
                    self.booklet.codex.nightmares_beaten += 1;
                    let tier = self.hunter.as_ref().map_or(1, |(_, b, _)| b.tier);
                    self.achieve(Some(achievements::Event::NightmareBeaten {
                        tier,
                        caught_in_it: self.caught_this_dream,
                    }));
                }
                log::info!("Nightmare beaten at depth {}", self.director.depth);
                self.eye_line = None;
                self.think(eye::Moment::NightmareBeaten, false);
            }
            if self.coop_active() {
                self.coop_enter_portal();
                return Ok(());
            }
            self.to_bottom = self.ending_due() && !self.director.nightmare;
            self.begin_melt(if self.director.theme == DreamTheme::Awakening {
                transition::Pending::Finish
            } else if self.to_bottom {
                log::info!("The portal leads to the bottom");
                transition::Pending::Wake
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
        let reduce_flashing = self.settings.reduce_flashing;
        let melt = self.transition.melt();
        let transition = self.transition;
        let strangeness = self.visual_strangeness();
        let mood = self
            .dream
            .as_ref()
            .map_or(dream::Mood::PLAIN, |d| d.theme.spec().mood);
        let motion = self.motion_scale();
        let pulse = mood.pulse(self.time) * motion;
        let player = self.player_position;
        let up = if self.inverted() && !self.debug_camera {
            -Vec3::Y
        } else {
            Vec3::Y
        };
        let sight_scale = self.twists.sight()
            * self.run.sight()
            * self
                .hunter
                .as_ref()
                .map_or(1.0, |(_, b, _)| b.sight_scale());
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
        let fp = self.first_person_active();
        let fov = if fp { fpv::FOV_DEG } else { 60.0_f32 };
        let (eye, target) = if self.debug_camera {
            (Vec3::new(0.0, 45.0, -35.0), Vec3::ZERO)
        } else if fp {
            fpv::camera(self.player_position, self.yaw, self.pitch)
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
        let white = self
            .dream
            .as_ref()
            .is_some_and(|d| d.theme == DreamTheme::WhiteDissolve);
        // The white dream fades out to more white, not to black.
        let dark = if white {
            [0.93, 0.93, 0.96]
        } else {
            tunnel::darkness(fog)
        };
        let clear = if tunnel { dark } else { fog };
        renderer.resize_if_needed(gl, drawable_size)?;
        renderer.set_trails(if self.mode == hud::Mode::Title {
            0.0
        } else {
            mood.trails * motion
        });
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
        let proj = Mat4::perspective_rh(fov.to_radians(), aspect, 0.1, 1000.0);
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
            if let Some(l) = loc("uBreathe") {
                gl.uniform_1_f32(Some(&l), mood.breathe * motion);
            }
            if let Some(l) = loc("uPulse") {
                gl.uniform_1_f32(Some(&l), pulse);
            }
            if let Some(l) = loc("uGlow") {
                gl.uniform_1_f32(Some(&l), mood.glow);
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
            for (_entity, (transform, mesh_renderer, uv, solid, lit, hero, water, body, halo)) in
                self.world
                    .query::<(
                        &Transform,
                        &MeshRenderer,
                        Option<&SurfaceUv>,
                        Option<&NoMelt>,
                        Option<&Lit>,
                        Option<&Hero>,
                        Option<&Translucent>,
                        Option<&PlayerBody>,
                        Option<&Halo>,
                    )>()
                    .iter()
            {
                // Through your own eyes you don't see your own body.
                if fp && (body.is_some() || halo.is_some()) {
                    continue;
                }
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
                tint_strength: gameplay::flash_tint(flash.strength, reduce_flashing),
            },
        );
        let install_fonts = !std::mem::replace(&mut self.fonts_installed, true);
        let (grain_strength, vignette_strength) = (self.settings.grain, self.settings.vignette);
        let card_art = &mut self.card_art;
        let pack = &self.pack;
        let grain = &mut self.grain;
        // Where the player is on screen (NDC -> 0..1), for the vignette centre.
        let clip = Mat4::perspective_rh(fov.to_radians(), aspect_for_overlay, 0.1, 1000.0)
            * Mat4::look_at_rh(eye, target, up)
            * player.extend(1.0);
        let player_uv = if fp {
            [0.5, 0.5]
        } else if clip.w > 0.0 {
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
                    tunnel::draw_overlay(
                        &p,
                        screen,
                        at,
                        strangeness,
                        time,
                        Some(tex),
                        grain_strength,
                        vignette_strength,
                    );
                }
                hud::draw(egui_ctx, &hud_view, card_art, pack);
            });
            ui.paint(drawable_size, output);
        }
        if std::mem::take(&mut self.export_requested) && self.mode == hud::Mode::Booklet {
            self.export_cards(ctx);
        }
        self.maybe_screenshot(ctx);
        Ok(())
    }
}

impl DreamscapeGame {
    /// DREAMSCAPE_SHOT=out.png: after DREAMSCAPE_SHOT_AT seconds (default 6)
    /// of the run, save the frame and quit. Used by tools/screenshots.sh.
    fn maybe_screenshot(&mut self, ctx: &mut Context) {
        let Ok(path) = crate::dev::var("DREAMSCAPE_SHOT") else {
            return;
        };
        let at: f32 = crate::dev::var("DREAMSCAPE_SHOT_AT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(6.0);
        if self.time < at || ctx.should_quit {
            return;
        }
        match Self::read_frame(ctx).map(|img| img.save(&path)) {
            Some(Ok(())) => log::info!("Screenshot saved to {path}"),
            other => log::error!("Screenshot {path} failed: {other:?}"),
        }
        ctx.should_quit = true;
    }

    /// Booklet [p]: each card on the page, cropped out of the frame, as a PNG.
    fn export_cards(&mut self, ctx: &mut Context) {
        let Some(frame) = Self::read_frame(ctx) else {
            return;
        };
        let dir = crate::dev::var_os("DREAMSCAPE_CARDS")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| paths::save_file("cards"));
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::error!("could not create {dir:?}: {e}");
            return;
        }
        let all: Vec<&cards::Card> = self.booklet.cards().collect();
        let page = &all[cards::page_range(all.len(), self.booklet_page)];
        let screen = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(frame.width() as f32, frame.height() as f32),
        );
        let mut saved = 0;
        for (card, r) in page.iter().zip(booklet_ui::card_rects(screen, page.len())) {
            let x = r.left().max(0.0) as u32;
            let y = r.top().max(0.0) as u32;
            let w = (r.width() as u32).min(frame.width().saturating_sub(x));
            let h = (r.height() as u32).min(frame.height().saturating_sub(y));
            if w == 0 || h == 0 {
                continue;
            }
            let crop = image::imageops::crop_imm(&frame, x, y, w, h).to_image();
            let slug: String = card
                .name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '_'
                    }
                })
                .collect();
            let path = dir.join(format!("card_{:03}_{slug}.png", card.number));
            match crop.save(&path) {
                Ok(()) => {
                    saved += 1;
                    log::info!("Card saved to {path:?}");
                }
                Err(e) => log::error!("could not save {path:?}: {e}"),
            }
        }
        if saved > 0 {
            self.flash.trigger([1.0, 1.0, 0.8], 0.4);
            self.sfx(Sound::Pick);
        }
    }

    /// The finished frame, top row first.
    fn read_frame(ctx: &mut Context) -> Option<image::RgbaImage> {
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
        image::RgbaImage::from_raw(w, h, flipped)
    }
}

fn main() -> anyhow::Result<()> {
    init_logging();
    crash::install();
    paths::migrate_repo_saves();
    let seed = run_seed();
    crash::SEED.store(seed, std::sync::atomic::Ordering::Relaxed);
    log::info!(
        "Dreamscape {} ({}), run seed = {seed} (replay with DREAMSCAPE_SEED={seed})",
        crash::VERSION,
        crash::GIT
    );
    if let Err(e) = App::run("Dreamscape", 1280, 720, DreamscapeGame::new(seed)) {
        crash::fatal(&e);
        return Err(e);
    }
    Ok(())
}

/// Headless smoke tests for the screens: build the real HUD snapshot, paint it
/// into an egui context, and make sure nothing panics and something is drawn.
#[cfg(test)]
mod screen_smoke {
    use super::*;
    use engine::ui::egui;

    /// A game on an empty booklet with its own scratch file (tests run in
    /// parallel and must never touch, or share, a real save).
    fn game(name: &str) -> DreamscapeGame {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let path = std::env::temp_dir().join(format!("dreamscape_smoke_{name}.ron"));
        let _ = std::fs::remove_file(&path);
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("DREAMSCAPE_BOOKLET", &path);
        let mut g = DreamscapeGame::new(7);
        g.booklet = cards::Booklet::default();
        g.booklet_path = path;
        g
    }

    fn shapes(g: &DreamscapeGame) -> usize {
        let v = g.hud_view();
        let ctx = egui::Context::default();
        hud::install_font(&ctx);
        let mut art = booklet_ui::CardArt::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(1280.0, 720.0),
            )),
            ..Default::default()
        };
        let out = ctx.run(input, |ctx| hud::draw(ctx, &v, &mut art, &g.pack));
        out.shapes.len()
    }

    #[test]
    fn the_lottery_screen_draws_idle_shuffling_and_revealed() {
        let mut g = game("lottery");
        g.booklet.stash.earn(500);
        g.open_store();
        g.store_tab = 1;
        assert!(shapes(&g) > 50, "idle deck and odds");
        g.lottery_pull();
        let outcome = g.lottery_outcome.clone().expect("a pull happened");
        assert_eq!(g.booklet.card_count(), 1);
        assert_eq!(g.booklet.stash.dust, 500 - lottery::PULL_COST);
        for age in [0.0, 0.5, lottery_ui::SHUFFLE, lottery_ui::SHUFFLE + 0.3, 5.0] {
            g.lottery_age = age;
            assert!(shapes(&g) > 50, "age {age}");
        }
        assert_eq!(g.hud_view().lottery.outcome.unwrap(), outcome);
    }

    #[test]
    fn a_broke_pull_says_so_and_costs_nothing() {
        let mut g = game("broke");
        g.open_store();
        g.store_tab = 1;
        g.lottery_pull();
        assert!(g.lottery_outcome.is_none());
        assert!(g.lottery_status.as_deref().unwrap().contains("more dust"));
        assert_eq!(g.booklet.card_count(), 0);
        assert!(shapes(&g) > 50);
    }

    #[test]
    fn the_store_shelves_title_and_summary_still_draw() {
        let mut g = game("shelves");
        g.open_store();
        assert!(shapes(&g) > 50, "store");
        for mode in [hud::Mode::Title, hud::Mode::Summary] {
            g.mode = mode;
            assert!(shapes(&g) > 20, "{mode:?}");
        }
    }

    #[test]
    fn pulled_cards_show_their_worth_in_the_booklet() {
        let mut g = game("worth");
        g.booklet.stash.earn(lottery::PULL_COST * 5);
        for _ in 0..5 {
            g.lottery_pull();
        }
        g.mode = hud::Mode::Booklet;
        g.booklet_page = 0;
        assert!(shapes(&g) > 100);
    }
}
