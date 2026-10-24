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

mod abilities2;
mod abilities3;
mod access;
mod attunement;
mod achievements;
mod balance;
mod booklet_ui;
mod boss;
mod cards;
mod codex_ui;
mod companion;
mod coop;
mod coop_game;
mod crash;
mod deck;
mod deck_ui;
mod dev;
mod dissolve;
mod dream;
mod ending;
mod events;
mod form;
mod enemy_ai;
mod eye;
mod eye_motion;
mod juice;
mod fortune;
mod lottery;
mod lottery_ui;
mod retention;
mod streaks;
mod worth;
mod fpv;
mod fusion;
mod gamble;
mod gameplay;
mod hand;
mod hud;
mod hunter;
mod combat;
mod edges;
mod lullaby;
mod newfoes;
mod tooth;
mod weeper;
mod foespawn;
mod leech;
mod mask;
mod moths;
mod loadout_ui;
mod nerve;
mod lore;
mod memories_ui;
mod merge_ui;
mod moth_shop;
mod music;
mod objective;
mod pack_art;
mod packs;
mod pad;
mod paths;
mod pixel_ui;
mod pixels;
mod powers;
mod progress;
mod records;
mod reveal_ui;
mod code_ui;
mod seedcode;
mod settings;
mod secrets;
mod settings_ui;
mod shop_ui;
mod sorting;
mod sounds;
mod specials;
mod steam;
mod store;
mod tickets;
mod tickets_ui;
mod summary_ui;
mod title_ui;
mod transition;
mod tunnel;
mod tutorial;
mod upgrade_ui;
mod upgrades;
mod pacts;
mod pressure;
mod shard_rules;
mod tactics;
mod theme_rules;

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
/// How far away a creature's sounds still carry (world units).
const SOUND_REACH: f32 = 26.0;
/// Seconds between pulses while one nerve is left.
const HEARTBEAT_EVERY: f32 = 1.3;
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

/// Distant scenery (clouds, islands) that shows through the tunnel-vision
/// dark as a dim silhouette (roadmap 5.3).
#[derive(Clone, Copy)]
struct Silhouette;

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
/// A shard that hops away when you come near (some shards, deeper down).
struct Skittish {
    hops_left: u8,
    /// Where the shard is (floor level), or is heading.
    home: Vec3,
    /// Where it is drawn right now, and the hop in the air: (from, to, 0..1).
    drawn: Vec3,
    flight: Option<(Vec3, Vec3, f32)>,
}

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
    // ENEMIES-B (added at the bottom)
    /// One moth of the dream's swarm (`moth_swarm`), by id.
    Moth(usize),
    Mask(mask::Mask),
    Leech(leech::Leech),
}

/// A swing's flash in flight: a fan of rays swept across the strike's arc.
struct Slash {
    /// Each ray's pieces (see `combat::SLASH_SEGMENTS`), and the flying sparks.
    rays: Vec<[Entity; 3]>,
    sparks: Vec<Entity>,
    age: f32,
    facing: Vec3,
    reach: f32,
    half_arc: f32,
    flip: bool,
}

/// What a swing asks for, until the next frame spawns it.
struct SlashStart {
    facing: Vec3,
    reach: f32,
    half_arc: f32,
    flip: bool,
}

// === abilities3 ===
/// The hand slot a settings action stands for.
fn ability_slot(a: Option<settings::Action>) -> Option<usize> {
    use settings::Action::*;
    match a? {
        Ability1 => Some(0),
        Ability2 => Some(1),
        Ability3 => Some(2),
        Ability4 => Some(3),
        Ability5 => Some(4),
        _ => None,
    }
}
// === end abilities3 ===

/// Something a strike, a dash or a shock can hit.
struct CombatTarget {
    entity: Entity,
    pos: Vec3,
    kind: combat::Kind,
    /// It has not noticed you.
    unaware: bool,
    /// Struck on armour (a Lullaby's front).
    guarded: bool,
    /// In a vulnerable recovery (a Tooth after its lunge).
    recovering: bool,
}

struct SpecialEnemy {
    entity: Entity,
    kind: EnemyKind,
    ai: SpecialAI,
}

// === enemies-a ===
enum NewAI {
    Tooth(tooth::Tooth),
    Weeper(weeper::Weeper),
    Lullaby(lullaby::Lullaby),
}

/// Tooth, Weeper or Lullaby, with the entities that dress it (tears, the
/// armour plate, the slam marker and ring), all despawned with it.
struct NewFoe {
    entity: Entity,
    kind: combat::Kind,
    ai: NewAI,
    base: Vec3,
    floor_y: f32,
    calm: Arc<GpuTexture>,
    tell: Arc<GpuTexture>,
    extras: Vec<Entity>,
    /// What it was doing last frame, so its sounds play once: (telling, lunging/slamming, tears).
    heard: (bool, bool, usize),
}

/// Segments of a Lullaby's slam ring.
const SLAM_SEGMENTS: usize = 32;
// === end enemies-a ===

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

/// A cracked wall in the current dream, and the door it hides once broken.
struct SecretWall {
    /// Floor-level centre of the wall cell.
    at: Vec3,
    /// Wall height, and the warm texture the door wears.
    height: f32,
    door_tex: Arc<GpuTexture>,
    /// The solid wall (cracked texture) and its glow, until it gives way.
    wall: Option<(Entity, Entity)>,
    /// The door that replaces it.
    door: Option<Entity>,
    /// You have stepped out of the doorway since you last used it (so
    /// leaving the shop doesn't throw you straight back in).
    armed: bool,
}

// === hazards ===
struct ShrinkFloor {
    /// Cells in the order they go (outer first; never the route).
    order: Vec<(i32, i32)>,
    /// Each still-standing cell's tile and resting position.
    tiles: HashMap<(i32, i32), (Entity, Vec3)>,
    gone: usize,
}
// === end hazards ===

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
    /// A friend's dream code: (seed, long run).
    Code(u64, bool),
}

/// Title menu entries.
#[derive(Clone, Copy, Debug, PartialEq)]
enum TitleItem {
    Continue,
    Start,
    Daily,
    DreamCode,
    RunLength,
    Form,
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

/// The title's note when the day's free fortune is unclaimed. // === fortune ===
const FORTUNE_LINE: &str = "a fortune is waiting in the Lucid Store";

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
    /// A between-dream event (roadmap 3.5), in place of the upgrade pick.
    Event(events::Event),
    /// The moth's room behind a cracked wall (cards in `moth_stock`).
    MothShop,
    /// === risk === the moth's coin: stakes, confirm, result.
    Gamble(GambleStage),
    /// === risk === the wager portal's deal.
    Wager,
}

/// === risk === Where the moth's coin screen is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GambleStage {
    Stakes,
    Confirm(u32),
    Result,
}

/// The icon on an event's choice card.
fn event_icon(plan: events::Plan) -> upgrades::Icon {
    use events::Plan::*;
    use upgrades::Icon;
    match plan {
        Nothing => Icon::Lid,
        DoorGamble | DigThrough => Icon::Magnet,
        Upgrade | CommonUpgrade => Icon::Gem,
        ChairDust | VoiceDust => Icon::Gem,
        VoiceCard => Icon::Shard,
        FreeReroll => Icon::Brain,
        MoonBlink | MoonSteady => Icon::Eye,
        // === events2 ===
        DustGamble | Handout | ChordGamble | SwapUpgrade => Icon::Gem,
        BorrowedPick | TwoRerolls => Icon::Brain,
        CurseForShard | ShardOnly | ShardThin => Icon::Shard,
        ShedCurse => Icon::Lid,
        TwistOnly | TwistDust | GildOnly | GildThin => Icon::Eye,
        // === end events2 ===
    }
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
    // === shards ===
    shard_kind: shard_rules::ShardKind,
    /// dream_age when a heavy shard was picked up.
    heavy_since: Option<f32>,
    /// Alert-radius multiplier currently applied to the pacers.
    heavy_applied: f32,
    /// Beam angle that points the guard sentry at its shard.
    guard_angle: Option<f32>,
    // === end shards ===
    /// Where the dreamer has been (the mimic walks it).
    trail: specials::Trail,
    mimic_awake_at: f32,
    jester_cooldown: f32,
    // === tactics ===
    tactics: tactics::State,
    // === end tactics ===
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
    /// DASH afterimages (entity, born at dream_age) and the spawn timer.
    dash_ghosts: Vec<(Entity, f32)>,
    ghost_timer: f32,
    /// STILLNESS: enemies turned blue, with the texture to give back.
    chilled: Vec<(Entity, Arc<GpuTexture>)>,
    /// Records the last wake broke (shown on the summary).
    new_records: Vec<(progress::NewRecord, u32)>,
    /// PHASE: the dreamer is drawn see-through.
    phased: bool,
    // === combat ===
    /// How many hits the dreamer can take (see `nerve.rs`).
    nerve: nerve::Nerve,
    /// The dreamer is drawn faint while invulnerable after a hit.
    hurt_ghost: bool,
    /// The dreamer's strike (swing timing) and a request to start one.
    strike: combat::Strike,
    strike_request: bool,
    /// Hit points of enemies that have been struck (made on first hit).
    foes: HashMap<Entity, combat::Foe>,
    /// Special enemies reeling from a strike: seconds left (pacers use `stunned`).
    staggered: HashMap<Entity, f32>,
    /// Enemies coming undone: (entity, age, full scale).
    unraveling: Vec<(Entity, f32, Vec3)>,
    /// Calm motes lying about: (entity, age).
    motes: Vec<(Entity, f32)>,
    /// Where calm motes should appear (spawned where there is a GL context).
    pending_motes: Vec<Vec3>,
    /// BLINK: until when your next strike is a sneak attack.
    backstab_until: f32,
    /// DASH: what this dash has already cut.
    dash_cut: Vec<Entity>,
    /// LEAP: you are in the air from one, and will shock on landing.
    leap_airborne: bool,
    /// ANCHOR: this use snapped you back (set for `combat_on_use`).
    anchor_snapped: bool,
    /// DREAM ANCHOR's last stand is once per dream.
    last_stand_used: bool,
    /// Strikes thrown this run (ECHO STRIKE), calm seconds (CALM MIND), the
    /// last moment you were airborne (SPRING), an air jump refunded (DOUBLE
    /// JUMP), and the pause between counters (COUNTER).
    strike_count: u32,
    calm_timer: f32,
    air_until: f32,
    air_refunded: bool,
    counter_cd: f32,
    /// THIN SKIN's lost nerve has been taken off this run's max. // === upgrades2 ===
    thin_skin_applied: bool,
    /// The combat edges the equipped cards teach.
    edges: Vec<edges::Edge>,
    /// FLOURISH's chain, SIPHON's tally, HEXMARK's marks, a LUNGE under way.
    flourish: edges::Chain,
    siphon: edges::Tally,
    marks: HashMap<Entity, f32>,
    lunge_left: f32,
    lunge_dir: Vec3,
    /// When each edge last did something (dream age), for the hand's pulse.
    edge_fired: Vec<(edges::Edge, f32)>,
    /// This blow is a FLOURISH (set while a strike resolves).
    flourish_now: bool,
    // === sounds ===
    /// Every sound asked for, in order (tests listen to this).
    #[cfg(test)]
    played: std::cell::RefCell<Vec<Sound>>,
    heart_timer: f32,
    was_grounded: bool,
    air_vy: f32,
    /// Which abilities were ready last frame (for the recharge chime).
    was_ready: Vec<bool>,
    // === end sounds ===
    /// The swing's flash: (entity, age); and one waiting to be spawned.
    slashes: Vec<Slash>,
    slash_pending: Option<SlashStart>,
    // === end combat ===
    // === enemies-a ===
    newfoes: Vec<NewFoe>,
    /// Wall boxes tears die on.
    foe_walls: Vec<weeper::Wall>,
    // === end enemies-a ===
    // === combat === ENEMIES-B
    /// The dream's moth group, when it has one.
    moth_swarm: Option<moths::Swarm>,
    /// Textures that change with an enemy's state (dim/bright moth, mask tells).
    b_tex: HashMap<&'static str, Arc<GpuTexture>>,
    /// The dreamer left the ground this frame (ground or air jump).
    jump_event: bool,
    // === end combat ===
    /// Each loadout ability and the dream colours of the card that taught it.
    ability_tints: Vec<(Ability, [u8; 3], [u8; 3])>,
    /// An ability just fired: burst here, in these colours (spawned next update).
    burst_pending: Option<(Vec3, [u8; 3], [u8; 3])>,
    /// Burst cubes: (entity, dream age at birth, index in the ring, centre).
    bursts: Vec<(Entity, f32, usize, Vec3)>,
    /// This dream's music, being composed on a worker thread.
    music_job: Option<std::sync::mpsc::Receiver<(Vec<f32>, Vec<f32>)>>,
    /// Adaptive music: how much of the tension layer is playing (0..=1).
    music_intensity: f32,
    /// Length of the current music loop, seconds.
    music_loop: f32,
    /// DECOY: where it stands and its marker.
    decoy: Option<(Vec3, Entity)>,
    /// ANCHOR: where it was dropped, its marker, and how long it has waited.
    anchor: Option<(Vec3, Entity, f32)>,
    /// The last ability used and when (dream seconds), for combos.
    last_ability: Option<(Ability, f32)>,
    /// When each ability was last used (dream age), for the hand's pop.
    ability_used: Vec<(Ability, f32)>,
    /// When each passive last did something (dream age), for the hand's pulse.
    passive_fired: Vec<(Upgrade, f32)>,
    /// Names of the cards that hold the abilities, for the hand.
    hand_cards: Vec<(Ability, cards::Card)>,
    // === deck ===
    /// The run's deck (`None` = the fixed loadout: co-op, autopilot, resumed runs).
    deck: Option<deck::Deck>,
    /// The cards dealt first: they give the run's passives, edges and dreams.
    deck_opening: Vec<cards::Card>,
    /// Animation clock for the deck (seconds, runs while playing).
    deck_clock: f32,
    /// Per hand slot: `deck_clock` when its card was drawn (future = not yet).
    deck_drawn_at: [f32; upgrades::HAND_SIZE],
    /// `deck_clock` when a card was last shuffled in.
    deck_riffle_at: f32,
    /// Cards flicking out: (snapshot, slot, `deck_clock` when used).
    deck_leaving: Vec<(hand::HandCard, usize, f32)>,
    // === end deck ===
    // === toolcards ===
    /// Tool card ids bought this run (kept for the save; the deck holds the live ones).
    tool_cards: Vec<u32>,
    // === end toolcards ===
    /// SLAM: the highest point of the hop, and whether LEAP made it a meteor.
    slam_apex: f32,
    slam_meteor: bool,
    /// High caches (floor spot, glow, beam) still to be picked up.
    caches: Vec<(Vec3, Entity, Entity)>,
    /// This dream's cracked wall, if it has one.
    secret: Option<SecretWall>,
    /// The crack's hint has been shown this launch.
    secret_hint_given: bool,
    /// What the moth still has on the shelf in this dream.
    moth_stock: Option<moth_shop::Shelf>,
    skittish: Option<Skittish>,
    /// Height of the ground you last stood on (for the climbing sound).
    ground_y: f32,
    /// Enemies reeling from a SLAM or GUST: seconds left.
    stunned: HashMap<Entity, f32>,
    /// SLAM: 0 = not slamming, 1 = rising from the hop, 2 = smashing down.
    slam: u8,
    /// STEP: where the next steps go (spawned with the GL context in hand), and the live ones.
    step_pending: Option<Vec<Vec3>>,
    // === abilities2 ===
    surge: abilities2::Surge,
    /// Speed multiplier of the dash in flight (a SURGE doubles it).
    dash_boost: f32,
    /// HASTE: the zone and the disc that draws it.
    haste: Option<(abilities2::HasteField, Option<Entity>)>,
    /// PULSE: pillars standing over what hides behind walls.
    pulse_marks: Vec<Entity>,
    // === end abilities2 ===
    // === abilities3 ===
    /// ECHO: the ability it would repeat.
    echo_last: Option<Ability>,
    /// WAGER: rolls this dream, wagers since a jackpot, the line on the HUD.
    wager_n: u32,
    wager_dry: u32,
    wager_line: Option<(String, f32)>,
    /// Extra duration for an ability fired from a borrowed state (a jackpot).
    borrow_stretch: f32,
    /// OVERLOAD: the charge in progress, the ring that shows it, and the
    /// charge being released right now.
    overload: Option<abilities3::Charge>,
    overload_ring: Option<Entity>,
    overload_fire: Option<f32>,
    /// Which ability keys are down right now.
    ability_held: [bool; upgrades::HAND_SIZE],
    bargain: abilities3::Bargain,
    snares: abilities3::Snares<Option<Entity>>,
    // === end abilities3 ===
    steps: Vec<Entity>,
    /// REWIND: where you've stood lately.
    rewind_trail: upgrades::Trail,
    echo_timer: f32,
    /// ECHO twist: the ghost of the dreamer from `ECHO_DELAY` seconds ago.
    echo_ghost: Option<Entity>,
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
    // === fortune ===
    /// What the last ticket scratch revealed, and how long ago.
    ticket_revealed: Option<tickets_ui::Revealed>,
    ticket_age: f32,
    /// The first of the two presses to scratch a ticket has been made.
    ticket_confirm: bool,
    ticket_status: Option<String>,
    /// Clean-dream tickets earned in the current run (capped).
    tickets_this_run: u32,
    /// Nerve was lost in the dream being played.
    nerve_lost_this_dream: bool,
    // === end fortune ===
    /// What the last pressed run did for its cards (shown on the pack screen).
    last_report: Option<retention::RunReport>,
    /// White Dissolve: the path state and its tiles (entity, full size).
    dissolve: Option<(dissolve::Dissolve, Vec<(Entity, Vec3)>)>,
    /// Special kinds met so far this run (each gets a hint the first time).
    seen_kinds: Vec<EnemyKind>,
    /// Shown under the title card.
    dream_hint: String,
    // === hazards ===
    /// This dream's theme rule (depth 3+, normal dreams).
    theme_rule: Option<theme_rules::RuleState>,
    /// Fog pocket discs and where each rests (they drift around it).
    fog_discs: Vec<(Entity, Vec3)>,
    /// Shrinking floor: order the cells go in, and their tiles.
    shrink: Option<ShrinkFloor>,
    // === end hazards ===
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
    /// Soft run-feel juice: time dip, shake, pickup chain, door hum.
    juice: juice::Juice,
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
    /// Who the dreamer is this run (roadmap 4.4).
    active_form: form::Form,
    /// A shared dream, if this is one.
    coop: Option<coop_game::CoopState>,
    /// Co-op: the run to start next frame (seed, long, dreamers).
    pending_coop_start: Option<(u64, bool, usize)>,
    /// Times caught in the current dream (untouched nightmares).
    caught_this_dream: u32,
    // === ascension ===
    /// Where you were caught this dream / the spot the last dream remembers (rule 12).
    asc_caught_here: Option<Vec3>,
    asc_remembered: Option<Vec3>,
    /// The eye's clock already ran out this dream (rule 15).
    asc_eye_failed: bool,
    // === end ascension ===
    /// Seconds spent holding a stalker still with your gaze.
    stare: f32,
    /// A just-earned achievement's name, and how long it has shown.
    achievement_toast: Option<(String, f32)>,
    /// This run's portal leads to the bottom (the ending plays on arrival).
    to_bottom: bool,
    /// The eye has said it is waiting to be asked (once per launch).
    ask_hinted: bool,
    /// The first-time guided dreams, while they run.
    prologue: Option<tutorial::Prologue>,
    /// The eye's line is a tutorial prompt: stays up until the step changes.
    eye_sticky: bool,
    /// Lines the eye says next, one after another.
    eye_queue: Vec<String>,
    dream_name: String,
    dream_whisper: String,
    run_log: Vec<cards::DreamRecord>,
    /// Dev: `DREAMSCAPE_OPEN` has already opened its screen.
    dev_opened: bool,
    /// Dev: `DREAMSCAPE_FIRE` has already fired its ability.
    dev_fired: bool,
    booklet: cards::Booklet,
    booklet_path: std::path::PathBuf,
    booklet_page: usize,
    /// Mode to return to when the booklet closes.
    booklet_return: hud::Mode,
    journal_status: Option<String>,
    card_art: booklet_ui::CardArt,
    pack: reveal_ui::PackView,
    /// Every card the run's opened packs gave so far; pressed together at the end.
    haul: Vec<cards::Recalled>,
    /// The sealed packs on offer after waking (empty outside `Mode::PackPick`).
    pick: reveal_ui::PackPickView,
    packs_art: pack_art::PackArt,
    /// Drawable size last frame, for mouse hit-testing on the pick screen.
    screen_size: (f32, f32),
    shop_shelf: usize,
    shop_col: usize,
    /// Title menu selection.
    title_sel: usize,
    /// Digits typed into the dream-code box, and why the last try failed.
    code_digits: String,
    code_error: String,
    /// The wake summary's code was copied to the clipboard.
    code_copied: bool,
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
    // === pressure ===
    pressure: pressure::Pressure,
    pressure_phase: pressure::Phase,
    /// Run dust when this dream began (a collapse halves what came after).
    pressure_dust_start: u32,
    /// This run ended in a collapse.
    pressure_collapsed: bool,
    // === end pressure ===
    /// Dust earned this run on top of the pack (HURRIED, GILDED).
    bonus_dust: u32,
    // === risk ===
    gamble: gamble::RunState,
    /// Gambles at the moth this visit.
    gamble_visit: u32,
    /// Run dust when this dream began (what a wager doubles is the gain).
    wager_dust_start: u32,
    /// Where this dream's wager portal stands, and whether it can be touched.
    wager_portal: Option<(Vec3, bool)>,
    /// Entering the portal now was agreed on the deal screen.
    wager_enter: bool,
    /// The pick being opened is the wagered dream's extra one.
    wager_pick: bool,
    // === end risk ===
    // === pacts ===
    /// Pacts taken this run, and the one the open pick offers.
    pacts: Vec<pacts::Pact>,
    pact_offer: Option<pacts::Pact>,
    // === end pacts ===
    /// Highest difficulty survived this run (scales the dust you wake with).
    peak_difficulty: f32,
    /// Offer an upgrade once the next dream has finished reforming.
    offer_upgrade: bool,
    /// The last between-dream event (so it never repeats), and whether the
    /// dream just before this one had one (so they never come back to back).
    last_event: Option<events::Id>,
    event_just_shown: bool,
    /// Set by events: the next dream is gilded / gets this extra twist.
    gild_next: bool,
    twist_next: Option<Variant>,
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
    ability_requests: [bool; upgrades::HAND_SIZE],
}

/// Notes that a passive just did its work (only if the run holds it), so its
/// card in the hand can pulse.
fn fire_passive(fired: &mut Vec<(Upgrade, f32)>, run: &RunUpgrades, age: f32, u: Upgrade) {
    if run.count(u) == 0 {
        return;
    }
    fired.retain(|f| f.0 != u);
    fired.push((u, age));
}

/// What the mouse did on a menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pointer {
    Hover,
    Click,
    /// Right button: step back (clear a pick, close).
    Back,
}

/// Screens whose choices can be clicked.
fn menu_pointer_mode(mode: hud::Mode) -> bool {
    matches!(
        mode,
        hud::Mode::Title | hud::Mode::Choice | hud::Mode::Loadout | hud::Mode::Merge
    )
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
            // === shards ===
            shard_kind: shard_rules::ShardKind::Plain,
            heavy_since: None,
            heavy_applied: 1.0,
            guard_angle: None,
            // === end shards ===
            trail: specials::Trail::default(),
            mimic_awake_at: 0.0,
            jester_cooldown: 0.0,
            // === tactics ===
            tactics: tactics::State::default(),
            // === end tactics ===
            crumbles: Vec::new(),
            fog: Vec::new(),
            veins: Vec::new(),
            gates: Vec::new(),
            shifters: Vec::new(),
            beat_tiles: Vec::new(),
            bpm: 0.0,
            loops: 0,
            echoes: Vec::new(),
            dash_ghosts: Vec::new(),
            ghost_timer: 0.0,
            chilled: Vec::new(),
            new_records: Vec::new(),
            phased: false,
            nerve: nerve::Nerve::default(),
            hurt_ghost: false,
            strike: combat::Strike::default(),
            strike_request: false,
            foes: HashMap::new(),
            staggered: HashMap::new(),
            unraveling: Vec::new(),
            motes: Vec::new(),
            pending_motes: Vec::new(),
            backstab_until: 0.0,
            dash_cut: Vec::new(),
            leap_airborne: false,
            anchor_snapped: false,
            last_stand_used: false,
            strike_count: 0,
            calm_timer: 0.0,
            air_until: 0.0,
            air_refunded: false,
            counter_cd: 0.0,
            thin_skin_applied: false,
            edges: Vec::new(),
            flourish: edges::Chain::default(),
            siphon: edges::Tally::default(),
            marks: HashMap::new(),
            lunge_left: 0.0,
            lunge_dir: Vec3::ZERO,
            edge_fired: Vec::new(),
            flourish_now: false,
            #[cfg(test)]
            played: std::cell::RefCell::new(Vec::new()),
            heart_timer: 0.0,
            was_grounded: true,
            air_vy: 0.0,
            was_ready: Vec::new(),
            slashes: Vec::new(),
            slash_pending: None,
            newfoes: Vec::new(), // enemies-a
            foe_walls: Vec::new(), // enemies-a
            moth_swarm: None,
            b_tex: HashMap::new(),
            jump_event: false,
            ability_tints: Vec::new(),
            burst_pending: None,
            bursts: Vec::new(),
            music_job: None,
            music_intensity: 0.0,
            music_loop: 1.0,
            decoy: None,
            anchor: None,
            last_ability: None,
            ability_used: Vec::new(),
            passive_fired: Vec::new(),
            hand_cards: Vec::new(),
            deck: None,
            deck_opening: Vec::new(),
            deck_clock: 0.0,
            deck_drawn_at: [f32::MIN; upgrades::HAND_SIZE],
            deck_riffle_at: hand::NEVER,
            deck_leaving: Vec::new(),
            tool_cards: Vec::new(), // === toolcards ===
            slam_apex: 0.0,
            slam_meteor: false,
            caches: Vec::new(),
            secret: None,
            secret_hint_given: false,
            moth_stock: None,
            skittish: None,
            ground_y: f32::MIN,
            stunned: HashMap::new(),
            slam: 0,
            step_pending: None,
            // === abilities2 ===
            surge: abilities2::Surge::default(),
            dash_boost: 1.0,
            haste: None,
            pulse_marks: Vec::new(),
            // === end abilities2 ===
            // === abilities3 ===
            echo_last: None,
            wager_n: 0,
            wager_dry: 0,
            wager_line: None,
            borrow_stretch: 1.0,
            overload: None,
            overload_ring: None,
            overload_fire: None,
            ability_held: [false; upgrades::HAND_SIZE],
            bargain: abilities3::Bargain::default(),
            snares: abilities3::Snares::new(),
            // === end abilities3 ===
            steps: Vec::new(),
            rewind_trail: upgrades::Trail::default(),
            echo_timer: 0.0,
            echo_ghost: None,
            player_tex: None,
            watchers: Vec::new(),
            dissolve: None,
            seen_kinds: Vec::new(),
            dream_hint: String::new(),
            theme_rule: None,
            fog_discs: Vec::new(), // hazards
            shrink: None,          // hazards
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
            juice: juice::Juice::new(),
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
            active_form: form::Form::Crystal,
            coop: None,
            pending_coop_start: None,
            caught_this_dream: 0,
            asc_caught_here: None,
            asc_remembered: None,
            asc_eye_failed: false,
            stare: 0.0,
            achievement_toast: None,
            to_bottom: false,
            ask_hinted: false,
            prologue: None,
            eye_sticky: false,
            eye_queue: Vec::new(),
            dream_name: String::new(),
            dream_whisper: String::new(),
            run_log: Vec::new(),
            dev_opened: false,
            dev_fired: false,
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
            haul: Vec::new(),
            pick: reveal_ui::PackPickView::default(),
            packs_art: pack_art::PackArt::default(),
            screen_size: (1280.0, 720.0),
            shop_shelf: 0,
            shop_col: 0,
            title_sel: 0,
            code_digits: String::new(),
            code_error: String::new(),
            code_copied: false,
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
            ticket_revealed: None,
            ticket_age: 0.0,
            ticket_confirm: false,
            ticket_status: None,
            tickets_this_run: 0,
            nerve_lost_this_dream: false,
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
            // === pressure ===
            pressure: pressure::Pressure::off(),
            pressure_phase: pressure::Phase::Calm,
            pressure_dust_start: 0,
            pressure_collapsed: false,
            // === end pressure ===
            bonus_dust: 0,
            gamble: gamble::RunState::default(),
            gamble_visit: 0,
            wager_dust_start: 0,
            wager_portal: None,
            wager_enter: false,
            wager_pick: false,
            pacts: Vec::new(),
            pact_offer: None,
            peak_difficulty: 1.0,
            offer_upgrade: false,
            last_event: None,
            event_just_shown: false,
            gild_next: false,
            twist_next: None,
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
            ability_requests: [false; upgrades::HAND_SIZE],
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
            enemies: gameplay::pressured_enemy_count(self.difficulty())
                * self.twists.enemy_count()
                * gamble::enemy_mult(self.gamble.wager_active), // === risk ===
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
            ^ if std::mem::take(&mut self.wager_pick) { 0x57A6_0000 } else { 0 } // === risk ===
            ^ (self.run.rerolls_used as u64).wrapping_mul(0x9E37_79B9);
        let roll = upgrades::Roll {
            depth: self.director.depth,
            boss: self.boss_reward,
        };
        let mut options = upgrades::roll_choices(seed, &self.run, roll);
        if options.is_empty() {
            return;
        }
        // === pacts ===
        let pfx = pacts::effects(&self.pacts);
        options.truncate(options.len().saturating_sub(pfx.fewer_cards).max(2).min(options.len()));
        self.pact_offer = None;
        if !self.boss_reward && self.prologue.is_none() && self.coop.is_none() && !self.autopilot {
            self.pact_offer = pacts::roll_pact(
                self.director.dream_seed() ^ self.director.depth as u64,
                self.director.depth,
                &self.pacts,
            );
            if self.pact_offer.is_some() && options.len() >= 3 {
                options.pop();
            } else {
                self.pact_offer = None;
            }
        }
        // === end pacts ===
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
            cards: options
                .iter()
                .map(|&u| card(u))
                // === pacts ===
                .chain(self.pact_offer.map(|p| upgrade_ui::ChoiceCard {
                    name: p.name().into(),
                    desc: p.desc().into(),
                    icon: upgrades::Icon::Lid,
                    color: [255, 90, 100],
                    frame: [255, 60, 80],
                    tag: format!("pact · +{:.2} dust", p.bonus()),
                }))
                // === end pacts ===
                .collect(),
            selected: 0,
            age: 0.0,
            hint: if pfx.no_reroll_skip {
                "[a/d] choose   [enter] take it".into()
            } else {
                format!(
                    "[a/d] choose   [enter] take it   [r] reroll ({} left)   [x] skip (+{} dust)",
                    self.run.rerolls_left(),
                    if pfx.skip_pays_nothing { 0 } else { self.run.skip_dust() }
                )
            },
        };
        self.choice = Some(ChoiceKind::Upgrade(options));
        self.input = PlayerInputState::default();
        self.mode = hud::Mode::Choice;
    }

    /// Between-dream events (roadmap 3.5): about one dream in six offers a
    /// small choice in place of the upgrade pick. Returns true if one opened.
    fn open_event_choice(&mut self) -> bool {
        let seed = self.director.dream_seed();
        let allowed = self.prologue.is_none() && self.coop.is_none();
        let due = allowed
            && events::due(
                seed,
                self.director.depth,
                self.event_just_shown,
                self.boss_reward,
            );
        if !due {
            self.event_just_shown = false;
            return false;
        }
        let id = events::pick(seed, self.last_event);
        self.show_event(id);
        true
    }

    /// The choice screen for event `id`.
    fn show_event(&mut self, id: events::Id) {
        let event = events::event(id);
        let dust = self.event_dust();
        let cards = event
            .choices
            .iter()
            .map(|c| {
                let can = events::available(c, &self.run, dust);
                let tag = if can {
                    if c.cost > 0 {
                        format!("costs {} dust", c.cost)
                    } else {
                        String::new()
                    }
                } else if c.cost > dust {
                    format!("needs {} dust", c.cost)
                } else {
                    "nothing to give".into()
                };
                upgrade_ui::ChoiceCard {
                    name: c.text.into(),
                    desc: c.detail.into(),
                    icon: event_icon(c.plan),
                    color: if can { [200, 190, 255] } else { [110, 110, 130] },
                    frame: if can { [150, 140, 220] } else { [80, 80, 100] },
                    tag,
                }
            })
            .collect();
        self.choice_view = upgrade_ui::ChoiceView {
            title: event.title.into(),
            subtitle: event.prompt.into(),
            cards,
            selected: 0,
            age: 0.0,
            hint: "[a/d] choose   [enter] decide".into(),
        };
        log::info!("Event: {:?} at depth {}", id, self.director.depth);
        self.last_event = Some(id);
        self.event_just_shown = true;
        self.choice = Some(ChoiceKind::Event(event));
        self.input = PlayerInputState::default();
        self.mode = hud::Mode::Choice;
    }

    /// Dust an event can ask for: this run's earnings, then the banked stash.
    fn event_dust(&self) -> u32 {
        self.bonus_dust + self.booklet.stash.dust
    }

    /// Pays `n` dust, run earnings first, then the stash.
    fn spend_event_dust(&mut self, n: u32) {
        let from_run = n.min(self.bonus_dust);
        self.bonus_dust -= from_run;
        let from_stash = (n - from_run).min(self.booklet.stash.dust);
        if from_stash > 0 {
            self.booklet.stash.dust -= from_stash;
            self.persist_booklet();
        }
    }

    /// Settles `index` of `event`. False if it can't be made (the screen stays).
    fn choose_event(&mut self, event: &events::Event, index: usize) -> bool {
        let seed = self.director.dream_seed();
        let Some(res) = events::resolve(
            event,
            index,
            seed,
            &self.run,
            self.director.depth,
            self.director.next,
            self.event_dust(),
        ) else {
            return false;
        };
        let mut open_pick = false;
        for a in &res.applied {
            events::apply_to_run(&mut self.run, a);
            match *a {
                events::Applied::Dust(n) if n >= 0 => self.bonus_dust += n as u32,
                events::Applied::Dust(n) => self.spend_event_dust(n.unsigned_abs()),
                events::Applied::BankShard => {
                    self.director.lucidity += 1;
                    self.shards_this_run += 1;
                }
                events::Applied::GildNext => self.gild_next = true,
                events::Applied::TwistNext(v) => self.twist_next = Some(v),
                events::Applied::OpenPick => open_pick = true,
                events::Applied::Take(_)
                | events::Applied::Lose(_)
                | events::Applied::FreeReroll
                | events::Applied::ThinNextPick => {}
            }
        }
        self.director.shard_bonus = self.shard_bonus();
        log::info!("Event choice {index}: {} ({:?})", res.line, res.applied);
        self.achievement_toast = Some((res.line, 0.0));
        self.sfx(Sound::Pick);
        if open_pick {
            self.open_upgrade_choice();
        }
        true
    }

    /// The moon and the voice reach into the dream after the event.
    fn apply_event_twists(&mut self) {
        if self.director.nightmare || self.prologue.is_some() {
            return;
        }
        if std::mem::take(&mut self.gild_next) && !self.twists.has(Variant::Gilded) {
            self.twists.0.push(Variant::Gilded);
        }
        if let Some(v) = self.twist_next.take() {
            if !self.twists.has(v) && !self.twists.0.iter().any(|&o| v.clashes(o)) {
                self.twists.0.push(v);
            }
        }
    }

    // === pacts ===
    /// Signs a pact: its drawback holds for the rest of the run.
    fn take_pact(&mut self, p: pacts::Pact) {
        self.pacts.push(p);
        self.pact_offer = None;
        self.run.thin_next_pick = false;
        self.director.shards_to_wake += p.effects().extra_shards;
        if p.effects().owed_dust > 0 {
            match pacts::owed_payment(self.event_dust()) {
                pacts::OwedPayment::Dust(n) => self.spend_event_dust(n),
                pacts::OwedPayment::LoseCard => {
                    if let Some(i) = self.run.taken.iter().rposition(|u| !u.is_ability()) {
                        self.run.taken.remove(i);
                    }
                }
            }
        }
        self.director.shard_bonus = self.shard_bonus();
        self.achievement_toast = Some((format!("pact: {}", p.name()), 0.0));
        self.sfx(Sound::Pick);
        log::info!("Pact taken: {p:?} (run: {:?})", self.pacts);
    }
    // === end pacts ===

    fn reroll_choice(&mut self) {
        if !matches!(self.choice, Some(ChoiceKind::Upgrade(_)))
            || self.run.rerolls_left() == 0
            || pacts::effects(&self.pacts).no_reroll_skip // === pacts ===
        {
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

    /// The ending is yours to ask for: every note found, and nothing in the
    /// way (a nightmare, a melt, a card pick, the entrance).
    fn can_ask_eye(&self) -> bool {
        self.mode == hud::Mode::Playing
            && self.ending_due()
            && self.choice.is_none()
            && !self.director.nightmare
            && self.director.theme != DreamTheme::Awakening
            && !self.transition.active()
            && self.entrance.done
    }

    /// [enter] while the eye is waiting: go down to the bottom.
    fn ask_eye(&mut self) {
        if !self.can_ask_eye() {
            return;
        }
        log::info!("You asked the eye; the way down opens");
        self.to_bottom = true;
        self.eye_line = None;
        self.flash.trigger([0.9, 0.85, 1.0], 0.8);
        self.sfx(Sound::WakeDoor);
        self.begin_melt(transition::Pending::Wake);
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

    // === shop ===
    /// After a beaten nightmare's reward (and on every `SHOP_EVERY`th depth)
    /// the moth opens her shop. Merging lives in the store's `[m]` now.
    /// Called once per pick, before `boss_reward` is cleared.
    fn moth_after_nightmare(&mut self) {
        // === risk === the wagered dream's extra pick comes first (rare or better)
        if self.gamble.pick_owed && self.risk_allowed() {
            self.gamble.pick_owed = false;
            self.boss_reward = true;
            self.wager_pick = true;
            self.open_upgrade_choice();
            return;
        }
        // === end risk ===
        if self.autopilot
            || self.prologue.is_some()
            || self.coop.is_some()
            || !moth_shop::due(
                self.boss_reward,
                self.director.depth,
                self.director.sanctuary(),
            )
        {
            return;
        }
        self.open_moth_shop_after_pick();
    }

    fn moth_terms(&self) -> moth_shop::Terms {
        let mut t = moth_shop::Terms::from_perks(&store::PerkFx::of(&self.perks));
        t.pct_off += self.run.moth_pct_off(); // === upgrades2 ===
        t
    }

    fn perk_fx(&self) -> store::PerkFx {
        store::PerkFx::of(&self.perks)
    }
    // === end shop ===

    fn skip_choice(&mut self) {
        if !matches!(self.choice, Some(ChoiceKind::Upgrade(_)))
            || pacts::effects(&self.pacts).no_reroll_skip // === pacts ===
        {
            return;
        }
        self.choice = None;
        self.run.thin_next_pick = false;
        self.mode = hud::Mode::Playing;
        self.moth_after_nightmare();
        self.boss_reward = false;
        if !pacts::effects(&self.pacts).skip_pays_nothing {
            self.bonus_dust += self.run.skip_dust() + self.perk_fx().skip_bonus; // === pacts ===
        }
        self.stats.skipped += 1;
        self.coop_ready();
        log::info!("Skipped the pick (+{} dust)", self.run.skip_dust());
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
                // === pacts ===
                if let (true, Some(p)) = (index == options.len(), self.pact_offer) {
                    self.take_pact(p);
                    self.moth_after_nightmare();
                    self.boss_reward = false;
                    self.coop_ready();
                    return;
                }
                // === end pacts ===
                let Some(&u) = options.get(index) else {
                    self.choice = Some(ChoiceKind::Upgrade(options));
                    self.mode = hud::Mode::Choice;
                    return;
                };
                let before = self.run.synergies();
                self.run.thin_next_pick = false;
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
            ChoiceKind::MothShop => self.buy_from_moth(index),
            ChoiceKind::Gamble(stage) => self.choose_gamble(stage, index), // === risk ===
            ChoiceKind::Wager => self.choose_wager(index), // === risk ===
            ChoiceKind::Event(event) => {
                if !self.choose_event(&event, index) {
                    self.choice = Some(ChoiceKind::Event(event));
                    self.mode = hud::Mode::Choice;
                }
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
            let music = music::render(&st, &notes, strangeness, music::BARS);
            let layer = music::render(&st, &music::tension_score(&st, music::BARS), 0.0, music::BARS);
            (music, layer)
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
        let Ok((samples, layer)) = rx.try_recv() else {
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
            audio.play_music_layer_samples(layer, sounds::RATE, offset);
            self.music_intensity = 0.0;
        }
    }

    fn mesh(&self, shape: Shape) -> anyhow::Result<Arc<GpuMesh>> {
        self.meshes
            .get(&shape)
            .cloned()
            .context("mesh not uploaded")
    }

    /// The equipped cards that hold abilities, with how long ago each was used.
    // === deck ===
    /// The card in hand slot `k` (the deck's, or for the fixed loadout the
    /// card that teaches `ability`).
    fn card_for_slot(&self, k: usize, ability: Ability) -> Option<&(Ability, cards::Card)> {
        match &self.deck {
            Some(d) => {
                let number = d.hand().get(k).copied().flatten()?;
                self.hand_cards.iter().find(|n| n.1.number == number)
            }
            None => self.hand_cards.iter().find(|n| n.0 == ability),
        }
    }

    /// The deck's piles for the HUD.
    fn hand_pile(&self) -> hand::HandPile {
        match &self.deck {
            Some(d) => hand::HandPile {
                shown: true,
                draw_count: d.draw_len(),
                riffle_since: self.deck_clock - self.deck_riffle_at,
                back: Some(self.booklet.stash.card_back).filter(|b| *b != store::CardBack::Dusk), // === smallfixes ===
                leaving: self
                    .deck_leaving
                    .iter()
                    .map(|(card, slot, at)| hand::Leaving {
                        card: card.clone(),
                        slot: *slot,
                        since: self.deck_clock - at,
                    })
                    .collect(),
            },
            None => hand::HandPile::default(),
        }
    }

    /// Deck runs: advances the deck, and turns its draws into hand slots.
    fn update_deck(&mut self, dt: f32) {
        let perks = self.run.deck_perks(); // === upgrades2 ===
        let Some(deck) = self.deck.as_mut() else {
            return;
        };
        deck.perks = perks; // === upgrades2 ===
        self.deck_clock += dt;
        deck.tick(dt);
        let events = deck.events();
        for ev in events {
            match ev {
                deck::DeckEvent::Drawn { slot, card } => {
                    let cd = self.deck.as_ref().map_or(0.0, |d| d.cooldown(card));
                    if let Some(n) = self.hand_cards.iter().find(|n| n.1.number == card) {
                        let ability = n.0;
                        self.run.deck_draw(slot, ability, cd);
                        self.deck_drawn_at[slot] = self.deck_clock;
                    } else if let Some(ability) = deck::tool_ability(card) {
                        // === toolcards === a bought tool card has no booklet card
                        self.run.deck_draw(slot, ability, cd);
                        self.deck_drawn_at[slot] = self.deck_clock;
                        // === end toolcards ===
                    }
                }
                deck::DeckEvent::Shuffled { .. } => self.deck_riffle_at = self.deck_clock,
                deck::DeckEvent::Played { .. } => {}
            }
        }
        let clock = self.deck_clock;
        self.deck_leaving.retain(|l| clock - l.2 < hand::LEAVE_TOTAL);
    }

    /// An ability just fired from `slot`: its card is discarded (the slot
    /// waits to draw) and flicks out on the HUD.
    fn deck_on_use(&mut self, slot: usize) {
        if self.deck.is_none() || slot >= self.run.deck_slots {
            return;
        }
        if let Some(snap) = self.hand_view().get(slot).cloned() {
            self.deck_leaving.push((snap, slot, self.deck_clock));
        }
        let cooldown = self.run.abilities[slot].cooldown;
        if let Some(d) = self.deck.as_mut() {
            d.play(slot, cooldown);
        }
        self.run.abilities[slot].spent = true;
    }

    /// Which booklet cards this run's deck is built from, if it uses one.
    /// `DREAMSCAPE_DECK=1,2,3` forces a deck (even on the autopilot).
    fn deck_wanted(&self) -> Option<Vec<u32>> {
        if self.prologue.is_some() || self.coop.is_some() {
            return None;
        }
        if let Ok(list) = crate::dev::var("DREAMSCAPE_DECK") {
            let forced: Vec<u32> = list.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            let forced = deck::clean(&forced, |n| self.booklet.card(n).is_some());
            if !forced.is_empty() {
                return Some(forced);
            }
        }
        if self.autopilot {
            return None;
        }
        let nums = self.booklet.deck_numbers();
        (!nums.is_empty()).then_some(nums)
    }

    /// Starts the deck for a run and returns the opening hand. `None` for
    /// the fixed loadout.
    fn start_deck(&mut self) -> Option<Vec<cards::Card>> {
        self.deck = None;
        self.tool_cards.clear(); // === toolcards === bought per run, never carried over
        self.deck_opening.clear();
        self.deck_leaving.clear();
        self.deck_riffle_at = hand::NEVER;
        let nums = self.deck_wanted()?;
        // Ascension rule 13 takes a slot from the hand, as it did from the loadout.
        let cut = self.booklet.lore.slots().saturating_sub(self.asc_slots());
        let hand_size = upgrades::HAND_SIZE.saturating_sub(cut).max(1);
        let mut d = deck::Deck::new(&nums, self.director.dream_seed());
        d.draw_to_fill(hand_size);
        let mut opening = Vec::new();
        for ev in d.events() {
            if let deck::DeckEvent::Drawn { slot, card } = ev {
                if let Some(c) = self.booklet.card(card) {
                    opening.push(c.clone());
                }
                // The first hand fans out one card after another.
                self.deck_drawn_at[slot] = self.deck_clock + 0.45 + 0.14 * slot as f32;
            }
        }
        log::info!("Deck: {:?} (hand {hand_size})", nums);
        self.hand_cards = nums
            .iter()
            .filter_map(|&n| self.booklet.card(n))
            .map(|c| (crate::powers::power_of(c).active, c.clone()))
            .collect();
        self.deck = Some(d);
        self.deck_opening = opening.clone();
        Some(opening)
    }
    // === end deck ===

    fn hand_view(&self) -> Vec<hand::HandCard> {
        self.run
            .abilities
            .iter()
            .enumerate()
            // === deck === spent ghosts finishing an effect are not cards in hand
            .take(if self.run.deck_slots > 0 { self.run.deck_slots } else { usize::MAX })
            // === end deck ===
            .filter(|(_, a)| self.run.deck_slots > 0 || !a.spent) // === abilities3 === a borrowed ability finishing its effect
            .map(|(k, a)| {
                let (tint, tint2) = self
                    .ability_tints
                    .iter()
                    .find(|t| t.0 == a.ability)
                    .map_or(([200, 150, 255], [120, 100, 150]), |&(_, x, y)| (x, y));
                let since_use = self
                    .ability_used
                    .iter()
                    .find(|u| u.0 == a.ability)
                    .map_or(hand::NEVER, |u| self.dream_age - u.1);
                let slot_card = self.card_for_slot(k, a.ability);
                let passive = slot_card.map(|n| crate::powers::power_of(&n.1).passive);
                let edge = slot_card.and_then(|n| loadout_ui::edges_of(&n.1).into_iter().next());
                // === deck ===
                let since_draw = if self.deck.is_some() {
                    self.deck_clock - self.deck_drawn_at[k.min(upgrades::HAND_SIZE - 1)]
                } else {
                    hand::NEVER
                };
                // === end deck ===
                hand::HandCard {
                    since_draw: if since_draw < 0.0 { 0.0 } else { since_draw },
                    empty: a.spent || since_draw < 0.0,
                    edge: edge.map(|e| (e.name(), e.color())),
                    edge_since: edge
                        .and_then(|e| self.edge_fired.iter().find(|f| f.0 == e))
                        .map_or(hand::NEVER, |f| self.dream_age - f.1),
                    passive: passive.map(|u| (u.info().name, u.info().color)),
                    passive_since: passive
                        .and_then(|u| self.passive_fired.iter().find(|f| f.0 == u))
                        .map_or(hand::NEVER, |f| self.dream_age - f.1),
                    cracked: self
                        .run
                        .taken
                        .iter()
                        .any(|u| matches!(u, Upgrade::Cursed(_))),
                    ability: a.ability.name(),
                    card: slot_card.map(|n| n.1.clone()),
                    // === toolcards === the card in this slot is a bought tool
                    tool: self
                        .deck
                        .as_ref()
                        .and_then(|d| d.hand().get(k).copied().flatten())
                        .is_some_and(deck::is_tool),
                    // === end toolcards ===
                    key: upgrades::SLOT_KEYS[k],
                    // === abilities3 === a charging OVERLOAD fills its card
                    charge: match (a.ability, self.overload) {
                        (Ability::Overload, Some(c)) if c.slot == k => c.fraction(),
                        _ => a.charge(self.run.cooldown_for(a.ability)),
                    },
                    // === end abilities3 ===
                    active: a.is_active()
                        || (a.ability == Ability::Anchor && self.anchor.is_some())
                        // === abilities3 ===
                        || (a.ability == Ability::Bargain && self.bargain.active())
                        || (a.ability == Ability::Overload && self.overload.is_some()),
                        // === end abilities3 ===
                    tint,
                    tint2,
                    since_use,
                }
            })
            .collect()
    }

    // === combat ===
    /// Strike, stagger timers, unravelling enemies, calm motes, the flash.
    fn update_combat(&mut self, dt: f32) {
        // Dev: DREAMSCAPE_STRIKE=<secs> lets the autopilot swing, over and
        // over, from that many seconds in (for screenshots).
        let dev_from = if self.autopilot {
            crate::dev::var("DREAMSCAPE_STRIKE").ok().and_then(|v| v.parse::<f32>().ok())
        } else {
            None
        };
        if dev_from.is_some_and(|from| self.dream_age >= from) {
            self.strike_request = true;
        }
        let active = !self.coop_active() && (!self.autopilot || dev_from.is_some());
        // === combat passives ===
        // === upgrades2 === THIN SKIN's price, then IRON NERVE and friends
        if self.run.has_curse(upgrades::Curse::ThinSkin) && !self.thin_skin_applied {
            self.thin_skin_applied = true;
            self.nerve.lower_max(1);
        }
        let want_max = self.run.nerve_max();
        // === end upgrades2 ===
        if want_max > self.nerve.max() {
            self.nerve.raise_max(want_max - self.nerve.max());
        }
        self.counter_cd = (self.counter_cd - dt).max(0.0);
        self.lunge_left = (self.lunge_left - dt).max(0.0);
        self.marks.retain(|_, left| {
            *left -= dt;
            *left > 0.0
        });
        if !self.player_grounded {
            self.air_until = self.dream_age;
        } else {
            self.air_refunded = false;
        }
        // CALM MIND: calm earns nerve back.
        if let Some(every) = combat::calm_interval(self.run.count(Upgrade::CalmMind)) {
            if self.enemies.iter().any(|p| p.ai.chasing) {
                self.calm_timer = 0.0;
            } else {
                self.calm_timer += dt;
                if self.calm_timer >= every {
                    self.calm_timer = 0.0;
                    self.nerve.heal(1);
                }
            }
        }
        // === end combat passives ===
        // === sounds === a pulse at one nerve; a chime when an ability is ready again
        if active && self.nerve.cur() == 1 && self.nerve.max() > 1 {
            self.heart_timer += dt;
            if self.heart_timer >= HEARTBEAT_EVERY {
                self.heart_timer = 0.0;
                self.sfx(Sound::Heartbeat);
            }
        } else {
            self.heart_timer = HEARTBEAT_EVERY * 0.6;
        }
        let ready_now: Vec<bool> = self
            .run
            .abilities
            .iter()
            .map(|a| a.cooldown <= 0.0 && !a.spent)
            .collect();
        if self.was_ready.len() == ready_now.len()
            && ready_now.iter().zip(&self.was_ready).any(|(now, was)| *now && !*was)
            && self.mode == hud::Mode::Playing
            && !self.autopilot
        {
            self.sfx(Sound::Ready);
        }
        self.was_ready = ready_now;
        // === end sounds ===
        let wants = std::mem::take(&mut self.strike_request);
        if active && wants {
            let mut quick = 0.9_f32.powi(self.run.count(Upgrade::Quicken) as i32)
                * self.run.strike_cooldown(); // === upgrades2 ===
            if self.has_edge(edges::Edge::Quickdraw) {
                quick *= edges::QUICKDRAW_MULT;
            }
            if self.strike.try_start(quick) {
                // === edges === LUNGE: the swing carries you a step forward
                if self.has_edge(edges::Edge::Lunge) {
                    self.lunge_left = edges::LUNGE_SECS;
                    self.lunge_dir = self.flat_facing();
                    self.edge_fires(edges::Edge::Lunge);
                }
                // === end edges ===
                // DOUBLE JUMP: a strike in the air gives one air jump back.
                if !self.player_grounded
                    && !self.air_refunded
                    && self.run.count(Upgrade::DoubleJump) > 0
                    && self.air_jumps_used > 0
                {
                    self.air_jumps_used -= 1;
                    self.air_refunded = true;
                }
                // Every other swing comes back, a little lower.
                self.sfx_pitched(Sound::Strike, if self.strike_count % 2 == 1 { 0.9 } else { 1.0 });
                let arc = self.strike_arc_cos();
                // === edges2 === CLEAVE and LONGARM show on the swing
                self.edge_fires(edges::Edge::Cleave);
                self.edge_fires(edges::Edge::Longarm);
                // === end edges2 ===
                self.slash_pending = Some(SlashStart {
                    facing: self.flat_facing(),
                    reach: self.strike_reach(),
                    half_arc: arc.acos(),
                    flip: self.strike_count % 2 == 1, // every other swing comes back
                });
            }
        }
        if self.strike.tick(dt) {
            self.resolve_strike();
        }
        // DASH cuts what it passes through, once each.
        if active && self.run.active(Ability::Dash) {
            let me = self.player_position;
            let cut: Vec<CombatTarget> = self
                .combat_targets()
                .into_iter()
                .filter(|t| t.pos.distance(me) <= combat::DASH_CUT_REACH && !self.dash_cut.contains(&t.entity))
                .collect();
            for t in &cut {
                self.dash_cut.push(t.entity);
                let dmg = self.strike_damage(t);
                self.hurt_foe(t, dmg, me, 0.0, combat::STAGGER_SECS);
            }
            if !cut.is_empty() {
                self.sfx(Sound::StrikeHit);
                self.juice.hit(juice::Hit::Strike, self.settings.reduced_motion);
            }
        }
        // LEAP: the landing shocks what is under you.
        if self.leap_airborne && self.player_grounded && self.dream_age > 0.2 {
            self.leap_airborne = false;
        self.last_stand_used = false;
        self.counter_cd = 0.0;
        self.marks.clear();
        self.lunge_left = 0.0;
        self.edge_fired.clear();
            if active {
                let at = self.player_position;
                if self.shock(at, combat::LEAP_SHOCK_REACH, 1.0, combat::STAGGER_SECS) > 0 {
                    self.sfx(Sound::StrikeHit);
                }
            }
        }
        // SHARD CALL draws calm motes in.
        if self.run.active(Ability::ShardCall) {
            let me = self.player_position;
            for (e, _) in &self.motes {
                if let Ok(mut t) = self.world.get::<&mut Transform>(*e) {
                    let d = me - t.position;
                    if d.length() > 0.1 {
                        t.position += d.normalize() * combat::MOTE_PULL_SPEED * dt;
                    }
                }
            }
        }
        // Special enemies recover.
        self.staggered.retain(|_, left| {
            *left -= dt;
            *left > 0.0
        });
        // Unravelling: swell, shrink, gone.
        let mut gone = Vec::new();
        for (k, (e, age, full)) in self.unraveling.iter_mut().enumerate() {
            *age += dt;
            if *age >= combat::UNRAVEL_SECS {
                gone.push(k);
            } else if let Ok(mut t) = self.world.get::<&mut Transform>(*e) {
                t.scale = *full * combat::unravel_scale(*age);
            }
        }
        for k in gone.into_iter().rev() {
            let (e, _, _) = self.unraveling.remove(k);
            let _ = self.world.despawn(e);
        }
        // The swing's flash: a fan sweeps across the arc from the dreamer's
        // centre, then narrows away, throwing sparks off its leading edge.
        let me = self.player_position;
        let reduced = self.settings.reduced_motion;
        let mut done = Vec::new();
        for (k, s) in self.slashes.iter_mut().enumerate() {
            s.age += dt;
            if s.age >= combat::SLASH_LIFE.max(combat::SLASH_SWEEP + combat::SPARK_LIFE) {
                done.push(k);
                continue;
            }
            let heading = |angle: f32| {
                let (sin, cos) = angle.sin_cos();
                // Positive angles lean to the right of the facing.
                Vec3::new(
                    s.facing.x * cos + s.facing.z * sin,
                    0.0,
                    -s.facing.x * sin + s.facing.z * cos,
                )
            };
            let n = s.rays.len();
            for (i, pieces) in s.rays.iter().enumerate() {
                let ray = combat::slash_ray(i, n, s.age, s.half_arc, s.flip, reduced);
                for (j, e) in pieces.iter().enumerate() {
                    let Ok(mut t) = self.world.get::<&mut Transform>(*e) else {
                        continue;
                    };
                    let Some(r) = ray else {
                        t.scale = Vec3::ZERO;
                        continue;
                    };
                    let (from, to, wide) = combat::SLASH_SEGMENTS[j];
                    let dir = heading(r.angle);
                    let len = s.reach * r.length;
                    t.position = me + dir * (len * (from + to) / 2.0) + Vec3::Y * 0.25;
                    t.rotation = Quat::from_rotation_arc(Vec3::Z, dir);
                    t.scale = Vec3::new((0.1 + 0.4 * r.width) * wide, 0.012, len * (to - from));
                }
            }
            for (k2, e) in s.sparks.iter().enumerate() {
                let Ok(mut t) = self.world.get::<&mut Transform>(*e) else {
                    continue;
                };
                match combat::slash_spark(k2, s.age, s.half_arc, s.flip).filter(|_| !reduced) {
                    Some((angle, dist, size)) => {
                        t.position = me + heading(angle) * (s.reach * dist) + Vec3::Y * 0.4;
                        t.scale = Vec3::splat(0.34 * size);
                    }
                    None => t.scale = Vec3::ZERO,
                }
            }
        }
        for k in done.into_iter().rev() {
            let s = self.slashes.remove(k);
            for e in s.rays.into_iter().flatten().chain(s.sparks) {
                let _ = self.world.despawn(e);
            }
        }
        // Calm motes bob; touching one steadies you.
        let coop = self.coop_active();
        let mut taken = Vec::new();
        for (k, (e, age)) in self.motes.iter_mut().enumerate() {
            *age += dt;
            let Ok(mut t) = self.world.get::<&mut Transform>(*e) else {
                taken.push((k, false));
                continue;
            };
            t.position.y += combat::mote_bob(*age) - combat::mote_bob(*age - dt);
            if *age > combat::MOTE_LIFE {
                taken.push((k, false));
            } else if t.position.distance(me) < combat::MOTE_PICKUP && !coop {
                taken.push((k, true));
            }
        }
        for (k, steadied) in taken.into_iter().rev() {
            let (e, _) = self.motes.remove(k);
            let _ = self.world.despawn(e);
            if steadied {
                let spore = self.run.count(Upgrade::CalmSpore) > 0;
                self.nerve.heal(if spore { 2 } else { 1 });
                if spore {
                    self.shock(me, combat::SPORE_REACH, 0.0, combat::FLARE_STAGGER);
                }
                self.sfx(Sound::Calm);
                self.flash.trigger([0.6, 1.0, 0.8], 0.25);
            }
        }
    }

    /// Everything that can be struck right now: (entity, position, kind, unaware).
    fn combat_targets(&self) -> Vec<CombatTarget> {
        let at = |w: &World, e: Entity| w.get::<&Transform>(e).ok().map(|t| t.position);
        let mut targets = Vec::new();
        let me = self.player_position;
        // === enemies-a === (armoured front, recovering) per new enemy
        for f in &self.newfoes {
            let Some(pos) = at(&self.world, f.entity) else { continue };
            let (unaware, guarded, recovering) = match &f.ai {
                NewAI::Tooth(a) => (a.unaware(pos, me), false, a.vulnerable()),
                NewAI::Weeper(a) => (a.unaware(pos, me), false, false),
                NewAI::Lullaby(a) => (a.unaware(pos, me), a.guarded_against(pos, me), false),
            };
            targets.push(CombatTarget { entity: f.entity, pos, kind: f.kind, unaware, guarded, recovering });
        }
        // === end enemies-a ===
        for p in &self.enemies {
            if let Some(pos) = at(&self.world, p.entity) {
                let kind = combat::Kind::Pacer {
                    elite: p.elite,
                    child: p.split && p.elite == Some(Elite::Splitter),
                };
                targets.push(CombatTarget { entity: p.entity, pos, kind, unaware: !p.ai.chasing, guarded: false, recovering: false });
            }
        }
        for sp in &self.specials {
            let kind = match sp.kind {
                EnemyKind::Stalker => combat::Kind::Stalker,
                EnemyKind::Sentry => combat::Kind::Sentry,
                EnemyKind::Drifter => combat::Kind::Drifter,
                EnemyKind::Jester => combat::Kind::Jester,
                EnemyKind::Mimic => continue, // it is only you
                EnemyKind::Moths => combat::Kind::Moth,
                EnemyKind::Mask => combat::Kind::Mask,
                EnemyKind::Leech => combat::Kind::Leech,
            };
            if let Some(pos) = at(&self.world, sp.entity) {
                // Sentries see you coming; the rest are unaware until chasing.
                targets.push(CombatTarget {
                    entity: sp.entity,
                    pos,
                    kind,
                    unaware: !matches!(
                        kind,
                        combat::Kind::Sentry | combat::Kind::Moth | combat::Kind::Mask | combat::Kind::Leech
                    ),
                    guarded: false,
                    recovering: false,
                });
            }
        }
        targets
    }

    /// The conditions your cards put on a strike against `t` right now.
    fn strike_ctx(&self, t: &CombatTarget) -> combat::Ctx {
        let me = self.player_position;
        combat::Ctx {
            // BLINK sets up a sneak attack; HUSH deepens it.
            unaware: t.unaware
                || self.dream_age < self.backstab_until
                || self.run.active(Ability::Veil), // === abilities2 ===
            airborne: !self.player_grounded
                || self.run.active(Ability::Float)
                || (self.run.count(Upgrade::Spring) > 0
                    && self.dream_age - self.air_until < combat::SPRING_AIR_GRACE),
            // STILLNESS freezes: a frozen foe is a reeling one.
            staggered: self.stunned.contains_key(&t.entity)
                || self.staggered.contains_key(&t.entity)
                || t.recovering
                || self.run.active(Ability::Stillness),
            guarded: t.guarded && !self.has_edge(edges::Edge::Rending),
            ghost: self.run.active(Ability::Phase),
            hush: self.run.active(Ability::Hush),
            high_ground: self.run.active(Ability::Step)
                && me.y - t.pos.y > combat::HIGH_GROUND_RISE,
        }
    }

    // === edges2 ===
    /// The strike's reach: LONG REACH cards, then LONGARM.
    fn strike_reach(&self) -> f32 {
        edges::reach(
            combat::reach_with(self.run.count(Upgrade::LongReach)),
            self.has_edge(edges::Edge::Longarm),
        )
    }

    /// The strike's half-arc cosine: WIDE EYES, then CLEAVE.
    fn strike_arc_cos(&self) -> f32 {
        let base = if self.run.count(Upgrade::WideEyes) > 0 {
            combat::ARC_COS_WIDE
        } else {
            combat::ARC_COS
        };
        edges::arc_cos(base, self.has_edge(edges::Edge::Cleave))
    }

    /// Is this foe still untouched?
    fn foe_is_fresh(&self, t: &CombatTarget) -> bool {
        self.foes.get(&t.entity).map_or(true, |f| f.hp >= f.max)
    }
    // === end edges2 ===

    fn has_edge(&self, e: edges::Edge) -> bool {
        self.edges.contains(&e)
    }

    /// An edge just did something: its card in the hand pulses.
    fn edge_fires(&mut self, e: edges::Edge) {
        if self.has_edge(e) {
            self.edge_fired.retain(|f| f.0 != e);
            self.edge_fired.push((e, self.dream_age));
            self.sfx(if e == edges::Edge::Hexmark { Sound::Mark } else { Sound::EdgeFire });
        }
    }

    /// What a strike (or a dash cut) does to `t` right now, cards included.
    fn strike_damage(&self, t: &CombatTarget) -> f32 {
        let ctx = self.strike_ctx(t);
        let mut dmg = combat::damage(1.0, ctx);
        if ctx.staggered {
            dmg += self.run.count(Upgrade::SharpEdge) as f32;
        }
        if ctx.unaware && self.run.has(upgrades::Synergy::Executioner) {
            dmg += 1.0;
        }
        // === edges ===
        if self.has_edge(edges::Edge::Reprisal) && self.nerve.invulnerable() {
            dmg += edges::EDGE_BONUS;
        }
        if self.flourish_now {
            dmg += edges::EDGE_BONUS;
        }
        if self.marks.contains_key(&t.entity) {
            dmg += edges::EDGE_BONUS;
        }
        // === end edges ===
        // === edges2 === AMBUSH, OPENER, PUNISHER
        dmg += edges::bonus(&self.edges, ctx.unaware, ctx.staggered, self.foe_is_fresh(t));
        // === end edges2 ===
        dmg * self.bargain.mult() // === abilities3 === BARGAIN cuts deeper
    }

    /// Damages one target: the shared end of a strike, a dash, a shock.
    /// `knock` shoves it that far from `from` (a frozen foe cannot be shoved).
    /// Returns true if it came undone.
    fn hurt_foe(&mut self, t: &CombatTarget, dmg: f32, from: Vec3, knock: f32, stagger: f32) -> bool {
        // === edges === a mark is spent by the blow that uses it
        if self.marks.remove(&t.entity).is_some() {
            self.edge_fires(edges::Edge::Hexmark);
        }
        // === end edges ===
        let outcome = self
            .foes
            .entry(t.entity)
            .or_insert_with(|| combat::Foe::new(t.kind))
            .hit(dmg);
        match outcome {
            combat::Outcome::Hurt => {
                // === edges === HEXMARK: the blow marks what it did not undo
                if self.has_edge(edges::Edge::Hexmark) {
                    self.marks.insert(t.entity, edges::HEXMARK_SECS);
                }
                // === end edges ===
                // === edges2 === RIPPLE: the foes beside the target reel
                if self.has_edge(edges::Edge::Ripple) {
                    self.ripple(t);
                }
                // === end edges2 ===
                let away = Vec3::new(t.pos.x - from.x, 0.0, t.pos.z - from.z).normalize_or_zero();
                if knock > 0.0 && !self.run.active(Ability::Stillness) {
                    if let Ok(mut tr) = self.world.get::<&mut Transform>(t.entity) {
                        tr.position += away * knock;
                    }
                }
                // === combat === a strike on a latched leech knocks it off
                if t.kind == combat::Kind::Leech {
                    self.shake_leech(t.entity);
                }
                // === end combat ===
                // === enemies-a ===
                if let Some(f) = self.newfoes.iter_mut().find(|f| f.entity == t.entity) {
                    match &mut f.ai {
                        // Its recovery is its own clock: no extra reel.
                        NewAI::Tooth(a) => a.hurt(),
                        NewAI::Weeper(a) => {
                            a.hurt();
                            self.staggered.insert(t.entity, stagger);
                        }
                        NewAI::Lullaby(a) => {
                            a.stagger();
                            self.staggered.insert(t.entity, stagger);
                        }
                    }
                } else
                // === end enemies-a ===
                if matches!(t.kind, combat::Kind::Pacer { .. }) {
                    self.stunned.insert(t.entity, stagger);
                    if let Some(p) = self.enemies.iter_mut().find(|p| p.entity == t.entity) {
                        p.ai.reset(); // a hit breaks its lunge
                    }
                } else {
                    self.staggered.insert(t.entity, stagger);
                }
                false
            }
            combat::Outcome::Unravel => {
                self.unravel(t.entity, t.pos);
                true
            }
        }
    }

    // === edges2 ===
    /// RIPPLE: every other foe near `t` reels briefly. Returns how many.
    fn ripple(&mut self, t: &CombatTarget) -> usize {
        let near: Vec<CombatTarget> = self
            .combat_targets()
            .into_iter()
            .filter(|o| o.entity != t.entity && o.pos.distance(t.pos) <= edges::RIPPLE_REACH)
            .collect();
        for o in &near {
            if matches!(o.kind, combat::Kind::Pacer { .. }) {
                self.stunned.insert(o.entity, edges::RIPPLE_STAGGER);
            } else {
                self.staggered.insert(o.entity, edges::RIPPLE_STAGGER);
            }
        }
        if !near.is_empty() {
            self.edge_fires(edges::Edge::Ripple);
        }
        near.len()
    }
    // === end edges2 ===

    /// An area effect: every foe within `reach` of `centre` takes `dmg`
    /// (0 = stagger only) and reels. Returns how many were touched.
    fn shock(&mut self, centre: Vec3, reach: f32, dmg: f32, stagger: f32) -> usize {
        let hit: Vec<CombatTarget> = self
            .combat_targets()
            .into_iter()
            .filter(|t| {
                Vec3::new(t.pos.x - centre.x, 0.0, t.pos.z - centre.z).length() <= reach
                    && (t.pos.y - centre.y).abs() <= combat::REACH_HEIGHT
            })
            .collect();
        for t in &hit {
            if dmg > 0.0 {
                self.hurt_foe(t, dmg, centre, 0.0, stagger);
            } else if matches!(t.kind, combat::Kind::Pacer { .. }) {
                self.stunned.insert(t.entity, stagger);
            } else {
                self.staggered.insert(t.entity, stagger);
            }
        }
        hit.len()
    }

    /// The swing lands: everything in the arc takes damage.
    fn resolve_strike(&mut self) {
        let me = self.player_position;
        let facing = self.flat_facing();
        let mut struck = false;
        let reach = self.strike_reach();
        let arc = self.strike_arc_cos();
        let mut knock = combat::KNOCKBACK
            * (1.0 + combat::HEAVY_KNOCK_PER * self.run.count(Upgrade::HeavyAir).min(2) as f32);
        // === edges === HAYMAKER shoves and staggers more; FLOURISH is the third of a chain
        let mut stagger = combat::STAGGER_SECS;
        if self.has_edge(edges::Edge::Haymaker) {
            knock *= edges::HAYMAKER_KNOCK;
            stagger *= edges::HAYMAKER_STAGGER;
        }
        self.flourish_now =
            self.has_edge(edges::Edge::Flourish) && self.flourish.strike(self.dream_age);
        if self.flourish_now {
            self.edge_fires(edges::Edge::Flourish);
        }
        // === end edges ===
        // ECHO STRIKE: every fourth strike is followed by a longer wave.
        self.strike_count += 1;
        let echo = self.run.count(Upgrade::EchoStrike) > 0
            && self.strike_count % combat::ECHO_EVERY == 0;
        let waves: &[f32] = if echo { &[1.0, combat::ECHO_REACH] } else { &[1.0] };
        for &stretch in waves {
            for t in self.combat_targets() {
                if !combat::in_arc(me, facing, t.pos, reach * stretch, arc) {
                    continue;
                }
                // The wave only reaches what the first did not (it is longer).
                if stretch > 1.0 && combat::in_arc(me, facing, t.pos, reach, arc) {
                    continue;
                }
                // === combat === a mask's shield turns the blow aside
                if t.kind == combat::Kind::Mask
                    && !self.has_edge(edges::Edge::Rending)
                    && self.mask_glances(t.entity, me)
                {
                    self.mask_glance(t.pos);
                    continue;
                }
                // === end combat ===
                struck = true;
                self.burst_pending = Some((t.pos + Vec3::Y * 0.5, [255, 150, 230], [255, 255, 255]));
                let dmg = self.strike_damage(&t);
                self.hurt_foe(&t, dmg, me, knock, stagger);
            }
        }
        self.flourish_now = false;
        self.bargain.spend_strike(); // === abilities3 ===
        // === combat === the nightmare: wisps fall to a blow; the hunter only
        // can be hurt while it rests.
        if let Some((entity, b, _)) = self.hunter.as_mut() {
            if b.strike_wisps(me, facing, reach, arc) > 0 {
                struck = true;
            }
            let hunter_at = self.world.get::<&Transform>(*entity).ok().map(|t| t.position);
            if let Some(hp) = hunter_at {
                if combat::in_arc(me, facing, hp, reach + boss::HUNTER_BODY, arc) {
                    let dmg = 1.0 + self.run.count(Upgrade::SharpEdge) as f32 * f32::from(b.vulnerable());
                    match b.hurt(dmg) {
                        boss::Blow::Deflected => {
                            if !b.defeated {
                                self.sfx(Sound::Denied);
                            }
                        }
                        boss::Blow::Hurt => {
                            struck = true;
                            if let Ok(mut t) = self.world.get::<&mut Transform>(*entity) {
                                let away = Vec3::new(hp.x - me.x, 0.0, hp.z - me.z).normalize_or_zero();
                                t.position += away * combat::KNOCKBACK * 0.5;
                            }
                        }
                        boss::Blow::Undone => {
                            struck = true;
                            let tier = b.tier;
                            self.undo_nightmare(tier, hp);
                        }
                    }
                }
            }
        }
        // === end combat ===
        if struck {
            self.sfx(Sound::StrikeHit);
            self.juice.hit(juice::Hit::Strike, self.settings.reduced_motion);
        }
    }

    /// The nightmare comes undone: dust and calm motes. (The portal still
    /// wants every sigil.)
    fn undo_nightmare(&mut self, tier: u32, at: Vec3) {
        self.bonus_dust += boss::UNDONE_DUST_PER_TIER * tier;
        for k in 0..boss::UNDONE_MOTES {
            self.pending_motes.push(at + Vec3::new(1.5 * (k as f32 - 0.5), 0.0, 1.0));
        }
        self.sfx(Sound::Triumph);
        self.flash.trigger([1.0, 0.9, 1.0], 0.4);
        log::info!("Nightmare undone (tier {tier})");
    }

    /// Your cards' combat effects, as they are used (see the combat plan, table 6).
    fn combat_on_use(&mut self, ability: Ability, origin: Vec3) {
        if self.coop_active() {
            return;
        }
        match ability {
            Ability::Blink => self.backstab_until = self.dream_age + combat::BLINK_BACKSTAB_SECS,
            Ability::Dash => self.dash_cut.clear(),
            Ability::Leap => self.leap_airborne = true,
            Ability::Flare => {
                // The light blinds and staggers; Shades cannot hide in it.
                let me = self.player_position;
                let shades: Vec<CombatTarget> = self
                    .combat_targets()
                    .into_iter()
                    .filter(|t| matches!(t.kind, combat::Kind::Pacer { elite: Some(Elite::Shade), .. }))
                    .filter(|t| t.pos.distance(me) <= combat::FLARE_REACH)
                    .collect();
                self.shock(me, combat::FLARE_REACH, 0.0, combat::FLARE_STAGGER);
                for t in &shades {
                    self.hurt_foe(t, 1.0, me, 0.0, combat::FLARE_STAGGER);
                }
            }
            Ability::Rewind => {
                let back = self.nerve.recover();
                if back > 0 {
                    log::info!("Rewind gave back {back} nerve");
                    self.sfx(Sound::Calm);
                }
            }
            Ability::Anchor if self.anchor_snapped => {
                // Snapping back leaves a wave where you stood.
                self.shock(origin, combat::ANCHOR_WAVE_REACH, 0.0, combat::STAGGER_SECS);
            }
            _ => {}
        }
    }

    /// The decoy comes undone: a small burst where it stood.
    fn decoy_burst(&mut self, at: Vec3) {
        if self.coop_active() {
            return;
        }
        if self.shock(at, combat::DECOY_BURST_REACH, 1.0, combat::STAGGER_SECS) > 0 {
            self.sfx(Sound::StrikeHit);
        }
    }

    /// WARD breaks: the hit goes back at what made it.
    fn ward_reflect(&mut self) {
        let me = self.player_position;
        let near = self
            .combat_targets()
            .into_iter()
            .filter(|t| t.pos.distance(me) <= combat::WARD_REFLECT_REACH)
            .min_by(|a, b| a.pos.distance(me).total_cmp(&b.pos.distance(me)));
        if let Some(t) = near {
            let back = if self.run.has(upgrades::Synergy::Bulwark) { 2.0 } else { 1.0 };
            self.hurt_foe(&t, back, me, combat::KNOCKBACK, combat::STAGGER_SECS);
        }
    }

    /// COUNTER: you slipped a blow; the attacker reels (RIPOSTE cuts it too).
    fn counter_attack(&mut self) {
        let me = self.player_position;
        let near = self
            .combat_targets()
            .into_iter()
            .filter(|t| t.pos.distance(me) <= combat::COUNTER_REACH)
            .min_by(|a, b| a.pos.distance(me).total_cmp(&b.pos.distance(me)));
        let Some(t) = near else {
            return;
        };
        self.counter_cd = combat::COUNTER_COOLDOWN;
        if self.run.has(upgrades::Synergy::Riposte) {
            self.hurt_foe(&t, 1.0, me, 0.0, combat::RIPOSTE_STAGGER);
        } else if matches!(t.kind, combat::Kind::Pacer { .. }) {
            self.stunned.insert(t.entity, combat::COUNTER_STAGGER);
        } else {
            self.staggered.insert(t.entity, combat::COUNTER_STAGGER);
        }
        self.sfx(Sound::StrikeHit);
        self.flash.trigger([1.0, 0.8, 0.5], 0.2);
    }

    /// THORNS: what hurt you takes a bite.
    fn thorns_bite(&mut self) {
        let me = self.player_position;
        let near = self
            .combat_targets()
            .into_iter()
            .filter(|t| t.pos.distance(me) <= combat::THORNS_REACH)
            .min_by(|a, b| a.pos.distance(me).total_cmp(&b.pos.distance(me)));
        if let Some(t) = near {
            self.hurt_foe(&t, 1.0, me, 0.0, combat::STAGGER_SECS);
        }
    }

    /// Is `p` inside a wall? (GUST shoves enemies into them.)
    fn in_wall(&self, p: Vec3) -> bool {
        self.dream.as_ref().is_some_and(|d| {
            d.blocks.iter().any(|b| {
                b.kind == dream::BlockKind::Wall && {
                    let h = b.size * 0.5;
                    (p.x - b.pos.x).abs() < h.x && (p.y - b.pos.y).abs() < h.y && (p.z - b.pos.z).abs() < h.z
                }
            })
        })
    }

    /// Hit-point pips for the enemies that have been struck (and, with WIDE
    /// EYES, for any enemy nearby): screen position (0..1) plus pips.
    fn foe_pips(&self, view_proj: Mat4) -> Vec<hud::FoePips> {
        let me = self.player_position;
        let eyes = self.run.count(Upgrade::WideEyes) > 0;
        let mut out = Vec::new();
        for t in self.combat_targets() {
            let foe = self.foes.get(&t.entity).copied().unwrap_or_else(|| combat::Foe::new(t.kind));
            let struck = foe.hp < foe.max;
            if !struck && !(eyes && t.pos.distance(me) <= combat::EYES_PIP_REACH) {
                continue;
            }
            let clip = view_proj * (t.pos + Vec3::Y * 1.7).extend(1.0);
            if clip.w <= 0.0 {
                continue;
            }
            out.push(hud::FoePips {
                at: [0.5 + 0.5 * clip.x / clip.w, 0.5 - 0.5 * clip.y / clip.w],
                hp: foe.hp,
                max: foe.max,
            });
            if out.len() >= 8 {
                break;
            }
        }
        out
    }

    /// An enemy comes undone: out of play at once, a short animation, loot.
    fn unravel(&mut self, entity: Entity, pos: Vec3) {
        self.marks.remove(&entity);
        // === edges === SIPHON: every third enemy undone steadies you
        if self.has_edge(edges::Edge::Siphon) && self.siphon.undone() {
            self.nerve.heal(1);
            self.edge_fires(edges::Edge::Siphon);
            self.sfx(Sound::Calm);
        }
        // === end edges ===
        self.foes.remove(&entity);
        self.stunned.remove(&entity);
        self.staggered.remove(&entity);
        self.enemies.retain(|p| p.entity != entity);
        // === enemies-a === its tears, plate, marker and ring go with it
        if let Some(k) = self.newfoes.iter().position(|f| f.entity == entity) {
            let f = self.newfoes.remove(k);
            for e in f.extras {
                let _ = self.world.despawn(e);
            }
        }
        // === end enemies-a ===
        if let Some(k) = self.specials.iter().position(|s| s.entity == entity) {
            let sp = self.specials.remove(k);
            if let SpecialAI::Sentry(_, beam) = sp.ai {
                let _ = self.world.despawn(beam);
            }
            // === combat === a dead moth leaves the ring; a dead leech lets go
            if let SpecialAI::Moth(id) = sp.ai {
                if let Some(s) = &mut self.moth_swarm {
                    s.kill(id);
                    if s.alive() == 0 {
                        self.moth_swarm = None;
                    }
                }
            }
            // === end combat ===
        }
        let full = self
            .world
            .get::<&Transform>(entity)
            .map_or(Vec3::ONE, |t| t.scale);
        // Take away its collider-like presence: it no longer touches anyone.
        self.unraveling.push((entity, 0.0, full));
        self.sfx(Sound::Unravel);
        let seed = (entity.id() as u64) << 20 ^ (self.dream_age * 1000.0) as u64 ^ self.director.dream_seed();
        let loot = combat::loot(
            seed,
            self.run.count(Upgrade::DustHoarder),
            combat::LUCKY_MOTE_PER * self.run.count(Upgrade::LuckyMemory) as f32,
        );
        self.bonus_dust += loot.dust;
        // === edges2 === WINDFALL: extra dust; LANTERN: always a mote
        if self.has_edge(edges::Edge::Windfall) {
            self.bonus_dust += edges::WINDFALL_DUST;
            self.edge_fires(edges::Edge::Windfall);
        }
        let lantern = self.has_edge(edges::Edge::Lantern);
        if lantern {
            self.edge_fires(edges::Edge::Lantern);
        }
        // === end edges2 ===
        self.sfx_pitched(Sound::Coin, 1.0 + 0.04 * loot.dust.min(5) as f32);
        if loot.mote || lantern {
            self.pending_motes.push(pos);
        }
        log::info!("Unravelled (+{} dust{})", loot.dust, if loot.mote { ", a calm mote" } else { "" });
    }

    // === end combat ===
    // === combat ===
    /// Something touched the dreamer. Costs nerve; returns the source only
    /// when that was the last point (the old *caught* flow then runs).
    fn take_nerve_hit(&mut self, by: &'static str) -> Option<&'static str> {
        let away = self.away_from_nearest_threat();
        let extra = nerve::IFRAMES_PER_BREATH * self.run.count(Upgrade::LongBreath) as f32
            + self.run.afterglow() // === upgrades2 ===
            + if self.settings.gentle_nerve { nerve::GENTLE_IFRAMES } else { 0.0 };
        match self.nerve.hit(nerve::damage_from(by), away, extra) {
            nerve::Hit::Down => {
                self.nerve_lost_this_dream = true; // === fortune ===
                // DREAM ANCHOR: once a dream, a lethal blow leaves you standing.
                if self.run.count(Upgrade::DreamAnchor) > 0 && !self.last_stand_used {
                    self.last_stand_used = true;
                    self.nerve.last_stand(extra);
                    self.passive_fires(Upgrade::DreamAnchor);
                    self.flash.trigger([1.0, 0.8, 0.4], 0.5);
                    self.sfx(Sound::LastStand);
                    log::info!("Last stand: {by} could not finish you");
                    return None;
                }
                Some(by)
            }
            nerve::Hit::Ignored => None,
            nerve::Hit::Hurt => {
                self.nerve_lost_this_dream = true; // === fortune ===
                log::info!(
                    "Hurt by: {by} (nerve {}/{}, depth {})",
                    self.nerve.cur(),
                    self.nerve.max(),
                    self.director.depth
                );
                // Reduce flashing: the red wash is a quarter as strong.
                let wash = if self.settings.reduce_flashing { 0.12 } else { 0.45 };
                self.flash.trigger([1.0, 0.1, 0.15], wash);
                self.juice.hit(juice::Hit::Caught, self.settings.reduced_motion);
                self.sfx(Sound::Hurt);
                if self.run.count(Upgrade::Thorns) > 0 {
                    self.thorns_bite();
                }
                None
            }
        }
    }

    /// Unit direction (x, z) from the closest enemy to the dreamer; zero when
    /// the hit came from the dream itself (a gate, a beat tile).
    fn away_from_nearest_threat(&self) -> (f32, f32) {
        let me = self.player_position;
        let at = |e: Entity| self.world.get::<&Transform>(e).ok().map(|t| t.position);
        let pacers = self.enemies.iter().filter_map(|p| at(p.entity));
        let specials = self.specials.iter().filter_map(|s| at(s.entity));
        let hunter = self.hunter.iter().filter_map(|(e, _, _)| at(*e));
        let fresh = self.newfoes.iter().filter_map(|f| at(f.entity)); // enemies-a
        pacers
            .chain(specials)
            .chain(hunter)
            .chain(fresh)
            .map(|p| Vec3::new(me.x - p.x, 0.0, me.z - p.z))
            .filter(|d| d.length() > 1e-3)
            .min_by(|a, b| a.length().total_cmp(&b.length()))
            .map_or((0.0, 0.0), |d| {
                let n = d.normalize();
                (n.x, n.z)
            })
    }
    // === end combat ===

    // === combat === ENEMIES-B: moths, mask, leech
    /// Puts the dream's new enemies in (placement and gates: `foespawn`,
    /// `moths`, `mask`, `leech`). DREAMSCAPE_ENEMY=Moths|Mask|Leech forces
    /// that one into every dream (and past the depth gate).
    fn spawn_enemies_b(&mut self, gl: &engine::glow::Context, dream: &Dream) -> anyhow::Result<()> {
        if self.coop_active() || self.coop.is_some() || self.prologue.is_some() || self.enemy_free() {
            return Ok(());
        }
        let forced = crate::dev::var("DREAMSCAPE_ENEMY").ok();
        let (depth, nightmare) = if forced.is_some() {
            (99, false)
        } else {
            (dream.depth, self.director.nightmare)
        };
        let theme = self.director.theme;
        let floor = shard_rules::floor_cells(&dream.blocks);
        let route: Vec<Vec3> = dream
            .lucid_route
            .iter()
            .chain(dream.route.iter())
            .map(|w| w.pos)
            .collect();
        let mut keep = vec![dream.spawn, dream.portal];
        keep.extend(dream.shard);
        let mut taken: Vec<Vec3> = Vec::new();
        let mut hints: Vec<&'static str> = Vec::new();
        for kind in [EnemyKind::Moths, EnemyKind::Mask, EnemyKind::Leech] {
            if taken.len() >= foespawn::MAX_NEW {
                break;
            }
            if forced.as_deref().is_some_and(|f| f != format!("{kind:?}")) {
                continue;
            }
            let plan = match kind {
                EnemyKind::Moths => moths::spawn_plan(depth, theme, dream.seed, nightmare),
                EnemyKind::Mask => mask::spawn_plan(depth, theme, dream.seed, nightmare),
                _ => leech::spawn_plan(depth, theme, dream.seed, nightmare),
            };
            let Some(&seed) = plan.first() else {
                continue;
            };
            let Some(at) = foespawn::place(seed, 0, &floor, &route, &keep, &taken) else {
                log::info!("No room for {kind:?}");
                continue;
            };
            taken.push(at);
            match kind {
                EnemyKind::Moths => {
                    let orb = self.mesh(Shape::Orb)?;
                    let dim = self.texture(gl, [190, 165, 255, 255]);
                    let bright = self.texture(gl, [255, 255, 225, 255]);
                    self.b_tex.insert("moth", dim.clone());
                    self.b_tex.insert("moth_bright", bright);
                    let swarm = moths::Swarm::new(at);
                    for m in &swarm.moths {
                        let entity = self.world.spawn((
                            Transform { position: m.pos, rotation: Quat::IDENTITY, scale: Vec3::splat(0.45) },
                            MeshRenderer { mesh: orb.clone(), texture: Some(dim.clone()) },
                            Lit,
                            Spin(5.0),
                        ));
                        self.specials.push(SpecialEnemy { entity, kind, ai: SpecialAI::Moth(m.id) });
                    }
                    self.moth_swarm = Some(swarm);
                }
                EnemyKind::Mask => {
                    let cube = self.mesh(Shape::Cube)?;
                    for (key, color) in [
                        ("mask", [232, 226, 205, 255]),
                        ("mask_tell", [255, 110, 80, 255]),
                        ("mask_down", [110, 120, 150, 255]),
                    ] {
                        let t = self.texture(gl, color);
                        self.b_tex.insert(key, t);
                    }
                    let a = at + Vec3::Y * 0.5;
                    let b = foespawn::neighbour(seed, at, &floor, &route).map_or(a, |n| n + Vec3::Y * 0.5);
                    let m = mask::Mask::new(a, b);
                    let entity = self.world.spawn((
                        Transform { position: a + Vec3::Y * 0.45, rotation: Quat::from_rotation_y(m.yaw), scale: Vec3::new(1.3, 1.9, 0.4) },
                        MeshRenderer { mesh: cube, texture: self.b_tex.get("mask").cloned() },
                        Lit,
                    ));
                    self.specials.push(SpecialEnemy { entity, kind, ai: SpecialAI::Mask(m) });
                }
                _ => {
                    let blob = self.mesh(Shape::Orb)?;
                    let tex = self.texture(gl, [170, 25, 55, 255]);
                    let pos = at + Vec3::Y * 0.35;
                    let entity = self.world.spawn((
                        Transform { position: pos, rotation: Quat::IDENTITY, scale: Vec3::new(0.9, 0.4, 0.55) },
                        MeshRenderer { mesh: blob, texture: Some(tex) },
                        Lit,
                    ));
                    self.specials.push(SpecialEnemy { entity, kind, ai: SpecialAI::Leech(leech::Leech::new(pos)) });
                }
            }
            log::info!("New enemy: {kind:?} at {at:?} (depth {})", dream.depth);
            if !self.seen_kinds.contains(&kind) {
                self.seen_kinds.push(kind);
                hints.push(kind.hint());
            }
        }
        if !hints.is_empty() {
            let more = hints.join(" · ");
            self.dream_hint = if self.dream_hint.is_empty() {
                more
            } else {
                format!("{} · {more}", self.dream_hint)
            };
        }
        Ok(())
    }

    /// Moths, masks and leeches, once per frame (called by `update_specials`).
    fn update_enemies_b(&mut self, dt: f32, held: bool) {
        let jumped = std::mem::take(&mut self.jump_event);
        if self.specials.is_empty() {
            return;
        }
        let player = self.player_position;
        let ps = gameplay::MOVE_SPEED;
        let reduced = self.settings.reduced_motion;
        let count = |s: &Option<moths::Swarm>| {
            s.as_ref().map_or((0, 0), |s| {
                (
                    s.moths.iter().filter(|m| matches!(m.mode, moths::Mode::Telegraph(..))).count(),
                    s.moths.iter().filter(|m| matches!(m.mode, moths::Mode::Dive(..))).count(),
                )
            })
        };
        let before = count(&self.moth_swarm);
        if let Some(sw) = &mut self.moth_swarm {
            sw.update(dt, player, ps, held);
        }
        let after = count(&self.moth_swarm);
        let mut sounds: Vec<Sound> = Vec::new();
        if after.0 > before.0 {
            sounds.push(Sound::MothTell);
        }
        if after.1 > before.1 {
            sounds.push(Sound::MothDive);
        }
        let mut touching: Vec<Entity> = Vec::new();
        let mut shaken: Vec<Entity> = Vec::new();
        for sp in &mut self.specials {
            match &mut sp.ai {
                SpecialAI::Moth(id) => {
                    let Some(m) = self.moth_swarm.as_ref().and_then(|s| s.moths.iter().find(|m| m.id == *id)) else {
                        continue;
                    };
                    if let Ok(mut t) = self.world.get::<&mut Transform>(sp.entity) {
                        t.position = m.pos;
                        // The tell: brighten, and (unless reduced motion) swell.
                        let swell = if reduced { 0.0 } else { 0.9 };
                        t.scale = Vec3::splat(0.45 * (1.0 + swell * m.glow()));
                    }
                    let key = if m.glow() > 0.25 { "moth_bright" } else { "moth" };
                    if let (Ok(mut r), Some(tex)) = (self.world.get::<&mut MeshRenderer>(sp.entity), self.b_tex.get(key)) {
                        r.texture = Some(tex.clone());
                    }
                }
                SpecialAI::Mask(mk) => {
                    let reeling = self.staggered.contains_key(&sp.entity);
                    let before = mk.state;
                    if !held && !reeling {
                        mk.update(dt, player, ps);
                    }
                    if matches!(before, mask::State::Telegraph(_)) && matches!(mk.state, mask::State::Slamming(_)) {
                        sounds.push(Sound::SlamLand);
                    }
                    if matches!(before, mask::State::Patrol) && matches!(mk.state, mask::State::Telegraph(_)) {
                        sounds.push(Sound::MaskTell);
                    }
                    if let Ok(mut t) = self.world.get::<&mut Transform>(sp.entity) {
                        t.position = mk.pos + Vec3::Y * 0.45;
                        t.rotation = Quat::from_rotation_y(mk.yaw);
                        let lean = if reduced { 0.0 } else { 0.25 };
                        t.scale = Vec3::new(1.3, 1.9, 0.4) * (1.0 + lean * mk.glow());
                    }
                    let key = if mk.glow() > 0.0 {
                        "mask_tell"
                    } else if !mk.shield_up() {
                        "mask_down"
                    } else {
                        "mask"
                    };
                    if let (Ok(mut r), Some(tex)) = (self.world.get::<&mut MeshRenderer>(sp.entity), self.b_tex.get(key)) {
                        r.texture = Some(tex.clone());
                    }
                }
                SpecialAI::Leech(lc) => {
                    if !held && !self.staggered.contains_key(&sp.entity) {
                        if lc.update(dt, player, ps).touching {
                            touching.push(sp.entity);
                        }
                        if lc.drain_due() {
                            sounds.push(Sound::LeechDrain);
                        }
                        if jumped && lc.latched() && lc.on_jump() {
                            shaken.push(sp.entity);
                        }
                    }
                    if let Ok(mut t) = self.world.get::<&mut Transform>(sp.entity) {
                        t.position = lc.pos;
                        t.rotation = Quat::from_rotation_y(mask::yaw_toward(lc.pos, player));
                        // It swells as the next drain nears.
                        let swell = if reduced { 0.0 } else { 0.5 * lc.drain_progress() };
                        t.scale = Vec3::new(0.9, 0.4, 0.55) * (1.0 + swell);
                    }
                }
                _ => {}
            }
        }
        for s in sounds {
            self.sfx(s);
        }
        for e in shaken {
            self.knock_leech(e, false);
        }
        // A touch latches, unless you are untouchable; WARD blocks it.
        let can = !self.autopilot
            && self.grace <= 0.0
            && !self.run.active(Ability::Dash)
            && !self.run.active(Ability::Phase)
            && !self.coop_ghost();
        for e in touching {
            if !can {
                continue;
            }
            if self.run.active(Ability::Ward) {
                self.break_ward();
                self.knock_leech(e, false);
                continue;
            }
            // === abilities2 ===
            if self.run.active(Ability::Mirror) {
                self.break_mirror();
                self.knock_leech(e, false);
                continue;
            }
            // === end abilities2 ===
            if let Some(SpecialAI::Leech(lc)) = self.specials.iter_mut().find(|s| s.entity == e).map(|s| &mut s.ai) {
                lc.latch_on();
            }
            self.sfx(Sound::Latch);
            if !reduced {
                self.flash.trigger([0.7, 0.1, 0.3], 0.2);
            }
            log::info!("A leech latched on (depth {})", self.director.depth);
        }
    }

    /// Throws a leech off you: it lands a step away and reels 1.5s. With
    /// `only_latched`, a leech that is not on you is left alone.
    fn knock_leech(&mut self, entity: Entity, only_latched: bool) {
        let me = self.player_position;
        let away = -self.flat_facing();
        let Some(SpecialAI::Leech(lc)) = self.specials.iter_mut().find(|s| s.entity == entity).map(|s| &mut s.ai) else {
            return;
        };
        if only_latched && !lc.latched() {
            return;
        }
        lc.release(me, away);
        self.staggered.insert(entity, leech::STAGGER_SECS);
        self.sfx(Sound::LeechShake);
        log::info!("A leech was shaken off");
    }

    /// A strike landed on a leech: if it was riding you, it lets go.
    fn shake_leech(&mut self, entity: Entity) {
        self.knock_leech(entity, true);
    }

    /// Being caught (the respawn) shakes every leech off.
    fn release_leeches(&mut self) {
        let latched: Vec<Entity> = self
            .specials
            .iter()
            .filter(|s| matches!(&s.ai, SpecialAI::Leech(l) if l.latched()))
            .map(|s| s.entity)
            .collect();
        for e in latched {
            self.knock_leech(e, true);
        }
    }

    /// Would the mask's shield turn a strike from `me` aside?
    fn mask_glances(&self, entity: Entity, me: Vec3) -> bool {
        self.specials
            .iter()
            .any(|s| s.entity == entity && matches!(&s.ai, SpecialAI::Mask(m) if m.blocks(me)))
    }

    /// The blow glances off: a spark, a bounce, a flash. No damage, no stagger.
    fn mask_glance(&mut self, at: Vec3) {
        self.sfx(Sound::Bounce);
        self.burst_pending = Some((at + Vec3::Y * 0.6, [255, 235, 150], [255, 255, 255]));
        if !self.settings.reduced_motion {
            self.flash.trigger([1.0, 0.95, 0.7], 0.12);
        }
    }

    /// Gust (push > 0) scatters moths and spins masks; Slam (no push) cracks
    /// mask shields.
    fn rattle_b(&mut self, radius: f32, push: f32) {
        let p = self.player_position;
        if push > 0.0 {
            if let Some(s) = &mut self.moth_swarm {
                s.scatter(p, radius);
            }
        }
        for sp in &mut self.specials {
            if let SpecialAI::Mask(m) = &mut sp.ai {
                let d = m.pos - p;
                if Vec3::new(d.x, 0.0, d.z).length() < radius {
                    if push > 0.0 {
                        m.gust();
                    } else {
                        m.crack();
                    }
                }
            }
        }
    }
    // === end combat ===

    /// A passive just did its work: its card in the hand pulses.
    fn passive_fires(&mut self, u: Upgrade) {
        fire_passive(&mut self.passive_fired, &self.run, self.dream_age, u);
    }

    /// Which cards of the hand make each active synergy (by what they give:
    /// the card's ability and its passive). Halves that came from a pick
    /// instead of a card have nothing to link.
    fn hand_links(&self) -> Vec<hand::HandLink> {
        let gives = |i: usize, u: Upgrade| {
            let Some(a) = self.run.abilities.get(i).filter(|s| !s.spent).map(|s| s.ability) else {
                return false;
            };
            let card = self.card_for_slot(i, a).map(|n| &n.1);
            u == Upgrade::Learn(a)
                || card.is_some_and(|c| {
                    crate::powers::power_of(c).passive == u
                        || c.fused.is_some_and(|b| powers::power(b).passive == u)
                })
        };
        let n = if self.run.deck_slots > 0 { self.run.deck_slots } else { self.run.abilities.len() };
        self.run
            .synergies()
            .into_iter()
            .filter_map(|s| {
                let [(u1, _), (u2, _)] = s.parts();
                let a = (0..n).find(|&i| gives(i, u1))?;
                let b = (0..n).find(|&i| i != a && gives(i, u2)).or_else(|| gives(a, u2).then_some(a))?;
                Some(hand::HandLink {
                    a: a.min(b),
                    b: a.max(b),
                    name: s.name(),
                })
            })
            .collect()
    }

    fn hud_view(&self) -> hud::HudView {
        let target = self.objective_target();
        hud::HudView {
            mode: self.mode,
            time: self.time,
            pack_dry: self.booklet.lottery.pack_dry,
            depth: self.director.depth,
            best: self.best_depth,
            lucidity: self.director.lucidity,
            lucid_target: self.director.shards_to_wake,
            unbanked: self.director.shard_this_dream,
            strangeness: self.visual_strangeness(),
            title: self.dream_name.clone(),
            whisper: self.dream_whisper.clone(),
            ask_eye: self.can_ask_eye(),
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
                    cards: self.sorted_cards(self.settings.sort_loadout).into_iter().cloned().collect(),
                    sort: self.settings.sort_loadout.label(),
                    chosen: self.booklet.lore.deck.clone(), // === deck ===
                    slots: deck::DECK_MAX,
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
                    cards: self.sorted_cards(self.settings.sort_merge).into_iter().cloned().collect(),
                    sort: self.settings.sort_merge.label(),
                    selected: self.merge_sel,
                    first: self.merge_first,
                    status: self.merge_status.clone(),
                    resonant_found: fusion::RESONANT
                        .iter()
                        .map(|&(a, b)| self.booklet.lore.recipes.contains(&fusion::recipe(a, b)))
                        .collect(),
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
            code: if self.mode == hud::Mode::Code {
                code_ui::CodeView {
                    digits: self.code_digits.clone(),
                    error: self.code_error.clone(),
                }
            } else {
                code_ui::CodeView::default()
            },
            run_code: if self.mode == hud::Mode::Summary {
                self.run_code()
            } else {
                String::new()
            },
            code_copied: self.code_copied,
            title_age: self.title_age,
            warning_age: self.warning_age,
            pickup_ring: self.juice.ring(),
            pickup_chain: self.juice.chain_len(),
            door_glow: self.juice.door_glow(),
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
                let all = self.sorted_cards(self.settings.sort_booklet);
                all[cards::page_range(all.len(), self.booklet_page)]
                    .iter()
                    .map(|c| (*c).clone())
                    .collect()
            } else {
                Vec::new()
            },
            booklet_sort: self.settings.sort_booklet.label(),
            booklet_page: self.booklet_page,
            booklet_pages: cards::page_count(self.booklet.card_count()),
            booklet_total: self.booklet.card_count(),
            booklet_attune: if self.mode == hud::Mode::Booklet {
                attunement::line(&attunement::counts(self.booklet.cards()))
            } else {
                String::new()
            },
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
                self.title_goal(&loadout)
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
            tickets: if self.mode == hud::Mode::Store && self.store_tab == 2 {
                self.tickets_view()
            } else {
                tickets_ui::TicketsView::default()
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
            hand: self.hand_view(),
            hand_links: self.hand_links(),
            hand_pile: self.hand_pile(), // === deck ===
            foe_pips: Vec::new(),
            boss_hp: self.hunter.as_ref().map(|(_, b, _)| (b.hp / b.max_hp, b.vulnerable())),
            // === combat ===
            nerve: if self.coop_active() { (0, 0) } else { (self.nerve.cur(), self.nerve.max()) },
            nerve_flinch: self.nerve.flinch(),
            // === end combat ===
            twist: {
                // === shards ===
                let t = self.twists.label();
                if self.heavy_elapsed().is_some_and(|e| e < shard_rules::HEAVY_SECONDS) {
                    if t.is_empty() { "heavy".to_string() } else { format!("{t} · heavy") }
                } else {
                    t
                }
                // === end shards ===
            },
            twist_blurb: self
                .twists
                .0
                .iter()
                .map(|v| v.blurb())
                .collect::<Vec<_>>()
                .join("; "),
            timer: self.dream_timer().map(|t| t - self.dream_age),
            // === pressure ===
            pressure_lid: (self.pressure.phase() != pressure::Phase::Calm).then(|| {
                (
                    self.pressure.lid(),
                    self.pressure.phase() != pressure::Phase::Stirring,
                )
            }),
            // === end pressure ===
            difficulty: self.director.hardness().map(|_| self.difficulty()),
            summary: if self.mode == hud::Mode::Summary {
                self.summary_view()
            } else {
                summary_ui::SummaryView::default()
            },
            // === abilities3 === the wager's roll shows for a moment
            hint: match &self.wager_line {
                Some((line, _)) => line.clone(),
                None => self.dream_hint.clone(),
            },
            // === end abilities3 ===
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
        // === combat === the swing's flash
        if let Some(start) = self.slash_pending.take() {
            let cube = self.mesh(Shape::Cube)?;
            let spark_mesh = self.mesh(Shape::Octahedron)?;
            // Dim violet at the dreamer, pink in the middle, white at the tip.
            let tex = [
                self.texture(gl, [150, 110, 255, 255]),
                self.texture(gl, [255, 170, 240, 255]),
                self.texture(gl, [255, 255, 255, 255]),
            ];
            let spark_tex = self.texture(gl, [255, 245, 200, 255]);
            let rays = (0..combat::SLASH_RAYS)
                .map(|_| {
                    [0, 1, 2].map(|j| {
                        let mut b = hecs::EntityBuilder::new();
                        b.add(Transform {
                            position: self.player_position,
                            rotation: Quat::IDENTITY,
                            scale: Vec3::ZERO,
                        });
                        b.add(MeshRenderer {
                            mesh: cube.clone(),
                            texture: Some(tex[j].clone()),
                        });
                        b.add(Lit);
                        // The dim and pink pieces glow (see-through); the tip is solid.
                        if j < 2 {
                            b.add(Translucent);
                        }
                        self.world.spawn(b.build())
                    })
                })
                .collect();
            let sparks = (0..combat::SLASH_SPARKS)
                .map(|_| {
                    self.world.spawn((
                        Transform {
                            position: self.player_position,
                            rotation: Quat::IDENTITY,
                            scale: Vec3::ZERO,
                        },
                        MeshRenderer {
                            mesh: spark_mesh.clone(),
                            texture: Some(spark_tex.clone()),
                        },
                        Spin(9.0),
                        Lit,
                        Translucent,
                    ))
                })
                .collect();
            self.slashes.push(Slash {
                rays,
                sparks,
                age: 0.0,
                facing: start.facing,
                reach: start.reach,
                half_arc: start.half_arc,
                flip: start.flip,
            });
        }
        for at in std::mem::take(&mut self.pending_motes) {
            let mesh = self.mesh(Shape::Octahedron)?;
            let tex = self.texture(gl, [150, 255, 210, 255]);
            let e = self.world.spawn((
                Transform {
                    position: at + Vec3::Y * 0.5,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::splat(0.35),
                },
                MeshRenderer {
                    mesh,
                    texture: Some(tex),
                },
                Spin(2.5),
                Lit,
                Hero,
            ));
            self.motes.push((e, 0.0));
            self.sfx(Sound::Mote);
        }
        // === end combat ===
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
        // STEP: the stair of air, and its end.
        if let Some(spots) = self.step_pending.take() {
            let cube = self.mesh(Shape::Cube)?;
            let tint = self
                .ability_tints
                .iter()
                .find(|t| t.0 == Ability::Step)
                .map_or([150, 200, 255], |&(_, a, _)| a);
            let tex = self.texture(gl, [tint[0], tint[1], tint[2], 255]);
            let size = Vec3::new(2.6, 0.3, 2.6);
            for at in spots {
                let e = self.world.spawn((
                    Transform {
                        position: at,
                        rotation: Quat::IDENTITY,
                        scale: size,
                    },
                    MeshRenderer {
                        mesh: cube.clone(),
                        texture: Some(tex.clone()),
                    },
                    Lit,
                    Collider {
                        shape: ColliderShape::Aabb {
                            half_extents: size * 0.5,
                        },
                        is_trigger: false,
                    },
                ));
                self.steps.push(e);
            }
        }
        if !self.run.active(Ability::Step) {
            for e in self.steps.drain(..) {
                let _ = self.world.despawn(e);
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

    /// Roadmap 5.5: the tension layer follows the nearest enemy and the
    /// run's difficulty.
    fn update_adaptive_music(&mut self, dt: f32) {
        let here = self.player_position;
        let nearest = self
            .enemies
            .iter()
            .filter_map(|p| self.world.get::<&Transform>(p.entity).ok())
            .map(|t| t.position.distance(here))
            .min_by(f32::total_cmp);
        let target = music::intensity(nearest, self.difficulty());
        self.music_intensity = music::follow(self.music_intensity, target, dt);
        if let Some(audio) = self.audio.as_mut() {
            audio.set_music_layer_gain(self.music_intensity * 0.8);
        }
    }

    /// Roadmap 5.1: DASH leaves afterimages, STILLNESS turns the frozen
    /// enemies blue, PHASE makes the dreamer see-through.
    fn update_ability_fx(
        &mut self,
        gl: &engine::glow::Context,
        player: Entity,
        dt: f32,
    ) -> anyhow::Result<()> {
        let now = self.dream_age;
        // DASH: a fading ghost every few hundredths of a second.
        self.ghost_timer -= dt;
        if self.run.active(Ability::Dash)
            && !self.settings.reduced_motion
            && self.ghost_timer <= 0.0
        {
            self.ghost_timer = gameplay::GHOST_EVERY;
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
                self.dash_ghosts.push((e, now));
            }
        }
        let mut gone = Vec::new();
        for (k, &(e, born)) in self.dash_ghosts.iter().enumerate() {
            let s = gameplay::ghost_scale(now - born);
            if s <= 0.0 {
                gone.push(k);
            } else if let Ok(mut t) = self.world.get::<&mut Transform>(e) {
                t.scale = Vec3::new(0.9, 1.25, 0.9) * s;
            }
        }
        for k in gone.into_iter().rev() {
            let (e, _) = self.dash_ghosts.remove(k);
            let _ = self.world.despawn(e);
        }

        // STILLNESS: frozen enemies go blue until it ends.
        let frozen = self.run.active(Ability::Stillness);
        if frozen && self.chilled.is_empty() {
            let blue = self.texture(gl, gameplay::CHILL_RGBA);
            for p in &self.enemies {
                if let Ok(mut r) = self.world.get::<&mut MeshRenderer>(p.entity) {
                    if let Some(old) = r.texture.replace(blue.clone()) {
                        self.chilled.push((p.entity, old));
                    }
                }
            }
        } else if !frozen && !self.chilled.is_empty() {
            for (e, old) in self.chilled.drain(..) {
                if let Ok(mut r) = self.world.get::<&mut MeshRenderer>(e) {
                    r.texture = Some(old);
                }
            }
        }

        // PHASE: see-through while it lasts.
        let phasing = self.run.active(Ability::Phase);
        if phasing != self.phased {
            self.phased = phasing;
            let tex = if phasing {
                Some(self.texture(gl, gameplay::PHASE_RGBA))
            } else {
                self.player_tex.clone()
            };
            if let Ok(mut r) = self.world.get::<&mut MeshRenderer>(player) {
                r.texture = tex;
            }
            if phasing {
                let _ = self.world.insert_one(player, Translucent);
            } else if !self.hurt_ghost {
                let _ = self.world.remove_one::<Translucent>(player);
            }
        }
        // === combat === faint while invulnerable after a hit
        let faint = self.nerve.faint(self.time, self.settings.reduced_motion);
        if faint != self.hurt_ghost {
            self.hurt_ghost = faint;
            if faint {
                let _ = self.world.insert_one(player, Translucent);
            } else if !self.phased {
                let _ = self.world.remove_one::<Translucent>(player);
            }
        }
        // === end combat ===
        Ok(())
    }

    /// ECHO dreams: where the ghost stands now (none in a shared dream, which
    /// has its own ghosts).
    fn echo_ghost_pos(&self) -> Option<Vec3> {
        if !self.twists.has_echo_ghost() || self.coop_active() {
            return None;
        }
        dream::echo_ghost(&self.trail, self.dream_age)
    }

    /// ECHO dreams: draws the ghost where the dreamer was five seconds ago.
    fn update_echo_ghost(&mut self) -> anyhow::Result<()> {
        let at = self.echo_ghost_pos();
        let scale = Vec3::new(0.9, 1.25, 0.9);
        match (self.echo_ghost, at) {
            (Some(e), Some(p)) => {
                if let Ok(mut t) = self.world.get::<&mut Transform>(e) {
                    t.position = p;
                    t.scale = scale;
                }
            }
            (Some(e), None) => {
                if let Ok(mut t) = self.world.get::<&mut Transform>(e) {
                    t.scale = Vec3::ZERO;
                }
            }
            (None, Some(p)) => {
                if let Some(tex) = self.player_tex.clone() {
                    let e = self.world.spawn((
                        Transform {
                            position: p,
                            rotation: Quat::IDENTITY,
                            scale,
                        },
                        MeshRenderer {
                            mesh: self.mesh(Shape::Octahedron)?,
                            texture: Some(tex),
                        },
                        Translucent,
                        Lit,
                    ));
                    self.echo_ghost = Some(e);
                }
            }
            (None, None) => {}
        }
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

    /// The wake door hums more and more as you walk toward it.
    fn update_door_hum(&mut self, dt: f32) {
        let door = self
            .shard_entities
            .first()
            .filter(|&&e| self.world.get::<&WakeMarker>(e).is_ok())
            .and_then(|&e| self.world.get::<&Transform>(e).ok().map(|t| t.position));
        let dist = door.map(|d| {
            Vec3::new(d.x - self.player_position.x, 0.0, d.z - self.player_position.z).length()
        });
        // Real time, not the dipped dt: the hum keeps its own rhythm.
        let hum = self.juice.door(dt, dist);
        if let Some(ratio) = hum {
            self.sfx_pitched(Sound::DoorHum, ratio);
        }
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

    // === hazards ===
    /// This dream's theme rule, if one applies (depth 3+, normal dreams).
    fn hazard_rule_state(&self) -> Option<theme_rules::RuleState> {
        theme_rules::RuleState::new(
            self.director.theme,
            gamble::hazard_depth(self.director.depth, self.gamble.wager_active), // === risk ===
            !self.director.nightmare
                && !self.enemy_free()
                && self.prologue.is_none()
                && self.coop.is_none()
                && self.director.theme != DreamTheme::Awakening,
            self.director.dream_seed() ^ 0x4841_5A41_5244,
        )
    }

    /// Shrinking floor: the outer cells go, shaking a second first. The tiles
    /// are the same one-cell slabs `update_crumbles` uses; none is ever on the
    /// route or its margin (see `theme_rules::shrink_order`).
    fn update_shrink(&mut self) {
        let Some(r) = self.theme_rule else { return };
        let Some(sf) = self.shrink.as_mut() else { return };
        let n = theme_rules::shrink_count(r.age, r.strength, sf.order.len());
        let soon = theme_rules::shrink_count(r.age + 1.0, r.strength, sf.order.len());
        let mut fell = false;
        for i in sf.gone..n {
            if let Some((e, _)) = sf.tiles.remove(&sf.order[i]) {
                let _ = self.world.despawn(e);
                fell = true;
            }
        }
        sf.gone = sf.gone.max(n);
        for i in n..soon {
            if let Some(&(e, base)) = sf.tiles.get(&sf.order[i]) {
                if let Ok(mut tr) = self.world.get::<&mut Transform>(e) {
                    tr.position = base + Vec3::new(0.06 * (self.time * 40.0).sin(), -0.03, 0.0);
                }
            }
        }
        if fell {
            self.sfx(Sound::Crumble);
        }
    }
    // === end hazards ===

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
                // === combat === ENEMIES-B are built by `spawn_enemies_b`
                EnemyKind::Moths | EnemyKind::Mask | EnemyKind::Leech => continue,
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
                EnemyKind::Moths | EnemyKind::Mask | EnemyKind::Leech => continue,
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
                let fog_e = self.world.spawn((
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
                self.fog_discs.push((fog_e, at)); // hazards
            }
            hints.push("purple fog slows you down");
        }
        self.dream_hint = hints.join(" · ");
        Ok(())
    }

    // === enemies-a ===
    /// Puts the dream's Tooth, Weeper and Lullaby in their lairs.
    fn spawn_newfoes(
        &mut self,
        gl: &engine::glow::Context,
        dream: &Dream,
        player_speed: f32,
    ) -> anyhow::Result<()> {
        // Dev switch: DREAMSCAPE_ENEMY=Tooth|Weeper|Lullaby in every dream.
        let dev = crate::dev::var("DREAMSCAPE_ENEMY")
            .ok()
            .and_then(|n| newfoes::forced(&n));
        let allowed = !self.coop_active()
            && self.prologue.is_none()
            && !self.director.nightmare
            && !self.enemy_free()
            && (!self.autopilot || dev.is_some());
        if !allowed {
            return Ok(());
        }
        let plan = match dev {
            Some(k) => newfoes::forced_plan(k),
            None => newfoes::plan(dream.depth, dream.theme, dream.seed, false),
        };
        self.foe_walls = dream
            .blocks
            .iter()
            .filter(|b| b.kind == BlockKind::Wall)
            .map(|b| (b.pos - b.size * 0.5, b.pos + b.size * 0.5))
            .collect();
        // With the dev switch, crowd them near the spawn so a screenshot sees them.
        let mut lairs = dream.lairs.clone();
        if dev.is_some() {
            lairs.sort_by(|a, b| {
                (*a - dream.spawn).length().total_cmp(&(*b - dream.spawn).length())
            });
            lairs.truncate(4);
            // DREAMSCAPE_ENEMY_NEAR=1 (screenshots): right in front of the spawn.
            if crate::dev::var("DREAMSCAPE_ENEMY_NEAR").is_ok() {
                let s = dream.spawn;
                lairs = vec![s + Vec3::new(-2.0, 0.0, 8.0), s + Vec3::new(5.0, 0.0, 10.0)];
            }
        }
        // The two enemy sets share one budget (the moths, mask and leech were
        // placed first).
        let b_count = self
            .specials
            .iter()
            .filter(|s| matches!(s.kind, EnemyKind::Moths | EnemyKind::Mask | EnemyKind::Leech))
            .count();
        let room = if crate::dev::var("DREAMSCAPE_ENEMY").is_ok() {
            usize::MAX
        } else {
            combat::MAX_NEW_FOES.saturating_sub(b_count)
        };
        for (kind, lair) in newfoes::place(&plan, &lairs, dream.seed).into_iter().take(room) {
            let (shape, base, calm, tell) = match kind {
                combat::Kind::Tooth => (
                    Shape::Cone,
                    Vec3::new(0.8, 1.3, 0.8),
                    [235, 228, 200, 255],
                    [255, 50, 40, 255],
                ),
                combat::Kind::Weeper => (
                    Shape::Cylinder,
                    Vec3::new(0.7, 1.9, 0.7),
                    [110, 150, 235, 255],
                    [225, 245, 255, 255],
                ),
                _ => (
                    Shape::Cube,
                    Vec3::new(1.9, 2.4, 1.9),
                    [140, 90, 170, 255],
                    [255, 200, 110, 255],
                ),
            };
            let calm_tex = self.texture(gl, calm);
            let tell_tex = self.texture(gl, tell);
            let at = Vec3::new(lair.x, lair.y + (base.y * 0.5).max(0.5), lair.z);
            let entity = self.world.spawn((
                Transform { position: at, rotation: Quat::IDENTITY, scale: base },
                MeshRenderer { mesh: self.mesh(shape)?, texture: Some(calm_tex.clone()) },
                Lit,
            ));
            let mut extras = Vec::new();
            let part = |g: &mut Self, shape: Shape, color: [u8; 4], translucent: bool| -> anyhow::Result<Entity> {
                let tex = g.texture(gl, color);
                let e = g.world.spawn((
                    Transform { position: at, rotation: Quat::IDENTITY, scale: Vec3::ZERO },
                    MeshRenderer { mesh: g.mesh(shape)?, texture: Some(tex) },
                    Lit,
                ));
                if translucent {
                    g.world.insert_one(e, Translucent)?;
                }
                Ok(e)
            };
            let ai = match kind {
                combat::Kind::Tooth => NewAI::Tooth(tooth::Tooth::new(at, player_speed)),
                combat::Kind::Weeper => {
                    for _ in 0..weeper::MAX_TEARS {
                        extras.push(part(self, Shape::Orb, [90, 230, 255, 255], false)?);
                    }
                    NewAI::Weeper(weeper::Weeper::new(at, player_speed))
                }
                _ => {
                    // The armour plate on its front, the slam marker, the ring.
                    extras.push(part(self, Shape::Cube, [235, 200, 110, 255], false)?);
                    extras.push(part(self, Shape::Torus, [255, 110, 80, 255], true)?);
                    for _ in 0..SLAM_SEGMENTS {
                        extras.push(part(self, Shape::Cube, [255, 90, 150, 255], false)?);
                    }
                    NewAI::Lullaby(lullaby::Lullaby::new(at, player_speed))
                }
            };
            log::info!("New enemy: {kind:?} at {at:?}");
            self.newfoes.push(NewFoe {
                entity,
                kind,
                ai,
                base,
                floor_y: lair.y,
                calm: calm_tex,
                tell: tell_tex,
                extras,
                heard: (false, false, 0),
            });
        }
        Ok(())
    }

    /// Thinks for each new enemy and mirrors it (and its tears, plate,
    /// marker and ring) onto its entities.
    fn update_newfoes(&mut self, dt: f32, held: bool) {
        if self.newfoes.is_empty() {
            return;
        }
        let player = self.player_position;
        let reduced = self.settings.reduced_motion;
        let aim = self.decoy.map_or(player, |(at, _)| at);
        let yaw_of = |f: Vec3| Quat::from_rotation_y(f.x.atan2(f.z));
        // What they do out loud this frame, played once the loop lets go of `self`.
        let mut heard: Vec<(Sound, Vec3)> = Vec::new();
        for f in &mut self.newfoes {
            let reeling = held || self.staggered.contains_key(&f.entity);
            // The body first (one Transform borrowed at a time: hecs checks
            // borrows per component column, not per entity).
            let (telling, pos) = {
                let Ok(mut t) = self.world.get::<&mut Transform>(f.entity) else {
                    continue;
                };
                let telling = match &mut f.ai {
                    NewAI::Tooth(a) => {
                        if !held {
                            a.update(&mut t.position, player, dt);
                        }
                        t.scale = f.base * a.scale_mult(reduced);
                        t.rotation = yaw_of(a.facing());
                        a.eye_lit()
                    }
                    NewAI::Weeper(a) => {
                        if !reeling {
                            a.update_aimed(&mut t.position, player, aim, &self.foe_walls, dt);
                        }
                        t.scale = f.base * a.scale_mult(reduced);
                        t.rotation = yaw_of(a.facing());
                        a.telling()
                    }
                    NewAI::Lullaby(a) => {
                        if !reeling {
                            a.update(&mut t.position, player, dt);
                        }
                        t.scale = f.base * a.scale_mult(reduced);
                        t.rotation = yaw_of(a.facing());
                        matches!(a.phase(), lullaby::Phase::Telegraph(_))
                    }
                };
                (telling, t.position)
            };
            // Then what it carries: tears, plate, marker, ring.
            match &f.ai {
                NewAI::Tooth(_) => {}
                NewAI::Weeper(a) => {
                    for (k, e) in f.extras.iter().enumerate() {
                        if let Ok(mut te) = self.world.get::<&mut Transform>(*e) {
                            match a.tears.get(k) {
                                Some(tear) => {
                                    te.position = tear.pos;
                                    te.scale = Vec3::splat(0.55);
                                }
                                None => te.scale = Vec3::ZERO,
                            }
                        }
                    }
                }
                NewAI::Lullaby(a) => {
                    let centre = Vec3::new(pos.x, f.floor_y, pos.z);
                    for (k, e) in f.extras.iter().enumerate() {
                        let Ok(mut te) = self.world.get::<&mut Transform>(*e) else {
                            continue;
                        };
                        match k {
                            0 => {
                                te.position = pos + a.facing() * (f.base.x * 0.5 + 0.1);
                                te.rotation = yaw_of(a.facing());
                                te.scale = Vec3::new(1.5, 1.7, 0.25);
                            }
                            1 => match a.marker() {
                                Some(r) => {
                                    te.position = centre + Vec3::Y * 0.06;
                                    te.rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
                                    te.scale = Vec3::new(2.0 * r, 2.0 * r, 0.05);
                                }
                                None => te.scale = Vec3::ZERO,
                            },
                            _ => match a.ring() {
                                Some(radius) => {
                                    let n = SLAM_SEGMENTS;
                                    let ang = std::f32::consts::TAU * (k - 2) as f32 / n as f32;
                                    let out = Vec3::new(ang.cos(), 0.0, ang.sin());
                                    let arc = std::f32::consts::TAU * radius / n as f32;
                                    te.position = centre + out * radius + Vec3::Y * boss::RING_H * 0.5;
                                    te.rotation = Quat::from_rotation_y(-ang + std::f32::consts::FRAC_PI_2);
                                    te.scale = Vec3::new(arc * 1.05, boss::RING_H, boss::RING_T);
                                }
                                None => te.scale = Vec3::ZERO,
                            },
                        }
                    }
                }
            }
            if let Ok(mut r) = self.world.get::<&mut MeshRenderer>(f.entity) {
                r.texture = Some(if telling { f.tell.clone() } else { f.calm.clone() });
            }
            // === sounds === each tell, lunge, spit and slam is heard once
            let (acting, tears) = match &f.ai {
                NewAI::Tooth(a) => (a.armed(), 0),
                NewAI::Weeper(a) => (false, a.tears.len()),
                NewAI::Lullaby(a) => (matches!(a.phase(), lullaby::Phase::Slam(_)), 0),
            };
            let now = (telling, acting, tears);
            for s in newfoes::cues(f.kind, f.heard, now) {
                heard.push((s, pos));
            }
            f.heard = now;
            // === end sounds ===
        }
        for (s, at) in heard {
            self.sfx_at(s, at);
        }
    }
    // === end enemies-a ===

    /// Specials, crumbling floor and the jester, once per frame.
    fn update_specials(&mut self, dt: f32, held: bool) -> anyhow::Result<()> {
        let player = self.player_position;
        self.trail.push(self.dream_age, player);
        self.update_enemies_b(dt, held); // === combat ===
        self.jester_cooldown = (self.jester_cooldown - dt).max(0.0);
        let strangeness = self.visual_strangeness();
        let eclipse = self
            .hunter
            .as_ref()
            .map_or(1.0, |(_, b, _)| b.sight_scale());
        let clear =
            (tunnel::sight_radius(strangeness)
                * self.twists.sight()
                * self.run.sight()
                * self.active_form.sight_mult()
                * eclipse
                * self // hazards
                    .theme_rule
                    .map_or(1.0, |r| theme_rules::dimmed_sight(1.0, &r.effect()))
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
        // === tactics ===
        let tactic_depth = self.director.depth;
        let mimic_extra = tactics::mimic_extra_delay(tactic_depth);
        let mimic_now = self
            .specials
            .iter()
            .find(|sp| sp.kind == EnemyKind::Mimic)
            .and_then(|_| {
                specials::mimic_pos(&self.trail, self.dream_age - mimic_extra, self.mimic_awake_at)
            });
        // === end tactics ===
        let mut calls = Vec::new();
        for sp in &mut self.specials {
            let Ok(mut t) = self.world.get::<&mut Transform>(sp.entity) else {
                continue;
            };
            // === combat === a struck special reels in place
            if self.staggered.contains_key(&sp.entity) && !matches!(sp.ai, SpecialAI::Mimic) {
                if let SpecialAI::Stalker(s) = &mut sp.ai {
                    s.moving = false;
                }
                continue;
            }
            // === end combat ===
            match &mut sp.ai {
                SpecialAI::Stalker(s) => {
                    if !held {
                        s.update(&mut t.position, player, clear, yaw, dt);
                    } else {
                        s.moving = false;
                    }
                }
                SpecialAI::Mimic => {
                    match specials::mimic_pos(&self.trail, self.dream_age - mimic_extra, self.mimic_awake_at) { // tactics
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
                    if !held && s.update_scaled(at, player, dt, tactics::sentry_boost(tactic_depth, at, s.facing(), mimic_now)) { // tactics
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
                // === combat === ENEMIES-B run in `update_enemies_b`
                SpecialAI::Moth(_) | SpecialAI::Mask(_) | SpecialAI::Leech(_) => {}
                // === end combat ===
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
        // === tactics ===
        let watchers: Vec<Vec3> = self
            .specials
            .iter()
            .filter(|sp| matches!(sp.ai, SpecialAI::Stalker(_) | SpecialAI::Sentry(..)))
            .filter_map(|sp| self.world.get::<&Transform>(sp.entity).ok().map(|t| t.position))
            .collect();
        let depth = self.director.depth;
        let landing = self.dream.as_ref().and_then(|d| {
            let landings: Vec<Option<Vec3>> = specials::throw_directions(seed)
                .into_iter()
                .map(|dir| {
                    gameplay::blink_target(&d.blocks, player, dir, specials::JESTER_THROW)
                })
                .collect();
            tactics::jester_dir(depth, &landings, &watchers).and_then(|i| landings[i])
        });
        // === end tactics ===
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
        let hunter = self.hunter.iter().filter(|(_, b, _)| !b.defeated).any(|(e, _, _)| {
            at(*e).is_some_and(|pos| {
                // Flat distance: it's too big to jump over.
                let d = pos - player;
                Vec3::new(d.x, 0.0, d.z).length() < hunter::HUNTER_TOUCH
            })
        });
        let special = self.specials.iter().any(|sp| {
            let armed = match &sp.ai {
                SpecialAI::Stalker(s) => s.moving,
                SpecialAI::Mimic => {
                    self.dream_age - tactics::mimic_extra_delay(self.director.depth)
                        >= self.mimic_awake_at // tactics
                }
                SpecialAI::Drifter(_) => true,
                SpecialAI::Sentry(..) | SpecialAI::Jester(_) => false,
                // === combat === these three have their own touches (below)
                SpecialAI::Moth(_) | SpecialAI::Mask(_) | SpecialAI::Leech(_) => false,
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
        // === enemies-a ===
        let tooth = self.newfoes.iter().any(|f| {
            matches!(&f.ai, NewAI::Tooth(t) if t.armed())
                && at(f.entity).is_some_and(|pos| gameplay::touches(pos, player, gameplay::ENEMY_TOUCH_RADIUS))
        });
        let tear = self
            .newfoes
            .iter()
            .any(|f| matches!(&f.ai, NewAI::Weeper(w) if w.touching(player)));
        let slam = self.newfoes.iter().any(|f| {
            matches!(&f.ai, NewAI::Lullaby(l) if at(f.entity).is_some_and(|pos| {
                l.hits(Vec3::new(pos.x, 0.0, pos.z), player - Vec3::Y * f.floor_y)
            }))
        });
        // === end enemies-a ===
        // === combat === ENEMIES-B
        let moth = self.moth_swarm.as_ref().is_some_and(|s| s.connected());
        let mask = self.specials.iter().any(|sp| matches!(&sp.ai, SpecialAI::Mask(m) if m.slam_hits(player)));
        let leech = self.specials.iter().any(|sp| matches!(&sp.ai, SpecialAI::Leech(l) if l.drain_due()));
        // === end combat ===
        [
            (pacers, "patrol"),
            (hunter, "hunter"),
            (boss_fx, "boss attack"),
            (slam, "slam"), // enemies-a
            (tooth, "tooth"), // enemies-a
            (tear, "tear"), // enemies-a
            (special, "special enemy"),
            (gate, "gate"),
            (beat, "beat tile"),
            (moth, "moth"),
            (mask, "mask"),
            (leech, "leech"),
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

    /// The vertical speed of a plain jump right now.
    fn jump_speed(&self) -> f32 {
        let feature = self
            .dream
            .as_ref()
            .map_or(dream::Feature::None, |d| d.theme.spec().feature);
        gameplay::JUMP_SPEED * self.run.jump() * self.twists.jump() * feature.jump()
    }

    fn set_player_vy(&mut self, vy: f32) {
        if let Some(p) = self.player {
            if let Ok(mut b) = self.world.get::<&mut RigidBody>(p) {
                b.velocity.y = vy;
            }
        }
    }

    /// The facing, flat on the ground (forward if you've not moved yet).
    fn flat_facing(&self) -> Vec3 {
        let f = Vec3::new(self.facing.x, 0.0, self.facing.z).normalize_or_zero();
        if f == Vec3::ZERO {
            Vec3::Z
        } else {
            f
        }
    }

    /// Knocks every enemy within `radius` back `push` units and stuns it.
    fn rattle_enemies(&mut self, radius: f32, push: f32, stun: f32) {
        let player = self.player_position;
        for p in &self.enemies {
            if let Ok(mut t) = self.world.get::<&mut Transform>(p.entity) {
                let d = t.position - player;
                let flat = Vec3::new(d.x, 0.0, d.z);
                if flat.length() < radius {
                    t.position += flat.normalize_or_zero() * push;
                    self.stunned.insert(p.entity, stun);
                }
            }
        }
        // === enemies-a === the new enemies reel too
        for f in &mut self.newfoes {
            if let Ok(mut t) = self.world.get::<&mut Transform>(f.entity) {
                let d = t.position - player;
                let flat = Vec3::new(d.x, 0.0, d.z);
                if flat.length() < radius {
                    t.position += flat.normalize_or_zero() * push;
                    match &mut f.ai {
                        NewAI::Tooth(a) => a.hurt(),
                        NewAI::Weeper(a) => {
                            a.hurt();
                            self.staggered.insert(f.entity, stun);
                        }
                        NewAI::Lullaby(a) => {
                            a.stagger();
                            self.staggered.insert(f.entity, stun);
                        }
                    }
                }
            }
        }
        // === end enemies-a ===
        self.rattle_b(radius, push); // === combat ===
    }

    /// Loud abilities carry: every enemy within `radius` that isn't reeling
    /// turns toward you.
    fn alert_enemies(&mut self, radius: f32) {
        // === hazards ===
        let radius = self
            .theme_rule
            .map_or(radius, |r| theme_rules::heard_radius(radius, &r.effect()));
        // === end hazards ===
        let player = self.player_position;
        for p in &mut self.enemies {
            if self.stunned.contains_key(&p.entity) {
                continue;
            }
            if let Ok(t) = self.world.get::<&Transform>(p.entity) {
                let d = t.position - player;
                if Vec3::new(d.x, 0.0, d.z).length() < radius {
                    p.ai.alert();
                }
            }
        }
    }

    /// WARD: it takes the hit meant for you.
    fn break_ward(&mut self) {
        for a in &mut self.run.abilities {
            if a.ability == Ability::Ward {
                a.active = 0.0;
            }
        }
        self.grace = self.grace.max(upgrades::WARD_GRACE);
        self.flash.trigger([1.0, 0.9, 0.5], 0.5);
        self.burst_pending = Some((self.player_position, [255, 230, 140], [255, 255, 255]));
        self.sfx(Sound::WardBreak);
        log::info!("The ward broke");
    }

    /// SLAM, once the hop is over: smash down, then a shockwave on landing.
    fn update_slam(&mut self) {
        if self.slam == 0 {
            return;
        }
        if !self.run.active(Ability::Slam) {
            self.slam = 0;
            return;
        }
        let Some(p) = self.player else {
            return;
        };
        let Some((grounded, vy)) = self
            .world
            .get::<&RigidBody>(p)
            .ok()
            .map(|b| (b.grounded, b.velocity.y))
        else {
            return;
        };
        let y = self.player_position.y;
        self.slam_apex = self.slam_apex.max(y);
        if self.slam == 1 && vy <= 0.0 {
            self.slam = 2;
        }
        if self.slam == 2 {
            if grounded {
                self.slam = 0;
                // The higher the hop, the bigger the wave (and a LEAP first makes it a meteor).
                let (radius, stun) =
                    upgrades::slam_power(self.slam_apex - y, std::mem::take(&mut self.slam_meteor));
                self.rattle_enemies(radius, 0.0, stun);
                // === combat === the smash hurts what it staggers
                if !self.coop_active() {
                    let at = self.player_position;
                    self.shock(at, radius, 1.0, stun.max(combat::STAGGER_SECS));
                }
                // === end combat ===
                self.alert_enemies(upgrades::SLAM_NOISE);
                self.flash.trigger([1.0, 0.8, 0.4], 0.45);
                self.burst_pending = Some((self.player_position, [255, 190, 90], [255, 240, 200]));
                self.sfx(Sound::SlamLand);
                // The smash is over: let the recharge run.
                for a in &mut self.run.abilities {
                    if a.ability == Ability::Slam {
                        a.active = 0.0;
                    }
                }
            } else {
                self.set_player_vy(-upgrades::SLAM_FALL_SPEED);
            }
        }
    }

    fn use_ability(&mut self, slot: usize) {
        self.fire_ability(slot, false);
    }

    /// `borrowed`: the state is a temporary one made by WAGER or ECHO (see
    /// `fire_borrowed`), not a card in hand.
    fn fire_ability(&mut self, slot: usize, borrowed: bool) {
        let Some(state) = self.run.abilities.get(slot).copied() else {
            return;
        };
        // === deck === a used card is gone until its slot draws again
        if !borrowed
            && (state.spent || (self.run.deck_slots > 0 && slot >= self.run.deck_slots))
        {
            return;
        }
        // === end deck ===
        let mult = self.run.cooldown_for(state.ability);
        let stretch = self.run.duration_for(state.ability) * self.borrow_stretch; // === abilities3 ===
        // === abilities3 === the card in this slot, before it is played
        let slot_card = self.deck.as_ref().and_then(|d| d.hand().get(slot).copied().flatten());
        let origin = self.player_position; // combat: where you were
        if state.cooldown > 0.0 {
            self.sfx(Sound::Denied);
            return;
        }
        // === abilities2 === nothing to act on: the charge is kept
        if !self.abilities2_ready(state.ability) {
            self.sfx(Sound::Denied);
            return;
        }
        // === end abilities2 ===
        // === abilities3 === refused with nothing to act on (the charge is kept)
        if !self.abilities3_ready(state.ability, slot_card) {
            self.sfx(Sound::Denied);
            return;
        }
        // OVERLOAD: the key down starts the charge; the release fires it.
        if state.ability == Ability::Overload && !borrowed && self.overload_fire.is_none() {
            if self.overload.is_none() {
                self.overload = Some(abilities3::Charge::new(slot));
                self.sfx(Sound::OverloadCharge);
            }
            return;
        }
        // === end abilities3 ===
        // Chained soon after another ability: a combo.
        let combo = self
            .last_ability
            .filter(|&(_, t)| self.dream_age - t <= upgrades::COMBO_WINDOW)
            .and_then(|(prev, _)| upgrades::combo(prev, state.ability));
        // Blink or Dash into a cracked wall: it gives way (see below).
        let cracks = self.secret_in_reach(state.ability);
        if state.ability == Ability::Blink {
            let target = self.dream.as_ref().and_then(|d| {
                gameplay::blink_target(
                    &d.blocks,
                    self.player_position,
                    self.facing,
                    upgrades::BLINK_CELLS,
                )
            });
            match target {
                Some(at) => self.teleport_player(at),
                // Striking the crack spends the charge even with nowhere to land.
                None if cracks => {}
                None => {
                    // Nowhere safe to land: don't spend the charge.
                    self.sfx(Sound::Denied);
                    return;
                }
            }
        }
        // ANCHOR is two-stage: the first use drops the marker (almost no
        // recharge), the second snaps you back and starts the real recharge.
        if state.ability == Ability::Anchor {
            match self.anchor.take() {
                None => {
                    if let (Some(tex), Ok(mesh)) =
                        (self.player_tex.clone(), self.mesh(Shape::Octahedron))
                    {
                        let at = self.player_position;
                        let e = self.world.spawn((
                            Transform {
                                position: at - Vec3::Y * 0.4,
                                rotation: Quat::IDENTITY,
                                scale: Vec3::new(0.5, 0.8, 0.5),
                            },
                            MeshRenderer {
                                mesh,
                                texture: Some(tex),
                            },
                            Spin(-2.0),
                            Lit,
                            Hero,
                        ));
                        self.anchor = Some((at, e, 0.0));
                        self.run.abilities[slot].cooldown = 0.8;
                        self.flash.trigger([0.6, 0.9, 1.0], 0.25);
                        self.sfx(Sound::AnchorDrop);
                        log::info!("Anchor dropped");
                    }
                    return;
                }
                Some((at, e, _)) => {
                    let _ = self.world.despawn(e);
                    self.anchor_snapped = true; // combat
                    self.teleport_player(at);
                }
            }
        }
        match state.ability {
            Ability::Leap => {
                let rise = if combo == Some(upgrades::Combo::Rise) {
                    upgrades::RISE_MULT
                } else {
                    1.0
                };
                let vy = self.jump_speed() * upgrades::LEAP_MULT * rise;
                self.set_player_vy(vy);
                self.air_jumps_used = 0;
            }
            Ability::Slam => {
                self.slam_apex = self.player_position.y;
                self.slam_meteor = combo == Some(upgrades::Combo::Meteor);
                if self.player_grounded {
                    let hop = self.jump_speed() * 0.7;
                    self.set_player_vy(hop);
                    self.slam = 1;
                } else {
                    self.slam = 2;
                    self.set_player_vy(-upgrades::SLAM_FALL_SPEED);
                }
            }
            Ability::Gust => {
                let (push, stun) = match combo {
                    Some(upgrades::Combo::Shockwave) => (
                        upgrades::GUST_PUSH * upgrades::SHOCKWAVE_PUSH_MULT,
                        upgrades::GUST_STUN * upgrades::SHOCKWAVE_STUN_MULT,
                    ),
                    Some(upgrades::Combo::Bait) => (
                        upgrades::GUST_PUSH,
                        upgrades::GUST_STUN * upgrades::BAIT_STUN_MULT,
                    ),
                    _ => (upgrades::GUST_PUSH, upgrades::GUST_STUN),
                };
                self.rattle_enemies(upgrades::GUST_RADIUS, push, stun);
                self.alert_enemies(upgrades::GUST_NOISE);
                // === combat === shoved into a wall hurts
                let me = self.player_position;
                let walled: Vec<CombatTarget> = self
                    .combat_targets()
                    .into_iter()
                    .filter(|t| t.pos.distance(me) <= upgrades::GUST_RADIUS + push && self.in_wall(t.pos))
                    .collect();
                for t in &walled {
                    self.hurt_foe(t, 1.0, me, 0.0, stun);
                }
                // === end combat ===
            }
            Ability::Dash => self.alert_enemies(upgrades::DASH_NOISE),
            Ability::Hush if combo == Some(upgrades::Combo::Sanctuary) => {
                for a in &mut self.run.abilities {
                    if a.ability == Ability::Ward && a.active > 0.0 {
                        a.active += upgrades::SANCTUARY_EXTRA;
                    }
                }
            }
            Ability::Step => {
                let dir = self.flat_facing();
                let feet = self.player_position.y - 1.0;
                let spots: Vec<Vec3> = (1..=upgrades::STEP_COUNT)
                    .map(|i| {
                        let top = feet + upgrades::STEP_RISE * i as f32;
                        let flat = self.player_position + dir * gameplay::CELL * i as f32;
                        Vec3::new(flat.x, top - 0.15, flat.z)
                    })
                    .collect();
                self.step_pending = Some(spots);
            }
            _ => {}
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
            // === combat ===
            self.combat_on_use(state.ability, origin);
            self.anchor_snapped = false;
            // === end combat ===
            self.abilities2_on_use(state.ability, mult, stretch); // === abilities2 ===
            let wager = self.abilities3_before_play(slot, state.ability); // === abilities3 ===
            if cracks {
                self.open_secret();
            }
            self.deck_on_use(slot); // === deck ===
            self.abilities3_after_play(state.ability, slot_card, wager); // === abilities3 ===
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
                Ability::Leap => [1.0, 0.9, 0.5],
                Ability::Slam => [1.0, 0.7, 0.3],
                Ability::Anchor => [0.6, 0.9, 1.0],
                Ability::Ward => [1.0, 0.9, 0.6],
                Ability::Step => [0.6, 0.8, 1.0],
                Ability::Hush => [0.7, 0.6, 1.0],
                Ability::Gust => [0.8, 1.0, 0.9],
                // === abilities2 ===
                Ability::Tether => [0.4, 1.0, 0.7],
                Ability::Mirror => [0.85, 0.9, 1.0],
                Ability::Quake => [0.8, 0.6, 0.4],
                Ability::Veil => [0.5, 0.45, 0.8],
                Ability::Surge => [1.0, 0.85, 0.3],
                Ability::Pulse => [0.6, 1.0, 1.0],
                Ability::Haste => [1.0, 0.6, 0.5],
                Ability::Bind => [0.6, 0.9, 0.4],
                // === end abilities2 ===
                // === abilities3 ===
                Ability::Wager => [1.0, 0.85, 0.4],
                Ability::Echo => [0.7, 0.8, 1.0],
                Ability::Swap => [0.9, 0.6, 1.0],
                Ability::Mulligan => [0.95, 0.95, 0.8],
                Ability::Recall => [0.6, 1.0, 0.9],
                Ability::Overload => [1.0, 0.5, 0.3],
                Ability::Bargain => [0.8, 0.2, 0.3],
                Ability::Snare => [0.7, 0.9, 0.5],
                // === end abilities3 ===
            };
            let tint = self
                .ability_tints
                .iter()
                .find(|t| t.0 == state.ability)
                .map(|&(_, a, b)| (a, b));
            let color = tint.map_or(color, |(a, _)| a.map(|c| c as f32 / 255.0));
            self.flash.trigger(color, 0.3);
            // Every ability bursts; a card's tint only changes the colours.
            let (a, b) = tint.unwrap_or_else(|| gameplay::burst_colors(color));
            self.burst_pending = Some((self.player_position, a, b));
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
                Ability::Leap => Sound::Leap,
                Ability::Slam => Sound::Deeper,
                Ability::Anchor => Sound::AnchorBack,
                Ability::Ward => Sound::WardUp,
                Ability::Step => Sound::StepRise,
                Ability::Hush => Sound::HushOn,
                Ability::Gust => Sound::Gust,
                // === abilities2 ===
                Ability::Tether => Sound::TetherPull,
                Ability::Mirror => Sound::MirrorUp,
                Ability::Quake => Sound::QuakeRumble,
                Ability::Veil => Sound::VeilOn,
                Ability::Surge => Sound::SurgeUp,
                Ability::Pulse => Sound::PulsePing,
                Ability::Haste => Sound::HasteOn,
                Ability::Bind => Sound::BindClamp,
                // === end abilities2 ===
                // === abilities3 ===
                Ability::Wager => Sound::WagerRoll,
                Ability::Echo => Sound::EchoRepeat,
                Ability::Swap => Sound::SwapPop,
                Ability::Mulligan => Sound::MulliganShuffle,
                Ability::Recall => Sound::RecallDraw,
                Ability::Overload => Sound::OverloadBlast,
                Ability::Bargain => Sound::BargainPact,
                Ability::Snare => Sound::SnarePlant,
                // === end abilities3 ===
            });
            self.last_ability = Some((state.ability, self.dream_age));
            // === abilities3 === ECHO repeats a real ability, never itself (or a wager's shell)
            if !matches!(state.ability, Ability::Echo | Ability::Wager) {
                self.echo_last = Some(state.ability);
            }
            // === end abilities3 ===
            self.ability_used.retain(|u| u.0 != state.ability);
            self.ability_used.push((state.ability, self.dream_age));
            if let Some(c) = combo {
                log::info!("Combo: {}", c.name());
                self.achievement_toast =
                    Some((format!("COMBO  {}: {}", c.name(), c.desc()), 0.0));
                self.flash.trigger([1.0, 0.95, 0.6], 0.4);
                self.sfx(Sound::Combo);
            }
        }
    }

    // === abilities2 ===
    /// Can `a` be used right now? (No target, or nothing underfoot: the
    /// charge is kept.)
    fn abilities2_ready(&self, a: Ability) -> bool {
        match a {
            Ability::Quake => abilities2::quake_allowed(self.player_grounded),
            Ability::Bind => self.bind_target().is_some(),
            Ability::Tether => self.tether_target().is_some(),
            _ => true,
        }
    }

    /// The nearest thing BIND can root.
    fn bind_target(&self) -> Option<CombatTarget> {
        let targets = self.combat_targets();
        let pts: Vec<Vec3> = targets.iter().map(|t| t.pos).collect();
        let i = abilities2::bind_pick(self.player_position, &pts)?;
        targets.into_iter().nth(i)
    }

    /// Roots one foe: a pacer is stunned, the rest stagger (a strike
    /// cuts a staggered foe deeper).
    fn bind_foe(&mut self, t: &CombatTarget) {
        if matches!(t.kind, combat::Kind::Pacer { .. }) {
            self.stunned.insert(t.entity, abilities2::BIND_SECS);
        } else {
            self.staggered.insert(t.entity, abilities2::BIND_SECS);
        }
    }

    /// The nearest shard or calm mote TETHER would pull: (where it is, the
    /// mote's entity or `None` for the shard).
    fn tether_target(&self) -> Option<(Vec3, Option<Entity>)> {
        let mut pts = Vec::new();
        let mut who = Vec::new();
        for (e, _) in &self.motes {
            if let Ok(t) = self.world.get::<&Transform>(*e) {
                pts.push(t.position);
                who.push(Some(*e));
            }
        }
        if let Some(&s) = self.shard_entities.first() {
            if let Ok(t) = self.world.get::<&Transform>(s) {
                pts.push(t.position);
                who.push(None);
            }
        }
        let i = abilities2::tether_pick(self.player_position, &pts)?;
        Some((pts[i], who[i]))
    }

    /// The effects that happen the moment an ability fires.
    fn abilities2_on_use(&mut self, a: Ability, _cd_mult: f32, stretch: f32) {
        let me = self.player_position;
        match a {
            Ability::Surge => self.surge.arm(),
            Ability::Dash => self.dash_boost = self.surge.take_dash(),
            Ability::Quake => {
                self.shock(me, abilities2::QUAKE_REACH, 0.0, abilities2::QUAKE_STAGGER);
                self.juice.hit(juice::Hit::Strike, self.settings.reduced_motion);
            }
            Ability::Bind => {
                if let Some(t) = self.bind_target() {
                    self.bind_foe(&t);
                }
            }
            Ability::Haste => {
                let secs = Ability::Haste.duration() * stretch;
                let disc = match (self.player_tex.clone(), self.mesh(Shape::Cylinder)) {
                    (Some(tex), Ok(mesh)) => Some(self.world.spawn((
                        Transform {
                            position: me - Vec3::Y * 0.95,
                            rotation: Quat::IDENTITY,
                            scale: Vec3::new(abilities2::HASTE_RADIUS * 2.0, 0.02, abilities2::HASTE_RADIUS * 2.0),
                        },
                        MeshRenderer { mesh, texture: Some(tex) },
                        Translucent,
                        Lit,
                    ))),
                    _ => None,
                };
                if let Some((_, Some(old))) = self.haste.take() {
                    let _ = self.world.despawn(old);
                }
                self.haste = Some((abilities2::HasteField::new(me, secs), disc));
            }
            Ability::Pulse => self.pulse_reveal(),
            _ => {}
        }
    }

    /// PULSE: a tall pillar over the cracked wall and over every Shade.
    fn pulse_reveal(&mut self) {
        let (Some(tex), Ok(mesh)) = (self.player_tex.clone(), self.mesh(Shape::Cube)) else {
            return;
        };
        let mut spots: Vec<Vec3> = Vec::new();
        if let Some(s) = self.secret.as_ref().filter(|s| s.wall.is_some()) {
            spots.push(s.at);
        }
        for p in self.enemies.iter().filter(|p| p.elite == Some(Elite::Shade)) {
            if let Ok(t) = self.world.get::<&Transform>(p.entity) {
                spots.push(t.position);
            }
        }
        for at in spots {
            let e = self.world.spawn((
                Transform {
                    position: at + Vec3::Y * abilities2::PULSE_PILLAR_HEIGHT * 0.5,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::new(0.2, abilities2::PULSE_PILLAR_HEIGHT, 0.2),
                },
                MeshRenderer { mesh: mesh.clone(), texture: Some(tex.clone()) },
                Translucent,
                Lit,
            ));
            self.pulse_marks.push(e);
        }
    }

    /// MIRROR: the blow is thrown back; the attacker is hurt, all near reel.
    fn break_mirror(&mut self) {
        for a in &mut self.run.abilities {
            if a.ability == Ability::Mirror {
                a.active = 0.0;
            }
        }
        self.grace = self.grace.max(abilities2::MIRROR_GRACE);
        let me = self.player_position;
        self.ward_reflect();
        self.shock(me, abilities2::MIRROR_REACH, 0.0, abilities2::MIRROR_STUN);
        self.flash.trigger([0.85, 0.9, 1.0], 0.5);
        self.burst_pending = Some((me, [200, 220, 255], [255, 255, 255]));
        self.sfx(Sound::MirrorBreak);
        log::info!("The mirror threw the blow back");
    }

    /// Per frame: TETHER pulls, the HASTE zone fades, SURGE and PULSE end.
    fn update_abilities2(&mut self, dt: f32) {
        if !self.run.active(Ability::Surge) {
            self.surge.clear();
        }
        if !self.run.active(Ability::Dash) {
            self.dash_boost = 1.0;
        }
        if self.run.active(Ability::Tether) {
            if let Some((pos, who)) = self.tether_target() {
                let next = abilities2::tether_step(pos, self.player_position, dt);
                match who {
                    Some(e) => {
                        if let Ok(mut t) = self.world.get::<&mut Transform>(e) {
                            t.position = next;
                        }
                    }
                    None => {
                        for &e in &self.shard_entities {
                            if let Ok(mut t) = self.world.get::<&mut Transform>(e) {
                                t.position += next - pos;
                            }
                        }
                    }
                }
            }
        }
        if let Some((mut f, disc)) = self.haste.take() {
            if f.tick(dt) {
                self.haste = Some((f, disc));
            } else if let Some(e) = disc {
                let _ = self.world.despawn(e);
            }
        }
        if !self.run.active(Ability::Pulse) {
            for e in self.pulse_marks.drain(..) {
                let _ = self.world.despawn(e);
            }
        }
    }
    // === end abilities2 ===

    // === abilities3 ===
    /// Is there anything for `a` to act on? (If not the charge is kept.)
    fn abilities3_ready(&self, a: Ability, slot_card: Option<u32>) -> bool {
        match a {
            Ability::Echo => abilities3::echo_target(self.echo_last).is_some(),
            Ability::Swap => self.swap_target().is_some(),
            Ability::Mulligan => self.deck.is_some(),
            Ability::Recall => self
                .deck
                .as_ref()
                .is_some_and(|d| d.recall_candidate(slot_card).is_some()),
            Ability::Bargain => abilities3::bargain_allowed(self.nerve.cur()),
            _ => true,
        }
    }

    /// Roots one foe for `secs`: a pacer is stunned, the rest stagger.
    fn root_foe(&mut self, t: &CombatTarget, secs: f32) {
        if matches!(t.kind, combat::Kind::Pacer { .. }) {
            self.stunned.insert(t.entity, secs);
        } else {
            self.staggered.insert(t.entity, secs);
        }
    }

    /// What SWAP would trade with: (the foe, or `None` for the decoy, and
    /// where it stands).
    fn swap_target(&self) -> Option<(Option<CombatTarget>, Vec3)> {
        let mut who: Vec<Option<CombatTarget>> = Vec::new();
        let mut pts: Vec<Vec3> = Vec::new();
        for t in self.combat_targets() {
            pts.push(t.pos);
            who.push(Some(t));
        }
        if let Some((at, _)) = self.decoy {
            pts.push(at);
            who.push(None);
        }
        let i = abilities3::swap_pick(self.player_position, &pts)?;
        let pos = pts[i];
        Some((who.swap_remove(i), pos))
    }

    /// Runs before the card leaves the hand: WAGER rolls (so its cooldown
    /// can be bent before the deck takes it) and SWAP/OVERLOAD/BARGAIN/SNARE act.
    fn abilities3_before_play(
        &mut self,
        slot: usize,
        a: Ability,
    ) -> Option<abilities3::WagerRoll> {
        let me = self.player_position;
        match a {
            Ability::Swap => {
                if let Some((foe, pos)) = self.swap_target() {
                    match foe {
                        Some(t) => {
                            if let Ok(mut tr) = self.world.get::<&mut Transform>(t.entity) {
                                tr.position = me;
                            }
                            self.root_foe(&t, abilities3::SWAP_STAGGER);
                        }
                        None => {
                            if let Some((_, e)) = self.decoy {
                                if let Ok(mut tr) = self.world.get::<&mut Transform>(e) {
                                    tr.position = me;
                                }
                                self.decoy = Some((me, e));
                            }
                        }
                    }
                    self.burst_pending = Some((me, [230, 160, 255], [255, 255, 255]));
                    self.teleport_player(pos);
                }
            }
            Ability::Overload => {
                let secs = self.overload_fire.take().unwrap_or(abilities3::OVERLOAD_AUTO);
                let reach = abilities3::overload_radius(secs);
                self.shock(me, reach, 0.0, abilities3::overload_stagger(secs));
                self.alert_enemies(upgrades::SLAM_NOISE);
                self.juice.hit(juice::Hit::Strike, self.settings.reduced_motion);
            }
            Ability::Bargain => {
                if self.nerve.spend(1) {
                    self.bargain.arm();
                }
            }
            Ability::Snare => self.plant_snare(),
            Ability::Wager => {
                let roll = abilities3::wager_roll(self.director.dream_seed(), self.wager_n, self.wager_dry);
                self.wager_n += 1;
                self.wager_dry = if roll.kind == abilities3::WagerKind::Jackpot { 0 } else { self.wager_dry + 1 };
                let timed = roll.ability.duration() > 0.0;
                let full = self.run.abilities.get(slot).map_or(0.0, |s| s.cooldown);
                if let Some(s) = self.run.abilities.get_mut(slot) {
                    s.cooldown = abilities3::wager_cooldown(roll.kind, full, timed);
                }
                self.wager_line = Some((abilities3::wager_line(roll), abilities3::WAGER_LINE_SECS));
                log::info!("Wager: {roll:?}");
                if roll.kind == abilities3::WagerKind::Jackpot {
                    self.sfx(Sound::WagerJackpot);
                }
                return Some(roll);
            }
            _ => {}
        }
        None
    }

    /// Runs after the card went to the discard: the effects that need the
    /// hand's empty slot (MULLIGAN, RECALL) and the borrowed ones (WAGER, ECHO).
    fn abilities3_after_play(
        &mut self,
        a: Ability,
        slot_card: Option<u32>,
        wager: Option<abilities3::WagerRoll>,
    ) {
        match a {
            Ability::Mulligan => {
                let snaps = self.hand_view();
                let thrown = self.deck.as_mut().map(|d| d.mulligan()).unwrap_or_default();
                for (slot, _) in thrown {
                    if let Some(snap) = snaps.get(slot).cloned() {
                        self.deck_leaving.push((snap, slot, self.deck_clock));
                    }
                }
                // Every slot waits for its redraw; none can be used meanwhile.
                for s in self.run.abilities.iter_mut().take(self.run.deck_slots) {
                    s.spent = true;
                }
                self.deck_riffle_at = self.deck_clock;
            }
            Ability::Recall => {
                if let Some(d) = self.deck.as_mut() {
                    d.recall(slot_card);
                }
            }
            Ability::Echo => {
                if let Some(target) = abilities3::echo_target(self.echo_last) {
                    self.fire_borrowed(target, 1.0);
                }
            }
            Ability::Wager => {
                if let Some(roll) = wager {
                    match roll.kind {
                        abilities3::WagerKind::Fizzle => {}
                        kind => {
                            let timed = roll.ability.duration() > 0.0;
                            let fired =
                                self.fire_borrowed(roll.ability, abilities3::wager_stretch(kind, timed));
                            if !fired {
                                self.wager_line = Some((
                                    format!("WAGER: {} (nothing to use it on)", roll.ability.name()),
                                    abilities3::WAGER_LINE_SECS,
                                ));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Uses `a` without it being in hand and without starting its own
    /// recharge: a temporary state is added, fired, and left to run out.
    /// False if it had nothing to act on.
    fn fire_borrowed(&mut self, a: Ability, stretch: f32) -> bool {
        self.run.abilities.push(upgrades::AbilityState::new(a));
        let idx = self.run.abilities.len() - 1;
        let held = std::mem::replace(&mut self.borrow_stretch, stretch);
        self.fire_ability(idx, true);
        self.borrow_stretch = held;
        let Some(st) = self.run.abilities.get_mut(idx) else {
            return false;
        };
        let fired = st.cooldown > 0.0;
        st.spent = true;
        st.cooldown = 0.0;
        if !fired || !st.is_active() {
            self.run.abilities.remove(idx);
        }
        fired
    }

    /// SNARE: a trap at your feet; the third takes up the oldest.
    fn plant_snare(&mut self) {
        let me = self.player_position;
        let entity = match (self.player_tex.clone(), self.mesh(Shape::Cylinder)) {
            (Some(tex), Ok(mesh)) => Some(self.world.spawn((
                Transform {
                    position: me - Vec3::Y * 0.95,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::new(abilities3::SNARE_RADIUS * 2.0, 0.03, abilities3::SNARE_RADIUS * 2.0),
                },
                MeshRenderer { mesh, texture: Some(tex) },
                Translucent,
                Lit,
            ))),
            _ => None,
        };
        if let Some(Some(old)) = self.snares.plant(me, entity) {
            let _ = self.world.despawn(old);
        }
    }

    /// Per frame: the OVERLOAD charge and its ring, BARGAIN's clock, the
    /// traps, the wager line.
    fn update_abilities3(&mut self, dt: f32) {
        if let Some((_, left)) = self.wager_line.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                self.wager_line = None;
            }
        }
        self.bargain.tick(dt);
        // OVERLOAD
        if let Some(mut c) = self.overload.take() {
            let still = self.run.abilities.get(c.slot).is_some_and(|s| s.ability == Ability::Overload && !s.spent);
            if !still {
                if let Some(e) = self.overload_ring.take() {
                    let _ = self.world.despawn(e);
                }
            } else {
                c.tick(dt);
                if self.ability_held[c.slot] && self.mode == hud::Mode::Playing {
                    self.overload = Some(c);
                    let r = abilities3::overload_radius(c.secs) * 2.0;
                    if self.overload_ring.is_none() {
                        if let (Some(tex), Ok(mesh)) = (self.player_tex.clone(), self.mesh(Shape::Cylinder)) {
                            self.overload_ring = Some(self.world.spawn((
                                Transform {
                                    position: self.player_position - Vec3::Y * 0.95,
                                    rotation: Quat::IDENTITY,
                                    scale: Vec3::new(r, 0.02, r),
                                },
                                MeshRenderer { mesh, texture: Some(tex) },
                                Translucent,
                                Lit,
                            )));
                        }
                    }
                    if let Some(e) = self.overload_ring {
                        if let Ok(mut t) = self.world.get::<&mut Transform>(e) {
                            t.position = self.player_position - Vec3::Y * 0.95;
                            t.scale = Vec3::new(r, 0.02, r);
                        }
                    }
                } else {
                    // Released: the blast.
                    if let Some(e) = self.overload_ring.take() {
                        let _ = self.world.despawn(e);
                    }
                    self.overload_fire = Some(c.secs);
                    self.fire_ability(c.slot, false);
                    self.overload_fire = None;
                }
            }
        }
        // SNARE
        if self.snares.len() != 0 {
            let pts: Vec<Vec3> = self.combat_targets().iter().map(|t| t.pos).collect();
            for (handle, i) in self.snares.step(&pts) {
                if let Some(e) = handle {
                    let _ = self.world.despawn(e);
                }
                if let Some(t) = self.combat_targets().into_iter().find(|t| t.pos == pts[i]) {
                    self.root_foe(&t, abilities3::SNARE_ROOT);
                }
                self.burst_pending = Some((pts[i], [180, 230, 130], [255, 255, 255]));
                self.sfx(Sound::SnareTrip);
            }
        }
    }
    // === end abilities3 ===

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

    /// A skittish shard: closing in makes it hop to somewhere else on the route.
    fn update_skittish(&mut self, dt: f32) {
        let Some(mut s) = self.skittish.take() else {
            return;
        };
        if self.shard_entities.is_empty() {
            return; // collected: nothing left to hop
        }
        if let Some((from, to, t)) = s.flight {
            let t = (t + dt / objective::FLINCH_TIME).min(1.0);
            let e = t * t * (3.0 - 2.0 * t);
            let arc = (t * std::f32::consts::PI).sin() * 3.2;
            let now = from.lerp(to, e) + Vec3::Y * arc;
            let delta = now - s.drawn;
            for &ent in &self.shard_entities {
                if let Ok(mut tr) = self.world.get::<&mut Transform>(ent) {
                    tr.position += delta;
                }
            }
            s.drawn = now;
            s.flight = if t >= 1.0 { None } else { Some((from, to, t)) };
            if t >= 1.0 {
                s.drawn = to;
                s.home = to;
            }
            self.skittish = Some(s);
            return;
        }
        let near = {
            let d = self.player_position - s.home;
            Vec3::new(d.x, 0.0, d.z).length() < objective::FLINCH_RANGE
                && (self.player_position.y - 1.0 - s.home.y).abs() < 2.5
        };
        if s.hops_left > 0 && near {
            let route: Vec<Vec3> = self
                .dream
                .as_ref()
                .map(|d| d.lucid_route.iter().map(|w| w.pos).collect())
                .unwrap_or_default();
            let seed = self.dream.as_ref().map_or(0, |d| d.seed);
            let pick = (seed as u32).wrapping_add(s.hops_left as u32 * 7919);
            match objective::flinch_dest(&route, self.player_position, s.home, pick) {
                Some(to) => {
                    s.hops_left -= 1;
                    s.flight = Some((s.home, to, 0.0));
                    self.sfx(Sound::Flinch);
                    self.achievement_toast = Some(("THE SHARD FLINCHES".to_string(), 0.0));
                    log::info!("The shard hops away");
                }
                None => s.hops_left = 0,
            }
        }
        self.skittish = Some(s);
    }

    /// Would `ability`, used right now, strike this dream's closed cracked wall?
    fn secret_in_reach(&self, ability: Ability) -> bool {
        self.secret.as_ref().is_some_and(|s| {
            s.wall.is_some()
                && secrets::strikes(ability, self.player_position, self.facing, s.at)
        })
    }

    /// The wall gives way: no collider, a door in its place.
    fn open_secret(&mut self) {
        let Some(sec) = self.secret.as_mut() else {
            return;
        };
        let Some((wall, glow)) = sec.wall.take() else {
            return;
        };
        let (at, height, tex) = (sec.at, sec.height, sec.door_tex.clone());
        let _ = self.world.despawn(wall);
        let _ = self.world.despawn(glow);
        if let Ok(mesh) = self.mesh(Shape::Cube) {
            // A warm slab filling the gap: lit, non-solid, a door.
            let door = self.world.spawn((
                Transform {
                    position: at + Vec3::Y * height * 0.5,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::new(gameplay::CELL * 0.7, height * 0.85, gameplay::CELL * 0.7),
                },
                MeshRenderer {
                    mesh,
                    texture: Some(tex),
                },
                Translucent,
                Lit,
            ));
            if let Some(sec) = self.secret.as_mut() {
                sec.door = Some(door);
            }
        }
        self.flash.trigger([1.0, 0.85, 0.55], 0.4);
        self.burst_pending = Some((at + Vec3::Y, [255, 214, 150], [255, 240, 210]));
        self.sfx(Sound::PortalOpen);
        self.achievement_toast = Some((secrets::OPENED.into(), 0.0));
        log::info!("Cracked wall opened at {at:?}");
    }

    /// Per frame: the first sight of a crack and a running DASH striking it.
    fn update_secret(&mut self) {
        let Some(sec) = self.secret.as_ref() else {
            return;
        };
        let at = sec.at;
        let closed = sec.wall.is_some();
        let (has_door, armed) = (sec.door.is_some(), sec.armed);
        let p = self.player_position;
        if closed {
            if !self.secret_hint_given && secrets::notices(p, at) {
                self.secret_hint_given = true;
                self.achievement_toast = Some((secrets::HINT.into(), 0.0));
            }
            if self.run.active(Ability::Dash) && secrets::strikes(Ability::Dash, p, self.facing, at) {
                self.open_secret();
            }
        } else if has_door {
            let inside = secrets::at_door(p, at);
            if !inside && !armed {
                if let Some(sec) = self.secret.as_mut() {
                    sec.armed = true;
                }
            } else if inside && armed && self.mode == hud::Mode::Playing {
                self.enter_moth_shop();
            }
        }
    }

    /// Through the door: the moth's room, as a shelf of cards over the frozen
    /// dream (so leaving puts you back exactly where you stood).
    fn enter_moth_shop(&mut self) {
        if self.mode != hud::Mode::Playing
            || self.choice.is_some()
            || self.transition.active()
            || self.coop.is_some()
            || self.prologue.is_some()
            || self.autopilot
        {
            return;
        }
        if let Some(sec) = self.secret.as_mut() {
            sec.armed = false;
        }
        self.ensure_moth_stock();
        log::info!("Entered the moth's room ({} dust)", self.bonus_dust);
        self.show_moth_shop(0);
    }

    // === shop ===
    fn ensure_moth_stock(&mut self) {
        if self.moth_stock.is_none() {
            self.moth_stock = Some(moth_shop::stock(
                self.director.dream_seed(),
                &self.run,
                self.director.depth,
                self.moth_terms(),
            ));
        }
    }

    /// The shop after a pick: same shelf as the secret room of this dream.
    fn open_moth_shop_after_pick(&mut self) {
        self.ensure_moth_stock();
        log::info!(
            "The moth's shop opens (depth {}, {} dust)",
            self.director.depth,
            self.bonus_dust
        );
        self.show_moth_shop(0);
    }

    /// Paints the shelf with the ordinary choice screen. The last card is
    /// the reroll.
    fn show_moth_shop(&mut self, selected: usize) {
        let shelf = self.moth_stock.clone().unwrap_or_default();
        let terms = self.moth_terms();
        let mut cards: Vec<upgrade_ui::ChoiceCard> = shelf
            .cards
            .iter()
            .enumerate()
            .map(|(i, &u)| {
                let info = u.info();
                let cost = moth_shop::price_at(&shelf, i, terms).unwrap_or(0);
                let can = moth_shop::buy(&shelf, i, self.bonus_dust, &self.run, terms).is_ok();
                let have = self.run.count(u);
                let level = if have > 0 {
                    format!(" · {} / {}", have + 1, info.max)
                } else {
                    String::new()
                };
                let special = if shelf.special == Some(i) { "special · " } else { "" };
                upgrade_ui::ChoiceCard {
                    name: info.name.into(),
                    desc: info.desc.into(),
                    icon: info.icon,
                    color: if can { info.color } else { [110, 110, 130] },
                    frame: if can { info.tier.frame() } else { [80, 80, 100] },
                    tag: if can {
                        format!("{special}{cost} dust · {}{level}", info.tier.label())
                    } else if self.bonus_dust < cost {
                        format!("needs {cost} dust")
                    } else {
                        "no use to you".into()
                    },
                }
            })
            .collect();
        // === toolcards === deck runs: the tool cards, after the cards
        let held = self.tool_cards_held();
        for (j, &a) in shelf.tools.iter().enumerate() {
            let bought = held.contains(&deck::tool_id(a));
            let cost = moth_shop::tool_price(a, held.len(), terms);
            let can = moth_shop::buy_tool(&shelf, shelf.cards.len() + j, self.bonus_dust, &held, terms).is_ok();
            cards.push(upgrade_ui::ChoiceCard {
                name: a.name().into(),
                desc: a.desc().into(),
                icon: upgrades::Icon::Gem,
                color: if can { hand::TOOL_FRAME } else { [110, 110, 130] },
                frame: if can { hand::TOOL_FRAME } else { [80, 80, 100] },
                tag: if bought {
                    moth_shop::TOOL_HELD.into()
                } else if held.len() >= moth_shop::MAX_TOOL_CARDS {
                    moth_shop::TOOL_NO_ROOM.into()
                } else if can {
                    format!("{cost} dust · {}", moth_shop::TOOL_TAG)
                } else {
                    format!("needs {cost} dust")
                },
            });
        }
        // === end toolcards ===
        // A sold-out shelf keeps the shop open: one EMPTY row stands first.
        if shelf.is_empty() {
            cards.push(upgrade_ui::ChoiceCard {
                name: "EMPTY".into(),
                desc: moth_shop::EMPTY.into(),
                icon: upgrades::Icon::Eye,
                color: [110, 110, 130],
                frame: [80, 80, 100],
                tag: "nothing for sale".into(),
            });
        }
        let rr = moth_shop::reroll_price(shelf.rerolls);
        let can_rr = self.bonus_dust >= rr;
        cards.push(upgrade_ui::ChoiceCard {
            name: "REROLL".into(),
            desc: "the moth shuffles her shelf. the special stays.".into(),
            icon: upgrades::Icon::Eye,
            color: if can_rr { [255, 210, 80] } else { [110, 110, 130] },
            frame: if can_rr { [255, 210, 80] } else { [80, 80, 100] },
            tag: if can_rr {
                format!("{rr} dust")
            } else {
                format!("needs {rr} dust")
            },
        });
        // === risk === the coin, after the reroll
        let shut = gamble::RunState::shut_reason(self.bonus_dust, self.gamble_visit);
        cards.push(upgrade_ui::ChoiceCard {
            name: gamble::ROW_NAME.into(),
            desc: gamble::ROW_DESC.into(),
            icon: upgrades::Icon::Gem,
            color: if shut.is_none() { [255, 150, 90] } else { [110, 110, 130] },
            frame: if shut.is_none() { [255, 150, 90] } else { [80, 80, 100] },
            tag: shut.map_or_else(|| "double or nothing".to_string(), str::to_string),
        });
        // === end risk ===
        let last = cards.len() - 1;
        self.choice_view = upgrade_ui::ChoiceView {
            title: moth_shop::TITLE.into(),
            subtitle: format!("you have {} dust · {}", self.bonus_dust, moth_shop::SUBTITLE),
            cards,
            selected: selected.min(last),
            age: 0.0,
            hint: "[a/d] choose   [enter] buy   [g] gamble   [x] leave".into(),
        };
        self.choice = Some(ChoiceKind::MothShop);
        self.input = PlayerInputState::default();
        self.mode = hud::Mode::Choice;
    }

    /// [enter] on a card: pay run dust for it (never the banked stash). The
    /// card past the last one is the reroll.
    fn buy_from_moth(&mut self, index: usize) {
        let shelf = self.moth_stock.clone().unwrap_or_default();
        // A sold-out shelf shows one EMPTY row first: it is not for sale.
        let index = if shelf.is_empty() {
            if index == 0 {
                self.sfx(Sound::Denied);
                self.show_moth_shop(0);
                return;
            }
            index - 1
        } else {
            index
        };
        let terms = self.moth_terms();
        if index == shelf.rows() + 1 {
            self.open_gamble_stakes(); // === risk ===
            return;
        }
        // === toolcards ===
        if index >= shelf.cards.len() && index < shelf.rows() {
            self.buy_tool_card(index);
            return;
        }
        // === end toolcards ===
        if index == shelf.rows() {
            match moth_shop::reroll(
                &shelf,
                self.director.dream_seed(),
                &self.run,
                self.director.depth,
                self.bonus_dust,
                terms,
            ) {
                Ok((next, cost)) => {
                    self.bonus_dust -= cost;
                    self.moth_stock = Some(next);
                    self.sfx(Sound::Reroll);
                    self.save_run();
                    log::info!("Moth rerolled the shelf for {cost} dust");
                }
                Err(why) => {
                    log::info!("Moth refused the reroll: {why:?}");
                    self.sfx(Sound::Denied);
                }
            }
            self.show_moth_shop(index);
            return;
        }
        match moth_shop::buy(&shelf, index, self.bonus_dust, &self.run, terms) {
            Ok((u, cost)) => {
                let before = self.run.synergies();
                self.bonus_dust -= cost;
                self.run.take(u);
                self.upgrade_taken_fx(u, &before);
                if let Some(s) = self.moth_stock.as_mut() {
                    s.remove(index);
                }
                // The card and its price are saved together, so a quit
                // right after can't keep one without the other.
                self.save_run();
                log::info!("Moth sold {u:?} for {cost} dust ({} left)", self.bonus_dust);
            }
            Err(why) => {
                log::info!("Moth refused: {why:?}");
                self.sfx(Sound::Denied);
            }
        }
        // Sold out: the shop stays open (EMPTY row, reroll, coin, leave).
        self.show_moth_shop(index);
    }

    #[cfg(test)]
    fn enter_moth_shop_for_test(&mut self) {
        self.ensure_moth_stock();
        self.show_moth_shop(0);
    }

    // === toolcards ===
    /// The tool card ids held this run: the live deck's, else the saved list.
    fn tool_cards_held(&self) -> Vec<u32> {
        match &self.deck {
            Some(d) => d.tool_cards(),
            None => self.tool_cards.clone(),
        }
    }

    /// A resumed run's saved tool cards go back into the deck if there is
    /// one (a resumed run keeps its fixed kit today, so this does nothing
    /// there and the list is only carried on in the save).
    fn restore_tool_cards(&mut self) {
        let seed = self.director.dream_seed();
        if let Some(d) = self.deck.as_mut() {
            for &id in &self.tool_cards {
                if deck::tool_ability(id).is_some() {
                    d.insert_shuffled(id, seed);
                }
            }
        }
    }

    /// [enter] on a tool card row: dust for a card shuffled into the draw
    /// pile for the rest of the run (never saved in the booklet).
    fn buy_tool_card(&mut self, index: usize) {
        let shelf = self.moth_stock.clone().unwrap_or_default();
        let terms = self.moth_terms();
        let held = self.tool_cards_held();
        let seed = self.director.dream_seed();
        match moth_shop::buy_tool(&shelf, index, self.bonus_dust, &held, terms) {
            Ok((a, cost)) if self.deck.is_some() => {
                self.bonus_dust -= cost;
                let id = deck::tool_id(a);
                if let Some(d) = self.deck.as_mut() {
                    d.insert_shuffled(id, seed);
                }
                self.tool_cards.push(id);
                self.deck_riffle_at = self.deck_clock; // the card flies into the stack
                self.save_run();
                log::info!("Moth sold the {a:?} tool card for {cost} dust ({} left)", self.bonus_dust);
            }
            Ok(_) => {}
            Err(why) => {
                log::info!("Moth refused the tool card: {why:?}");
                self.sfx(Sound::Denied);
            }
        }
        self.show_moth_shop(index);
    }
    // === end toolcards ===

    /// [x]: back to the dream, exactly where you stood.
    fn leave_moth_shop(&mut self) {
        if !matches!(self.choice, Some(ChoiceKind::MothShop)) {
            return;
        }
        self.choice = None;
        self.mode = hud::Mode::Playing;
        self.sfx(Sound::Reroll);
    }
    // === end shop ===

    // === risk ===
    /// The gamble row's index in the shop (after the shelf and the reroll).
    fn gamble_row(&self) -> usize {
        self.moth_stock.as_ref().map_or(0, |s| s.rows() + 1 + usize::from(s.is_empty()))
    }

    fn risk_allowed(&self) -> bool {
        !self.autopilot && self.prologue.is_none() && self.coop.is_none()
    }

    fn gamble_card(name: &str, desc: String, tag: String, hot: bool) -> upgrade_ui::ChoiceCard {
        let c = if hot { [255, 150, 90] } else { [150, 150, 175] };
        upgrade_ui::ChoiceCard {
            name: name.into(),
            desc,
            icon: upgrades::Icon::Gem,
            color: c,
            frame: c,
            tag,
        }
    }

    fn show_gamble(&mut self, stage: GambleStage, subtitle: String, cards: Vec<upgrade_ui::ChoiceCard>, hint: &str) {
        self.choice_view = upgrade_ui::ChoiceView {
            title: gamble::STAKE_TITLE.into(),
            subtitle,
            cards,
            selected: 0,
            age: 0.0,
            hint: hint.into(),
        };
        self.choice = Some(ChoiceKind::Gamble(stage));
        self.input = PlayerInputState::default();
        self.mode = hud::Mode::Choice;
    }

    /// Step one: choose a stake. The odds are on this screen, as rolled.
    fn open_gamble_stakes(&mut self) {
        let stakes = gamble::RunState::stakes(self.bonus_dust);
        if !self.risk_allowed() || stakes.is_empty() || self.gamble_visit >= gamble::MAX_PER_VISIT {
            self.sfx(Sound::Denied);
            let row = self.gamble_row();
            self.show_moth_shop(row);
            return;
        }
        let odds = self.gamble.odds();
        let mut cards: Vec<_> = stakes
            .iter()
            .map(|&s| Self::gamble_card(&format!("{s} DUST"), odds.line(), format!("win: you get {} · lose: {s} gone", s * 2), true))
            .collect();
        cards.push(Self::gamble_card("BACK", "keep your dust. nothing happens.".into(), "no stake".into(), false));
        self.show_gamble(
            GambleStage::Stakes,
            format!("you have {} dust · at most half of it · {} left this visit", self.bonus_dust, gamble::MAX_PER_VISIT - self.gamble_visit),
            cards,
            "[a/d] choose   [enter] pick a stake   [x] back",
        );
    }

    /// Step two: confirm. Nothing is staked until [enter] here.
    fn open_gamble_confirm(&mut self, stake: u32) {
        let odds = self.gamble.odds();
        let cards = vec![
            Self::gamble_card(&format!("FLIP FOR {stake}"), odds.line(), format!("win +{stake} · lose -{stake}"), true),
            Self::gamble_card("NOT NOW", "keep your dust. nothing happens.".into(), "no stake".into(), false),
        ];
        self.show_gamble(
            GambleStage::Confirm(stake),
            format!("{stake} run dust on the coin · you have {}", self.bonus_dust),
            cards,
            "[a/d] choose   [enter] decide   [x] back",
        );
        self.choice_view.selected = 1; // nothing is staked by default
    }

    fn choose_gamble(&mut self, stage: GambleStage, index: usize) {
        match stage {
            GambleStage::Stakes => {
                let stakes = gamble::RunState::stakes(self.bonus_dust);
                match stakes.get(index) {
                    Some(&s) => self.open_gamble_confirm(s),
                    None => self.gamble_back(),
                }
            }
            GambleStage::Confirm(stake) if index == 0 => self.flip_coin(stake),
            GambleStage::Confirm(_) => self.open_gamble_stakes(),
            GambleStage::Result => {
                let row = self.gamble_row();
                self.show_moth_shop(row);
            }
        }
    }

    /// [x] on a coin screen: one step back.
    fn gamble_back(&mut self) {
        match self.choice {
            Some(ChoiceKind::Gamble(GambleStage::Confirm(_))) => self.open_gamble_stakes(),
            Some(ChoiceKind::Gamble(_)) => {
                let row = self.gamble_row();
                self.show_moth_shop(row);
            }
            _ => {}
        }
    }

    /// The flip: decided now from its own seed stream, paid, and saved
    /// before the result is shown, so quitting cannot re-roll it.
    fn flip_coin(&mut self, stake: u32) {
        let allowed = self.risk_allowed()
            && gamble::RunState::stakes(self.bonus_dust).contains(&stake)
            && self.gamble_visit < gamble::MAX_PER_VISIT;
        if !allowed {
            self.sfx(Sound::Denied);
            self.open_gamble_stakes();
            return;
        }
        let was_owed = self.gamble.owed();
        let won = self.gamble.roll(self.director.dream_seed());
        self.gamble.settle(won);
        self.gamble_visit += 1;
        self.bonus_dust = gamble::after_flip(self.bonus_dust, stake, won);
        // A wagered dream doubles what it earned, not what the coin moved.
        self.wager_dust_start = if won {
            self.wager_dust_start + stake
        } else {
            self.wager_dust_start.saturating_sub(stake)
        };
        self.save_run();
        self.sfx(if won { Sound::Pick } else { Sound::Denied });
        log::info!("Moth coin: stake {stake}, won {won}, owed {was_owed}, dust {}", self.bonus_dust);
        let (name, desc, tag) = if won {
            ("THE COIN FELL YOUR WAY", format!("your {stake} dust came back doubled: +{stake}."), format!("+{stake} dust"))
        } else {
            ("THE COIN FELL HER WAY", format!("the moth keeps your {stake} dust."), format!("-{stake} dust"))
        };
        let cards = vec![Self::gamble_card(name, desc, tag, won)];
        self.show_gamble(
            GambleStage::Result,
            format!("you have {} dust", self.bonus_dust),
            cards,
            "[enter] back to the shelf",
        );
    }

    /// Per frame (Playing): standing in the second door asks for the deal.
    fn update_wager(&mut self) {
        let Some((at, armed)) = self.wager_portal else {
            return;
        };
        let inside = gamble::at_wager(self.player_position, at);
        if !inside && !armed {
            self.wager_portal = Some((at, true));
        } else if inside
            && armed
            && self.mode == hud::Mode::Playing
            && self.choice.is_none()
            && !self.transition.active()
            && self.risk_allowed()
        {
            self.wager_portal = Some((at, false));
            self.open_wager_deal();
        }
    }

    /// The plain deal screen: what gets harder, what pays double.
    fn open_wager_deal(&mut self) {
        let card = |name: &str, desc: &str, tag: &str, c: [u8; 3]| upgrade_ui::ChoiceCard {
            name: name.into(),
            desc: desc.into(),
            icon: upgrades::Icon::Eye,
            color: c,
            frame: c,
            tag: tag.into(),
        };
        self.choice_view = upgrade_ui::ChoiceView {
            title: gamble::WAGER_TITLE.into(),
            subtitle: gamble::WAGER_SUBTITLE.into(),
            cards: vec![
                card(gamble::WAGER_YES, gamble::WAGER_YES_DESC, "harder · x2 dust · extra pick", [255, 120, 200]),
                card(gamble::WAGER_NO, gamble::WAGER_NO_DESC, "no change", [150, 150, 175]),
            ],
            selected: 1, // nothing is taken by default
            age: 0.0,
            hint: "[a/d] choose   [enter] decide   [x] not now".into(),
        };
        self.choice = Some(ChoiceKind::Wager);
        self.input = PlayerInputState::default();
        self.mode = hud::Mode::Choice;
    }

    /// [enter] on the deal: 0 takes the door (the dream melts as through the
    /// ordinary portal, and the next one is wagered), 1 stays.
    fn choose_wager(&mut self, index: usize) {
        if index == 0 && self.risk_allowed() {
            self.wager_enter = true;
        }
    }

    /// Spawns this dream's second door, if it has one.
    fn spawn_wager_portal(&mut self, gl: &engine::glow::Context, dream: &Dream) -> anyhow::Result<()> {
        self.wager_portal = None;
        let next = self.director.depth + 1;
        let special = self.director.is_nightmare_depth(next) || self.director.is_sanctuary_depth(next);
        let normal = self.pressure_applies() && !self.autopilot && self.run_active;
        let Some(at) = gamble::wager_portal(self.director.dream_seed(), self.director.depth, normal, special, &dream.lairs) else {
            return Ok(());
        };
        let tex = self.texture(gl, [255, 120, 200, 255]);
        let mesh = self.mesh(Shape::Orb)?;
        self.world.spawn((
            Transform {
                position: Vec3::new(at.x, 0.7, at.z),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(0.9, 1.4, 0.9),
            },
            MeshRenderer { mesh, texture: Some(tex) },
            Spin(1.2),
            Lit,
        ));
        self.wager_portal = Some((at, true));
        log::info!("A wager portal stands at {at:?}");
        Ok(())
    }
    // === end risk ===

    /// What a freshly taken upgrade does around the edges (shared by the
    /// moth's shelf; the after-dream pick has its own copy of this).
    fn upgrade_taken_fx(&mut self, u: Upgrade, before: &[upgrades::Synergy]) {
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
        self.flash
            .trigger(u.info().color.map(|c| c as f32 / 255.0), 0.5);
        self.sfx(Sound::Pick);
    }

    /// High caches: reach the pillar and the gold is yours (a run bonus).
    fn update_caches(&mut self) {
        if self.caches.is_empty() {
            return;
        }
        let p = self.player_position;
        let depth = self.director.depth;
        let mut taken = Vec::new();
        self.caches.retain(|&(at, glow, beam)| {
            let flat = Vec3::new(p.x - at.x, 0.0, p.z - at.z).length();
            if flat < 1.4 && (p.y - 1.0 - at.y).abs() < 1.8 {
                taken.push((glow, beam));
                false
            } else {
                true
            }
        });
        for (glow, beam) in taken {
            let _ = self.world.despawn(glow);
            let _ = self.world.despawn(beam);
            let dust = 8 + 2 * depth.min(20);
            self.bonus_dust += dust;
            self.flash.trigger([1.0, 0.85, 0.3], 0.45);
            self.sfx(Sound::CacheGet);
            self.achievement_toast = Some((format!("HIGH CACHE  +{dust} DUST"), 0.0));
            log::info!("Cache: +{dust} dust (run bonus {})", self.bonus_dust);
        }
    }

    /// Climbing onto a higher terrace: a note that rises with the height.
    fn update_climb(&mut self) {
        let grounded = self
            .player
            .and_then(|p| self.world.get::<&RigidBody>(p).ok().map(|b| b.grounded))
            .unwrap_or(false);
        if !grounded {
            return;
        }
        let y = self.player_position.y;
        let tiered = self
            .dream
            .as_ref()
            .is_some_and(|d| d.terrain != dream::Terrain::Flat);
        if tiered && self.ground_y > f32::MIN && y - self.ground_y > 0.5 {
            let levels = y / dream::LEVEL_H;
            self.sfx_pitched(Sound::Climb, (1.0 + 0.045 * levels).min(2.4));
        }
        self.ground_y = y;
    }

    /// Leaving a dream through the portal: HURRIED and GILDED pay out.
    /// `DREAMSCAPE_BALANCE=path.csv` (dev builds): one row per finished dream.
    fn log_balance_row(&self) {
        let Some(path) = dev::var_os("DREAMSCAPE_BALANCE") else {
            return;
        };
        let theme = format!("{:?}", self.director.theme);
        let row = balance::Row {
            depth: self.director.depth,
            theme: &theme,
            difficulty: self.difficulty(),
            seconds: self.dream_age,
            catches: self.stats.caught,
            upgrades: self.run.taken.len(),
        };
        if let Err(e) = balance::append(std::path::Path::new(&path), &row) {
            log::warn!("could not write balance log: {e}");
        }
    }

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
        // === ascension ===
        (self.run.shard_bonus() * self.active_asc.pickup_factor()) - self.active_asc.shard_penalty()
        // === end ascension ===
    }

    // === ascension ===
    /// The ascension a run started now would use (none for daily/co-op/prologue/autopilot).
    fn run_asc(&self) -> progress::Ascension {
        if self.daily.is_some() || self.autopilot || self.prologue.is_some() || self.coop.is_some() {
            progress::Ascension(0)
        } else {
            progress::Ascension(self.ascension)
        }
    }

    /// Loadout slots this run may use (rule 13).
    fn asc_slots(&self) -> usize {
        self.run_asc().loadout_slots(self.booklet.lore.slots())
    }

    /// Clock for this dream: the twist's, or the closing eye's (rule 15).
    fn dream_timer(&self) -> Option<f32> {
        let eye = (!self.director.nightmare && self.prologue.is_none() && self.coop.is_none())
            .then(|| self.active_asc.eye_timer(self.director.depth))
            .flatten();
        match (self.twists.timer(), eye) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }
    // === end ascension ===

    /// The title menu, top to bottom.
    fn title_items(&self) -> Vec<TitleItem> {
        let mut items = Vec::new();
        if progress::load(&self.save_path).is_some() {
            items.push(TitleItem::Continue);
        }
        if !self.booklet.lore.prologue_done {
            items.push(TitleItem::Prologue);
        }
        items.extend([
            TitleItem::Start,
            TitleItem::Daily,
            TitleItem::DreamCode,
            TitleItem::RunLength,
        ]);
        if form::ALL
            .iter()
            .filter(|f| f.unlocked(&self.booklet.codex))
            .count()
            > 1
        {
            items.push(TitleItem::Form);
        }
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
            TitleItem::DreamCode => (
                "k",
                "dream a friend's code".into(),
                "the same dream, from a code".into(),
            ),
            TitleItem::RunLength => (
                "r",
                format!("run: {}", self.run_length.label()),
                title_ui::run_blurb(self.run_length.label()).into(),
            ),
            TitleItem::Form => {
                let f = self.booklet.codex.form;
                let locked: Vec<String> = form::ALL
                    .iter()
                    .filter(|f| !f.unlocked(codex))
                    .map(|f| format!("{}: {}", f.label(), f.unlock_hint()))
                    .collect();
                let mut blurb = f.blurb().to_string();
                if !locked.is_empty() {
                    blurb.push_str(&format!(" · locked — {}", locked.join(", ")));
                }
                ("f", format!("form: {}", f.label()), blurb)
            }
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
            TitleItem::DreamCode => self.open_code_entry(),
            TitleItem::RunLength => self.run_length = self.run_length.toggled(),
            TitleItem::Form => {
                let next = self.booklet.codex.form.cycle(&self.booklet.codex, dir);
                if next != self.booklet.codex.form {
                    self.booklet.codex.form = next;
                    self.persist_booklet();
                }
            }
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

    fn open_code_entry(&mut self) {
        self.code_digits.clear();
        self.code_error.clear();
        self.mode = hud::Mode::Code;
    }

    /// A key in the dream-code box.
    fn code_key(&mut self, key: Keycode) {
        match key {
            Keycode::Escape => self.mode = hud::Mode::Title,
            Keycode::Backspace => {
                self.code_digits.pop();
                self.code_error.clear();
            }
            Keycode::Return | Keycode::KpEnter => match seedcode::decode(&self.code_digits) {
                Ok((seed, long)) => {
                    self.code_error.clear();
                    self.pending_start = Some(StartKind::Code(seed, long));
                }
                Err(e) => self.code_error = e.to_string(),
            },
            k => {
                // Letters and digits arrive as their key names ("A", "7").
                let name = k.name();
                let mut it = name.chars();
                if let (Some(c), None) = (it.next(), it.next()) {
                    if seedcode::push(&mut self.code_digits, c) {
                        self.code_error.clear();
                    }
                }
            }
        }
    }

    /// Paste a whole code (or as much of it as fits).
    fn code_paste(&mut self, text: &str) {
        self.code_digits.clear();
        for c in seedcode::clean(text).chars() {
            seedcode::push(&mut self.code_digits, c);
        }
        self.code_error.clear();
    }

    /// The code of the run just played, for sharing.
    fn run_code(&self) -> String {
        seedcode::encode(self.run_seed, self.run_length == RunLength::Long)
    }

    fn copy_run_code(&mut self, ctx: &mut Context) {
        let code = self.run_code();
        match ctx.platform.video.clipboard().set_clipboard_text(&code) {
            Ok(()) => {
                self.code_copied = true;
                log::info!("Copied dream code {code}");
            }
            Err(e) => log::warn!("Could not copy the dream code: {e}"),
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
                self.deck = None; // === deck === a resumed run keeps its fixed kit
                self.loadout_scale = s.loadout_scale.unwrap_or(1.0);
                self.active_form = s.form;
                self.run.penalty_cards =
                    self.active_asc.fewer_cards() + self.active_form.fewer_choices();
                self.run.no_free_reroll = !self.active_asc.free_rerolls();
                self.refresh_card_effects();
                self.just_continued = true;
                self.director.shard_bonus = self.shard_bonus();
                self.motif = s.motif.map(|m| m.kind());
                self.perks = s.perks.clone();
                self.shards_this_run = s.shards_this_run;
                self.bonus_dust = s.bonus_dust;
                self.gamble = s.gamble.clone(); // === risk ===
                self.tool_cards = s.tool_cards.clone(); // === toolcards ===
                self.restore_tool_cards();
                self.pacts = s.pacts.clone(); // === pacts ===
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
                self.pick = reveal_ui::PackPickView::default();
                self.choice = None;
                self.offer_upgrade = false;
                // The dream is regenerated; don't log it twice.
                self.run_log.pop();
                log::info!("Continuing a run at depth {}", self.director.depth);
            }
            StartKind::Code(seed, long) => {
                self.prologue = None;
                self.run_seed = seed;
                self.run_length = if long {
                    RunLength::Long
                } else {
                    RunLength::Short
                };
                crash::SEED.store(seed, std::sync::atomic::Ordering::Relaxed);
                self.daily = None;
                self.director = DreamDirector::with_length(seed, self.run_length);
                self.motif = None;
                self.run_log.clear();
                self.begin_run();
                log::info!("Dream code: seed {seed}, {}", self.run_length.label());
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
            pacts: self.pacts.clone(), // === pacts ===
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
            form: self.active_form,
            gamble: {
                // === risk === the baseline and visit count ride along
                let mut g = self.gamble.clone();
                g.stamp(self.wager_dust_start, self.gamble_visit);
                g
            },
            tool_cards: self.tool_cards_held(), // === toolcards ===
        };
        if let Err(e) = progress::save(&self.save_path, &s) {
            log::warn!("could not save the run: {e}");
        }
    }

    /// The dreamer's walking speed right now.
    fn player_speed(&self) -> f32 {
        gameplay::MOVE_SPEED
            * pacts::effects(&self.pacts).player_speed_mult // === pacts ===
            * self.run.speed()
            * self.twists.walk()
            * if self.has_perk(store::Perk::LongStride) {
                store::LONG_STRIDE
            } else {
                1.0
            }
            * (1.0 + self.card_fx.speed_pct as f32 / 100.0)
            * self.active_form.speed_mult()
            * self.haste.map_or(1.0, |h| h.0.player_mult(self.player_position)) // === abilities2 ===
            // === shards ===
            * shard_rules::heavy_mults(self.heavy_elapsed()).0
            // === end shards ===
    }

    // === shards ===
    /// Seconds since a heavy shard was picked up, if it still weighs on you.
    fn heavy_elapsed(&self) -> Option<f32> {
        self.heavy_since.map(|t| (self.dream_age - t).max(0.0))
    }
    // === end shards ===

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
        // === fortune === a ticket for each new achievement (solo play only)
        if !self.coop_active() && !self.autopilot && self.prologue.is_none() {
            tickets::give(
                &mut self.booklet.stash,
                tickets::TICKETS_PER_ACHIEVEMENT * fresh.len() as u32,
            );
        }
        // === end fortune ===
        self.achievement_toast =
            Some((achievements::name(fresh[fresh.len() - 1]).to_string(), 0.0));
        self.sfx(Sound::Achievement);
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

    /// Wake up: roll the run's sealed packs and let the player choose one.
    fn start_pack_pick(&mut self) {
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
        let owned: Vec<cards::Card> = self.booklet.cards().cloned().collect();
        let offers = packs::offer_packs(&log, &owned, boost);
        self.begin_pick(offers);
    }

    /// Show `offers` on the pick screen. Nothing on offer (an empty log):
    /// straight to the reveal with an empty pack, as an empty run always did.
    fn begin_pick(&mut self, offers: Vec<packs::PackOffer>) {
        self.pack = reveal_ui::PackView::default();
        self.haul.clear();
        if offers.is_empty() {
            self.pick = reveal_ui::PackPickView::default();
            self.reveal_pack(Vec::new());
            return;
        }
        let picks_left = packs::picks_for(self.run_log.len()).min(offers.len());
        self.pick = reveal_ui::PackPickView {
            offers,
            selected: 0,
            age: 0.0,
            picks_left,
        };
        self.mode = hud::Mode::PackPick;
    }

    /// Move the selection by `delta`, wrapping round.
    fn pick_move(&mut self, delta: i32) {
        let n = self.pick.offers.len();
        if n == 0 {
            return;
        }
        let at = self.pick.selected.min(n - 1) as i32;
        self.pick.selected = (at + delta).rem_euclid(n as i32) as usize;
    }

    /// A key on the pick screen. Escape does nothing: unpicked packs are lost,
    /// so there is no way to back out by accident.
    fn pick_key(&mut self, key: Keycode) {
        match key {
            Keycode::A | Keycode::Left => self.pick_move(-1),
            Keycode::D | Keycode::Right => self.pick_move(1),
            Keycode::Num1 => self.pick_select(0),
            Keycode::Num2 => self.pick_select(1),
            Keycode::Num3 => self.pick_select(2),
            Keycode::Num4 => self.pick_select(3),
            Keycode::Num5 => self.pick_select(4),
            Keycode::Num6 => self.pick_select(5),
            // Packs slide in first, so a held key can't open one blind.
            Keycode::Return | Keycode::Space if self.pick.age > reveal_ui::PICK_LOCKOUT => {
                self.pick_open();
            }
            _ => {}
        }
    }

    /// Select pack `i`; an index with no pack behind it is ignored.
    fn pick_select(&mut self, i: usize) {
        if i < self.pick.offers.len() {
            self.pick.selected = i;
        }
    }

    /// Open the selected pack; the others are dropped for good.
    fn pick_open(&mut self) {
        if self.mode != hud::Mode::PackPick || self.pick.offers.is_empty() {
            return;
        }
        let mut view = std::mem::take(&mut self.pick);
        let at = view.selected.min(view.offers.len() - 1);
        let screen = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(self.screen_size.0, self.screen_size.1),
        );
        let from_rect = reveal_ui::pick_selected_rect(screen, view.offers.len(), at).unwrap_or(screen);
        let kept = view.offers.remove(at);
        let cards = packs::open(&kept);
        log::info!(
            "Pack picked: {:?}, {} of {} dreams",
            kept.theme,
            cards.len(),
            kept.cards.len()
        );
        // More packs to open: the rest stay on the table for the next pick.
        view.picks_left = view.picks_left.saturating_sub(1);
        if view.picks_left > 0 && !view.offers.is_empty() {
            view.selected = 0;
            view.age = 0.0;
            self.pick = view;
        }
        self.reveal_pack(cards);
        // Full motion: the pack tears open first (a negative age counts it down).
        // Dev: DREAMSCAPE_TEAR_FREEZE=0.5 holds the tear half way (for screenshots).
        let freeze = crate::dev::var("DREAMSCAPE_TEAR_FREEZE")
            .ok()
            .and_then(|v| v.parse::<f32>().ok());
        if self.settings.motion() >= 1.0 && (!self.autopilot || freeze.is_some()) {
            self.pack.wrapper = Some(pack_art::Wrapper {
                theme: kept.theme,
                emblem: kept.emblem,
                seed: kept.seed,
                pips: packs::take_odds(kept.cards.len()).len(),
                from: from_rect,
            });
            let held = freeze.unwrap_or(0.0).clamp(0.0, 1.0);
            self.pack.age = -pack_art::TEAR_TIME * (1.0 - held);
        }
    }

    /// The pointer moved over (or clicked) the pick screen.
    /// A key press, as if typed (the mouse uses the keys' own code paths).
    fn press(&mut self, ctx: &mut Context, key: Keycode) {
        let ev = Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(key),
            scancode: None,
            keymod: engine::sdl2::keyboard::Mod::NOMOD,
            repeat: false,
        };
        self.handle_event(ctx, &ev);
    }

    /// The title menu's rows as drawn, and whether the goal line shows.
    fn title_rows(&self) -> (Vec<String>, bool) {
        let rows = self
            .title_items()
            .into_iter()
            .map(|i| {
                let (k, l, _) = self.title_label(i);
                crate::pad::prompt(&format!("[{k}]  {l}"), self.mode, &self.settings, self.using_pad)
            })
            .collect();
        let loadout: Vec<u32> = self.booklet.loadout_cards().iter().map(|c| c.number).collect();
        let goal = self.title_goal(&loadout);
        (rows, !goal.is_empty())
    }

    /// Where the cursor sits on whichever menu is up.
    fn menu_cursor(&self) -> usize {
        match self.mode {
            hud::Mode::Title => self.title_sel,
            hud::Mode::Choice => self.choice_view.selected,
            hud::Mode::Loadout => self.loadout_sel,
            hud::Mode::Merge => self.merge_sel,
            _ => 0,
        }
    }

    // === pointer ===
    /// The mouse over a menu: hovering selects, clicking does what Enter does
    /// (the key to press is returned), right-clicking steps back. Positions
    /// are mapped the way the screen draws.
    fn menu_pointer(&mut self, x: i32, y: i32, what: Pointer) -> Option<Keycode> {
        let (at, screen) = hud::to_layout(
            hud::design_for(self.mode, self.store_tab),
            self.screen_size,
            x as f32,
            y as f32,
        );
        let click = what == Pointer::Click;
        match self.mode {
            hud::Mode::Title => {
                let (rows, goal) = self.title_rows();
                if let Some(i) = title_ui::menu_hit(screen, &rows, goal, at) {
                    self.title_sel = i;
                    return click.then_some(Keycode::Return);
                }
            }
            hud::Mode::Choice => {
                let n = self.choice_view.cards.len();
                let hit = upgrade_ui::card_rects(screen, n)
                    .iter()
                    .position(|r| r.contains(at));
                if let Some(i) = hit {
                    self.choice_view.selected = i;
                    return click.then_some(Keycode::Return);
                }
            }
            hud::Mode::Loadout => {
                let lay = loadout_ui::layout(screen);
                if click {
                    // A card in the deck: click its pip to take it out. // === deck ===
                    if let Some(k) = deck_ui::pips(screen).iter().position(|r| r.contains(at)) {
                        let chosen = self.booklet.lore.deck.clone();
                        if let Some(&number) = chosen.get(k) {
                            self.booklet.lore.deck.retain(|&n| n != number);
                            self.sfx(Sound::Unequip);
                            self.persist_booklet();
                        }
                        return None;
                    }
                }
                let n = self.booklet.card_count();
                if let Some(i) = loadout_ui::grid_hit(screen, n, self.loadout_sel, lay.grid_top, lay.grid_w, at) {
                    self.loadout_sel = i;
                    return click.then_some(Keycode::Return);
                }
            }
            hud::Mode::Merge => {
                if what == Pointer::Back {
                    if self.merge_first.is_some() {
                        self.merge_first = None;
                    } else {
                        self.mode = self.merge_return;
                    }
                    return None;
                }
                let lay = merge_ui::layout(screen);
                if click && lay.slots[0].contains(at) {
                    // The first card's slot: click to take it back out.
                    self.merge_first = None;
                    return None;
                }
                if click && lay.slots[2].contains(at) {
                    // The result: click to make it.
                    return self.merge_first.is_some().then_some(Keycode::Return);
                }
                let n = self.booklet.card_count();
                if let Some(i) = loadout_ui::grid_hit(screen, n, self.merge_sel, lay.grid_top, lay.grid_w, at) {
                    self.merge_sel = i;
                    return click.then_some(Keycode::Return);
                }
            }
            _ => {}
        }
        None
    }
    // === end pointer ===

    fn pick_pointer(&mut self, x: i32, y: i32, click: bool) {
        let screen = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(self.screen_size.0, self.screen_size.1),
        );
        let at = egui::Pos2::new(x as f32, y as f32);
        if let Some(i) = reveal_ui::pick_hit(screen, self.pick.offers.len(), at) {
            self.pick_select(i);
            if click && self.pick.age > 0.2 {
                self.pick_open();
            }
        }
    }

    /// Autopilot / dev shortcut: offer the packs and take the first.
    fn open_pack(&mut self) {
        self.start_pack_pick();
        while self.mode == hud::Mode::PackPick {
            self.pick_open();
        }
    }

    /// The chosen pack goes to the reveal. Pack pity runs on this pack only.
    fn reveal_pack(&mut self, mut pack: Vec<cards::Recalled>) {
        lottery::apply_pack_pity(&mut self.booklet.lottery, &mut pack);
        self.haul.extend(pack.iter().cloned());
        self.pack = reveal_ui::PackView {
            pack,
            picks_left: self.pick.picks_left,
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
        // Every pack the run opened, pressed together.
        let haul = if self.haul.is_empty() {
            self.pack.pack.clone()
        } else {
            self.haul.clone()
        };
        let before = self.booklet.clone();
        let (added, dust) = self.booklet.press(
            self.run_seed,
            &haul,
            self.shards_this_run,
            self.has_perk(store::Perk::DustMagnet),
        );
        // Run upgrades and hard runs pay out on top of the pack.
        let mult = self.dust_multiplier();
        let extra = (dust as f32 * (mult - 1.0)).round() as u32 + self.bonus_dust;
        self.booklet.stash.earn(extra);
        let dust = dust + extra;
        let deepest = haul.iter().map(|r| r.card.depth).max().unwrap_or(0);
        let report = retention::settle_run(
            &mut self.booklet,
            &self.loadout_numbers,
            &haul,
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
            tickets::give(&mut self.booklet.stash, tickets::TICKETS_PER_WEEKLY); // === fortune ===
        }
        self.after_pack_feelings(&haul, &report, weekly_dust);
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
    fn after_pack_feelings(
        &mut self,
        pack: &[cards::Recalled],
        report: &retention::RunReport,
        weekly_dust: Option<u32>,
    ) {
        let remembered = || pack.iter().filter(|r| r.remembered);
        let foil = remembered().any(|r| worth::is_foil(&r.card));
        let prophetic = remembered().any(|r| r.card.rarity >= cards::Rarity::Prophetic);
        let near_miss = pack.iter().any(reveal_ui::near_miss);
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
            fortune: self.fortune_view(featured), // === fortune ===
        }
    }

    // === fortune ===
    /// A fortune is waiting today (solo play only; co-op and autopilot never see it).
    fn fortune_waiting(&self) -> bool {
        self.mode == hud::Mode::Title
            && !self.coop_active()
            && !self.autopilot
            && self.booklet.fortune.available(today())
    }

    /// The title's bottom line: the goal, plus a plain note when a fortune
    /// is waiting (no pulsing, no urgency).
    fn title_goal(&self, loadout: &[u32]) -> String {
        let goal = streaks::teaser(&retention::progress_inputs(&self.booklet, loadout, self.best_depth));
        if !self.fortune_waiting() {
            return goal;
        }
        if goal.is_empty() {
            FORTUNE_LINE.to_string()
        } else {
            format!("{goal}  ·  {FORTUNE_LINE}")
        }
    }

    /// The free pull's exact odds, shown only while one is waiting.
    fn fortune_view(&self, featured: dream::DreamTheme) -> Option<lottery_ui::FortuneView> {
        let day = today();
        if !self.booklet.fortune.available(day) {
            return None;
        }
        let o = fortune::fortune_odds(&self.booklet, day, Some(featured));
        Some(lottery_ui::FortuneView {
            odds: lottery::display_odds(&o),
            nudge: self.booklet.fortune.nudge(day),
            days: self.booklet.fortune.prior_days(day),
        })
    }

    /// Claim today's free pull. The card is chosen from this save's own seed
    /// stream and written before the reveal plays.
    fn fortune_claim(&mut self) {
        if self.coop_active() || self.autopilot {
            return;
        }
        let before = self.booklet.clone();
        let save_seed = self.booklet.lore.save_seed;
        match fortune::claim(&mut self.booklet, today(), save_seed) {
            None => {
                self.lottery_status = Some("today's fortune is already yours".into());
                self.sfx(Sound::Denied);
            }
            Some(mut p) => {
                p.pull.near_miss = None; // no near-miss teasing on a free gift
                if self.persist_booklet() {
                    log::info!("Fortune: {:?} -> {}", p.card.rarity, p.card.name);
                    self.show_pull(p);
                } else {
                    self.booklet = before;
                    self.lottery_status = Some("the deck couldn't write it down (see log)".into());
                }
            }
        }
    }

    fn tickets_view(&self) -> tickets_ui::TicketsView {
        let stash = &self.booklet.stash;
        let left = !tickets::unowned_cosmetics(stash).is_empty();
        let mut odds: Vec<(String, f32)> = tickets::odds(stash.ticket_dry, left)
            .into_iter()
            .map(|(k, share)| (k.label(), share))
            .collect();
        odds.sort_by(|a, b| b.1.total_cmp(&a.1));
        tickets_ui::TicketsView {
            tickets: stash.tickets,
            odds,
            pity_left: tickets::scratches_to_pity(stash.ticket_dry),
            scratches: stash.ticket_scratches,
            revealed: self.ticket_revealed.clone(),
            age: self.ticket_age,
            confirming: self.ticket_confirm,
            status: self.ticket_status.clone(),
        }
    }

    /// Scratch a ticket: the first press asks, the second confirms. The
    /// prize is rolled and saved at the confirm; the animation only uncovers it.
    fn ticket_press(&mut self) {
        if self.coop_active() || self.autopilot {
            return;
        }
        if self.booklet.stash.tickets == 0 {
            self.ticket_confirm = false;
            self.ticket_status = Some("no tickets yet: clean dreams, achievements and weekly challenges earn them".into());
            self.sfx(Sound::Denied);
            return;
        }
        if !self.ticket_confirm {
            self.ticket_confirm = true;
            self.ticket_status = Some("scratch one ticket? press enter again to confirm".into());
            return;
        }
        self.ticket_confirm = false;
        let before = self.booklet.clone();
        let seed = self.booklet.lore.save_seed;
        let got = match tickets::scratch(&mut self.booklet, seed, today()) {
            Ok(g) => g,
            Err(_) => return,
        };
        if !self.persist_booklet() {
            self.booklet = before;
            self.ticket_status = Some("the ticket couldn't be written down (see log)".into());
            return;
        }
        let (title, detail) = match &got.prize {
            tickets::Prize::Dust(n) => (format!("{n} DREAM DUST"), "added to your stash".to_string()),
            tickets::Prize::Pull => {
                let c = got.pulled.as_ref().map(|p| &p.card);
                (
                    "A LOTTERY PULL".to_string(),
                    c.map_or(String::new(), |c| format!("{} ({}), in your booklet", c.name, c.rarity.label())),
                )
            }
            tickets::Prize::Cosmetic(i) => (
                i.info().name.to_uppercase(),
                format!("a new {} item, yours now", i.shelf().label().to_lowercase()),
            ),
            tickets::Prize::Hint => (
                "A DREAM OF YOUR CHOOSING".to_string(),
                "a spark pick is ready in the lottery tab".to_string(),
            ),
        };
        log::info!("Ticket: {title} ({detail})");
        self.sfx(if matches!(got.prize, tickets::Prize::Dust(n) if n < 80) { Sound::Pick } else { Sound::WakeDoor });
        self.ticket_revealed = Some(tickets_ui::Revealed { title, detail });
        self.ticket_age = 0.0;
        self.ticket_status = None;
    }

    /// Tickets for a clean dream, once per finished dream, capped per run.
    /// Solo runs only.
    fn ticket_for_clean_dream(&mut self) {
        if self.coop_active() || self.autopilot || self.prologue.is_some() {
            return;
        }
        if !self.nerve_lost_this_dream
            && self.caught_this_dream == 0
            && tickets::for_clean_dream(&mut self.booklet.stash, &mut self.tickets_this_run) > 0
        {
            log::info!("Ticket: +1 for a clean dream ({} this run)", self.tickets_this_run);
        }
    }
    // === end fortune ===

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
        self.ticket_status = None; // === fortune ===
        self.ticket_confirm = false;
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
        // === deck === the deck: cards still owned; empty starts as what would be used
        let b = &self.booklet;
        let mut d = deck::clean(&b.lore.deck, |n| b.card(n).is_some());
        if d.is_empty() {
            d = b.deck_numbers();
        }
        self.booklet.lore.deck = d;
        // === end deck ===
        self.loadout_sel = self.loadout_sel.min(self.booklet.card_count() - 1);
        self.mode = hud::Mode::Loadout;
    }

    // === sort ===
    /// The booklet's cards in the given order.
    fn sorted_cards(&self, sort: sorting::Sort) -> Vec<&cards::Card> {
        let all: Vec<&cards::Card> = self.booklet.cards().collect();
        // `order` wants a slice of cards; sorting is by value so clone the (small) list.
        let tmp: Vec<cards::Card> = all.iter().map(|c| (*c).clone()).collect();
        sorting::order(&tmp, sort).into_iter().map(|i| all[i]).collect()
    }

    /// `[t]` (cycle = true) or `[r]` on the booklet, merge or loadout screen: the cursor
    /// stays on the same card.
    fn sort_press(&mut self, cycle: bool) {
        let mode = self.mode;
        let sort_of = |s: &settings::Settings| match mode {
            hud::Mode::Booklet => s.sort_booklet,
            hud::Mode::Merge => s.sort_merge,
            _ => s.sort_loadout,
        };
        let old = sort_of(&self.settings);
        let cursor = match mode {
            hud::Mode::Booklet => self.booklet_page * cards::CARDS_PER_PAGE,
            hud::Mode::Merge => self.merge_sel,
            _ => self.loadout_sel,
        };
        let number = self.sorted_cards(old).get(cursor).map(|c| c.number);
        let mut new = old;
        if cycle {
            new.cycle_key();
        } else {
            new.flip();
        }
        match mode {
            hud::Mode::Booklet => self.settings.sort_booklet = new,
            hud::Mode::Merge => self.settings.sort_merge = new,
            _ => self.settings.sort_loadout = new,
        }
        let at = number
            .and_then(|n| self.sorted_cards(new).iter().position(|c| c.number == n))
            .unwrap_or(0);
        match mode {
            hud::Mode::Booklet => self.booklet_page = at / cards::CARDS_PER_PAGE,
            hud::Mode::Merge => self.merge_sel = at,
            _ => self.loadout_sel = at,
        }
        self.sfx(Sound::Pick);
        self.save_settings();
    }
    // === end sort ===

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
                let Some(number) = self.sorted_cards(self.settings.sort_merge).get(self.merge_sel).map(|c| c.number)
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
                                .sorted_cards(self.settings.sort_merge)
                                .iter()
                                .position(|x| x.number == a)
                                .unwrap_or(0);
                            self.flash.trigger([1.0, 0.8, 1.0], 0.5);
                            self.sfx(Sound::Merge);
                            self.persist_booklet();
                        }
                        None => self.sfx(Sound::Denied),
                    },
                }
            }
            // Change your mind without hunting for the first card: Tab makes the
            // card under the cursor the first one, Backspace puts it back.
            Keycode::Tab => {
                if let Some(number) = self.sorted_cards(self.settings.sort_merge).get(self.merge_sel).map(|c| c.number) {
                    self.merge_first = Some(number);
                    self.merge_status = None;
                    self.sfx(Sound::Pick);
                }
            }
            Keycode::T => self.sort_press(true),
            Keycode::R => self.sort_press(false),
            Keycode::Backspace if self.merge_first.is_some() => self.merge_first = None,
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
                let Some(number) = self.sorted_cards(self.settings.sort_loadout).get(self.loadout_sel).map(|c| c.number)
                else {
                    return;
                };
                // === deck === choosing cards builds the deck
                let was_in = self.booklet.lore.deck.contains(&number);
                if deck::toggle(&mut self.booklet.lore.deck, number) {
                    self.sfx(if was_in { Sound::Unequip } else { Sound::Equip });
                    self.persist_booklet();
                } else {
                    self.sfx(Sound::Denied); // the slots are full
                }
            }
            Keycode::T => self.sort_press(true),
            Keycode::R => self.sort_press(false),
            Keycode::C if !self.booklet.lore.dreamers.is_empty() => {
                self.booklet.lore.companion = self.booklet.lore.next_companion();
                self.persist_booklet();
            }
            // === deck === too small a deck cannot start (none = automatic)
            Keycode::Space
                if !deck::valid(self.booklet.lore.deck.len(), self.booklet.card_count()) =>
            {
                self.sfx(Sound::Denied)
            }
            // === end deck ===
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
        } else if self.deck.is_some() {
            self.deck_opening.clone() // === deck ===
        } else {
            // === ascension === (rule 13: one fewer slot)
            let mut c = self.booklet.loadout_cards();
            c.truncate(self.asc_slots());
            c
            // === end ascension ===
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
        self.tickets_this_run = 0; // === fortune ===

        // Abilities come from the loadout's cards (none in the prologue),
        // and each card's dream is planned into the run.
        let cards = if self.prologue.is_some() {
            self.deck = None;
            Vec::new()
        } else if let Some(hand) = self.start_deck() {
            hand // === deck === the opening hand
        } else {
            // === ascension === (rule 13: one fewer slot)
            let mut c = self.booklet.loadout_cards();
            c.truncate(self.asc_slots());
            c
            // === end ascension ===
        };
        if !cards.is_empty() {
            log::info!(
                "Loadout: {:?}",
                cards.iter().map(|c| c.name.as_str()).collect::<Vec<_>>()
            );
        }
        // Forms are for solo runs: the daily and co-op stay the same for everyone.
        self.active_form = if self.daily.is_some()
            || self.autopilot
            || self.prologue.is_some()
            || self.coop.is_some()
        {
            form::Form::Crystal
        } else {
            self.booklet.codex.form
        };
        if self.active_form != form::Form::Crystal {
            log::info!("Form: {}", self.active_form.label());
        }
        let gifts: Vec<Upgrade> = if self.active_form == form::Form::Crystal
            && (self.daily.is_some() || self.prologue.is_some() || self.coop.is_some())
        {
            Vec::new()
        } else {
            let counts = attunement::counts(self.booklet.cards());
            attunement::attuned(&counts)
                .into_iter()
                .map(attunement::reward)
                .collect()
        };
        self.run = if self.deck.is_some() {
            RunUpgrades::from_hand(&cards) // === deck ===
        } else {
            RunUpgrades::from_loadout(&cards)
        }
        .with_attunements(&gifts)
            .with_start_ability(self.active_form.start_ability());
        self.nerve = if self.settings.gentle_nerve {
            nerve::Nerve::new(nerve::GENTLE_START)
        } else {
            nerve::Nerve::default()
        };
        self.strike_count = 0;
        self.thin_skin_applied = false; // === upgrades2 ===
        self.calm_timer = 0.0;
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
            gameplay::loadout_difficulty(self.run.power()) * self.active_form.difficulty_mult()
        };
        log::info!(
            "Starting kit power {:.1}: difficulty x{:.2}",
            self.run.power(),
            self.loadout_scale
        );
        self.edges = {
            let mut e: Vec<edges::Edge> = Vec::new();
            for c in &cards {
                for x in loadout_ui::edges_of(c) {
                    if !e.contains(&x) {
                        e.push(x);
                    }
                }
            }
            e
        };
        self.flourish.reset();
        self.siphon.reset();
        if self.deck.is_none() {
            self.hand_cards = cards
                .iter()
                .map(|c| (crate::powers::power_of(c).active, c.clone()))
                .collect();
        }
        let tint_cards: Vec<&cards::Card> = if self.deck.is_some() {
            self.hand_cards.iter().map(|n| &n.1).collect()
        } else {
            cards.iter().collect()
        };
        self.ability_tints = tint_cards
            .into_iter()
            .map(|c| {
                let accents = c.fused.unwrap_or(c.theme).spec().accents;
                (
                    crate::powers::power_of(c).active,
                    crate::powers::power_of(c).tint,
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
        self.asc_caught_here = None; // ascension 12: a new run forgets
        self.asc_remembered = None;
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
        self.run.penalty_cards = asc.fewer_cards() + self.active_form.fewer_choices();
        self.run.no_free_reroll = !asc.free_rerolls();
        self.pacts.clear(); // === pacts ===
        self.pact_offer = None;
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
        self.wager_dry = 0; // === abilities3 ===
        self.seen_kinds.clear();
        self.boss_reward = false;
        self.bonus_dust = 0;
        self.gamble = gamble::RunState::default(); // === risk ===
        self.pressure_collapsed = false; // pressure
        self.peak_difficulty = 1.0;
        self.offer_upgrade = false;
        self.last_event = None;
        self.event_just_shown = false;
        self.gild_next = false;
        self.twist_next = None;
        self.choice = None;
        log::info!(
            "Run length: {:?} ({} shards to wake)",
            self.run_length,
            self.director.shards_to_wake
        );
        self.pack = reveal_ui::PackView::default();
        self.pick = reveal_ui::PackPickView::default();
        self.perks = if self.autopilot || self.prologue.is_some() {
            Vec::new()
        } else {
            self.booklet.stash.take_for_run()
        };
        if !self.perks.is_empty() {
            log::info!("Perks this run: {:?}", self.perks);
            self.persist_booklet();
        }
        // === shop ===
        let fx = self.perk_fx();
        self.bonus_dust += fx.start_dust;
        self.run.bonus_rerolls += fx.rerolls;
        // === end shop ===
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
            self.new_records = if self.daily.is_none() {
                self.booklet.codex.record_wake(
                    self.run_length == RunLength::Long,
                    self.stats.time as u32,
                    depth,
                    self.run.taken.len(),
                )
            } else {
                Vec::new()
            };
            let dust: u32 = self.new_records.iter().map(|r| r.1).sum();
            self.booklet.stash.dust += dust;
            for (r, d) in &self.new_records {
                log::info!("{} (+{d} dust)", r.label());
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
        (self.run.dust() + pacts::dust_bonus(&self.pacts)) // === pacts ===
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
        // === pressure ===
        if self.pressure_collapsed {
            rows.push((String::new(), pressure::SUMMARY_LINE.into()));
        }
        // === end pressure ===
        for (r, d) in &self.new_records {
            rows.push((String::new(), format!("{} (+{d} dust)", r.label())));
        }
        // === pacts ===
        for line in pacts::summary_lines(&self.pacts) {
            rows.push(("pact".into(), line));
        }
        // === end pacts ===
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
                "dust x{:.2}  (hoarding x{:.2} · pacts +{:.2} · nightmare x{:.2})  + {} bonus",
                self.dust_multiplier(),
                self.run.dust(),
                pacts::dust_bonus(&self.pacts),
                gameplay::difficulty_reward(self.peak_difficulty),
                self.bonus_dust
            ),
        }
    }

    /// Begin melting toward `pending`. Ignored if a melt is already running
    /// (so touching the portal for several frames can't queue two descents).
    /// A quiet place: the dream after a nightmare, where nothing can hurt you.
    fn sanctuary(&self) -> bool {
        self.prologue.is_none() && self.director.sanctuary()
    }

    /// A dream with no enemies, gates, hazards or twists placed in it: a
    /// sanctuary, or the Lobby. Effects that belong to a real sanctuary
    /// (nerve refill, its chord and hint) stay on [`Self::sanctuary`].
    fn enemy_free(&self) -> bool {
        self.prologue.is_none() && self.director.enemy_free()
    }

    // === pressure ===
    /// Normal dreams only: no nightmares, the awakening, the prologue or co-op.
    fn pressure_applies(&self) -> bool {
        self.prologue.is_none()
            && self.coop.is_none()
            && !self.director.nightmare
            && self.director.theme != DreamTheme::Awakening
            && self.director.theme != DreamTheme::Lobby
            && !self.sanctuary()
    }

    /// Per frame (Playing only): tick the clock, stir the pacers, speak,
    /// and collapse the dream when the eye closes.
    fn update_pressure(&mut self, dt: f32) {
        if !self.pressure.active() || self.transition.active() || self.dream.is_none() {
            return;
        }
        self.pressure.tick(dt);
        let phase = self.pressure.phase();
        let cap = 0.92 * self.player_speed() / enemy_ai::CHASE_BOOST;
        let (sm, am) = (self.pressure.speed_mult(), self.pressure.chase_mult());
        for p in &mut self.enemies {
            p.ai.set_pressure(sm, am, cap);
        }
        if phase == self.pressure_phase {
            return;
        }
        self.pressure_phase = phase;
        match phase {
            pressure::Phase::Stirring => self.say_pressure(pressure::LINE_STIRRING),
            pressure::Phase::Warning => self.say_pressure(pressure::LINE_WARNING),
            pressure::Phase::Collapsed => {
                self.bonus_dust =
                    pressure::collapse_dust(self.pressure_dust_start, self.bonus_dust);
                self.pressure_collapsed = true;
                log::info!("Pressure: the dream collapsed at depth {}", self.director.depth);
                self.flash.trigger([1.0, 1.0, 1.0], 1.0);
                self.sfx(Sound::WakeDoor);
                self.begin_melt(transition::Pending::Wake);
            }
            pressure::Phase::Calm => {}
        }
    }

    fn say_pressure(&mut self, text: &str) {
        log::info!("Pressure: {text}");
        self.eye_line = Some((text.to_string(), 0.0));
    }
    // === end pressure ===

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
                self.nerve.heal(1); // combat: a portal steadies you
                self.gamble.begin_dream(); // === risk ===
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
                self.gamble.clear_wager(); // === risk ===
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
        self.sfx_pitched(sound, 1.0);
    }

    /// A sound from somewhere in the dream: heard only when it is near.
    fn sfx_at(&self, sound: Sound, at: Vec3) {
        if at.distance(self.player_position) <= SOUND_REACH {
            self.sfx(sound);
        }
    }

    /// Play a sound faster/slower (a ratio: 2.0 is an octave up).
    fn sfx_pitched(&self, sound: Sound, ratio: f32) {
        #[cfg(test)]
        self.played.borrow_mut().push(sound);
        if let (Some(audio), Some(samples)) = (&self.audio, self.sounds.get(&sound)) {
            audio.play_samples(samples, (sounds::RATE as f32 * ratio) as u32);
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
        } else if self.twists.has(Variant::NightShift) {
            0.25
        } else {
            0.15
        };
        let beacon = self.world.spawn((
            Transform {
                position: at
                    + Vec3::Y
                        * (size.y + 0.4 + BEACON_HEIGHT * self.active_asc.pickup_factor() as f32 * 0.5),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(
                    girth,
                    BEACON_HEIGHT * self.active_asc.pickup_factor() as f32, // ascension 14
                    girth,
                ),
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
        let chance = if owner == 0 { 0.70 } else { 0.30 };
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
            // Never in nightmares, the prologue, co-op or the Awakening (all
            // returned above or are not descent dreams); never above depth 2.
            secret: self.director.theme != DreamTheme::Awakening
                && dream::secret_rolls(seed, self.director.depth, dream::SECRET_CHANCE),
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
        let shard = self
            .shard_entities
            .iter()
            .step_by(2)
            .filter_map(|&e| self.world.get::<&Transform>(e).ok().map(|t| t.position))
            .min_by(|a, b| {
                a.distance_squared(self.player_position)
                    .total_cmp(&b.distance_squared(self.player_position))
            });
        // The shard (or the objective) is done: the eye turns to the way out.
        // (A nightmare's portal stays sealed until every sigil is taken.)
        shard.or_else(|| {
            (self.objective.is_none() && self.sealed_portal.is_none())
                .then(|| self.dream.as_ref().map(|d| d.portal))
                .flatten()
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
        // The eye lifts its lid and looks at the way out, right now.
        self.objective_clock = eye_motion::next_glance_clock(self.objective_clock);
        self.flash.trigger([0.3, 1.0, 1.0], 0.6);
        let pitch = self.juice.chain_pitch();
        self.juice.hit(juice::Hit::Shard, self.settings.reduced_motion);
        self.sfx_pitched(Sound::Shard, pitch);
        self.director.collect_shard();
        self.pressure.refund_on_shard(); // pressure
        // === shards ===
        self.heavy_since = (self.shard_kind == shard_rules::ShardKind::Heavy).then_some(self.dream_age);
        // === end shards ===
        if let Some(r) = self.theme_rule.as_mut() {
            r.on_shard(); // hazards
        }
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
        self.twists = if self.director.nightmare || self.prologue.is_some() || self.enemy_free() {
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
        // === ascension === 16: a twist every dream; 12: remember the last catch; 15: fresh clock
        if self.active_asc.always_twist(self.director.depth)
            && self.twists.0.is_empty()
            && !self.director.nightmare
            && self.prologue.is_none()
        {
            let fits: Vec<Variant> = dream::ALL_VARIANTS
                .iter()
                .copied()
                .filter(|v| v.fits(theme))
                .collect();
            if let Some(v) = progress::forced_twist(self.director.dream_seed(), &fits) {
                self.twists.0.push(v);
            }
        }
        self.asc_remembered = self.asc_caught_here.take();
        self.asc_eye_failed = false;
        // === end ascension ===
        self.apply_event_twists();
        if self.twists.has(Variant::Gilded) {
            self.director.has_shard = true;
        }
        self.peak_difficulty = self.peak_difficulty.max(self.difficulty());
        self.dream_age = 0.0;
        // === pressure ===
        self.pressure = pressure::Pressure::for_dream(
            self.director.depth,
            self.pressure_applies(),
            self.twists.timer().is_some(),
        );
        self.pressure_phase = pressure::Phase::Calm;
        self.pressure_dust_start = self.bonus_dust;
        self.wager_dust_start = self.gamble.take_wager_start(self.bonus_dust); // === risk ===
        // === end pressure ===
        // The dream just left counts toward the clean streak (a run loaded
        // from a save has no finished dream to count).
        if !self.run_log.is_empty() && !std::mem::take(&mut self.just_continued) {
            self.ticket_for_clean_dream(); // === fortune ===
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
        self.nerve_lost_this_dream = false; // === fortune ===
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
        // === sanctuary === nothing here can hurt you
        if self.enemy_free() {
            dream.calm();
            log::info!("Enemy-free dream: {:?} at depth {}", self.director.theme, self.director.depth);
        }
        // === end sanctuary ===
        if let Some(b) = self.director.blend {
            dream::blend_surfaces(&mut dream.surfaces, b);
            log::info!("Fused dream: {theme:?} with {b:?}");
        }
        if let Some(s) = dream.secret {
            log::info!("Cracked wall at {:?} (reach it from {:?})", s.wall, s.approach);
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
        // === shards ===
        self.heavy_since = None;
        self.heavy_applied = 1.0;
        self.shard_kind = shard_rules::ShardKind::Plain;
        self.guard_angle = None;
        if let Some(shard) = dream.shard {
            let eligible = !self.director.nightmare
                && !self.enemy_free()
                && self.prologue.is_none()
                && self.coop.is_none()
                && self.director.depth >= shard_rules::MIN_DEPTH
                && !(self.director.lucid() && dream.shard.is_some())
                && matches!(self.objective, None | Some(objective::Active::Shard));
            let want = shard_rules::kind_for(
                dream.seed,
                dream.depth,
                self.twists.has(Variant::Gilded),
                eligible,
            );
            self.shard_kind = want;
            if want == shard_rules::ShardKind::Guarded {
                let route: Vec<Vec3> = dream.lucid_route.iter().map(|w| w.pos).collect();
                let floor = shard_rules::floor_cells(&dream.blocks);
                match shard_rules::guard_cell(dream.seed, shard, &floor, &route, &[dream.spawn, dream.portal]) {
                    Some(at) => {
                        dream.specials.push(dream::Special {
                            kind: dream::EnemyKind::Sentry,
                            at,
                            a: at,
                            b: at,
                        });
                        self.guard_angle = Some(shard_rules::facing_angle(at, shard));
                        log::info!("Guarded shard: sentry at {at:?}");
                    }
                    None => self.shard_kind = shard_rules::ShardKind::Plain,
                }
            }
            if self.shard_kind != shard_rules::ShardKind::Plain {
                log::info!("Shard kind: {:?}", self.shard_kind);
            }
        }
        // === end shards ===
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
            "Dream depth={} {:?} terrain={:?} seed={} strangeness={:.2} lucidity={}/{} blocks={} enemies={} route={} shard={:?} portal={:?} motif={:?} wake_door={} next={:?} patterns=({:?},{:?},{:?})",
            dream.depth,
            dream.theme,
            dream.terrain,
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
        self.newfoes.clear(); // enemies-a
        self.foe_walls.clear(); // enemies-a
        self.moth_swarm = None; // === combat ===
        self.jump_event = false;
        self.crumbles.clear();
        self.trail.clear();
        self.mimic_awake_at = specials::MIMIC_DELAY;
        self.jester_cooldown = 0.0;
        self.fog = dream.fog_pockets.clone();
        self.fog_discs.clear(); // hazards
        self.shrink = None; // hazards
        self.veins = dream.veins.clone();
        self.gates.clear();
        self.shifters.clear();
        self.beat_tiles.clear();
        self.bpm = spec.mood.bpm;
        self.loops = 0;
        self.echoes.clear();
        self.echo_ghost = None; // its entity went with world.clear()
        self.dash_ghosts.clear();
        self.chilled.clear();
        self.phased = false;
        self.bursts.clear();
        self.burst_pending = None;
        self.decoy = None; // the old dream's world (and its marker) is gone
        self.anchor = None;
        // === abilities2 === (the world was cleared: the entities are gone)
        self.haste = None;
        self.pulse_marks.clear();
        self.surge.clear();
        self.dash_boost = 1.0;
        // === end abilities2 ===
        // === abilities3 === (the old world's rings and traps are gone with it)
        self.echo_last = None;
        self.wager_n = 0;
        self.wager_line = None;
        self.overload = None;
        self.overload_ring = None;
        self.overload_fire = None;
        self.bargain = abilities3::Bargain::default();
        self.snares = abilities3::Snares::new();
        // === end abilities3 ===
        self.last_ability = None;
        self.ability_used.clear();
        self.passive_fired.clear();
        self.slam_apex = 0.0;
        self.slam_meteor = false;
        self.caches.clear();
        self.secret = None;
        self.moth_stock = None;
        self.gamble_visit = self.gamble.take_visit_flips(); // === risk === a resumed visit keeps its count
        self.skittish = None;
        self.ground_y = f32::MIN;
        self.stunned.clear();
        // === combat ===
        self.foes.clear();
        self.staggered.clear();
        self.unraveling.clear();
        self.motes.clear();
        self.pending_motes.clear();
        self.backstab_until = 0.0;
        self.dash_cut.clear();
        self.leap_airborne = false;
        self.slashes.clear();
        self.slash_pending = None;
        self.strike.reset();
        self.strike_request = false;
        // === end combat ===
        self.slam = 0;
        self.step_pending = None;
        self.steps.clear();
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
        // A cracked wall wears the wall pattern with a bright hairline split.
        let crack_tex = if dream.secret.is_some() {
            let mut px = dream.surfaces.wall.rgba();
            secrets::draw_crack(&mut px, TEX_SIZE, dream.seed);
            let tex = Arc::new(GpuTexture::from_rgba8(
                gl,
                &px,
                TEX_SIZE,
                TEX_SIZE,
                TextureFilter::Bilinear,
            )?);
            self.dream_textures.push(tex.clone());
            Some(tex)
        } else {
            None
        };
        let mut cracked: Option<(Entity, Vec3, f32)> = None;
        let cb = self.settings.colorblind_markers;
        let shard_spec = if cb {
            access::shard(dream.surfaces.shard.seed)
        } else {
            dream.surfaces.shard.clone()
        };
        self.shard_tex = Some(self.upload_surface(gl, &shard_spec)?);
        let preview = if theme == DreamTheme::Awakening {
            DreamTheme::Awakening
        } else {
            self.director.next
        };
        let (portal_spec, wake_spec) = if cb {
            (access::portal(dream.seed ^ 0x5EED), access::portal(dream.seed))
        } else {
            (
                dream::portal_surface(preview, dream.seed ^ 0x5EED),
                dream::portal_surface(DreamTheme::Awakening, dream.seed),
            )
        };
        let portal_tex = self.upload_surface(gl, &portal_spec)?;
        self.wake_tex = Some(self.upload_surface(gl, &wake_spec)?);
        // One texture repeat per two cells: big, readable swirls instead of noise.
        let per_cell = 0.5 / gameplay::CELL;
        let mut dissolve_tiles: Vec<(Vec3, Entity, Vec3)> = Vec::new();
        // === hazards === shrinking floor: which cells may go, outer first
        let mut shrink_order: Vec<(i32, i32)> = Vec::new();
        let mut shrink_tiles: HashMap<(i32, i32), (Entity, Vec3)> = HashMap::new();
        if self
            .hazard_rule_state()
            .is_some_and(|r| r.kind == theme_rules::Kind::ShrinkingFloor)
            && !matches!(
                self.objective,
                Some(objective::Active::Chase(_)) | Some(objective::Active::HoldOn(_))
            )
        {
            let key = |p: Vec3| theme_rules::cell_key(p.x, p.z, gameplay::CELL);
            let dissolving: Vec<(i32, i32)> = dream.dissolve.iter().map(|&c| key(c)).collect();
            let mut cells = Vec::new();
            for b in dream.blocks.iter().filter(|b| b.kind == BlockKind::Floor) {
                let n = (b.size.x / gameplay::CELL).round().max(1.0) as i32;
                for i in 0..n {
                    let at = b.pos + Vec3::X * (i as f32 - (n - 1) as f32 * 0.5) * gameplay::CELL;
                    if !dissolving.contains(&key(at)) {
                        cells.push(key(at));
                    }
                }
            }
            let mut route: Vec<(i32, i32)> = dream
                .route
                .iter()
                .chain(dream.lucid_route.iter())
                .map(|w| key(w.pos))
                .collect();
            route.extend([dream.spawn, dream.portal].into_iter().map(key));
            route.extend(dream.shard.into_iter().map(key));
            route.extend(dream.fragments.iter().copied().map(key));
            route.extend(dream.sigils.iter().copied().map(key));
            route.extend(dream.crumbles.iter().copied().map(key));
            if let (Some(lo), Some(hi)) = (
                cells.iter().copied().reduce(|a, b| (a.0.min(b.0), a.1.min(b.1))),
                cells.iter().copied().reduce(|a, b| (a.0.max(b.0), a.1.max(b.1))),
            ) {
                let center = ((lo.0 + hi.0) / 2, (lo.1 + hi.1) / 2);
                shrink_order = theme_rules::shrink_order(&cells, &route, center);
            }
        }
        // === end hazards ===
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
            // === hazards === a run holding shrinkable cells becomes one slab per cell
            if !shrink_order.is_empty()
                && block.kind == BlockKind::Floor
                && (block.size.z - gameplay::CELL).abs() < 1e-3
            {
                let n = (block.size.x / gameplay::CELL).round().max(1.0) as i32;
                let at = |i: i32| {
                    block.pos + Vec3::X * (i as f32 - (n - 1) as f32 * 0.5) * gameplay::CELL
                };
                let key = |p: Vec3| theme_rules::cell_key(p.x, p.z, gameplay::CELL);
                if (0..n).any(|i| shrink_order.contains(&key(at(i)))) {
                    for i in 0..n {
                        let mut b = block.clone();
                        b.pos = at(i);
                        b.size.x = gameplay::CELL;
                        let e = self.spawn_tile(&b, floor_tex.clone())?;
                        if shrink_order.contains(&key(b.pos)) {
                            shrink_tiles.insert(key(b.pos), (e, b.pos));
                        }
                    }
                    continue;
                }
            }
            // === end hazards ===
            let is_crack = block.kind == BlockKind::Wall
                && dream.secret.is_some_and(|s| {
                    (s.wall.x - block.pos.x).abs() < 1e-3 && (s.wall.z - block.pos.z).abs() < 1e-3
                });
            let (texture, uv) = match block.kind {
                BlockKind::Floor => (floor_tex.clone(), per_cell),
                BlockKind::Wall => match (&crack_tex, is_crack) {
                    // One repeat across the cracked cell: the split stays whole.
                    (Some(c), true) => (c.clone(), 1.0 / gameplay::CELL),
                    _ => (wall_tex.clone(), per_cell),
                },
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
            if is_crack {
                cracked = Some((entity, block.pos, block.size.y));
            }
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
                    if matches!(block.shape, Shape::Cloud | Shape::Island) {
                        self.world
                            .insert_one(entity, Silhouette)
                            .expect("entity was just spawned");
                    }
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

        if let (Some((wall, pos, height)), Some(sec)) = (cracked, dream.secret) {
            // A faint glow on the face you approach from, so it isn't decoration.
            let toward = (sec.approach - sec.wall).normalize_or_zero();
            let glow_tex = self.texture(gl, [190, 215, 255, 255]);
            let glow = self.world.spawn((
                Transform {
                    position: Vec3::new(
                        pos.x + toward.x * (gameplay::CELL * 0.5 + 0.03),
                        pos.y,

                        pos.z + toward.z * (gameplay::CELL * 0.5 + 0.03),
                    ),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::new(
                        if toward.x.abs() > 0.5 { 0.05 } else { 0.35 },
                        height * 0.9,
                        if toward.z.abs() > 0.5 { 0.05 } else { 0.35 },
                    ),
                },
                MeshRenderer {
                    mesh: self.mesh(Shape::Cube)?,
                    texture: Some(glow_tex),
                },
                Translucent,
                Lit,
            ));
            let door_tex = self.texture(gl, [255, 214, 150, 255]);
            self.secret = Some(SecretWall {
                at: sec.wall,
                height,
                door_tex,
                wall: Some((wall, glow)),
                door: None,
                armed: true,
            });
        }
        self.spawn_wager_portal(gl, &dream)?; // === risk ===
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
        // High caches on the pillars: gold, with a beam you can see from below.
        if !dream.caches.is_empty() {
            let gem = self.mesh(Shape::Octahedron)?;
            let beam = self.mesh(Shape::Cylinder)?;
            let gold = self.texture(gl, [255, 210, 80, 255]);
            for &at in &dream.caches {
                let glow = self.world.spawn((
                    Transform {
                        position: at + Vec3::Y * 1.1,
                        rotation: Quat::IDENTITY,
                        scale: Vec3::new(0.7, 1.0, 0.7),
                    },
                    MeshRenderer {
                        mesh: gem.clone(),
                        texture: Some(gold.clone()),
                    },
                    Spin(2.0),
                    Bob {
                        base: at.y + 1.1,
                        amp: 0.2,
                        speed: 2.0,
                        phase: at.x,
                    },
                    Lit,
                    Hero,
                ));
                let pillar_beam = self.world.spawn((
                    Transform {
                        position: at + Vec3::Y * 4.0,
                        rotation: Quat::IDENTITY,
                        scale: Vec3::new(0.12, 8.0, 0.12),
                    },
                    MeshRenderer {
                        mesh: beam.clone(),
                        texture: Some(gold.clone()),
                    },
                    Lit,
                    Hero,
                ));
                self.caches.push((at, glow, pillar_beam));
            }
        }
        // Some shards are skittish: they hop away twice as you close in.
        if matches!(self.objective, Some(objective::Active::Shard))
            && !wake_door
            && !self.autopilot
            && !self.coop_active()
            && !self.director.nightmare
        {
            if let Some(at) = dream.shard {
                if objective::is_skittish(dream.seed, self.director.depth) {
                    self.skittish = Some(Skittish {
                        hops_left: objective::FLINCH_HOPS,
                        home: at,
                        drawn: at,
                        flight: None,
                    });
                }
            }
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
        self.camera_pos = spawn + gameplay::CAMERA_OFFSET * self.twists.camera_zoom();
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
            * pacts::effects(&self.pacts).enemy_speed_mult // === pacts ===
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
                * pacts::effects(&self.pacts).alert_mult // === pacts ===
        };
        let cap = 0.92 * player_speed / enemy_ai::CHASE_BOOST;
        let mut elite_tex: HashMap<Elite, Arc<GpuTexture>> = HashMap::new();
        // === ascension === 12: one more watcher where you fell last dream
        let mut patrols = dream.patrols.clone();
        if let Some(c) = self.asc_remembered.filter(|_| {
            self.active_asc.remembers(self.director.depth) && !self.director.nightmare
        }) {
            let walk: Vec<(f32, f32)> = dream.lucid_route.iter().map(|w| (w.pos.x, w.pos.z)).collect();
            if let Some((x, z)) = progress::remembered_spot(
                dream.seed,
                (c.x, c.z),
                (dream.spawn.x, dream.spawn.z),
                &walk,
            ) {
                let y = dream.lucid_route.iter().find(|w| w.pos.x == x && w.pos.z == z).map_or(dream.spawn.y, |w| w.pos.y);
                let at = Vec3::new(x, y, z);
                patrols.push((at, at));
            }
        }
        // === end ascension ===
        for (k, &(a, b)) in patrols.iter().enumerate() {
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
        self.spawn_newfoes(gl, &dream, player_speed)?; // enemies-a
        // === shards ===
        if let (Some(a), Some(SpecialEnemy { ai: SpecialAI::Sentry(s, _), .. })) =
            (self.guard_angle, self.specials.last_mut())
        {
            s.angle = a;
        }
        // === end shards ===
        self.spawn_enemies_b(gl, &dream)?; // === combat ===
        // === sanctuary === a place to breathe: nerve refills
        if self.sanctuary() {
            self.nerve.heal(self.nerve.max());
            self.sfx(Sound::Sanctuary);
            self.dream_hint = "a quiet place · nothing here can hurt you".to_string();
        }
        // === end sanctuary ===
        // === hazards ===
        self.theme_rule = self.hazard_rule_state();
        if self.theme_rule.is_some() {
            let line = theme_rules::rule_for(self.director.theme).line;
            self.dream_hint = if self.dream_hint.is_empty() {
                line.to_string()
            } else {
                format!("{line} · {}", self.dream_hint)
            };
        }
        // Beat floor: more hot tiles on the route, on the existing beat clock.
        if self
            .theme_rule
            .is_some_and(|r| r.kind == theme_rules::Kind::BeatFloor)
        {
            let key = |p: Vec3| theme_rules::cell_key(p.x, p.z, gameplay::CELL);
            let jumpy = |i: usize| {
                dream.route[i].jump || dream.route.get(i + 1).is_some_and(|w| w.jump)
            };
            let route: Vec<(i32, i32)> = dream
                .route
                .iter()
                .enumerate()
                .filter(|&(i, _)| !jumpy(i))
                .map(|(_, w)| key(w.pos))
                .collect();
            let mut keep: Vec<(i32, i32)> = dream.beat_tiles.iter().copied().map(key).collect();
            keep.extend([dream.spawn, dream.portal].into_iter().map(key));
            keep.extend(dream.shard.into_iter().map(key));
            keep.extend(dream.crumbles.iter().copied().map(key));
            keep.extend(dream.shifters.iter().map(|&(c, _)| key(c)));
            let r = self.theme_rule.expect("checked above");
            let cells = theme_rules::beat_hazard_cells(&route, &keep, r.seed, r.strength);
            if !cells.is_empty() {
                let beat_tex = self.texture(gl, [255, 80, 200, 255]);
                let plate = self.mesh(Shape::Cube)?;
                for w in dream.route.iter() {
                    if !cells.contains(&key(w.pos)) {
                        continue;
                    }
                    let at = w.pos;
                    let e = self.world.spawn((
                        Transform {
                            position: at + Vec3::Y * 0.03,
                            rotation: Quat::IDENTITY,
                            scale: Vec3::new(gameplay::CELL * 0.5, 0.005, gameplay::CELL * 0.5),
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
        }
        self.shrink = (!shrink_order.is_empty()).then(|| ShrinkFloor {
            order: shrink_order,
            tiles: shrink_tiles,
            gone: 0,
        });
        // === end hazards ===
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
        self.camera_pos = spawn + gameplay::CAMERA_OFFSET * self.twists.camera_zoom();
        if let Some((d, _)) = self.dissolve.as_mut() {
            d.reset();
        }
        self.route_index = 0;
        self.grace = if self.active_asc.no_safe_respawn(self.director.depth) {
            0.0 // ascension 11: no grace
        } else {
            1.0
        } * gameplay::RESPAWN_GRACE
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
            Ok("tickets") | Ok("tickets_scratch") => {
                // === fortune === dev hook: the tickets tab, with a few tickets
                self.open_store();
                self.store_tab = 2;
                tickets::give(&mut self.booklet.stash, 3);
                if crate::dev::var("DREAMSCAPE_SCREEN").as_deref() == Ok("tickets_scratch") {
                    self.ticket_press();
                    self.ticket_press();
                }
            }
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
            Ok("pick") => {
                // Dev only: a made-up run across five themes, then the pick screen.
                self.run_log.clear();
                for (i, &theme) in dream::ALL_THEMES.iter().take(5).enumerate() {
                    for k in 0..(1 + i % 2) as u64 {
                        let seed = 100 + i as u64 * 7 + k;
                        self.run_log.push(cards::DreamRecord {
                            theme,
                            seed,
                            depth: i as u32 + 1,
                            name: dream::dream_name(theme, seed),
                            whisper: dream::whisper(theme, seed),
                            strangeness: 0.3,
                            enemies: 2,
                            shard_taken: true,
                            art: pack_art::body_spec(theme, seed),
                            blend: None,
                        });
                    }
                }
                self.start_pack_pick();
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
            Event::MouseButtonDown {
                mouse_btn: engine::sdl2::mouse::MouseButton::Left,
                x,
                y,
                ..
            } if self.mode == hud::Mode::PackPick => {
                self.pick_pointer(*x, *y, true);
                return;
            }
            // === pointer === click a menu choice; right-click steps back
            Event::MouseButtonDown {
                mouse_btn: engine::sdl2::mouse::MouseButton::Left,
                x,
                y,
                ..
            } if menu_pointer_mode(self.mode) => {
                if let Some(k) = self.menu_pointer(*x, *y, Pointer::Click) {
                    self.press(ctx, k);
                }
                return;
            }
            Event::MouseButtonDown {
                mouse_btn: engine::sdl2::mouse::MouseButton::Right,
                x,
                y,
                ..
            } if menu_pointer_mode(self.mode) => {
                self.menu_pointer(*x, *y, Pointer::Back);
                return;
            }
            // === end pointer ===
            // A real nudge of the mouse (not the synthetic event a window
            // gets when it opens under the cursor) switches back to keys.
            Event::MouseMotion { x, y, xrel, yrel, .. } => {
                if self.mode == hud::Mode::PackPick {
                    self.pick_pointer(*x, *y, false);
                }
                // === pointer === hovering a choice selects it
                if menu_pointer_mode(self.mode) && xrel.abs() + yrel.abs() > 0 {
                    let before = self.menu_cursor();
                    self.menu_pointer(*x, *y, Pointer::Hover);
                    if self.menu_cursor() != before {
                        self.sfx(Sound::Tick);
                    }
                }
                // === end pointer ===
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
        // === abilities3 === OVERLOAD charges while its key is held
        if !repeat {
            if let Some(i) = ability_slot(self.settings.action_for(key)) {
                self.ability_held[i] = down;
            }
        }
        // === end abilities3 ===
        // Ctrl+V in the code box pastes from the clipboard.
        if let Event::KeyDown {
            keycode: Some(Keycode::V),
            keymod,
            repeat: false,
            ..
        } = event
        {
            if self.mode == hud::Mode::Code
                && keymod.intersects(
                    engine::sdl2::keyboard::Mod::LCTRLMOD | engine::sdl2::keyboard::Mod::RCTRLMOD,
                )
            {
                if let Ok(text) = ctx.platform.video.clipboard().clipboard_text() {
                    self.code_paste(&text);
                }
                return;
            }
        }
        if down && !repeat && self.mode == hud::Mode::Settings {
            self.settings_key(ctx, key);
            return;
        }
        if down && !repeat {
            let revealed = reveal_ui::reveal_done_pack(&self.pack.pack, self.pack.age);
            if self.mode == hud::Mode::Code {
                self.code_key(key);
                return;
            }
            if self.mode == hud::Mode::Summary {
                if key == Keycode::C {
                    self.copy_run_code(ctx);
                }
                if matches!(key, Keycode::Return | Keycode::Space) && self.title_age > 0.5 {
                    self.title_age = 0.0;
                    self.start_pack_pick();
                }
                return;
            }
            if self.mode == hud::Mode::PackPick {
                self.pick_key(key);
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
                    Keycode::Num1
                    | Keycode::Num2
                    | Keycode::Num3
                    | Keycode::Num4
                    | Keycode::Num5
                    | Keycode::Num6
                    | Keycode::Num7 => {
                        let i = [
                            Keycode::Num1,
                            Keycode::Num2,
                            Keycode::Num3,
                            Keycode::Num4,
                            Keycode::Num5,
                            Keycode::Num6,
                            Keycode::Num7,
                        ]
                            .iter()
                            .position(|&k| k == key)
                            .expect("matched above");
                        if i < n {
                            self.choose(i);
                        }
                    }
                    Keycode::G if self.choice == Some(ChoiceKind::MothShop) => {
                        self.choose(self.gamble_row()); // === risk ===
                    }
                    Keycode::X | Keycode::Escape if matches!(self.choice, Some(ChoiceKind::Gamble(_))) => {
                        self.gamble_back(); // === risk ===
                    }
                    Keycode::X | Keycode::Escape if self.choice == Some(ChoiceKind::Wager) => {
                        self.choose(1); // === risk === not now
                    }
                    Keycode::R if self.choice == Some(ChoiceKind::MothShop) => {
                        self.choose(self.gamble_row() - 1);
                    }
                    Keycode::R => self.reroll_choice(),
                    Keycode::X | Keycode::Escape if self.choice == Some(ChoiceKind::MothShop) => {
                        self.leave_moth_shop()
                    }
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
                // P pauses too: a browser keeps Escape for itself (it lets go of
                // the mouse and leaves fullscreen), so it can't be the only key.
                (hud::Mode::Playing, Keycode::Escape | Keycode::P)
                    if key == Keycode::Escape || self.settings.action_for(key).is_none() =>
                {
                    ctx.platform.sdl.mouse().set_relative_mouse_mode(false);
                    self.mouse_captured = false;
                    self.mode = hud::Mode::Paused;
                    self.input = PlayerInputState::default();
                    return;
                }
                (hud::Mode::Paused, Keycode::Escape | Keycode::P) => {
                    self.mode = hud::Mode::Playing;
                    return;
                }
                // M, not Q: Q on the title menu quits (on the web that reloads
                // the page), so one more tap of Q must never follow this one.
                (hud::Mode::Paused, Keycode::M) => {
                    self.dream = None;
                    self.mode = hud::Mode::Title;
                    return;
                }
                (hud::Mode::Reveal, Keycode::Space | Keycode::Return) if !revealed => {
                    self.pack.age = f32::MAX;
                    return;
                }
                // More packs to open: back to the table for the next one.
                (hud::Mode::Reveal, Keycode::Space | Keycode::Return)
                    if self.pick.picks_left > 0 =>
                {
                    self.pick.age = 0.0;
                    self.mode = hud::Mode::PackPick;
                    return;
                }
                (hud::Mode::Reveal, Keycode::R | Keycode::B | Keycode::L | Keycode::Escape | Keycode::W)
                    if self.pick.picks_left > 0 =>
                {
                    return;
                }
                (hud::Mode::Reveal, Keycode::S) if self.pick.picks_left > 0 => {
                    return;
                }
                (hud::Mode::Reveal, Keycode::R) if revealed => {
                    self.restart_requested = true;
                    return;
                }
                (hud::Mode::Reveal, Keycode::S) => {
                    // Not waiting for the flips: S skips them and presses.
                    self.pack.age = f32::MAX;
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
                // W wakes (Escape still does natively; a browser eats it).
                (hud::Mode::Reveal, Keycode::Escape | Keycode::W) if revealed => {
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
                (hud::Mode::Booklet, Keycode::T) => {
                    self.sort_press(true);
                    return;
                }
                (hud::Mode::Booklet, Keycode::R) => {
                    self.sort_press(false);
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
                    self.store_tab = (self.store_tab + 1) % 3; // === fortune === perks, lottery, tickets
                    self.lottery_status = None;
                    self.ticket_status = None;
                    self.ticket_confirm = false;
                    return;
                }
                // === fortune ===
                (hud::Mode::Store, Keycode::Return | Keycode::Space) if self.store_tab == 2 => {
                    self.ticket_press();
                    return;
                }
                (hud::Mode::Store, _) if self.store_tab == 2 => {
                    self.ticket_confirm = false;
                    self.ticket_status = None;
                    return;
                }
                (hud::Mode::Store, Keycode::F) if self.store_tab == 1 => {
                    self.fortune_claim();
                    return;
                }
                // === end fortune ===
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
                    if matches!(
                        item,
                        TitleItem::Ascension | TitleItem::RunLength | TitleItem::Form
                    ) {
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
                (hud::Mode::Title, Keycode::F) => {
                    if self.title_items().contains(&TitleItem::Form) {
                        self.title_select(ctx, TitleItem::Form, 1);
                    }
                    return;
                }
                (hud::Mode::Title, Keycode::K) => {
                    self.title_select(ctx, TitleItem::DreamCode, 1);
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
                // Quitting a web page only reloads it, so there Q and Escape do
                // nothing on the title; "stay awake" is still in the menu.
                #[cfg(not(target_os = "emscripten"))]
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
            Some(Action::Ability3) if down && !repeat => self.ability_requests[2] = true,
            Some(Action::Ability4) if down && !repeat => self.ability_requests[3] = true,
            Some(Action::Ability5) if down && !repeat => self.ability_requests[4] = true,
            Some(Action::Strike) if down && !repeat => self.strike_request = true,
            _ => {}
        }
        match key {
            Keycode::Return if down && !repeat => self.ask_eye(),
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
            self.ticket_age += dt; // === fortune ===
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
        if self.can_ask_eye() {
            if self.autopilot {
                // E2E runs ask straight away.
                self.ask_eye();
            } else if !self.ask_hinted {
                self.ask_hinted = true;
                self.eye_line = Some((
                    "you found them all. ask me when you're ready.".to_string(),
                    0.0,
                ));
            }
        }
        if crate::dev::var_os("DREAMSCAPE_FPS").is_some() {
            self.frame_ms.0 += dt * 1000.0;
            self.frame_ms.1 += 1;
            if self.frame_ms.1 == 300 {
                log::info!("avg frame {:.1} ms", self.frame_ms.0 / 300.0);
                self.frame_ms = (0.0, 0);
            }
        }
        // A soft time dip on catches and shards: the world slows, then eases back.
        let dt = self.juice.step(dt.min(gameplay::MAX_DT));
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
        self.nerve.tick(dt);
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
            self.start_pack_pick();
        }
        if self.mode == hud::Mode::PackPick {
            self.pick.age += dt;
            // E2E runs: take the first pack after a beat.
            if self.autopilot && self.pick.age > 0.6 {
                self.pick_open();
            }
        }
        if self.mode == hud::Mode::Choice {
            self.choice_view.age += dt;
            // E2E runs: take the first card (WAKE at the door) after a beat.
            if self.autopilot && self.choice_view.age > 0.6 && crate::dev::var("DREAMSCAPE_SCREEN").is_err() {
                self.choose(0);
            }
            return Ok(());
        }
        if self.mode == hud::Mode::Reveal && self.pack.age < f32::MAX {
            let before = self.pack.age;
            let frozen = before < 0.0 && crate::dev::var("DREAMSCAPE_TEAR_FREEZE").is_ok();
            if !frozen {
                self.pack.age += dt;
            }
            // The foil lets go: ripping sound.
            let rip = -pack_art::TEAR_TIME * (1.0 - pack_art::TEAR_RIP_AT);
            if before < rip && self.pack.age >= rip {
                self.sfx(Sound::Tear);
            }
        }
        if std::mem::take(&mut self.restart_requested) {
            return self.restart(ctx);
        }
        if let Some(kind) = self.pending_start.take() {
            return self.start_pending(ctx, kind);
        }
        // Dev: DREAMSCAPE_OPEN=loadout|merge opens that screen once, for screenshots.
        if matches!(self.mode, hud::Mode::Title | hud::Mode::Warning)
            && self.time > 0.8
            && !self.dev_opened
        {
            if let Ok(which) = crate::dev::var("DREAMSCAPE_OPEN") {
                self.dev_opened = true;
                match which.as_str() {
                    "loadout" => self.open_loadout(),
                    "title" => self.mode = hud::Mode::Title,
                    "memories" => {
                        self.memories_sel = 0;
                        self.mode = hud::Mode::Memories;
                    }
                    "settings" => self.open_settings(),
                    "codex" => self.open_codex(),
                    "booklet" => self.open_booklet(),
                    "store" => self.open_store(),
                    // === shop === "store:<shelf>:<col>" for shelf screenshots.
                    st if st.starts_with("store:") => {
                        let mut n = st[6..].split(':').map(|x| x.parse::<usize>().unwrap_or(0));
                        self.open_store();
                        self.shop_shelf = n.next().unwrap_or(0).min(store::Shelf::ALL.len() - 1);
                        self.shop_col = shop_ui::clamp_col(self.shop_shelf, n.next().unwrap_or(0));
                    }
                    // === end shop ===
                    "lottery" => {
                        self.open_store();
                        self.store_tab = 1;
                    }
                    "tickets" => {
                        self.open_store();
                        self.store_tab = 2; // === fortune ===
                    }
                    "summary" => {
                        self.title_age = 0.0;
                        self.mode = hud::Mode::Summary;
                    }
                    "pick" => self.start_pack_pick(),
                    "merge" => self.open_merge(hud::Mode::Title),
                    // The first card picked, the cursor on one that can merge with it.
                    "merge2" => {
                        self.open_merge(hud::Mode::Title);
                        let picked = {
                            let cards = self.sorted_cards(self.settings.sort_merge);
                            cards.first().map(|a| {
                                let i = cards.iter().position(|b| fusion::can_merge(a, b));
                                (a.number, i)
                            })
                        };
                        if let Some((number, i)) = picked {
                            self.merge_first = Some(number);
                            if let Some(i) = i {
                                self.merge_sel = i;
                            }
                        }
                    }
                    _ => {}
                }
            }
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
                        if !self.open_event_choice() {
                            self.open_upgrade_choice();
                        }
                        return Ok(());
                    }
                }
            }
        }
        if self.transition.active() {
            self.camera_pos = gameplay::follow_camera(
                self.camera_pos,
                self.player_position,
                dt,
                self.twists.camera_zoom(),
            );
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
        if let Some(r) = self.theme_rule.as_mut() {
            r.tick(dt); // hazards
        }
        // === hazards === drifting fog: the pockets (and what slows you) move
        if let Some(r) = self.theme_rule {
            let drift = r.effect().fog_drift;
            if drift > 0.0 && self.fog.len() == self.fog_discs.len() {
                for (i, &(e, base)) in self.fog_discs.iter().enumerate() {
                    let (x, z) = theme_rules::fog_center((base.x, base.z), i, r.age, drift, r.seed);
                    self.fog[i].0 = Vec3::new(x, base.y, z);
                    if let Ok(mut tr) = self.world.get::<&mut Transform>(e) {
                        tr.position = Vec3::new(x, base.y + 0.3, z);
                    }
                }
            }
        }
        self.update_shrink();
        // === end hazards ===
        // === ascension === 15: the eye's clock runs out once per dream
        if !self.asc_eye_failed
            && self.active_asc.eye_timer(self.director.depth).is_some_and(|t| self.dream_age > t)
            && self.dream_timer().is_some()
            && !self.director.nightmare
            && self.prologue.is_none()
        {
            self.asc_eye_failed = true;
            if let Some(u) = progress::lose_last_card(&mut self.run.taken) {
                log::info!("The eye closed: {u:?} is gone");
            }
        }
        // === end ascension ===
        self.stats.time += dt;
        self.run.tick(dt);
        self.update_deck(dt); // === deck ===
        // === pressure ===
        self.update_pressure(dt);
        // === end pressure ===

        // 0b. Abilities
        let requests = std::mem::take(&mut self.ability_requests);
        for (slot, _) in requests.iter().enumerate().filter(|(_, &r)| r) {
            self.use_ability(slot);
        }
        // Dev: DREAMSCAPE_FIRE=<slot> with the autopilot uses that ability at
        // DREAMSCAPE_FIRE_AT seconds (and again each second after), for screenshots.
        if self.autopilot {
            if let Some(slot) = crate::dev::var("DREAMSCAPE_FIRE")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
            {
                let at: f32 = crate::dev::var("DREAMSCAPE_FIRE_AT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(2.0);
                if self.dream_age >= at && !self.dev_fired {
                    self.dev_fired = true;
                    self.use_ability(slot);
                }
            }
        }
        // === shop ===
        // Dev: DREAMSCAPE_SCREEN=moth opens the moth's room at 1 s of dream
        // time with the autopilot (which then stops picking), for screenshots.
        if self.autopilot && self.dream_age >= 1.0 && !self.dev_fired {
            if let Ok(what) = crate::dev::var("DREAMSCAPE_SCREEN") {
                self.dev_fired = true;
                let mut it = what.split(':');
                match it.next() {
                    Some("moth") => {
                        self.bonus_dust = 40;
                        self.ensure_moth_stock();
                        self.show_moth_shop(0);
                    }
                    _ => {}
                }
            }
        }
        // === end shop ===
        // === combat ===
        self.update_combat(dt);
        // === end combat ===
        self.update_abilities2(dt); // === abilities2 ===
        self.update_abilities3(dt); // === abilities3 ===
        if self.run.active(Ability::ShardCall) {
            self.pull_shard(dt);
        }
        self.update_objective(ctx, dt);
        self.update_skittish(dt);
        self.update_caches();
        self.update_secret();
        self.update_wager(); // === risk ===
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
            } * self.theme_rule.map_or(1.0, |r| r.effect().speed); // hazards
            let speed = self.player_speed() * ground;
            // MIRRORED swaps the actions (left <-> right), after any key remap.
            let mirrored = self.twists.mirrors_controls();
            let walk = if fp {
                fpv::velocity(&self.input, self.yaw)
            } else {
                gameplay::horizontal_velocity(&dream::mirror_input(self.input, mirrored))
            };
            let mut v = walk * (speed / gameplay::MOVE_SPEED);
            // The left stick, if it's pushed, wins (analogue: half-push walks).
            if let Some((x, z)) = settings::stick_dir(ctx.input.left_stick())
                .map(|s| dream::mirror_stick(s, mirrored))
            {
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
                self.facing * gameplay::MOVE_SPEED * upgrades::DASH_SPEED_MULT * self.dash_boost
            } else {
                desired
            };
            // === hazards ===
            let push = self.theme_rule.map_or((0.0, 0.0), |r| r.effect().push);
            // === combat === the shove from a hit
            let knock = self.nerve.knock_velocity();
            let walk = if self.strike.recovering() {
                if self.run.count(Upgrade::SwiftFeet) > 0 {
                    combat::RECOVERY_WALK_SWIFT
                } else {
                    combat::RECOVERY_WALK
                }
            } else {
                1.0
            };
            let lunge = self.lunge_dir * edges::lunge_speed(self.lunge_left);
            body.velocity.x = v.x * walk + push.0 + knock.0 + lunge.x;
            body.velocity.z = v.z * walk + push.1 + knock.1 + lunge.z;
            // === end combat ===
            // === end hazards ===
            // === sounds === a thud when a fall ends
            if body.grounded && !self.was_grounded && self.air_vy < -4.0 && !self.autopilot {
                let pitch = (1.25 + self.air_vy / 40.0).clamp(0.7, 1.2);
                self.sfx_pitched(Sound::Land, pitch);
            }
            if !body.grounded {
                self.air_vy = body.velocity.y;
            }
            self.was_grounded = body.grounded;
            // === end sounds ===
            self.player_grounded = body.grounded;
            if body.grounded {
                self.air_jumps_used = 0;
            }
            if wants_jump && body.grounded {
                body.velocity.y = jump_speed * self.surge.take_jump(); // === abilities2 ===
                self.jump_event = true; // === combat === a leech counts it
                if jump_pressed {
                    self.sfx(Sound::Jump);
                    fire_passive(&mut self.passive_fired, &self.run, self.dream_age, Upgrade::Spring);
                }
            } else if jump_pressed && !body.grounded && self.air_jumps_used < self.run.air_jumps() {
                self.air_jumps_used += 1;
                body.velocity.y = jump_speed * self.surge.take_jump(); // === abilities2 ===
                self.jump_event = true; // === combat === a leech counts it
                self.sfx(Sound::AirJump);
                fire_passive(&mut self.passive_fired, &self.run, self.dream_age, Upgrade::DoubleJump);
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
        let ghost = self.echo_ghost_pos();
        let mut splits = Vec::new();
        let mut noticed = false;
        // === tactics ===
        let tactic_depth = self.director.depth;
        let tactics_on = tactics::active(tactic_depth, !hunted.is_empty() || self.hunter.is_some());
        self.tactics.tick(self.enemies.len(), dt);
        let mut chase_starts: Vec<(usize, Option<Elite>)> = Vec::new();
        // === end tactics ===
        let hush = if self.run.active(Ability::Hush) {
            upgrades::HUSH_SLOW
        } else {
            1.0
        };
        // === abilities2 ===
        let veiled = self.run.active(Ability::Veil);
        let pulsing = self.run.active(Ability::Pulse);
        let haste = self.haste.map(|h| h.0);
        // === end abilities2 ===
        // === shards ===
        let want_alert = shard_rules::heavy_mults(self.heavy_elapsed()).1;
        if (want_alert - self.heavy_applied).abs() > 1e-4 {
            let f = want_alert / self.heavy_applied;
            for p in &mut self.enemies {
                p.ai.scale_alert(f);
            }
            self.heavy_applied = want_alert;
        }
        // === end shards ===
        for (k, p) in self.enemies.iter_mut().enumerate() {
            if held || puppet {
                continue;
            }
            // Reeling from a SLAM or GUST.
            if let Some(&left) = self.stunned.get(&p.entity) {
                if left - dt <= 0.0 {
                    self.stunned.remove(&p.entity);
                } else {
                    self.stunned.insert(p.entity, left - dt);
                }
                continue;
            }
            if let Ok(mut t) = self.world.get::<&mut Transform>(p.entity) {
                let was = p.ai.chasing;
                let target = if self.run.active(Ability::Decoy) {
                    target
                } else if hunted.is_empty() {
                    // ECHO: they chase your ghost too, if it is nearer.
                    dream::echo_chase_target(t.position, target, ghost)
                } else {
                    nearest(&hunted, t.position)
                };
                // === abilities2 === VEIL: they lose you; HASTE: they drag in the zone
                if veiled && p.ai.chasing {
                    p.ai.reset();
                }
                let drag = haste.map_or(1.0, |h| h.foe_mult(t.position));
                p.ai.update(
                    &mut t.position,
                    abilities2::seen_from(target, veiled),
                    dt * hush * drag,
                );
                // === end abilities2 ===
                // Swell when they notice you: the tell that you've been seen.
                let size = if p.ai.chasing { 1.25 } else { 0.9 }
                    * match p.elite {
                        Some(Elite::Big) => 1.35,
                        Some(Elite::Splitter) if p.split => 0.75,
                        _ => 1.0,
                    };
                let hidden = p.elite == Some(Elite::Shade)
                    && (t.position - self.player_position).length()
                        > if pulsing {
                            abilities2::PULSE_SHADE_SHOWS // === abilities2 ===
                        } else {
                            specials::SHADE_SHOWS_WITHIN
                        };
                t.scale = Vec3::splat(if hidden { 0.0 } else { size });
                if p.ai.chasing && !was && !self.autopilot {
                    log::info!("An enemy noticed you");
                    noticed = true;
                    chase_starts.push((k, p.elite)); // tactics
                    if p.elite == Some(Elite::Splitter) && !p.split {
                        p.split = true;
                        splits.push((k, t.position));
                    }
                }
            }
        }
        // === tactics ===
        if tactics_on && !chase_starts.is_empty() {
            let pos_of = |w: &World, e: Entity| w.get::<&Transform>(e).ok().map(|t| t.position);
            let snapshot: Vec<(usize, Vec3)> = self
                .enemies
                .iter()
                .enumerate()
                .filter_map(|(k, p)| pos_of(&self.world, p.entity).map(|v| (k, v)))
                .collect();
            for (k, elite) in chase_starts {
                let Some(&(_, from)) = snapshot.iter().find(|&&(i, _)| i == k) else {
                    continue;
                };
                let others: Vec<(usize, Vec3, f32)> = snapshot
                    .iter()
                    .map(|&(i, v)| (i, v, self.tactics.cooldown(i)))
                    .collect();
                for i in tactics::shout_targets(tactic_depth, k, from, &others) {
                    self.tactics.mark(i);
                    self.enemies[i].ai.alert();
                }
                for i in tactics::pack_follow(tactic_depth, k, elite, from, &snapshot) {
                    self.enemies[i].ai.alert();
                }
            }
        }
        // === end tactics ===
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
            if let Some((at, e)) = self.decoy.take() {
                let _ = self.world.despawn(e);
                self.decoy_burst(at); // combat
            }
        }
        // An anchor left too long fades; so does one whose card is gone.
        if let Some((_, e, age)) = self.anchor.as_mut().map(|a| (a.0, a.1, &mut a.2)) {
            *age += dt;
            if *age > upgrades::ANCHOR_SECONDS {
                let _ = self.world.despawn(e);
                self.anchor = None;
            }
        }
        self.update_specials(dt, held)?;
        self.update_newfoes(dt, held); // enemies-a
        self.update_features();
        let mut boss_events = Vec::new();
        if let Some((entity, b, _)) = self.hunter.as_mut() {
            if b.defeated {
                // === combat === undone: it comes apart where it stands
                if let Ok(mut t) = self.world.get::<&mut Transform>(*entity) {
                    t.scale *= (1.0 - 4.0 * dt).max(0.0);
                }
                // === end combat ===
            } else if !held && !puppet {
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
        self.update_slam();
        self.update_climb();
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
        self.camera_pos = gameplay::follow_camera(
                self.camera_pos,
                self.player_position,
                dt,
                self.twists.camera_zoom(),
            );
        self.update_door_hum(dt);
        self.update_echoes(player, dt)?;
        self.update_echo_ghost()?;
        self.update_bursts(ctx.gl())?;
        self.update_ability_fx(ctx.gl(), player, dt)?;
        self.update_adaptive_music(dt);
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
        let untouchable = self.run.active(Ability::Dash)
            || self.run.active(Ability::Phase)
            || self.nerve.invulnerable();
        let caught_by =
            if !self.autopilot && self.grace <= 0.0 && !untouchable && !self.coop_ghost() {
                self.threat_touching()
            } else {
                None
            };
        // === combat === COUNTER: slipping a blow with a dash or phase
        if !self.autopilot
            && self.grace <= 0.0
            && !self.coop_active()
            && self.counter_cd <= 0.0
            && self.run.count(Upgrade::Counter) > 0
            && (self.run.active(Ability::Dash) || self.run.active(Ability::Phase))
            && self.threat_touching().is_some()
        {
            self.counter_attack();
        }
        // === end combat ===
        // WARD takes the hit meant for you.
        let caught_by = if caught_by.is_some() && self.run.active(Ability::Ward) {
            self.break_ward();
            self.ward_reflect(); // combat
            None
        } else if caught_by.is_some() && self.run.active(Ability::Mirror) {
            self.break_mirror(); // === abilities2 ===
            None
        } else {
            caught_by
        };
        // === combat === a touch costs nerve; only the last point is a catch
        let caught_by = match caught_by {
            Some(by) if !self.coop_active() => self.take_nerve_hit(by),
            other => other,
        };
        // === end combat ===
        let caught = caught_by.is_some();
        if caught {
            self.release_leeches(); // === combat === the respawn shakes them off
        }
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
            self.juice.hit(juice::Hit::Caught, self.settings.reduced_motion);
            self.sfx(Sound::Caught);
            self.stats.caught += 1;
            self.caught_this_dream += 1;
            self.coop_caught();
            return Ok(());
        }
        if caught {
            self.flash.trigger([1.0, 0.1, 0.15], 0.7);
            self.juice.hit(juice::Hit::Caught, self.settings.reduced_motion);
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
                self.passive_fires(Upgrade::DreamAnchor);
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
                    if let Some(s) = &mut self.skittish {
                        s.home = at;
                        s.drawn = at;
                        s.flight = None;
                    }
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
                // === ascension === 11: half your lucidity; 12: the spot is remembered
                if self.active_asc.no_safe_respawn(self.director.depth) {
                    self.director.lucidity = progress::Ascension::halve_lucidity(self.director.lucidity);
                }
                if self.active_asc.remembers(self.director.depth) {
                    self.asc_caught_here = Some(self.player_position);
                }
                // === end ascension ===
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
        let mut touched: Vec<Entity> = overlaps
            .iter()
            .filter(|&&(a, _)| a == player)
            .map(|&(_, b)| b)
            .collect();
        // ECHO: the ghost banks a shard (or a fragment) it walks up to, exactly
        // as if the dreamer had touched it. Nothing else: not the wake door, the
        // portal, a note or a chased memory.
        if let Some(g) = self.echo_ghost_pos() {
            for (e, (t, _)) in self.world.query::<(&Transform, &ShardMarker)>().iter() {
                if dream::echo_reaches(g, t.position) && !touched.contains(&e) {
                    touched.push(e);
                }
            }
        }
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
                    let pitch = self.juice.chain_pitch();
                    self.juice.hit(juice::Hit::Pickup, self.settings.reduced_motion);
                    self.sfx_pitched(Sound::Sigil, pitch);
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
            let pitch = self.juice.chain_pitch();
            self.juice.hit(juice::Hit::Pickup, self.settings.reduced_motion);
            self.sfx_pitched(Sound::Sigil, pitch);
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
        let wagered = std::mem::take(&mut self.wager_enter); // === risk ===
        if (wagered
            || touched
                .iter()
                .any(|&e| self.world.get::<&PortalMarker>(e).is_ok()))
            && !waiting
            && !self.coop_ghost()
        {
            log::info!("Portal entered at depth {}", self.director.depth);
            self.log_balance_row();
            self.tutorial(tutorial::Event::Portal);
            self.dream_bonus();
            self.stats.twists.extend(self.twists.0.iter().copied());
            if self.director.nightmare {
                self.stats.nightmares += 1;
                self.bonus_dust += 10
                    * self.hunter.as_ref().map_or(1, |(_, b, _)| b.tier)
                    * self.perk_fx().nightmare_mult; // === shop ===
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
            // === risk === a wagered dream pays double; taking the second door
            // makes the next one wagered
            if self.gamble.wager_active {
                self.bonus_dust = gamble::doubled(self.wager_dust_start, self.bonus_dust);
            }
            self.gamble.finish_dream();
            if wagered {
                self.gamble.take_wager();
                log::info!("The wager door is taken at depth {}", self.director.depth);
            }
            // === end risk ===
            if self.coop_active() {
                self.coop_enter_portal();
                return Ok(());
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
        let mut hud_view = self.hud_view();
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
        let fog_push = if self.debug_camera {
            0.0
        } else {
            self.twists.fog_push()
        };
        let sight_scale = self
            .twists
            .sight_at(self.dream_age, self.settings.reduce_flashing)
            * self.run.sight()
            * self.active_form.sight_mult()
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
        // === hazards ===
        let sight_scale = sight_scale
            * self
                .theme_rule
                .map_or(1.0, |r| theme_rules::dimmed_sight(1.0, &r.effect()));
        // === end hazards ===
        let sight = if tunnel {
            tunnel::sight_radius(strangeness) * sight_scale + sight_bonus
        } else {
            0.0
        };
        let fp = self.first_person_active();
        let fov = if fp { fpv::FOV_DEG } else { 60.0_f32 };
        let shake = if self.mode == hud::Mode::Playing {
            self.juice.shake_offset()
        } else {
            Vec3::ZERO
        };
        let (eye, target) = if self.debug_camera {
            (Vec3::new(0.0, 45.0, -35.0), Vec3::ZERO)
        } else if fp {
            fpv::camera(self.player_position, self.yaw, self.pitch)
        } else {
            (self.camera_pos, self.player_position)
        };
        let (eye, target) = (eye + shake, target + shake);
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
                gl.uniform_1_f32(Some(&l), params.fog_start + fog_push);
            }
            if let Some(l) = loc("uFogEnd") {
                gl.uniform_1_f32(Some(&l), params.fog_end + fog_push);
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
            for (_entity, (transform, mesh_renderer, uv, solid, lit, hero, water, body, halo, sil)) in
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
                        Option<&Silhouette>,
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
                        gl.uniform_1_f32(Some(l), gameplay::lit_level(lit.is_some(), sil.is_some()));
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
        self.screen_size = (drawable_size.0 as f32, drawable_size.1 as f32);
        // === combat === hit-point pips over enemies, projected to the screen
        if hud_view.mode == hud::Mode::Playing && !fp && !self.coop_active() {
            let view_proj = Mat4::perspective_rh(fov.to_radians(), aspect_for_overlay, 0.1, 1000.0)
                * Mat4::look_at_rh(eye, target, up);
            hud_view.foe_pips = self.foe_pips(view_proj);
        }
        // === end combat ===
        let card_art = &mut self.card_art;
        let pack = &self.pack;
        let pick = &self.pick;
        let packs_art = &mut self.packs_art;
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
            ui.ui_scale = hud::ui_scale(hud_view.mode, drawable_size.0 as f32, drawable_size.1 as f32);
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
                hud::draw(egui_ctx, &hud_view, card_art, pack, pick, packs_art);
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
        let all = self.sorted_cards(self.settings.sort_booklet);
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
        // Never read or write the shared settings file (sort choices persist there).
        g.settings = settings::Settings::default();
        g.settings_path = std::env::temp_dir().join(format!("dreamscape_smoke_{name}_settings.ron"));
        g
    }

    fn shapes(g: &DreamscapeGame) -> usize {
        frame(g, 1280.0, 720.0).len()
    }

    /// One drawn frame of the current screen at the given size.
    fn frame(g: &DreamscapeGame, w: f32, h: f32) -> Vec<egui::epaint::ClippedShape> {
        frame_with(g, w, h, &|_| {})
    }

    /// `frame`, with the view bent first (to put long strings on screen).
    fn frame_with(
        g: &DreamscapeGame,
        w: f32,
        h: f32,
        tweak: &dyn Fn(&mut hud::HudView),
    ) -> Vec<egui::epaint::ClippedShape> {
        let mut v = g.hud_view();
        tweak(&mut v);
        let ctx = egui::Context::default();
        hud::install_font(&ctx);
        let mut art = booklet_ui::CardArt::default();
        let mut packs_art = pack_art::PackArt::default();
        // Points, as the game gives egui them: the HUD is drawn scaled.
        let k = hud::ui_scale(v.mode, w, h);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(w / k, h / k),
            )),
            ..Default::default()
        };
        let out = ctx.run(input, |ctx| {
            hud::draw(ctx, &v, &mut art, &g.pack, &g.pick, &mut packs_art)
        });
        out.shapes
    }

    /// A hand-built offer (never calls `offer_packs`).
    fn offer(theme: dream::DreamTheme, cards: usize, seed: u64) -> packs::PackOffer {
        let cards: Vec<cards::Recalled> = (0..cards)
            .map(|i| cards::Recalled {
                card: cards::card_from(&cards::tests::record(theme, seed + i as u64, 1 + i as u32, 0.2, false)),
                remembered: i % 2 == 0,
            })
            .collect();
        packs::PackOffer {
            theme,
            emblem: packs::Emblem::ALL[seed as usize % packs::Emblem::ALL.len()],
            seed,
            memory_chances: [0.6, 0.5, 0.4, 0.3, 0.2][..cards.len()].to_vec(),
            cards,
        }
    }

    fn offers(n: usize) -> Vec<packs::PackOffer> {
        use dream::DreamTheme::*;
        [(Garden, 3), (CursedForest, 1), (MirrorHall, 5)]
            .into_iter()
            .take(n)
            .enumerate()
            .map(|(i, (t, k))| offer(t, k, 100 + i as u64 * 7))
            .collect()
    }

    #[test]
    fn the_pick_screen_draws_one_two_and_three_packs_at_two_sizes() {
        for n in 1..=3 {
            let mut g = game(&format!("pick{n}"));
            g.begin_pick(offers(n));
            assert_eq!(g.mode, hud::Mode::PackPick);
            for (w, h) in [(1280.0, 720.0), (640.0, 360.0)] {
                let shapes = frame(&g, w, h).len();
                assert!(shapes > 20 + 5 * n, "{n} offers at {w}x{h}: {shapes} shapes");
            }
        }
    }

    #[test]
    fn reduced_motion_makes_the_pick_screen_static() {
        let mut g = game("pick_reduced");
        g.settings.reduced_motion = true;
        g.begin_pick(offers(3));
        let a = frame(&g, 1280.0, 720.0);
        g.time += 3.3;
        g.pick.age += 3.3;
        let b = frame(&g, 1280.0, 720.0);
        assert!(a == b, "reduced motion: frames differ over time");
        // Full motion does animate (so the test above is not vacuous).
        g.settings.reduced_motion = false;
        let c = frame(&g, 1280.0, 720.0);
        g.time += 0.7;
        let d = frame(&g, 1280.0, 720.0);
        assert!(c != d, "full motion should animate");
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

    // === fortune ===
    #[test]
    fn the_daily_fortune_is_free_once_a_day_and_the_title_says_so_plainly() {
        let mut g = game("fortune_day");
        g.mode = hud::Mode::Title;
        assert!(g.title_goal(&[]).contains(FORTUNE_LINE));
        assert!(g.title_rows().1, "the title reserves room for the line");
        g.open_store();
        g.store_tab = 1;
        let view = g.hud_view().lottery;
        let f = view.fortune.expect("odds shown while a fortune waits");
        assert!((f.odds.iter().map(|o| o.1).sum::<f32>() - 1.0).abs() < 1e-4);
        assert!(shapes(&g) > 50);
        g.fortune_claim();
        assert_eq!(g.booklet.card_count(), 1, "a free card");
        assert_eq!(g.booklet.stash.dust, 0, "it costs nothing");
        assert_eq!(g.booklet.lottery.total_pulls, 0, "and does not move pity");
        assert!(g.lottery_outcome.as_ref().unwrap().near_miss.is_none());
        g.fortune_claim();
        assert_eq!(g.booklet.card_count(), 1, "once per day");
        assert!(g.hud_view().lottery.fortune.is_none());
        g.mode = hud::Mode::Title;
        assert!(!g.title_goal(&[]).contains(FORTUNE_LINE));
        // And it is saved: a reloaded booklet has no fortune waiting.
        let again = cards::load(&g.booklet_path);
        assert!(!again.fortune.available(today()));
    }

    #[test]
    fn a_ticket_needs_two_presses_and_the_prize_is_saved_before_it_shows() {
        let mut g = game("tickets_scratch");
        g.open_store();
        g.store_tab = 2;
        assert!(shapes(&g) > 40);
        g.ticket_press();
        assert!(g.ticket_revealed.is_none(), "no tickets: nothing happens");
        tickets::give(&mut g.booklet.stash, 2);
        g.ticket_press();
        assert_eq!(g.booklet.stash.tickets, 2, "first press only asks");
        assert!(g.ticket_confirm);
        assert!(shapes(&g) > 40, "confirm state draws");
        g.ticket_press();
        assert_eq!(g.booklet.stash.tickets, 1);
        assert_eq!(g.booklet.stash.ticket_scratches, 1);
        assert!(g.ticket_revealed.is_some());
        let saved = cards::load(&g.booklet_path);
        assert_eq!(saved.stash.tickets, 1, "written before the animation");
        assert_eq!(saved.stash.ticket_scratches, 1);
        for age in [0.0, 0.4, tickets_ui::SCRATCH, 3.0] {
            g.ticket_age = age;
            assert!(shapes(&g) > 40, "age {age}");
        }
        g.settings.reduced_motion = true;
        assert!(shapes(&g) > 40);
        let v = g.hud_view().tickets;
        assert!((v.odds.iter().map(|o| o.1).sum::<f32>() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn clean_dreams_earn_tickets_up_to_the_run_cap_and_a_hurt_dream_earns_none() {
        let mut g = game("tickets_clean");
        g.nerve_lost_this_dream = true;
        g.ticket_for_clean_dream();
        assert_eq!(g.booklet.stash.tickets, 0, "nerve was lost");
        g.nerve_lost_this_dream = false;
        for _ in 0..tickets::TICKETS_PER_RUN_CAP + 4 {
            g.ticket_for_clean_dream();
        }
        assert_eq!(g.booklet.stash.tickets, tickets::TICKETS_PER_RUN_CAP);
        g.tickets_this_run = 0; // a new run
        g.ticket_for_clean_dream();
        assert_eq!(g.booklet.stash.tickets, tickets::TICKETS_PER_RUN_CAP + 1);
        let mut c = game("tickets_autopilot");
        c.autopilot = true;
        c.ticket_for_clean_dream();
        assert_eq!(c.booklet.stash.tickets, 0, "inert on the autopilot");
    }

    // === end fortune ===

    fn card(ability: &'static str, since_use: f32) -> hand::HandCard {
        hand::HandCard {
            ability,
            card: None,
            key: "1",
            charge: 0.5,
            active: false,
            tint: [255, 120, 200],
            tint2: [90, 200, 255],
            since_use,
            passive: Some(("SPRING", [120, 255, 200])),
            passive_since: hand::NEVER,
            edge: Some(("LUNGE", [120, 220, 255])),
            edge_since: hand::NEVER,
            cracked: false,
            since_draw: hand::NEVER,
            empty: false,
            tool: false,
        }
    }

    fn themed(theme: dream::DreamTheme, seed: u64) -> cards::Card {
        cards::card_from(&cards::tests::record(theme, seed, 3, 0.4, false))
    }

    // === abilities2 ===
    #[test]
    fn quake_staggers_what_is_near_and_needs_the_ground() {
        let mut g = fighter("quake", Some(Ability::Quake));
        let near = put_pacer(&mut g, Vec3::new(3.0, 1.0, 0.0), false);
        let far = put_pacer(&mut g, Vec3::new(abilities2::QUAKE_REACH + 4.0, 1.0, 0.0), false);
        assert!(g.abilities2_ready(Ability::Quake));
        g.abilities2_on_use(Ability::Quake, 1.0, 1.0);
        assert_eq!(g.stunned.get(&near), Some(&abilities2::QUAKE_STAGGER));
        assert!(!g.stunned.contains_key(&far));
        g.player_grounded = false;
        assert!(!g.abilities2_ready(Ability::Quake), "no ground, no quake");
    }

    #[test]
    fn bind_roots_only_the_nearest_enemy_and_needs_one() {
        let mut g = fighter("bind", Some(Ability::Bind));
        assert!(!g.abilities2_ready(Ability::Bind), "nothing to bind");
        let a = put_pacer(&mut g, Vec3::new(8.0, 1.0, 0.0), true);
        let b = put_pacer(&mut g, Vec3::new(2.0, 1.0, 0.0), true);
        assert!(g.abilities2_ready(Ability::Bind));
        g.abilities2_on_use(Ability::Bind, 1.0, 1.0);
        assert_eq!(g.stunned.get(&b), Some(&abilities2::BIND_SECS));
        assert!(!g.stunned.contains_key(&a));
    }

    #[test]
    fn a_bound_special_staggers_for_the_whole_bind() {
        let mut g = fighter("bind2", Some(Ability::Bind));
        let e = put_special(&mut g, EnemyKind::Stalker, Vec3::new(3.0, 1.0, 0.0), SpecialAI::Stalker(specials::Stalker::new(Vec3::new(3.0, 1.0, 0.0), 5.0)));
        g.abilities2_on_use(Ability::Bind, 1.0, 1.0);
        assert_eq!(g.staggered.get(&e), Some(&abilities2::BIND_SECS));
    }

    #[test]
    fn a_mirror_throws_the_blow_back_once() {
        let mut g = fighter("mirror", Some(Ability::Mirror));
        let hitter = put_pacer(&mut g, Vec3::new(1.0, 1.0, 0.0), true);
        let bystander = put_pacer(&mut g, Vec3::new(4.0, 1.0, 3.0), false);
        g.break_mirror();
        assert!(!g.run.active(Ability::Mirror), "spent");
        assert!(g.grace >= abilities2::MIRROR_GRACE);
        assert_eq!(g.stunned.get(&bystander), Some(&abilities2::MIRROR_STUN));
        assert_eq!(g.stunned.get(&hitter), Some(&abilities2::MIRROR_STUN));
        assert_eq!(g.foes[&hitter].hp, 1.0, "the attacker took a hit");
    }

    #[test]
    fn veil_makes_every_blow_a_sneak_attack() {
        let mut g = fighter("veil", Some(Ability::Quake)); // no veil yet
        let e = put_pacer(&mut g, Vec3::new(1.0, 1.0, 0.0), true);
        let t = g.combat_targets().into_iter().find(|t| t.entity == e).unwrap();
        assert!(!g.strike_ctx(&t).unaware, "it is chasing you");
        g.run.take(Upgrade::Learn(Ability::Veil));
        let slot = g.run.abilities.iter().position(|a| a.ability == Ability::Veil).unwrap();
        g.run.abilities[slot].active = 3.0;
        assert!(g.strike_ctx(&t).unaware);
        assert!(g.strike_damage(&t) > 1.0 - 1e-6);
    }

    #[test]
    fn surge_doubles_the_next_dash_and_fades_unused() {
        let mut g = fighter("surge", Some(Ability::Surge));
        g.abilities2_on_use(Ability::Surge, 1.0, 1.0);
        g.abilities2_on_use(Ability::Dash, 1.0, 1.0);
        assert_eq!(g.dash_boost, abilities2::SURGE_MULT);
        assert_eq!(g.surge.take_jump(), abilities2::SURGE_MULT, "the jump is still owed");
        g.update_abilities2(0.016);
        assert_eq!(g.dash_boost, 1.0, "no dash in flight");
        g.abilities2_on_use(Ability::Surge, 1.0, 1.0);
        g.run.abilities[0].active = 0.0;
        g.update_abilities2(0.016);
        assert_eq!(g.surge.take_jump(), 1.0, "the surge faded");
    }

    #[test]
    fn haste_quickens_you_inside_its_zone_until_it_fades() {
        let mut g = fighter("haste", Some(Ability::Haste));
        let base = g.player_speed();
        g.abilities2_on_use(Ability::Haste, 1.0, 1.0);
        assert!((g.player_speed() / base - abilities2::HASTE_PLAYER).abs() < 1e-4);
        g.player_position = Vec3::new(abilities2::HASTE_RADIUS + 3.0, 1.0, 0.0);
        assert!((g.player_speed() - base).abs() < 1e-4, "outside the zone");
        g.player_position = Vec3::new(0.0, 1.0, 0.0);
        g.update_abilities2(Ability::Haste.duration() + 0.1);
        assert!(g.haste.is_none());
        assert!((g.player_speed() - base).abs() < 1e-4);
    }

    #[test]
    fn tether_drags_the_nearest_mote_to_you() {
        let mut g = fighter("tether", Some(Ability::Tether));
        assert!(!g.abilities2_ready(Ability::Tether), "nothing to pull");
        let mote = g.world.spawn((
            Transform { position: Vec3::new(10.0, 1.0, 0.0), rotation: Quat::IDENTITY, scale: Vec3::ONE },
            Spin(1.0),
        ));
        g.motes.push((mote, 0.0));
        assert!(g.abilities2_ready(Ability::Tether));
        for _ in 0..10 {
            g.update_abilities2(1.0 / 30.0);
        }
        let x = g.world.get::<&Transform>(mote).unwrap().position.x;
        assert!(x < 6.0 && x > 3.0, "pulled a third of a second to {x}");
    }
    // === end abilities2 ===

    // === abilities3 ===
    fn with_ability(name: &str, a: Ability) -> DreamscapeGame {
        let mut g = game(name);
        g.player_position = Vec3::new(0.0, 1.0, 0.0);
        g.player_grounded = true;
        g.dream_age = 5.0;
        g.run.take(Upgrade::Learn(a));
        g
    }

    #[test]
    fn swap_trades_places_with_the_nearest_foe_and_staggers_it() {
        let mut g = with_ability("swap", Ability::Swap);
        g.use_ability(0);
        assert!(g.run.abilities[0].cooldown <= 0.0, "nothing to swap with: charge kept");
        let near = put_pacer(&mut g, Vec3::new(6.0, 1.0, 0.0), true);
        let _far = put_pacer(&mut g, Vec3::new(10.0, 1.0, 0.0), true);
        g.use_ability(0);
        assert_eq!(g.player_position, Vec3::new(6.0, 1.0, 0.0));
        assert_eq!(g.world.get::<&Transform>(near).unwrap().position, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(g.stunned.get(&near), Some(&abilities3::SWAP_STAGGER));
        assert!(g.run.abilities[0].cooldown > 0.0);
    }

    #[test]
    fn swap_can_trade_with_the_decoy() {
        let mut g = with_ability("swap2", Ability::Swap);
        let e = g.world.spawn((Transform { position: Vec3::new(4.0, 1.0, 0.0), rotation: Quat::IDENTITY, scale: Vec3::ONE }, Spin(1.0)));
        g.decoy = Some((Vec3::new(4.0, 1.0, 0.0), e));
        g.use_ability(0);
        assert_eq!(g.player_position, Vec3::new(4.0, 1.0, 0.0));
        assert_eq!(g.decoy.map(|d| d.0), Some(Vec3::new(0.0, 1.0, 0.0)));
        assert_eq!(g.world.get::<&Transform>(e).unwrap().position, Vec3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn bargain_costs_a_nerve_and_triples_three_strikes_then_ends() {
        let mut g = with_ability("bargain", Ability::Bargain);
        let e = put_pacer(&mut g, Vec3::new(0.0, 1.0, 2.0), false);
        let t = g.combat_targets().into_iter().find(|t| t.entity == e).unwrap();
        let plain = g.strike_damage(&t);
        let nerve = g.nerve.cur();
        g.use_ability(0);
        assert_eq!(g.nerve.cur(), nerve - 1);
        assert!((g.strike_damage(&t) - plain * abilities3::BARGAIN_MULT).abs() < 1e-4);
        for _ in 0..abilities3::BARGAIN_STRIKES {
            assert!(g.bargain.active());
            g.bargain.spend_strike();
        }
        assert!((g.strike_damage(&t) - plain).abs() < 1e-4, "three strikes and it is over");
    }

    #[test]
    fn bargain_refuses_at_one_nerve() {
        let mut g = with_ability("bargain2", Ability::Bargain);
        while g.nerve.cur() > 1 {
            assert!(g.nerve.spend(1));
        }
        g.use_ability(0);
        assert_eq!(g.nerve.cur(), 1);
        assert!(!g.bargain.active());
        assert!(g.run.abilities[0].cooldown <= 0.0, "nothing was spent");
    }

    #[test]
    fn a_snare_roots_the_first_foe_to_step_on_it_and_is_spent() {
        let mut g = with_ability("snare", Ability::Snare);
        g.use_ability(0);
        assert_eq!(g.snares.len(), 1);
        let far = put_pacer(&mut g, Vec3::new(20.0, 1.0, 0.0), true);
        g.update_abilities3(0.016);
        assert_eq!(g.snares.len(), 1, "nobody on it yet");
        let near = put_pacer(&mut g, Vec3::new(0.5, 1.0, 0.0), true);
        g.update_abilities3(0.016);
        assert_eq!(g.snares.len(), 0, "spent");
        assert_eq!(g.stunned.get(&near), Some(&abilities3::SNARE_ROOT));
        assert!(!g.stunned.contains_key(&far));
    }

    #[test]
    fn only_two_snares_stay_down() {
        let mut g = with_ability("snare2", Ability::Snare);
        for _ in 0..3 {
            g.run.abilities[0].cooldown = 0.0;
            g.player_position.x += 5.0;
            g.use_ability(0);
        }
        assert_eq!(g.snares.len(), abilities3::SNARE_MAX);
    }

    #[test]
    fn overload_charges_while_held_and_blasts_on_release_by_charge() {
        let blast = |held_secs: f32| -> (f32, bool) {
            let mut g = with_ability(&format!("overload{held_secs}"), Ability::Overload);
            let mid = put_pacer(&mut g, Vec3::new(5.0, 1.0, 0.0), true);
            let far = put_pacer(&mut g, Vec3::new(9.0, 1.0, 0.0), true);
            g.mode = hud::Mode::Playing;
            g.ability_held[0] = true;
            g.use_ability(0);
            assert!(g.overload.is_some() && g.run.abilities[0].cooldown <= 0.0, "charging, nothing spent yet");
            let mut left = held_secs;
            while left > 0.0 {
                g.update_abilities3(0.1);
                left -= 0.1;
            }
            assert!(g.stunned.is_empty(), "nothing before the release");
            g.ability_held[0] = false;
            g.update_abilities3(0.016);
            assert!(g.overload.is_none() && g.run.abilities[0].cooldown > 0.0);
            let stun = g.stunned.get(&mid).copied().unwrap_or(0.0);
            (stun, g.stunned.contains_key(&far))
        };
        let (short, short_far) = blast(0.4);
        let (full, full_far) = blast(3.0);
        assert!(short == 0.0 && !short_far, "radius {} does not reach 5", abilities3::overload_radius(0.4));
        assert!(full > 0.0 && full_far, "a full charge reaches both");
        assert!((full - abilities3::OVERLOAD_STAGGER_MAX).abs() < 0.2);
    }

    #[test]
    fn echo_needs_something_to_repeat_and_costs_nothing_extra() {
        let mut g = with_ability("echo", Ability::Echo);
        g.use_ability(0);
        assert!(g.run.abilities[0].cooldown <= 0.0, "nothing to echo: denied, charge kept");
        g.run.take(Upgrade::Learn(Ability::Snare));
        g.use_ability(1);
        assert_eq!(g.snares.len(), 1);
        assert_eq!(g.echo_last, Some(Ability::Snare));
        let snare_cd = g.run.abilities[1].cooldown;
        g.player_position.x += 5.0;
        g.use_ability(0);
        assert_eq!(g.snares.len(), 2, "the snare was set again");
        assert_eq!(g.run.abilities[1].cooldown, snare_cd, "its recharge did not restart");
        assert!(g.run.abilities[0].cooldown > 0.0, "echo itself recharges");
        assert_eq!(g.echo_last, Some(Ability::Snare), "never echoes itself");
        g.run.tick(1.0);
        assert_eq!(g.run.abilities.len(), 2, "the borrowed state is gone");
    }

    #[test]
    fn wager_rolls_once_shows_its_pick_and_bends_its_cooldown() {
        let mut g = with_ability("wager", Ability::Wager);
        let roll = abilities3::wager_roll(g.director.dream_seed(), 0, 0);
        g.use_ability(0);
        assert_eq!(g.wager_n, 1);
        assert!(g.wager_line.as_ref().is_some_and(|l| l.0.contains(roll.ability.name())), "{:?}", g.wager_line);
        let full = effective_cd(Ability::Wager);
        let cd = g.run.abilities[0].cooldown;
        let timed = roll.ability.duration() > 0.0;
        assert!((cd - abilities3::wager_cooldown(roll.kind, full, timed)).abs() < 1e-3, "{roll:?} {cd}");
        assert!(g.run.abilities.iter().all(|a| a.ability != roll.ability) || roll.ability == Ability::Wager);
        g.run.tick(30.0);
        assert_eq!(g.run.abilities.len(), 1, "the borrowed state ran out");
        let line1 = g2_line(&g);
        let mut g2 = with_ability("wager2", Ability::Wager);
        g2.use_ability(0);
        assert_eq!(g2_line(&g2), line1, "same dream, same roll");
    }

    fn g2_line(g: &DreamscapeGame) -> Option<String> {
        g.wager_line.as_ref().map(|l| l.0.clone())
    }

    fn effective_cd(a: Ability) -> f32 {
        upgrades::effective_cooldown(a, 1.0, 1.0)
    }

    fn deck_game(name: &str) -> DreamscapeGame {
        let mut g = game(name);
        g.player_position = Vec3::new(0.0, 1.0, 0.0);
        g.player_grounded = true;
        g.dream_age = 5.0;
        let mut d = deck::Deck::new(&[1, 2, 3, 4, 5, 6, 7], 3);
        d.draw_to_fill(5);
        d.events();
        g.deck = Some(d);
        g
    }

    #[test]
    fn mulligan_and_recall_are_inert_without_a_deck() {
        let g = game("nodeck");
        assert!(!g.abilities3_ready(Ability::Mulligan, None));
        assert!(!g.abilities3_ready(Ability::Recall, None));
        assert!(!RunUpgrades::default().available(Upgrade::Learn(Ability::Mulligan)));
        assert!(!RunUpgrades::default().available(Upgrade::Learn(Ability::Recall)));
        assert!(RunUpgrades::default().available(Upgrade::Learn(Ability::Snare)));
    }

    #[test]
    fn mulligan_redeals_the_hand_and_flicks_the_old_cards_out() {
        let mut g = deck_game("mull");
        assert!(g.abilities3_ready(Ability::Mulligan, None));
        let before: Vec<_> = g.deck.as_ref().unwrap().hand().to_vec();
        g.abilities3_after_play(Ability::Mulligan, before[0], None);
        let d = g.deck.as_mut().unwrap();
        assert!(d.hand().iter().all(|h| h.is_some()), "a fresh full hand");
        assert_eq!(d.events().iter().filter(|e| matches!(e, deck::DeckEvent::Drawn { .. })).count(), 5);
        let mut all = d.all_cards();
        all.sort_unstable();
        assert_eq!(all, vec![1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn recall_returns_the_last_played_card_to_the_hand() {
        let mut g = deck_game("recall");
        let d = g.deck.as_mut().unwrap();
        let first = d.hand()[0].unwrap();
        d.play(0, 6.0);
        let own = d.hand()[2].unwrap();
        d.play(2, 14.0);
        assert!(g.abilities3_ready(Ability::Recall, Some(own)));
        g.abilities3_after_play(Ability::Recall, Some(own), None);
        let d = g.deck.as_mut().unwrap();
        assert!(d.hand().contains(&Some(first)));
        let mut all = d.all_cards();
        all.sort_unstable();
        assert_eq!(all, vec![1, 2, 3, 4, 5, 6, 7], "no card lost or doubled");
    }
    // === end abilities3 ===

    #[test]
    fn touches_cost_nerve_one_at_a_time_and_only_the_last_is_a_catch() {
        let mut g = game("nerve");
        assert_eq!(g.take_nerve_hit("patrol"), None);
        assert_eq!(g.nerve.cur(), 2);
        assert!(g.nerve.invulnerable());
        assert_eq!(g.take_nerve_hit("patrol"), None, "still invulnerable");
        assert_eq!(g.nerve.cur(), 2);
        g.nerve.tick(2.0);
        assert_eq!(g.take_nerve_hit("patrol"), None);
        g.nerve.tick(2.0);
        assert_eq!(g.take_nerve_hit("patrol"), Some("patrol"), "nerve gone: caught");
        assert_eq!(g.nerve.cur(), nerve::REFILL);
    }

    /// A pacer standing `ahead` units in front of a dreamer at the origin.
    fn put_pacer(g: &mut DreamscapeGame, at: Vec3, chasing: bool) -> Entity {
        let entity = g.world.spawn((
            Transform {
                position: at,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            Spin(1.0),
        ));
        let mut ai = EnemyAI::new(at, at + Vec3::X * 3.0, 2.0, 0.0, 9.0);
        ai.chasing = chasing;
        g.enemies.push(Pacer {
            entity,
            ai,
            elite: None,
            a: at,
            b: at + Vec3::X * 3.0,
            split: false,
        });
        entity
    }

    #[test]
    fn a_strike_unravels_an_unaware_pacer_in_one_blow_and_pays_dust() {
        let mut g = game("strike1");
        g.player_position = Vec3::new(0.0, 1.0, 0.0);
        g.player_grounded = true;
        let e = put_pacer(&mut g, Vec3::new(0.0, 1.0, 2.0), false);
        let dust = g.bonus_dust;
        g.resolve_strike();
        assert!(g.enemies.is_empty(), "sneak attack: x2 damage undoes a 2 hp pacer");
        assert_eq!(g.unraveling.len(), 1);
        assert_eq!(g.unraveling[0].0, e);
        assert!(g.bonus_dust > dust, "it left dust");
    }

    #[test]
    fn an_aware_pacer_takes_two_blows_and_is_staggered_between() {
        let mut g = game("strike2");
        g.player_position = Vec3::new(0.0, 1.0, 0.0);
        g.player_grounded = true;
        let e = put_pacer(&mut g, Vec3::new(0.0, 1.0, 2.0), true);
        g.resolve_strike();
        assert_eq!(g.enemies.len(), 1, "one blow does not undo it");
        assert_eq!(g.foes[&e].hp, 1.0);
        assert!(g.stunned.contains_key(&e), "it reels");
        assert!(!g.enemies[0].ai.chasing, "the lunge is broken");
        // It was shoved back; step to meet it. A reeling enemy takes double.
        let pos = g.world.get::<&Transform>(e).unwrap().position;
        assert!(pos.z > 2.5, "knocked back, got {pos:?}");
        g.player_position = Vec3::new(0.0, 1.0, pos.z - 1.5);
        g.resolve_strike();
        assert!(g.enemies.is_empty(), "the second blow finishes it");
    }

    #[test]
    fn a_strike_misses_what_is_behind_or_far_and_never_hits_the_mimic() {
        let mut g = game("strike3");
        g.player_position = Vec3::new(0.0, 1.0, 0.0);
        put_pacer(&mut g, Vec3::new(0.0, 1.0, -2.0), true); // behind
        put_pacer(&mut g, Vec3::new(0.0, 1.0, 6.0), true); // far
        g.resolve_strike();
        assert_eq!(g.enemies.len(), 2);
        assert!(g.foes.is_empty() && g.stunned.is_empty());
    }

    #[test]
    fn pressing_strike_swings_once_per_cooldown_and_not_in_co_op_or_autopilot() {
        let mut g = game("strike4");
        g.player_position = Vec3::new(0.0, 1.0, 0.0);
        g.strike_request = true;
        g.update_combat(0.0);
        assert!(g.slash_pending.is_some(), "a swing started");
        assert!(!g.strike_request, "the press was used");
        g.slash_pending = None;
        g.strike_request = true;
        g.update_combat(0.05);
        assert!(g.slash_pending.is_none(), "no second swing mid-swing");
        let mut a = game("strike5");
        a.autopilot = true;
        a.strike_request = true;
        a.update_combat(0.0);
        assert!(a.slash_pending.is_none(), "the autopilot does not fight");
    }

    /// A game with `ability` held and running, a dreamer at (0,1,0) on the ground.
    fn fighter(name: &str, ability: Option<Ability>) -> DreamscapeGame {
        let mut g = game(name);
        g.player_position = Vec3::new(0.0, 1.0, 0.0);
        g.player_grounded = true;
        g.dream_age = 5.0;
        if let Some(a) = ability {
            g.run.take(Upgrade::Learn(a));
            g.run.abilities[0].active = 5.0;
        }
        g
    }

    // === combat === ENEMIES-B wiring
    fn put_special(g: &mut DreamscapeGame, kind: EnemyKind, at: Vec3, ai: SpecialAI) -> Entity {
        let entity = g.world.spawn((
            Transform { position: at, rotation: Quat::IDENTITY, scale: Vec3::ONE },
            Spin(1.0),
        ));
        g.specials.push(SpecialEnemy { entity, kind, ai });
        entity
    }

    fn standing(name: &str) -> DreamscapeGame {
        let mut g = game(name);
        g.player_position = Vec3::new(0.0, 0.5, 0.0);
        g.player_grounded = true;
        g.dream_age = 5.0;
        g
    }

    #[test]
    fn a_dash_cuts_what_it_passes_once_and_leaves_the_rest() {
        let mut g = fighter("dash", Some(Ability::Dash));
        let near = put_pacer(&mut g, Vec3::new(0.5, 1.0, 1.0), true);
        put_pacer(&mut g, Vec3::new(0.0, 1.0, 8.0), true);
        g.update_combat(0.016);
        assert_eq!(g.foes[&near].hp, 1.0, "cut for 1");
        g.update_combat(0.016);
        assert_eq!(g.foes[&near].hp, 1.0, "only once per dash");
        assert_eq!(g.foes.len(), 1, "the far one is untouched");
        g.run.abilities[0].active = 0.0;
        g.run.abilities[0].cooldown = 0.0;
        g.combat_on_use(Ability::Dash, Vec3::ZERO); // a new dash
        g.run.abilities[0].active = 5.0;
        g.update_combat(0.016);
        assert!(g.enemies.len() == 1, "the second cut finishes it");
    }

    #[test]
    fn blink_sets_up_a_sneak_attack_even_on_an_aware_foe() {
        let mut g = fighter("blink", None);
        let e = put_pacer(&mut g, Vec3::new(0.0, 1.0, 2.0), true);
        g.combat_on_use(Ability::Blink, Vec3::ZERO);
        g.resolve_strike();
        assert!(g.enemies.is_empty(), "x2 on a 2 hp pacer");
        assert!(g.unraveling.iter().any(|u| u.0 == e));
        // The window ends.
        let mut h = fighter("blink2", None);
        put_pacer(&mut h, Vec3::new(0.0, 1.0, 2.0), true);
        h.combat_on_use(Ability::Blink, Vec3::ZERO);
        h.dream_age += combat::BLINK_BACKSTAB_SECS + 0.1;
        h.resolve_strike();
        assert_eq!(h.enemies.len(), 1);
    }

    #[test]
    fn stillness_makes_a_foe_take_double_and_stay_put() {
        let mut g = fighter("still", Some(Ability::Stillness));
        let e = put_pacer(&mut g, Vec3::new(0.0, 1.0, 2.0), true);
        g.resolve_strike();
        assert!(g.enemies.is_empty(), "frozen: x2 undoes it in one blow");
        let mut h = fighter("still2", Some(Ability::Stillness));
        let big = h.world.spawn((Transform { position: Vec3::new(0.0, 1.0, 2.0), rotation: Quat::IDENTITY, scale: Vec3::ONE }, Spin(1.0)));
        let mut ai = EnemyAI::new(Vec3::ZERO, Vec3::X, 2.0, 0.0, 9.0);
        ai.chasing = true;
        h.enemies.push(Pacer { entity: big, ai, elite: Some(Elite::Big), a: Vec3::ZERO, b: Vec3::X, split: false });
        h.resolve_strike();
        let z = h.world.get::<&Transform>(big).unwrap().position.z;
        assert_eq!(z, 2.0, "frozen things are not shoved");
        assert_eq!(h.foes[&big].hp, 2.0, "4 hp, x2");
        let _ = e;
    }

    #[test]
    fn flare_staggers_what_it_lights_and_hurts_shades() {
        let mut g = fighter("flare", None);
        let plain = put_pacer(&mut g, Vec3::new(3.0, 1.0, 2.0), false);
        let far = put_pacer(&mut g, Vec3::new(20.0, 1.0, 2.0), false);
        let shade = put_pacer(&mut g, Vec3::new(-3.0, 1.0, 2.0), false);
        g.enemies.last_mut().unwrap().elite = Some(Elite::Shade);
        g.combat_on_use(Ability::Flare, Vec3::ZERO);
        assert!(g.stunned.contains_key(&plain), "lit and reeling");
        assert!(!g.stunned.contains_key(&far), "out of the light");
        assert_eq!(g.foes[&shade].hp, 1.0, "the shade cannot hide in it");
        assert!(!g.foes.contains_key(&plain), "plain things are staggered, not hurt");
    }

    #[test]
    fn rewind_gives_nerve_back_through_the_ability() {
        let mut g = fighter("rewind", None);
        g.nerve.hit(1, (0.0, 0.0), 0.0);
        g.nerve.tick(2.0);
        g.nerve.hit(1, (0.0, 0.0), 0.0);
        assert_eq!(g.nerve.cur(), 1);
        g.combat_on_use(Ability::Rewind, Vec3::ZERO);
        assert_eq!(g.nerve.cur(), 3);
    }

    #[test]
    fn a_decoys_end_bursts_and_a_broken_ward_reflects_and_anchor_leaves_a_wave() {
        let mut g = fighter("burst", None);
        let near = put_pacer(&mut g, Vec3::new(1.0, 1.0, 1.0), true);
        let far = put_pacer(&mut g, Vec3::new(9.0, 1.0, 9.0), true);
        g.decoy_burst(Vec3::ZERO);
        assert_eq!(g.foes[&near].hp, 1.0);
        assert!(!g.foes.contains_key(&far));
        let mut w = fighter("ward", None);
        let hit = put_pacer(&mut w, Vec3::new(1.5, 1.0, 0.0), true);
        w.ward_reflect();
        assert_eq!(w.foes[&hit].hp, 1.0, "the blow goes back");
        let mut a = fighter("anchor", None);
        let e = put_pacer(&mut a, Vec3::new(1.0, 1.0, 0.0), true);
        a.anchor_snapped = true;
        a.combat_on_use(Ability::Anchor, Vec3::ZERO);
        assert!(a.stunned.contains_key(&e), "the wave staggers");
        assert!(!a.foes.contains_key(&e), "but does not hurt");
    }

    #[test]
    fn a_leaps_landing_shocks_once() {
        let mut g = fighter("leap", None);
        let e = put_pacer(&mut g, Vec3::new(1.0, 1.0, 1.0), true);
        g.combat_on_use(Ability::Leap, Vec3::ZERO);
        assert!(g.leap_airborne);
        g.player_grounded = false;
        g.update_combat(0.016);
        assert!(g.foes.is_empty(), "still in the air");
        g.player_grounded = true;
        g.update_combat(0.016);
        assert_eq!(g.foes[&e].hp, 1.0);
        g.update_combat(0.016);
        assert_eq!(g.foes[&e].hp, 1.0, "once per leap");
    }

    #[test]
    fn shard_call_pulls_calm_motes_to_you() {
        let mut g = fighter("pull", Some(Ability::ShardCall));
        let m = g.world.spawn((Transform { position: Vec3::new(8.0, 1.0, 0.0), rotation: Quat::IDENTITY, scale: Vec3::ONE }, Spin(1.0)));
        g.motes = vec![(m, 0.0)];
        g.update_combat(0.5);
        let x = g.world.get::<&Transform>(m).unwrap().position.x;
        assert!(x < 7.0, "drawn in, x = {x}");
    }

    #[test]
    fn second_heartbeat_raises_the_ceiling_and_heals() {
        let mut g = fighter("heartbeat", None);
        g.run.take(Upgrade::SecondHeartbeat);
        g.update_combat(0.0);
        assert_eq!((g.nerve.cur(), g.nerve.max()), (4, 4));
        g.run.take(Upgrade::SecondHeartbeat);
        g.update_combat(0.0);
        assert_eq!(g.nerve.max(), 5);
        g.update_combat(0.0);
        assert_eq!(g.nerve.max(), 5, "never past five");
    }

    // === upgrades2 ===
    #[test]
    fn iron_nerve_raises_the_ceiling_and_thin_skin_lowers_it_once() {
        let mut g = fighter("ironnerve", None);
        g.run.take(Upgrade::IronNerve);
        g.update_combat(0.0);
        assert_eq!((g.nerve.cur(), g.nerve.max()), (4, 4));
        let mut t = fighter("thinskin", None);
        t.run.take(Upgrade::Cursed(upgrades::Curse::ThinSkin));
        t.update_combat(0.0);
        assert_eq!((t.nerve.cur(), t.nerve.max()), (2, 2));
        t.update_combat(0.0);
        assert_eq!(t.nerve.max(), 2, "taken off once, not every frame");
    }

    #[test]
    fn pack_rat_pays_on_a_skip_and_the_moths_favour_cuts_prices() {
        let mut g = reward_game("packrat");
        g.run.take(Upgrade::PackRat);
        let dust = g.bonus_dust;
        g.skip_choice();
        assert_eq!(g.bonus_dust, dust + upgrades::SKIP_DUST + upgrades::PACK_RAT_DUST);
        let base = g.moth_terms().pct_off;
        g.run.take(Upgrade::MothsFavour);
        assert_eq!(g.moth_terms().pct_off, base + upgrades::MOTHS_FAVOUR_PCT);
    }

    #[test]
    fn nimble_draw_and_the_free_reshuffle_reach_the_deck() {
        let mut g = fighter("deckperks", None);
        g.deck = Some(deck::Deck::new(&[1, 2, 3, 4, 5], 9));
        g.run.take(Upgrade::NimbleDraw);
        g.update_deck(0.0);
        let want = g.run.deck_perks();
        assert_eq!(g.deck.as_ref().unwrap().perks, want);
        assert!(want.draw_mult < 1.0);
    }
    // === end upgrades2 ===

    #[test]
    fn sharp_edge_cuts_deeper_into_a_reeling_foe() {
        let mut g = fighter("edge", None);
        g.run.take(Upgrade::SharpEdge);
        let e = put_pacer(&mut g, Vec3::new(0.0, 1.0, 2.0), true);
        g.stunned.insert(e, 1.0);
        g.resolve_strike();
        assert!(g.enemies.is_empty(), "x2 for reeling, +1 for the edge: 3 on a 2 hp pacer");
        let mut h = fighter("edge2", None);
        let big = put_pacer(&mut h, Vec3::new(0.0, 1.0, 2.0), true);
        h.run.take(Upgrade::SharpEdge);
        h.foes.insert(big, combat::Foe { hp: 4.0, max: 4.0 });
        h.resolve_strike();
        assert_eq!(h.foes[&big].hp, 3.0, "not reeling: the edge adds nothing");
    }

    #[test]
    fn long_reach_and_wide_eyes_stretch_and_widen_the_strike() {
        let far = Vec3::new(0.0, 1.0, 2.9);
        let mut g = fighter("reach", None);
        put_pacer(&mut g, far, true);
        g.resolve_strike();
        assert!(g.foes.is_empty(), "2.9 is out of a 2.4 reach");
        g.run.take(Upgrade::LongReach);
        g.resolve_strike();
        assert_eq!(g.foes.len(), 1, "3.0 reaches it");
        // 60 degrees off the facing: outside the 120 degree fan's edge.
        let side = Vec3::new(1.9, 1.0, 1.1); // about 60 deg off, distance 2.2
        let mut w = fighter("wide", None);
        put_pacer(&mut w, Vec3::new(2.1, 1.0, 0.9), true);
        w.resolve_strike();
        assert!(w.foes.is_empty(), "65 degrees is outside");
        w.run.take(Upgrade::WideEyes);
        w.resolve_strike();
        assert_eq!(w.foes.len(), 1, "wide eyes see further round");
        let _ = side;
    }

    #[test]
    fn every_fourth_strike_sends_an_echo_further() {
        let mut g = fighter("echo", None);
        g.run.take(Upgrade::EchoStrike);
        put_pacer(&mut g, Vec3::new(0.0, 1.0, 3.6), true);
        for _ in 0..3 {
            g.resolve_strike();
        }
        assert!(g.foes.is_empty(), "3.6 is past a plain reach");
        g.resolve_strike();
        assert_eq!(g.foes.len(), 1, "the fourth wave reaches it");
    }

    #[test]
    fn heavy_air_shoves_further() {
        let mut a = fighter("heavy0", None);
        let e = put_pacer(&mut a, Vec3::new(0.0, 1.0, 2.0), true);
        a.resolve_strike();
        let plain = a.world.get::<&Transform>(e).unwrap().position.z - 2.0;
        let mut b = fighter("heavy1", None);
        b.run.take(Upgrade::HeavyAir);
        let e = put_pacer(&mut b, Vec3::new(0.0, 1.0, 2.0), true);
        b.resolve_strike();
        let heavy = b.world.get::<&Transform>(e).unwrap().position.z - 2.0;
        assert!(heavy > plain * 1.3, "{plain} -> {heavy}");
    }

    #[test]
    fn dream_anchor_is_a_last_stand_once_a_dream() {
        let mut g = fighter("stand", None);
        g.run.take(Upgrade::DreamAnchor);
        g.nerve.hit(1, (0.0, 0.0), 0.0);
        g.nerve.tick(2.0);
        g.nerve.hit(1, (0.0, 0.0), 0.0);
        g.nerve.tick(2.0);
        assert_eq!(g.take_nerve_hit("patrol"), None, "stood back up");
        assert_eq!(g.nerve.cur(), 1);
        g.nerve.tick(3.0);
        assert_eq!(g.take_nerve_hit("patrol"), Some("patrol"), "only once");
        g.last_stand_used = false; // a new dream
        g.nerve.tick(3.0);
        g.nerve.heal(0);
        let cur = g.nerve.cur();
        assert!(cur >= 1);
    }

    #[test]
    fn thorns_bite_what_hurts_you_and_counter_and_riposte_answer_a_slipped_blow() {
        let mut g = fighter("thorns", None);
        g.run.take(Upgrade::Thorns);
        let e = put_pacer(&mut g, Vec3::new(1.0, 1.0, 0.0), true);
        assert_eq!(g.take_nerve_hit("patrol"), None);
        assert_eq!(g.foes[&e].hp, 1.0, "bitten");
        let mut c = fighter("counter", None);
        let e = put_pacer(&mut c, Vec3::new(1.0, 1.0, 0.0), true);
        c.counter_attack();
        assert!(c.stunned.contains_key(&e) && !c.foes.contains_key(&e), "reels, unhurt");
        assert!(c.counter_cd > 0.0);
        let mut r = fighter("riposte", None);
        r.run.take(Upgrade::Learn(Ability::Dash));
        r.run.take(Upgrade::Counter);
        assert!(r.run.has(upgrades::Synergy::Riposte));
        let e = put_pacer(&mut r, Vec3::new(1.0, 1.0, 0.0), true);
        r.counter_attack();
        assert_eq!(r.foes[&e].hp, 1.0, "riposte cuts as well");
    }

    #[test]
    fn calm_mind_regenerates_only_in_calm_and_executioner_cuts_deeper() {
        let mut g = fighter("calm", None);
        g.run.take(Upgrade::CalmMind);
        g.nerve.hit(1, (0.0, 0.0), 0.0);
        let e = put_pacer(&mut g, Vec3::new(9.0, 1.0, 9.0), true);
        g.update_combat(26.0);
        assert_eq!(g.nerve.cur(), 2, "a chaser is near: no calm");
        g.enemies.clear();
        g.world.despawn(e).ok();
        g.update_combat(26.0);
        assert_eq!(g.nerve.cur(), 3, "calm earned one");
        let mut x = fighter("exec", None);
        x.run.take(Upgrade::Learn(Ability::Hush));
        x.run.take(Upgrade::SharpEdge);
        assert!(x.run.has(upgrades::Synergy::Executioner));
        let big = put_pacer(&mut x, Vec3::new(0.0, 1.0, 2.0), false);
        x.foes.insert(big, combat::Foe { hp: 6.0, max: 6.0 });
        x.resolve_strike();
        assert_eq!(x.foes[&big].hp, 3.0, "x2 unaware, +1 executioner");
    }

    #[test]
    fn calm_spore_heals_two_and_staggers_what_is_near_and_a_strike_in_air_refunds_a_jump() {
        let mut g = fighter("spore", None);
        g.run.take(Upgrade::CalmSpore);
        g.nerve.hit(1, (0.0, 0.0), 0.0);
        g.nerve.tick(2.0);
        g.nerve.hit(1, (0.0, 0.0), 0.0);
        let mote = g.world.spawn((Transform { position: Vec3::new(0.4, 1.0, 0.0), rotation: Quat::IDENTITY, scale: Vec3::ONE }, Spin(1.0)));
        let e = put_pacer(&mut g, Vec3::new(3.0, 1.0, 0.0), true);
        g.motes = vec![(mote, 0.0)];
        g.update_combat(0.016);
        assert_eq!(g.nerve.cur(), 3, "1 + 2");
        assert!(g.stunned.contains_key(&e), "the spores stagger what is near");
        let mut j = fighter("jump", None);
        j.run.take(Upgrade::DoubleJump);
        j.player_grounded = false;
        j.air_jumps_used = 1;
        j.strike_request = true;
        j.update_combat(0.0);
        assert_eq!(j.air_jumps_used, 0, "refunded");
        j.air_jumps_used = 1;
        j.strike_request = true;
        j.update_combat(1.0);
        j.strike_request = true;
        j.update_combat(0.0);
        assert_eq!(j.air_jumps_used, 1, "once per airtime");
    }

    #[test]
    fn the_hunter_glances_off_a_blow_until_it_rests_and_wisps_fall_in_one() {
        let mut g = fighter("hunter", None);
        let e = g.world.spawn((
            Transform { position: Vec3::new(0.0, 1.0, 2.5), rotation: Quat::IDENTITY, scale: Vec3::splat(2.0) },
            Spin(1.0),
        ));
        let mut b = boss::Boss::new(10, 6.0);
        b.hunter.phase = hunter::Phase::Stalk;
        b.wisps.push(boss::Wisp { pos: Vec3::new(0.0, 0.0, 1.5), life: 5.0 });
        g.hunter = Some((e, b, Vec3::ZERO));
        let hp0 = g.hunter.as_ref().unwrap().1.hp;
        g.resolve_strike();
        assert_eq!(g.hunter.as_ref().unwrap().1.hp, hp0, "hunting: glances off");
        assert!(g.hunter.as_ref().unwrap().1.wisps.is_empty(), "the wisp fell in one blow");
        g.hunter.as_mut().unwrap().1.hunter.rest();
        g.resolve_strike();
        let b = &g.hunter.as_ref().unwrap().1;
        assert_eq!(b.hp, hp0 - 1.0, "resting: it hurts");
        assert!(!b.defeated);
        // Pushed back a little by the blow.
        let z = g.world.get::<&Transform>(e).unwrap().position.z;
        assert!(z > 2.5);
        // A last blow undoes it and pays out.
        g.hunter.as_mut().unwrap().1.hp = 1.0;
        g.player_position = Vec3::new(0.0, 1.0, z - 2.0);
        let dust = g.bonus_dust;
        g.resolve_strike();
        assert!(g.hunter.as_ref().unwrap().1.defeated);
        assert_eq!(g.bonus_dust, dust + boss::UNDONE_DUST_PER_TIER * 2);
        assert_eq!(g.pending_motes.len(), boss::UNDONE_MOTES);
        assert!(g.threat_touching().is_none(), "an undone nightmare touches nothing");
    }

    #[test]
    fn the_nightmares_strength_bar_draws_and_flashes_when_it_can_be_hurt() {
        let mut g = game("bossbar");
        g.mode = hud::Mode::Playing;
        let n = |hp: Option<(f32, bool)>| {
            frame_with(&g, 1280.0, 720.0, &|v| {
                v.boss = Some(("THE HUNTER".into(), 1, 3, None));
                v.boss_hp = hp;
            })
            .len()
        };
        assert!(n(Some((0.5, false))) > n(None), "the bar draws");
        n(Some((0.0, true)));
        n(Some((1.0, true)));
    }

    #[test]
    fn hit_point_pips_show_for_struck_enemies_and_for_any_nearby_with_wide_eyes() {
        let mut g = fighter("pips", None);
        let struck = put_pacer(&mut g, Vec3::new(0.0, 1.0, 3.0), true);
        put_pacer(&mut g, Vec3::new(2.0, 1.0, 3.0), true);
        put_pacer(&mut g, Vec3::new(40.0, 1.0, 3.0), true);
        g.foes.insert(struck, combat::Foe { hp: 1.0, max: 2.0 });
        let pips = g.foe_pips(Mat4::IDENTITY);
        assert_eq!(pips.len(), 1, "only the struck one");
        assert_eq!((pips[0].hp, pips[0].max), (1.0, 2.0));
        g.run.take(Upgrade::WideEyes);
        assert_eq!(g.foe_pips(Mat4::IDENTITY).len(), 2, "wide eyes: the near unstruck one too, not the far one");
        // Frames with pips draw more than frames without, at several sizes.
        g.mode = hud::Mode::Playing;
        for (w, h) in [(960.0, 540.0), (1280.0, 720.0), (1920.0, 1080.0)] {
            let bare = frame_with(&g, w, h, &|_| {}).len();
            let with = frame_with(&g, w, h, &|v| {
                v.foe_pips = vec![hud::FoePips { at: [0.5, 0.4], hp: 1.0, max: 3.0 }];
            })
            .len();
            assert!(with > bare, "{w}x{h}");
        }
    }

    #[test]
    fn a_masks_shield_turns_a_front_strike_aside_but_not_one_from_behind() {
        // The dreamer faces +Z with a mask 1.5 ahead.
        let mut g = standing("mask_front");
        let at = Vec3::new(0.0, 0.5, 1.5);
        let facing_me = mask::Mask::new(at, at - Vec3::Z * 3.0); // heads for -Z: faces the dreamer
        let m = put_special(&mut g, EnemyKind::Mask, at, SpecialAI::Mask(facing_me));
        g.resolve_strike();
        assert!(g.foes.is_empty(), "glanced: no damage");
        assert!(!g.staggered.contains_key(&m), "and no stagger");
        assert!(g.burst_pending.is_some(), "a spark");
        // The same mask turned away takes the hit.
        let mut h = standing("mask_back");
        let turned = mask::Mask::new(at, at + Vec3::Z * 3.0);
        let m = put_special(&mut h, EnemyKind::Mask, at, SpecialAI::Mask(turned));
        h.resolve_strike();
        assert_eq!(h.foes[&m].hp, 2.0, "3 hp, 1 damage");
        assert!(h.staggered.contains_key(&m));
        // A gust spins the first one, shield down, and then it can be hit.
        g.rattle_b(20.0, 5.0);
        g.resolve_strike();
        assert_eq!(g.foes[&m_of(&g)].hp, 2.0);
    }

    fn m_of(g: &DreamscapeGame) -> Entity {
        g.specials.iter().find(|s| matches!(s.ai, SpecialAI::Mask(_))).unwrap().entity
    }

    #[test]
    fn a_slam_cracks_the_shield_so_a_frontal_strike_lands() {
        let mut g = standing("mask_slam");
        let at = Vec3::new(0.0, 0.5, 1.5);
        let m = put_special(&mut g, EnemyKind::Mask, at, SpecialAI::Mask(mask::Mask::new(at, at - Vec3::Z * 3.0)));
        g.rattle_b(20.0, 0.0);
        g.resolve_strike();
        assert_eq!(g.foes[&m].hp, 2.0);
    }

    #[test]
    fn a_mask_slam_costs_one_nerve_by_name() {
        let mut g = standing("mask_hit");
        let at = Vec3::new(0.0, 0.5, 2.0);
        let mut mk = mask::Mask::new(at, at - Vec3::Z * 3.0);
        mk.state = mask::State::Slamming(mask::SLAM_ACTIVE);
        put_special(&mut g, EnemyKind::Mask, at, SpecialAI::Mask(mk));
        assert_eq!(g.threat_touching(), Some("mask"));
        g.player_position = Vec3::new(0.0, 0.5, -5.0);
        assert_eq!(g.threat_touching(), None, "behind it, out of reach");
    }

    #[test]
    fn a_leech_latches_drains_through_the_nerve_path_and_three_jumps_shake_it() {
        let mut g = standing("leech");
        let at = Vec3::new(0.0, 0.35, 0.5);
        let e = put_special(&mut g, EnemyKind::Leech, at, SpecialAI::Leech(leech::Leech::new(at)));
        g.update_enemies_b(0.016, false);
        let latched = |g: &DreamscapeGame| g.specials.iter().any(|s| matches!(&s.ai, SpecialAI::Leech(l) if l.latched()));
        assert!(latched(&g), "touching latches");
        assert_eq!(g.threat_touching(), None, "the latch itself costs nothing");
        // Two seconds on: one drain, by name, through take_nerve_hit.
        g.update_enemies_b(2.0, false);
        assert_eq!(g.threat_touching(), Some("leech"));
        assert_eq!(g.take_nerve_hit("leech"), None);
        assert_eq!(g.nerve.cur(), 2);
        g.update_enemies_b(0.016, false);
        assert_eq!(g.threat_touching(), None, "one drain per period");
        // Invulnerable: the next drain is ignored.
        g.update_enemies_b(2.0, false);
        assert_eq!(g.take_nerve_hit("leech"), None);
        assert_eq!(g.nerve.cur(), 2, "iframes");
        // Three jumps in two seconds.
        for _ in 0..3 {
            g.jump_event = true;
            g.update_enemies_b(0.3, false);
        }
        assert!(!latched(&g), "shaken off");
        assert!(g.staggered.contains_key(&e), "and reeling");
        // After 1.5s it resumes crawling at you.
        g.update_combat(1.6);
        g.update_enemies_b(0.5, false);
        assert!(!g.staggered.contains_key(&e));
    }

    #[test]
    fn a_strike_unlatches_a_leech_and_a_second_one_unravels_it() {
        let mut g = standing("leech_strike");
        let at = Vec3::new(0.0, 0.35, 0.5);
        let e = put_special(&mut g, EnemyKind::Leech, at, SpecialAI::Leech(leech::Leech::new(at)));
        g.update_enemies_b(0.016, false);
        g.resolve_strike();
        assert_eq!(g.foes[&e].hp, 1.0);
        assert!(g.specials.iter().all(|s| !matches!(&s.ai, SpecialAI::Leech(l) if l.latched())), "off you");
        assert!(g.staggered.contains_key(&e));
        // Staggered x2: one more finishes it, and nothing rides you.
        g.player_position = Vec3::new(0.0, 0.5, 0.0);
        if let Some(SpecialAI::Leech(l)) = g.specials.iter_mut().map(|s| &mut s.ai).next() {
            l.release(Vec3::new(0.0, 0.5, 0.0), Vec3::Z);
        }
        g.strike.reset();
        g.resolve_strike();
        assert!(g.specials.is_empty(), "unravelled");
    }

    #[test]
    fn moths_die_to_one_strike_each_and_the_swarm_forgets_them() {
        let mut g = standing("moths");
        let swarm = moths::Swarm::new(Vec3::ZERO);
        let first = swarm.moths[0];
        let ids: Vec<Entity> = swarm
            .moths
            .iter()
            .map(|m| put_special(&mut g, EnemyKind::Moths, m.pos, SpecialAI::Moth(m.id)))
            .collect();
        g.moth_swarm = Some(swarm);
        // Put moth 0 right in front of the dreamer.
        g.world.get::<&mut Transform>(ids[0]).unwrap().position = Vec3::new(0.0, 0.5, 1.5);
        let _ = first;
        g.resolve_strike();
        assert_eq!(g.specials.len(), 3);
        assert_eq!(g.moth_swarm.as_ref().unwrap().alive(), 3);
        assert!(g.unraveling.iter().any(|u| u.0 == ids[0]));
    }

    #[test]
    fn a_gust_scatters_the_swarm() {
        let mut g = standing("moths_gust");
        g.moth_swarm = Some(moths::Swarm::new(Vec3::ZERO));
        g.rattle_b(upgrades::GUST_RADIUS.max(30.0), upgrades::GUST_PUSH);
        assert!(g.moth_swarm.as_ref().unwrap().moths.iter().all(|m| matches!(m.mode, moths::Mode::Scattered(..))));
    }
    // === end combat ===

    #[test]
    fn a_sanctuary_has_no_hazards_no_clock_and_a_nerve_refill_waits_in_it() {
        let mut g = game("sanct");
        // A hazard dream at a depth that is not a sanctuary...
        g.director.theme = DreamTheme::NightmareFactory;
        g.director.depth = 8;
        let normal = g.hazard_rule_state().is_some();
        assert!(normal, "the factory has a rule at depth 8");
        assert!(!g.sanctuary());
        // ...and the dream right after a nightmare.
        g.director.depth = 11;
        assert!(g.sanctuary());
        assert!(g.hazard_rule_state().is_none(), "no hazard in a sanctuary");
        assert!(!g.pressure_applies(), "no clock either");
        // The prologue is never one, nor a nightmare.
        g.director.nightmare = true;
        assert!(!g.sanctuary());
    }

    #[test]
    fn the_lobby_is_enemy_free_but_not_a_sanctuary_and_the_next_dream_is_not() {
        let mut g = game("lobby_safe");
        g.director.theme = DreamTheme::Lobby;
        g.director.depth = 0;
        assert!(g.enemy_free(), "no enemies in the Lobby");
        assert!(!g.sanctuary(), "it has no sanctuary refill or chord");
        assert!(g.hazard_rule_state().is_none());
        assert!(!g.pressure_applies());
        // Any other dream, and a nightmare, are not.
        g.director.theme = DreamTheme::Garden;
        g.director.depth = 4;
        assert!(!g.enemy_free());
        g.director.theme = DreamTheme::Lobby;
        g.director.nightmare = true;
        assert!(!g.enemy_free(), "a nightmare is never safe");
        g.director.nightmare = false;
        // The sanctuary after a nightmare still is.
        g.director.theme = DreamTheme::Garden;
        g.director.depth = 11;
        assert!(g.enemy_free() && g.sanctuary());
    }

    #[test]
    fn once_the_shard_is_taken_the_eye_points_at_the_way_out() {
        let mut g = game("exit_eye");
        let d = dream::generate_with(DreamTheme::Garden, 3, 2, None, true, dream::Pressure::default(), dream::Goals::default());
        let portal = d.portal;
        g.dream = Some(d);
        g.objective = None;
        // No shard slot is on screen: the way out is the target.
        assert_eq!(g.objective_target(), Some(portal));
        // Still hunting for something that has no beacon (a held-on objective): no pointing.
        g.objective = Some(objective::Active::FindMemory);
        assert_eq!(g.objective_target(), None);
    }

    fn three_cards(g: &mut DreamscapeGame) {
        let recs: Vec<_> = [DreamTheme::Garden, DreamTheme::TheTunnel, DreamTheme::Lobby]
            .iter()
            .enumerate()
            .map(|(i, &t)| cards::tests::record(t, i as u64 + 1, 1, 0.3, false))
            .collect();
        g.booklet.add_run(1, &recs);
        g.screen_size = (1280.0, 720.0);
    }

    fn tile_centre(g: &DreamscapeGame, sel: usize, grid_top: f32, grid_w: f32, i: usize) -> (i32, i32) {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(1280.0, 720.0));
        let n = g.booklet.card_count();
        let r = loadout_ui::grid_tiles(screen, n, sel, grid_top, grid_w)
            .into_iter()
            .find(|(k, _)| *k == i)
            .expect("tile on screen")
            .1;
        (r.center().x as i32, r.center().y as i32)
    }

    #[test]
    fn the_mouse_picks_and_merges_and_changes_its_mind_in_the_merge_menu() {
        let mut g = game("merge_mouse");
        three_cards(&mut g);
        g.open_merge(hud::Mode::Title);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(1280.0, 720.0));
        let lay = merge_ui::layout(screen);
        let at = |g: &DreamscapeGame, i| tile_centre(g, g.merge_sel, lay.grid_top, lay.grid_w, i);
        // Hover moves the cursor and does not act.
        let (x, y) = at(&g, 1);
        assert_eq!(g.menu_pointer(x, y, Pointer::Hover), None);
        assert_eq!(g.merge_sel, 1);
        assert_eq!(g.merge_first, None);
        // A click is an Enter on that card: it becomes the first.
        let (x, y) = at(&g, 0);
        assert_eq!(g.menu_pointer(x, y, Pointer::Click), Some(Keycode::Return));
        g.merge_key(Keycode::Return);
        assert_eq!(g.merge_first, g.booklet.cards().next().map(|c| c.number));
        // Changing your mind: click the first card's slot to take it back...
        let s0 = lay.slots[0].center();
        assert_eq!(g.menu_pointer(s0.x as i32, s0.y as i32, Pointer::Click), None);
        assert_eq!(g.merge_first, None);
        // ...or Tab makes the card under the cursor the first, Backspace clears it.
        g.merge_sel = 2;
        g.merge_key(Keycode::Tab);
        assert_eq!(g.merge_first, g.booklet.cards().nth(2).map(|c| c.number));
        g.merge_sel = 1;
        g.merge_key(Keycode::Tab);
        assert_eq!(g.merge_first, g.booklet.cards().nth(1).map(|c| c.number), "swapped, no unpicking first");
        g.merge_key(Keycode::Backspace);
        assert_eq!(g.merge_first, None);
        // Right-click: clears a pick, and on an empty altar leaves.
        g.merge_first = g.booklet.cards().next().map(|c| c.number);
        g.menu_pointer(10, 10, Pointer::Back);
        assert_eq!(g.merge_first, None);
        assert_eq!(g.mode, hud::Mode::Merge);
        g.menu_pointer(10, 10, Pointer::Back);
        assert_eq!(g.mode, hud::Mode::Title);
        // Clicking the result slot merges once a first card is in and the cursor is on a legal second.
        g.open_merge(hud::Mode::Title);
        g.merge_first = g.booklet.cards().next().map(|c| c.number);
        g.merge_sel = 1;
        let s2 = lay.slots[2].center();
        assert_eq!(g.menu_pointer(s2.x as i32, s2.y as i32, Pointer::Click), Some(Keycode::Return));
        g.merge_key(Keycode::Return);
        assert_eq!(g.booklet.card_count(), 2, "merged");
    }

    #[test]
    fn the_mouse_equips_and_unequips_in_the_loadout() {
        let mut g = game("loadout_mouse");
        three_cards(&mut g);
        g.open_loadout();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(1280.0, 720.0));
        // The loadout is drawn at its design size; a 1280x720 window shows it 1:1 or larger.
        let (pt, lscreen) = hud::to_layout(hud::design_for(hud::Mode::Loadout, 0), (1280.0, 720.0), 0.0, 0.0);
        assert_eq!(pt, egui::Pos2::ZERO);
        assert_eq!(lscreen, screen, "no scaling at 720p");
        let lay = loadout_ui::layout(screen);
        let (x, y) = tile_centre(&g, g.loadout_sel, lay.grid_top, lay.grid_w, 1);
        assert_eq!(g.menu_pointer(x, y, Pointer::Click), Some(Keycode::Return));
        assert_eq!(g.loadout_sel, 1);
        g.loadout_key(Keycode::Return);
        let first = g.booklet.cards().nth(1).map(|c| c.number).unwrap();
        assert!(g.booklet.lore.deck.contains(&first), "added to the deck");
        // Click the pip it sits in: out it comes.
        let s = deck_ui::pips(screen)[g.booklet.lore.deck.iter().position(|&n| n == first).unwrap()].center();
        assert_eq!(g.menu_pointer(s.x as i32, s.y as i32, Pointer::Click), None);
        assert!(!g.booklet.lore.deck.contains(&first), "removed by clicking its pip");
        // A small window scales the screen down, and the mouse scales with it.
        let (pt, small) = hud::to_layout(hud::design_for(hud::Mode::Loadout, 0), (400.0, 225.0), 200.0, 100.0);
        assert!((pt.x - 400.0).abs() < 1e-3 && (pt.y - 200.0).abs() < 1e-3, "{pt:?}");
        assert!((small.width() - 800.0).abs() < 1e-3);
    }

    // === deck ===
    #[test]
    fn a_small_deck_cannot_start_but_an_empty_or_big_enough_one_can() {
        let mut g = game("deck_start");
        three_cards(&mut g);
        g.open_loadout();
        assert_eq!(g.booklet.lore.deck.len(), 1, "prefilled with what would be used");
        g.mode = hud::Mode::Loadout;
        g.loadout_key(Keycode::Space);
        assert_eq!(g.mode, hud::Mode::Loadout, "1 of 3 cards: denied");
        for n in g.booklet.cards().map(|c| c.number).collect::<Vec<_>>() {
            if !g.booklet.lore.deck.contains(&n) {
                g.booklet.lore.deck.push(n);
            }
        }
        assert!(deck::valid(g.booklet.lore.deck.len(), g.booklet.card_count()));
    }

    #[test]
    fn a_deck_run_deals_three_cards_and_a_used_one_is_redrawn_with_its_cooldown() {
        let mut g = game("deck_run");
        three_cards(&mut g);
        g.booklet.lore.deck = g.booklet.cards().map(|c| c.number).collect();
        g.begin_run();
        assert!(g.deck.is_some());
        assert_eq!(g.run.deck_slots, 3);
        assert_eq!(g.run.abilities.len(), 3);
        let view = g.hand_view();
        assert_eq!(view.len(), 3);
        assert_eq!(view.iter().map(|c| c.key).collect::<Vec<_>>(), vec!["shift", "e", "q"]);
        assert!(view.iter().all(|c| c.card.is_some()), "every slot knows its card");
        // Use slot 1.
        let before = view[1].card.as_ref().unwrap().number;
        assert!(g.run.abilities[1].try_use(1.0, 1.0));
        g.deck_on_use(1);
        assert!(g.run.abilities[1].spent);
        assert_eq!(g.deck_leaving.len(), 1);
        assert!(g.hand_view()[1].empty, "the slot waits to draw");
        assert_eq!(g.hand_pile().leaving.len(), 1);
        g.update_deck(deck::DRAW_DELAY + 0.05);
        assert!(!g.run.abilities[1].spent, "drew again");
        let again = g.hand_view();
        assert_eq!(again[1].card.as_ref().unwrap().number, before, "a deck as small as the hand deals it back");
        assert!(again[1].since_draw < 0.2 && !again[1].empty);
        assert!(g.run.abilities[1].cooldown > 0.0, "its cooldown came with it");
        assert!(g.hand_pile().riffle_since < 0.5, "the stack shuffled");
        // The flick finishes and is forgotten.
        g.update_deck(hand::LEAVE_TOTAL);
        assert!(g.deck_leaving.is_empty());
        // Co-op keeps the fixed loadout.
        g.coop = None;
        g.prologue = None;
    }

    #[test]
    fn a_fixed_loadout_run_has_no_deck() {
        let mut g = game("deck_none");
        three_cards(&mut g);
        g.autopilot = true;
        g.begin_run();
        assert!(g.deck.is_none());
        assert_eq!(g.run.deck_slots, 0);
        assert!(g.hand_view().iter().all(|c| !c.empty && c.since_draw == hand::NEVER));
        assert!(!g.hand_pile().shown);
    }
    // === end deck ===

    #[test]
    fn sorting_keeps_the_cursor_on_the_card_and_the_mouse_follows_the_order() {
        let mut g = game("sort_keys");
        three_cards(&mut g);
        let nums: Vec<u32> = g.booklet.cards().map(|c| c.number).collect();
        let newest = *nums.iter().max().unwrap();
        let oldest = *nums.iter().min().unwrap();
        // Merge: flip to newest-first; the cursor (on the oldest) follows its card.
        g.open_merge(hud::Mode::Title);
        g.merge_sel = 0;
        g.merge_key(Keycode::R);
        assert_eq!(g.settings.sort_merge.dir, sorting::Dir::Desc);
        assert_eq!(g.merge_sel, nums.len() - 1);
        assert_eq!(g.sorted_cards(g.settings.sort_merge)[g.merge_sel].number, oldest);
        // A mouse click on the first tile picks the newest card.
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(1280.0, 720.0));
        let lay = merge_ui::layout(screen);
        let (x, y) = tile_centre(&g, g.merge_sel, lay.grid_top, lay.grid_w, 0);
        assert_eq!(g.menu_pointer(x, y, Pointer::Click), Some(Keycode::Return));
        g.merge_key(Keycode::Return);
        assert_eq!(g.merge_first, Some(newest));
        g.merge_key(Keycode::T);
        assert_eq!(g.settings.sort_merge.key, sorting::SortKey::Rarity);
        assert_eq!(g.merge_first, Some(newest), "the picked card survives a re-sort");
        assert_eq!(g.settings.sort_booklet, sorting::Sort::default(), "per screen");
        g.open_loadout();
        g.loadout_key(Keycode::R);
        g.loadout_sel = 0;
        g.booklet.lore.deck.clear(); // opening pre-fills the deck with the default picks
        g.loadout_key(Keycode::Return);
        // Return puts the card under the (sorted) cursor into the deck.
        assert_eq!(g.booklet.lore.deck, vec![newest]);
        // Booklet: paging follows the sorted order.
        g.mode = hud::Mode::Booklet;
        g.booklet_page = 0;
        g.settings.sort_booklet = sorting::Sort { key: sorting::SortKey::Number, dir: sorting::Dir::Desc };
        assert_eq!(g.hud_view().booklet_cards.first().map(|c| c.number), Some(newest));
    }

    #[test]
    fn sorted_screens_draw_and_old_settings_still_load() {
        let mut g = game("sort_frames");
        three_cards(&mut g);
        g.settings.sort_booklet = sorting::Sort { key: sorting::SortKey::Rarity, dir: sorting::Dir::Desc };
        g.settings.sort_merge = sorting::Sort { key: sorting::SortKey::Ability, dir: sorting::Dir::Asc };
        g.settings.sort_loadout = sorting::Sort { key: sorting::SortKey::Dream, dir: sorting::Dir::Desc };
        for mode in [hud::Mode::Booklet, hud::Mode::Merge, hud::Mode::Loadout] {
            g.mode = mode;
            assert!(shapes(&g) > 10, "{mode:?} drew nothing");
        }
        let old: settings::Settings = ron::from_str("(music: 0.5)").unwrap();
        assert_eq!(old.sort_merge, sorting::Sort::default());
        let text = ron::ser::to_string(&g.settings).unwrap();
        let back: settings::Settings = ron::from_str(&text).unwrap();
        assert_eq!(back.sort_loadout, g.settings.sort_loadout);
    }

    #[test]
    fn the_mouse_picks_an_upgrade_and_a_title_row() {
        let mut g = game("choice_mouse");
        g.screen_size = (1280.0, 720.0);
        g.mode = hud::Mode::Choice;
        let card = || upgrade_ui::ChoiceCard {
            name: "X".into(),
            desc: String::new(),
            icon: upgrades::Icon::Boot,
            color: [1, 2, 3],
            frame: [1, 2, 3],
            tag: String::new(),
        };
        g.choice_view.cards = vec![card(), card(), card()];
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(1280.0, 720.0));
        let rects = upgrade_ui::card_rects(screen, 3);
        let c = rects[2].center();
        assert_eq!(g.menu_pointer(c.x as i32, c.y as i32, Pointer::Hover), None);
        assert_eq!(g.choice_view.selected, 2);
        let c = rects[0].center();
        assert_eq!(g.menu_pointer(c.x as i32, c.y as i32, Pointer::Click), Some(Keycode::Return));
        assert_eq!(g.choice_view.selected, 0);
        assert_eq!(g.menu_pointer(2, 2, Pointer::Click), None, "off the cards: nothing");
        // The title menu: a row under the pointer is selected, and a click enters it.
        g.mode = hud::Mode::Title;
        let (rows, goal) = g.title_rows();
        assert!(rows.len() >= 2);
        let l = title_ui::menu_layout(screen, &rows, goal);
        let y = l.menu_top + 1.0 * l.step + l.step * 0.4;
        assert_eq!(g.menu_pointer(640, y as i32, Pointer::Click), Some(Keycode::Return));
        assert_eq!(g.title_sel, 1);
        assert_eq!(g.menu_pointer(5, y as i32, Pointer::Hover), None, "outside the panel");
        assert_eq!(g.title_sel, 1);
    }

    /// A struck pacer forgets it was chasing; make it aware again so a sneak
    /// attack's double does not muddy the numbers.
    fn rechase(g: &mut DreamscapeGame) {
        for p in &mut g.enemies {
            p.ai.chasing = true;
            if let Ok(mut t) = g.world.get::<&mut Transform>(p.entity) {
                t.position.z = t.position.z.min(2.0); // back within reach
            }
        }
        g.stunned.clear();
    }

    fn with_edge(name: &str, e: edges::Edge) -> DreamscapeGame {
        let mut g = fighter(name, None);
        g.edges = vec![e];
        g
    }

    #[test]
    fn rending_strikes_through_a_lullabys_armour_and_a_masks_shield() {
        let mut g = with_edge("rending", edges::Edge::Rending);
        let t = CombatTarget {
            entity: g.world.spawn((Transform { position: Vec3::new(0.0, 1.0, 2.0), rotation: Quat::IDENTITY, scale: Vec3::ONE }, Spin(1.0))),
            pos: Vec3::new(0.0, 1.0, 2.0),
            kind: combat::Kind::Pacer { elite: None, child: false },
            unaware: false,
            guarded: true,
            recovering: false,
        };
        assert!(!g.strike_ctx(&t).guarded, "armour does nothing against it");
        g.edges.clear();
        assert!(g.strike_ctx(&t).guarded);
    }

    #[test]
    fn reprisal_cuts_deeper_only_while_shaking_off_a_hit() {
        let mut g = with_edge("reprisal", edges::Edge::Reprisal);
        let e = put_pacer(&mut g, Vec3::new(0.0, 1.0, 2.0), true);
        g.foes.insert(e, combat::Foe { hp: 6.0, max: 6.0 });
        g.resolve_strike();
        assert_eq!(g.foes[&e].hp, 5.0, "plain: one");
        g.nerve.hit(1, (0.0, 0.0), 0.0);
        rechase(&mut g);
        g.resolve_strike();
        assert_eq!(g.foes[&e].hp, 3.0, "invulnerable after a hit: two");
    }

    #[test]
    fn flourish_cuts_deeper_on_the_third_quick_strike() {
        let mut g = with_edge("flourish", edges::Edge::Flourish);
        let e = put_pacer(&mut g, Vec3::new(0.0, 1.0, 2.0), true);
        g.foes.insert(e, combat::Foe { hp: 9.0, max: 9.0 });
        let mut hp = Vec::new();
        for _ in 0..3 {
            g.dream_age += 0.4;
            rechase(&mut g);
            g.resolve_strike();
            hp.push(g.foes[&e].hp);
            let z = g.world.get::<&Transform>(e).unwrap().position.z;
            g.world.get::<&mut Transform>(e).unwrap().position.z = z.min(2.0);
        }
        assert_eq!(hp, vec![8.0, 7.0, 5.0], "1, 1, then 2 on the third");
    }

    #[test]
    fn haymaker_shoves_twice_as_far() {
        let far = |edge: Option<edges::Edge>| {
            let mut g = fighter("hay", None);
            g.edges = edge.into_iter().collect();
            let e = put_pacer(&mut g, Vec3::new(0.0, 1.0, 2.0), true);
            g.resolve_strike();
            let z = g.world.get::<&Transform>(e).unwrap().position.z;
            z - 2.0
        };
        let (plain, hay) = (far(None), far(Some(edges::Edge::Haymaker)));
        assert!((hay / plain - edges::HAYMAKER_KNOCK).abs() < 0.01, "{plain} -> {hay}");
    }

    #[test]
    fn siphon_steadies_on_every_third_unravel_and_hexmark_primes_the_next_blow() {
        let mut g = with_edge("siphon", edges::Edge::Siphon);
        g.nerve.hit(1, (0.0, 0.0), 0.0);
        g.nerve.tick(2.0);
        g.nerve.hit(1, (0.0, 0.0), 0.0);
        assert_eq!(g.nerve.cur(), 1);
        for k in 0..3 {
            let e = put_pacer(&mut g, Vec3::new(0.0, 1.0, 40.0 + k as f32), false);
            g.unravel(e, Vec3::ZERO);
        }
        assert_eq!(g.nerve.cur(), 2, "the third steadied");
        // Hexmark: a blow marks, the next one cuts a point deeper and spends the mark.
        let mut h = with_edge("hex", edges::Edge::Hexmark);
        let e = put_pacer(&mut h, Vec3::new(0.0, 1.0, 2.0), true);
        h.foes.insert(e, combat::Foe { hp: 9.0, max: 9.0 });
        h.resolve_strike();
        assert_eq!(h.foes[&e].hp, 8.0);
        assert!(h.marks.contains_key(&e), "marked");
        rechase(&mut h);
        let z = h.world.get::<&Transform>(e).unwrap().position.z;
        h.player_position = Vec3::new(0.0, 1.0, z - 2.0);
        h.resolve_strike();
        assert_eq!(h.foes[&e].hp, 6.0, "the marked blow cut two");
        h.update_combat(edges::HEXMARK_SECS + 1.0);
        assert!(h.marks.values().all(|&l| l > 0.0), "marks fade");
    }

    #[test]
    fn lunge_carries_the_dreamer_and_quickdraw_swings_sooner() {
        let mut g = with_edge("lunge", edges::Edge::Lunge);
        g.strike_request = true;
        g.update_combat(0.0);
        assert!(g.lunge_left > 0.0 && g.lunge_dir != Vec3::ZERO, "a step is under way");
        g.update_combat(edges::LUNGE_SECS + 0.01);
        assert_eq!(g.lunge_left, 0.0);
        // Quickdraw: the cooldown is shorter, so a second swing comes sooner.
        let slow = strike_gap(None);
        let quick = strike_gap(Some(edges::Edge::Quickdraw));
        assert!(quick < slow, "{quick} < {slow}");
    }

    // === edges2 ===
    fn hp_after_one_blow(edge: Option<edges::Edge>, setup: impl Fn(&mut DreamscapeGame, Entity)) -> f32 {
        let mut g = fighter("e2", None);
        g.edges = edge.into_iter().collect();
        let e = put_pacer(&mut g, Vec3::new(0.0, 1.0, 2.0), true);
        g.foes.insert(e, combat::Foe { hp: 9.0, max: 9.0 });
        setup(&mut g, e);
        g.resolve_strike();
        g.foes[&e].hp
    }

    #[test]
    fn opener_ambush_and_punisher_each_cut_a_point_deeper_when_they_apply() {
        let nothing = |_: &mut DreamscapeGame, _: Entity| {};
        let plain = hp_after_one_blow(None, nothing);
        assert_eq!(hp_after_one_blow(Some(edges::Edge::Opener), nothing), plain - 1.0);
        // Not fresh: no opener bonus.
        let hurt = |g: &mut DreamscapeGame, e: Entity| {
            g.foes.insert(e, combat::Foe { hp: 8.0, max: 9.0 });
        };
        assert_eq!(hp_after_one_blow(Some(edges::Edge::Opener), hurt), 8.0 - (9.0 - plain));
        // Ambush: an unaware foe (not chasing) takes a point more than without it.
        let calm = |g: &mut DreamscapeGame, _: Entity| {
            for p in &mut g.enemies {
                p.ai.chasing = false;
            }
        };
        assert_eq!(
            hp_after_one_blow(Some(edges::Edge::Ambush), calm),
            hp_after_one_blow(None, calm) - 1.0
        );
        assert_eq!(hp_after_one_blow(Some(edges::Edge::Ambush), nothing), plain, "it was aware");
        // Punisher: a stunned foe.
        let stun = |g: &mut DreamscapeGame, e: Entity| {
            g.stunned.insert(e, 5.0);
        };
        assert_eq!(
            hp_after_one_blow(Some(edges::Edge::Opportunist), stun),
            hp_after_one_blow(None, stun) - 1.0
        );
        assert_eq!(hp_after_one_blow(Some(edges::Edge::Opportunist), nothing), plain);
    }

    #[test]
    fn cleave_hits_to_the_side_and_longarm_hits_further() {
        let hit = |edge: Option<edges::Edge>, at: Vec3| {
            let mut g = fighter("e2reach", None);
            g.edges = edge.into_iter().collect();
            let e = put_pacer(&mut g, at, true);
            g.foes.insert(e, combat::Foe { hp: 9.0, max: 9.0 });
            g.resolve_strike();
            g.foes[&e].hp < 9.0
        };
        let side = Vec3::new(2.0, 1.0, 0.8);
        assert!(!hit(None, side));
        assert!(hit(Some(edges::Edge::Cleave), side));
        let far = Vec3::new(0.0, 1.0, 2.8);
        assert!(!hit(None, far));
        assert!(hit(Some(edges::Edge::Longarm), far));
        assert!(!hit(Some(edges::Edge::Longarm), Vec3::new(0.0, 1.0, 3.2)), "still a limit");
    }

    #[test]
    fn ripple_rattles_the_neighbours_and_windfall_and_lantern_pay_on_unravel() {
        let mut g = with_edge("ripple", edges::Edge::Ripple);
        let a = put_pacer(&mut g, Vec3::new(0.0, 1.0, 2.0), true);
        let b = put_pacer(&mut g, Vec3::new(1.5, 1.0, 2.0), true);
        let far = put_pacer(&mut g, Vec3::new(20.0, 1.0, 20.0), true);
        g.resolve_strike();
        assert!(g.stunned.contains_key(&b) || g.staggered.contains_key(&b), "the neighbour reels");
        assert!(!g.stunned.contains_key(&far));
        let _ = a;
        // Windfall: extra dust. Lantern: a mote.
        let mut w = with_edge("windfall", edges::Edge::Windfall);
        let base = w.bonus_dust;
        let e = put_pacer(&mut w, Vec3::new(0.0, 1.0, 40.0), false);
        w.unravel(e, Vec3::ZERO);
        assert!(w.bonus_dust - base >= edges::WINDFALL_DUST);
        let mut l = with_edge("lantern", edges::Edge::Lantern);
        let e = put_pacer(&mut l, Vec3::new(0.0, 1.0, 40.0), false);
        l.unravel(e, Vec3::ZERO);
        assert_eq!(l.pending_motes.len(), 1, "a calm mote");
    }
    // === end edges2 ===

    /// Seconds between one swing and the next, with an edge.
    fn strike_gap(edge: Option<edges::Edge>) -> f32 {
        let mut g = fighter("cd", None);
        g.edges = edge.into_iter().collect();
        g.strike_request = true;
        g.update_combat(0.0);
        let mut t = 0.0;
        loop {
            g.slash_pending = None;
            g.strike_request = true;
            g.update_combat(0.01);
            t += 0.01;
            if g.slash_pending.is_some() || t > 3.0 {
                return t;
            }
        }
    }

    #[test]
    fn a_card_that_teaches_an_edge_brings_it_into_the_run_and_the_hand_shows_it() {
        let mut g = game("edge_hand");
        // A Chord dream teaches FLOURISH.
        let c = cards::card_from(&cards::tests::record(DreamTheme::SynesthesiaHall, 5, 3, 0.4, false));
        assert_eq!(loadout_ui::edges_of(&c), vec![edges::Edge::Flourish]);
        g.run.take(Upgrade::Learn(Ability::Slam));
        g.hand_cards = vec![(Ability::Slam, c)];
        g.edges = vec![edges::Edge::Flourish];
        g.dream_age = 4.0;
        g.edge_fires(edges::Edge::Flourish);
        g.dream_age = 4.2;
        let card = &g.hand_view()[0];
        assert_eq!(card.edge.map(|e| e.0), Some("FLOURISH"));
        assert!(hand::pulse(card.edge_since) > 0.0);
        // A fused card carries both dreams' edges.
        let mut f = cards::card_from(&cards::tests::record(DreamTheme::SynesthesiaHall, 6, 3, 0.4, false));
        f.fused = Some(DreamTheme::NightmareFactory);
        assert_eq!(loadout_ui::edges_of(&f), vec![edges::Edge::Flourish, edges::Edge::Rending]);
    }

    #[test]
    fn going_to_the_menu_and_waking_use_different_keys_and_q_leaves_nothing() {
        let mut g = game("web_keys");
        // Paused: M goes to the menu; Q does not (a second Q on the title would quit).
        g.mode = hud::Mode::Paused;
        let (hud_pause, hud_wake) = (
            "[m] back to menu",
            "[w] wake",
        );
        assert_ne!(hud_pause, hud_wake);
        assert_eq!(pad::key_for(engine::sdl2::controller::Button::Y, hud::Mode::Paused, &g.settings), Some(Keycode::M));
        assert_eq!(pad::key_for(engine::sdl2::controller::Button::B, hud::Mode::Reveal, &g.settings), Some(Keycode::W));
        // The prompts a controller player sees still name real buttons.
        let s = settings::Settings::default();
        assert_eq!(pad::prompt("[m] back to menu", hud::Mode::Paused, &s, true), "[Y] back to menu");
        assert_eq!(pad::prompt("[w] wake", hud::Mode::Reveal, &s, true), "[B] wake");
        // Every screen's own words use the new keys.
        let paused = frame_with(&g, 1280.0, 720.0, &|_| {});
        let words: Vec<String> = texts(&paused).into_iter().map(|t| t.0).collect();
        assert!(words.iter().any(|w| w.contains("[esc / p] keep dreaming")), "{words:?}");
        assert!(words.iter().any(|w| w.contains("[m] back to menu")));
        assert!(!words.iter().any(|w| w.contains("[q]")), "no Q left on the pause screen");
    }

    fn heard(g: &DreamscapeGame) -> Vec<Sound> {
        g.played.borrow().clone()
    }

    #[test]
    fn a_hit_hurts_out_loud_and_the_last_point_is_the_old_caught_sound_from_the_fail_path() {
        let mut g = game("snd_hurt");
        g.take_nerve_hit("patrol");
        assert_eq!(heard(&g), vec![Sound::Hurt]);
        g.played.borrow_mut().clear();
        g.nerve.tick(2.0);
        g.take_nerve_hit("patrol");
        g.nerve.tick(2.0);
        g.run.take(Upgrade::DreamAnchor);
        g.played.borrow_mut().clear();
        assert_eq!(g.take_nerve_hit("patrol"), None);
        assert_eq!(heard(&g), vec![Sound::LastStand], "dream anchor stands you back up");
    }

    #[test]
    fn at_one_nerve_the_heart_beats_every_second_and_a_third_and_only_then() {
        let mut g = fighter("snd_heart", None);
        g.mode = hud::Mode::Playing;
        for _ in 0..200 {
            g.update_combat(0.05);
        }
        assert!(!heard(&g).contains(&Sound::Heartbeat), "full nerve: silent");
        g.nerve.hit(2, (0.0, 0.0), 0.0);
        g.nerve.tick(2.0);
        assert_eq!(g.nerve.cur(), 1);
        g.played.borrow_mut().clear();
        for _ in 0..200 {
            g.update_combat(0.05); // ten seconds
        }
        let beats = heard(&g).iter().filter(|&&s| s == Sound::Heartbeat).count();
        assert!((6..=9).contains(&beats), "about one every 1.3s, got {beats}");
    }

    #[test]
    fn an_ability_chimes_when_it_is_ready_again_but_not_when_it_starts_ready() {
        let mut g = fighter("snd_ready", Some(Ability::Dash));
        g.mode = hud::Mode::Playing;
        g.update_combat(0.016);
        assert!(!heard(&g).contains(&Sound::Ready), "ready from the start: no chime");
        g.run.abilities[0].cooldown = 2.0;
        g.update_combat(0.016);
        assert!(!heard(&g).contains(&Sound::Ready), "going on cooldown is silent");
        g.run.abilities[0].cooldown = 0.0;
        g.update_combat(0.016);
        assert_eq!(heard(&g).iter().filter(|&&s| s == Sound::Ready).count(), 1);
        g.update_combat(0.016);
        assert_eq!(heard(&g).iter().filter(|&&s| s == Sound::Ready).count(), 1, "once");
    }

    #[test]
    fn edges_ring_when_they_act_and_hexmark_has_its_own_sound() {
        let mut g = with_edge("snd_edge", edges::Edge::Flourish);
        g.edge_fires(edges::Edge::Flourish);
        g.edge_fires(edges::Edge::Hexmark); // not equipped: silent
        assert_eq!(heard(&g), vec![Sound::EdgeFire]);
        let mut h = with_edge("snd_mark", edges::Edge::Hexmark);
        h.edge_fires(edges::Edge::Hexmark);
        assert_eq!(heard(&h), vec![Sound::Mark]);
    }

    #[test]
    fn a_creatures_sounds_carry_only_when_it_is_near() {
        let far = fighter("snd_far", None);
        far.sfx_at(Sound::ToothTell, Vec3::new(500.0, 1.0, 0.0));
        far.sfx_at(Sound::ToothTell, Vec3::new(5.0, 1.0, 0.0));
        assert_eq!(heard(&far), vec![Sound::ToothTell], "only the near one carries");
    }

    #[test]
    fn an_equip_an_unequip_and_a_merge_each_have_their_own_sound() {
        let mut g = game("snd_ui");
        three_cards(&mut g);
        // (An achievement may chime on the first card; only the card sounds matter here.)
        let cards = |g: &DreamscapeGame| -> Vec<Sound> {
            heard(g)
                .into_iter()
                .filter(|s| matches!(s, Sound::Equip | Sound::Unequip | Sound::Merge | Sound::Denied))
                .collect()
        };
        g.open_loadout();
        g.loadout_sel = 0;
        g.loadout_key(Keycode::Return);
        assert_eq!(cards(&g), vec![Sound::Equip]);
        g.loadout_key(Keycode::Return);
        assert_eq!(cards(&g), vec![Sound::Equip, Sound::Unequip]);
        g.played.borrow_mut().clear();
        g.open_merge(hud::Mode::Title);
        g.merge_sel = 0;
        g.merge_key(Keycode::Return);
        g.merge_sel = 1;
        g.merge_key(Keycode::Return);
        assert!(cards(&g).contains(&Sound::Merge));
        // The cursor moving is what the hover tick listens for.
        let before = g.menu_cursor();
        g.merge_sel = (before + 1).min(g.booklet.card_count().saturating_sub(1));
        assert_ne!(g.menu_cursor(), before);
    }

    #[test]
    fn calm_motes_heal_when_touched_and_fade_when_ignored() {
        let mut g = game("motes");
        g.player_position = Vec3::new(0.0, 1.0, 0.0);
        let near = g.world.spawn((
            Transform { position: Vec3::new(0.5, 1.0, 0.0), rotation: Quat::IDENTITY, scale: Vec3::ONE },
            Spin(1.0),
        ));
        let far = g.world.spawn((
            Transform { position: Vec3::new(9.0, 1.0, 9.0), rotation: Quat::IDENTITY, scale: Vec3::ONE },
            Spin(1.0),
        ));
        g.nerve.hit(1, (0.0, 0.0), 0.0);
        assert_eq!(g.nerve.cur(), 2);
        g.motes = vec![(near, 0.0), (far, 0.0)];
        g.update_combat(0.016);
        assert_eq!(g.nerve.cur(), 3, "the near mote steadied you");
        assert_eq!(g.motes.len(), 1);
        g.update_combat(combat::MOTE_LIFE + 1.0);
        assert!(g.motes.is_empty(), "the far one faded");
    }

    #[test]
    fn the_hearts_draw_and_a_broken_one_flashes() {
        let mut g = game("hearts");
        g.mode = hud::Mode::Playing;
        let n = |cur, flinch| {
            frame_with(&g, 1280.0, 720.0, &|v| {
                v.nerve = (cur, 3);
                v.nerve_flinch = flinch;
            })
            .len()
        };
        let hidden = frame_with(&g, 1280.0, 720.0, &|v| v.nerve = (0, 0)).len();
        assert!(n(3, 0.0) > hidden, "hearts draw");
        assert!(n(1, 0.0) > 0);
        // co-op hides them
        assert_eq!(hidden, frame_with(&g, 1280.0, 720.0, &|v| v.nerve = (0, 0)).len());
        // nerve cap: every heart row still fits the screen
        n(5, 0.5);
    }

    #[test]
    fn two_cards_that_make_a_synergy_are_linked_and_one_that_does_not_is_not() {
        let mut g = game("links");
        g.run.take(Upgrade::Learn(Ability::Dash));
        g.run.take(Upgrade::Learn(Ability::Phase));
        g.hand_cards = vec![
            (Ability::Dash, themed(dream::DreamTheme::TheTunnel, 1)),
            (Ability::Phase, themed(dream::DreamTheme::LiminalOffice, 2)),
        ];
        let links = g.hand_links();
        assert_eq!(
            links,
            vec![hand::HandLink { a: 0, b: 1, name: "GHOST STEP" }]
        );
        // Without the second half there is nothing to link.
        g.run = RunUpgrades::default();
        g.run.take(Upgrade::Learn(Ability::Dash));
        assert!(g.hand_links().is_empty());
    }

    #[test]
    fn a_passive_pulses_only_if_the_run_holds_it_and_curses_crack_the_hand() {
        let mut g = game("pulse");
        g.dream_age = 10.0;
        g.passive_fires(Upgrade::DoubleJump);
        assert!(g.passive_fired.is_empty(), "not held, so it cannot fire");
        g.run.take(Upgrade::DoubleJump);
        g.run.take(Upgrade::Learn(Ability::Dash));
        g.hand_cards = vec![(Ability::Dash, themed(dream::DreamTheme::SkyStairs, 3))];
        // SkyStairs gives DoubleJump as its passive.
        assert_eq!(hand::pulse(g.hand_view()[0].passive_since), 0.0);
        g.passive_fires(Upgrade::DoubleJump);
        g.dream_age = 10.2;
        assert!(hand::pulse(g.hand_view()[0].passive_since) > 0.0);
        assert!(!g.hand_view()[0].cracked);
        g.run.take(Upgrade::Cursed(upgrades::Curse::Greed));
        assert!(g.hand_view()[0].cracked);
    }

    #[test]
    fn every_bought_card_back_paints_its_own_stack_and_draw_flight() {
        use store::CardBack::*;
        let mut g = game("backs");
        g.mode = hud::Mode::Playing;
        let pile = |back: Option<store::CardBack>| hand::HandPile {
            shown: true,
            draw_count: 5,
            riffle_since: hand::NEVER,
            leaving: vec![],
            back,
        };
        let paint = |back: Option<store::CardBack>, since_draw: f32, reduced: bool| {
            let mut s = String::new();
            let sh = frame_with(&g, 1280.0, 720.0, &|v| {
                let mut c = card("DASH", hand::NEVER);
                c.since_draw = since_draw;
                v.hand = vec![c];
                v.hand_pile = pile(back);
                v.settings_snapshot.reduced_motion = reduced;
            });
            for x in &sh {
                s.push_str(&format!("{x:?}"));
            }
            (sh.len(), s)
        };
        let mut seen = std::collections::HashSet::new();
        let base = paint(None, hand::NEVER, false);
        for b in [Moth, Eye, Stars, Checker, Ember] {
            let st = paint(Some(b), hand::NEVER, false);
            assert!(st.0 > base.0, "{b:?}: the pattern adds shapes");
            assert!(seen.insert(st.1), "{b:?} paints a different stack");
            let fl = paint(Some(b), 0.1, false);
            assert_ne!(fl.1, paint(Some(b), hand::NEVER, false).1, "{b:?}: the flight shows a back");
            assert_eq!(
                paint(Some(b), 0.1, true).1,
                paint(Some(b), hand::NEVER, true).1,
                "reduced motion keeps the card still"
            );
        }
        // The default back is untouched.
        assert_eq!(paint(None, hand::NEVER, false).1, base.1);
    }

    #[test]
    fn the_hand_draws_at_rest_and_popping_adds_a_ring_without_panicking() {
        let mut g = game("hand");
        g.mode = hud::Mode::Playing;
        let bare = shapes(&g);
        let rest = |n: usize, since: f32| {
            frame_with(&g, 1280.0, 720.0, &|v| {
                v.hand = ["DASH", "BLINK", "PHASE", "FLARE", "FLOAT"][..n]
                    .iter()
                    .map(|a| card(a, since))
                    .collect();
            })
            .len()
        };
        assert!(rest(1, hand::NEVER) > bare, "a held card draws");
        assert!(rest(5, hand::NEVER) > rest(1, hand::NEVER));
        assert!(rest(2, 0.1) > rest(2, hand::NEVER), "a used card rings and flashes");
        let shapes_with = |links: Vec<hand::HandLink>, cracked: bool, since: f32| {
            frame_with(&g, 1280.0, 720.0, &|v| {
                v.hand = vec![card("DASH", hand::NEVER), card("PHASE", hand::NEVER)];
                for c in &mut v.hand {
                    c.cracked = cracked;
                    c.passive_since = since;
                }
                v.hand_links = links.clone();
            })
            .len()
        };
        let plain = shapes_with(vec![], false, hand::NEVER);
        let link = hand::HandLink { a: 0, b: 1, name: "GHOST STEP" };
        assert!(shapes_with(vec![link], false, hand::NEVER) > plain, "a link draws an arc");
        assert!(shapes_with(vec![], true, hand::NEVER) > plain, "a curse draws cracks");
        assert!(shapes_with(vec![], false, 0.2) > plain, "a passive pulse draws");
        for (w, h) in [(960.0, 540.0), (1280.0, 800.0), (1920.0, 1080.0)] {
            frame_with(&g, w, h, &|v| {
                let mut with_art = card("DASH", 0.05);
                with_art.card = Some(cards::card_from(&cards::tests::record(
                    dream::DreamTheme::Garden,
                    9,
                    3,
                    0.4,
                    false,
                )));
                v.hand = vec![with_art, card("BLINK", hand::NEVER)];
            });
        }
    }

    // === deck ===
    #[test]
    fn the_deck_stack_draws_a_riffle_and_cards_in_flight() {
        let mut g = game("deck_draw");
        g.mode = hud::Mode::Playing;
        let n_shapes = |reduced: bool, pile: hand::HandPile, draw: f32| {
            frame_with(&g, 1280.0, 720.0, &|v| {
                v.settings_snapshot.reduced_motion = reduced;
                let mut c = card("DASH", hand::NEVER);
                c.since_draw = draw;
                v.hand = vec![c, card("PHASE", hand::NEVER), card("BLINK", hand::NEVER)];
                v.hand_pile = pile.clone();
            })
            .len()
        };
        let shown = |count: usize, riffle: f32, leaving: Vec<hand::Leaving>| hand::HandPile {
            shown: true,
            draw_count: count,
            riffle_since: riffle,
            leaving,
            back: None,
        };
        let none = n_shapes(false, hand::HandPile::default(), hand::NEVER);
        let stack = n_shapes(false, shown(5, hand::NEVER, vec![]), hand::NEVER);
        assert!(stack > none, "a stack with a badge draws");
        assert!(n_shapes(false, shown(5, 0.2, vec![]), hand::NEVER) > stack, "a riffle adds sheets");
        let flying = vec![hand::Leaving { card: card("DASH", hand::NEVER), slot: 1, since: 0.1 }];
        assert!(n_shapes(false, shown(5, hand::NEVER, flying.clone()), hand::NEVER) > stack, "a flicked card draws");
        assert_eq!(
            n_shapes(true, shown(5, hand::NEVER, flying), hand::NEVER),
            n_shapes(true, shown(5, hand::NEVER, vec![]), hand::NEVER),
            "reduced motion: no flick"
        );
        assert!(n_shapes(false, shown(0, hand::NEVER, vec![]), hand::NEVER) > none, "an empty pile is an outline");
        n_shapes(false, shown(5, hand::NEVER, vec![]), 0.1); // a card mid-draw paints
    }
    // === end deck ===

    #[test]
    fn the_pickup_ring_and_door_glow_draw_only_when_active() {
        let mut g = game("juice");
        g.mode = hud::Mode::Playing;
        let calm = shapes(&g);
        g.juice.hit(juice::Hit::Pickup, false);
        g.juice.chain_pitch();
        let ringing = shapes(&g);
        assert!(ringing > calm, "the ring adds shapes: {ringing} vs {calm}");
        g.juice.step(juice::RING_SECS + 0.1);
        assert_eq!(shapes(&g), calm, "ring is gone after its time");
        g.juice.door(0.01, Some(1.0));
        assert!(shapes(&g) > calm, "the door glow adds shapes");
    }

    #[test]
    fn the_dream_code_box_types_pastes_rejects_typos_and_starts_the_same_run() {
        let mut g = game("code");
        g.open_code_entry();
        assert_eq!(g.mode, hud::Mode::Code);
        assert!(shapes(&g) > 5, "the box draws");

        // Typing the code of a known run, key by key, ends up starting it.
        let code = seedcode::encode(123_456_789, true);
        for ch in seedcode::clean(&code).chars() {
            let k = Keycode::from_name(&ch.to_string()).expect("key for digit");
            g.code_key(k);
        }
        assert_eq!(g.code_digits, seedcode::clean(&code));
        g.code_key(Keycode::Return);
        assert_eq!(g.pending_start, Some(StartKind::Code(123_456_789, true)));
        assert!(g.code_error.is_empty());

        // A pasted code with dashes, lower case and look-alikes works too.
        g.code_paste(&format!("  {}\n", code.to_lowercase().replace('0', "o")));
        assert_eq!(g.code_digits, seedcode::clean(&code));

        // One wrong digit is refused with a reason, and starts nothing.
        g.pending_start = None;
        let mut bad = seedcode::clean(&code).into_bytes();
        bad[2] = if bad[2] == b'7' { b'8' } else { b'7' };
        g.code_paste(&String::from_utf8(bad).unwrap());
        g.code_key(Keycode::Return);
        assert_eq!(g.pending_start, None);
        assert!(g.code_error.contains("typo"), "{}", g.code_error);
        assert!(shapes(&g) > 5, "the error draws");

        // Backspace edits, escape goes back.
        g.code_key(Keycode::Backspace);
        assert_eq!(g.code_digits.len(), seedcode::DIGITS - 1);
        g.code_key(Keycode::Escape);
        assert_eq!(g.mode, hud::Mode::Title);
    }

    #[test]
    fn the_wake_summary_offers_the_runs_own_code_which_reads_back() {
        let mut g = game("summary_code");
        g.run_seed = 987_654_321;
        g.run_length = RunLength::Long;
        g.mode = hud::Mode::Summary;
        let v = g.hud_view();
        assert_eq!(
            seedcode::decode(&v.run_code),
            Ok((987_654_321, true)),
            "the summary shows a code that replays this run"
        );
        g.title_age = 10.0;
        assert!(shapes(&g) > 20);
        // Off the summary screen it stays out of the way.
        g.mode = hud::Mode::Title;
        assert!(g.hud_view().run_code.is_empty());
    }

    #[test]
    fn the_title_menu_offers_the_dream_code() {
        let g = game("title_code");
        assert!(g.title_items().contains(&TitleItem::DreamCode));
        let (key, label, _) = g.title_label(TitleItem::DreamCode);
        assert_eq!(key, "k");
        assert!(label.contains("code"));
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

    // ---- pick-a-pack flow ----

    fn pad_key(b: engine::sdl2::controller::Button) -> Option<Keycode> {
        pad::key_for(b, hud::Mode::PackPick, &settings::Settings::default())
    }

    #[test]
    fn selection_wraps_with_the_keys_and_clamps_with_numbers() {
        let mut g = game("pick_sel");
        g.begin_pick(offers(3));
        assert_eq!(g.pick.selected, 0);
        g.pick_key(Keycode::A);
        assert_eq!(g.pick.selected, 2, "left of the first wraps to the last");
        g.pick_key(Keycode::D);
        assert_eq!(g.pick.selected, 0, "right of the last wraps to the first");
        g.pick_key(Keycode::Right);
        g.pick_key(Keycode::Right);
        g.pick_key(Keycode::Left);
        assert_eq!(g.pick.selected, 1);
        g.pick_key(Keycode::Num3);
        assert_eq!(g.pick.selected, 2);
        g.pick_key(Keycode::Num1);
        assert_eq!(g.pick.selected, 0);

        // Two offers: 3 means nothing, and 1..2 land where they say.
        let mut g = game("pick_sel2");
        g.begin_pick(offers(2));
        g.pick_key(Keycode::Num2);
        g.pick_key(Keycode::Num3);
        assert_eq!(g.pick.selected, 1, "no third pack to select");
        g.pick.selected = 9; // stale index
        g.pick_move(1);
        assert_eq!(g.pick.selected, 0, "a stale index clamps before it moves");
        g.pick.selected = 9;
        g.pick_open();
        assert_eq!(g.pack.pack, offers(2)[1].cards, "a stale index opens the last pack");
    }

    #[test]
    fn the_pad_moves_and_opens() {
        use engine::sdl2::controller::Button as B;
        assert_eq!(pad_key(B::DPadLeft), Some(Keycode::Left));
        assert_eq!(pad_key(B::DPadRight), Some(Keycode::Right));
        assert_eq!(pad_key(B::A), Some(Keycode::Return));
        assert_eq!(pad_key(B::B), None, "no backing out of the pick");
        let mut g = game("pick_pad");
        g.begin_pick(offers(3));
        g.pick_key(pad_key(B::DPadRight).unwrap());
        g.pick.age = 1.0;
        g.pick_key(pad_key(B::A).unwrap());
        assert_eq!(g.mode, hud::Mode::Reveal);
        assert_eq!(g.pack.pack, packs::open(&offers(3)[1]));
    }

    #[test]
    fn return_opens_the_selected_pack_and_drops_the_others() {
        let mut g = game("pick_open");
        let all = offers(3);
        g.begin_pick(all.clone());
        g.pick_key(Keycode::Num3);
        g.pick_key(Keycode::Return);
        assert_eq!(g.mode, hud::Mode::PackPick, "a held key can't open blind");
        g.pick.age = reveal_ui::PICK_LOCKOUT + 0.01;
        g.pick_key(Keycode::Space);
        assert_eq!(g.mode, hud::Mode::Reveal);
        assert_eq!(g.pack.pack, packs::open(&all[2]), "the pack is what the chosen offer opens to");
        assert!(g.pick.offers.is_empty(), "the other offers are gone");
        assert_eq!(g.pick, reveal_ui::PackPickView::default());
        // Opening again (a second Return on a stale screen) does nothing.
        g.pick_open();
        assert_eq!(g.pack.pack, packs::open(&all[2]));
    }

    #[test]
    fn a_longer_run_opens_several_packs_and_presses_them_together() {
        let mut g = game("pick_many");
        // Seven dreams played: three packs to open.
        for i in 0..7 {
            g.run_log
                .push(cards::tests::record(dream::DreamTheme::Garden, 900 + i, 1 + i as u32, 0.2, false));
        }
        assert_eq!(packs::picks_for(g.run_log.len()), 3);
        g.begin_pick(offers(3));
        assert_eq!(g.pick.picks_left, 3);
        let mut seen = 0;
        for left in (0..3).rev() {
            assert_eq!(g.mode, hud::Mode::PackPick);
            g.pick_open();
            assert_eq!(g.mode, hud::Mode::Reveal);
            assert_eq!(g.pack.picks_left, left, "the reveal knows how many are left");
            assert_eq!(g.pick.picks_left, left);
            seen += g.pack.pack.len();
            assert_eq!(g.haul.len(), seen, "every opened card joins the haul");
            if left > 0 {
                assert_eq!(g.pick.offers.len(), left, "the rest stay on the table");
                g.pick.age = 0.0;
                g.mode = hud::Mode::PackPick; // what [enter] does on the reveal
            }
        }
        assert_eq!(g.pick, reveal_ui::PackPickView::default(), "done: nothing left to pick");
        g.run_seed = 5151;
        g.save_journal();
        assert!(g.pack.pressed);
        let remembered: std::collections::HashSet<u64> =
            g.haul.iter().filter(|r| r.remembered).map(|r| r.card.seed).collect();
        let held: std::collections::HashSet<u64> = g.booklet.cards().map(|c| c.seed).collect();
        assert_eq!(held, remembered, "all three packs pressed, nothing else");
    }

    #[test]
    fn you_cannot_open_more_packs_than_are_on_the_table() {
        let mut g = game("pick_clamp");
        for i in 0..12 {
            g.run_log
                .push(cards::tests::record(dream::DreamTheme::Garden, 700 + i, 1, 0.2, false));
        }
        assert_eq!(packs::picks_for(12), 4);
        g.begin_pick(offers(2));
        assert_eq!(g.pick.picks_left, 2);
    }

    #[test]
    fn unchosen_cards_never_reach_the_booklet() {
        let mut g = game("pick_press");
        let all = offers(3);
        g.begin_pick(all.clone());
        g.pick_select(1);
        g.pick_open();
        g.run_seed = 4242;
        g.save_journal();
        assert!(g.pack.pressed);
        let held: Vec<u64> = g.booklet.cards().map(|c| c.seed).collect();
        let opened = packs::open(&all[1]);
        assert!(!opened.is_empty() && opened.len() <= all[1].cards.len());
        assert_eq!(g.pack.pack, opened);
        let chosen: Vec<u64> = opened.iter().filter(|r| r.remembered).map(|r| r.card.seed).collect();
        for seed in &chosen {
            assert!(held.contains(seed), "chosen card {seed} pressed");
        }
        for (i, o) in all.iter().enumerate().filter(|(i, _)| *i != 1) {
            for r in &o.cards {
                assert!(!held.contains(&r.card.seed), "offer {i} card {} leaked", r.card.seed);
            }
        }
    }

    /// A pack of one faded Faint dream: pity counts it as dry.
    fn dry_offer(seed: u64) -> packs::PackOffer {
        let mut o = offer(dream::DreamTheme::Garden, 1, seed);
        o.cards[0].card.rarity = cards::Rarity::Faint;
        o.cards[0].remembered = false;
        o
    }

    #[test]
    fn pity_advances_only_on_the_opened_pack() {
        let mut g = game("pick_pity");
        g.begin_pick(vec![dry_offer(1), dry_offer(2), dry_offer(3)]);
        assert_eq!(g.booklet.lottery.pack_dry, 0, "offering touches nothing");
        g.pick_move(1);
        g.pick_move(-1);
        g.pick_select(2);
        assert_eq!(g.booklet.lottery.pack_dry, 0, "browsing touches nothing");
        g.pick_open();
        assert_eq!(g.booklet.lottery.pack_dry, 1, "one opened pack, one dry count");

        // At the threshold the opened pack is upgraded; the unopened never are.
        let mut g = game("pick_pity2");
        g.booklet.lottery.pack_dry = lottery::PACK_PITY;
        let mut a = dry_offer(10);
        a.cards.push(dry_offer(11).cards.remove(0));
        let b = dry_offer(12);
        g.begin_pick(vec![a.clone(), b.clone()]);
        assert_eq!(g.pick.offers[0], a, "pity is not applied while browsing");
        assert_eq!(g.pick.offers[1], b);
        g.pick_open();
        assert!(g.pack.pack.iter().any(|r| r.remembered && r.card.rarity >= cards::Rarity::Lucid));
        assert_eq!(g.booklet.lottery.pack_dry, 0);
    }

    #[test]
    fn an_empty_offer_goes_straight_to_an_empty_reveal() {
        let mut g = game("pick_empty");
        g.begin_pick(Vec::new());
        assert_eq!(g.mode, hud::Mode::Reveal);
        assert!(g.pack.pack.is_empty());
        assert_eq!(g.pick, reveal_ui::PackPickView::default());
        assert_eq!(g.booklet.lottery.pack_dry, 0);
        assert!(shapes(&g) > 10, "the empty reveal still draws");
        // Keys on a pick screen that isn't up do nothing.
        g.pick_open();
        assert_eq!(g.mode, hud::Mode::Reveal);
    }

    #[test]
    fn the_mouse_hovers_selects_and_clicks_opens() {
        let mut g = game("pick_mouse");
        g.begin_pick(offers(3));
        g.screen_size = (1280.0, 720.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(1280.0, 720.0));
        let rects = reveal_ui::pick_rects(screen, 3);
        assert_eq!(rects.len(), 3);
        assert!(rects.windows(2).all(|w| w[0].right() < w[1].left()), "side by side");
        let c = |i: usize| (rects[i].center().x as i32, rects[i].center().y as i32);
        g.pick_pointer(c(2).0, c(2).1, false);
        assert_eq!(g.pick.selected, 2, "hover selects");
        assert_eq!(g.mode, hud::Mode::PackPick, "hover does not open");
        g.pick_pointer(1, 1, false);
        assert_eq!(g.pick.selected, 2, "off the packs, nothing changes");
        g.pick_pointer(c(1).0, c(1).1, true);
        assert_eq!(g.pick.selected, 1);
        assert_eq!(g.mode, hud::Mode::PackPick, "too early: the screen is still sliding in");
        g.pick.age = 1.0;
        g.pick_pointer(c(1).0, c(1).1, true);
        assert_eq!(g.mode, hud::Mode::Reveal);
        assert_eq!(g.pack.pack, packs::open(&offers(3)[1]));
    }

    /// Every text shape in a frame, flattened: (text, its rect on screen).
    fn texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<(String, egui::Rect)> {
        fn walk(s: &egui::Shape, out: &mut Vec<(String, egui::Rect)>) {
            match s {
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                egui::Shape::Text(t) if !t.galley.is_empty() => {
                    out.push((t.galley.text().to_string(), t.galley.rect.translate(t.pos.to_vec2())))
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        shapes.iter().for_each(|c| walk(&c.shape, &mut out));
        out
    }

    /// Filled panels and boxes in a frame (the containers text sits in).
    fn boxes(shapes: &[egui::epaint::ClippedShape]) -> Vec<egui::Rect> {
        fn walk(s: &egui::Shape, out: &mut Vec<egui::Rect>) {
            match s {
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                egui::Shape::Rect(r) if r.fill.a() > 0 && r.rect.width() > 8.0 && r.rect.height() > 8.0 => {
                    out.push(r.rect)
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        shapes.iter().for_each(|c| walk(&c.shape, &mut out));
        out
    }

    const AUDIT_SIZES: [(f32, f32); 9] = [
        (640.0, 360.0),
        (800.0, 600.0),
        (1024.0, 768.0),
        (1280.0, 720.0),
        (1366.0, 768.0),
        (1920.0, 1080.0),
        (2560.0, 1440.0),
        (3440.0, 1440.0),
        (3840.0, 2160.0),
    ];

    /// Text that runs off the screen, or out of the smallest panel it starts in.
    fn overflows(g: &DreamscapeGame, label: &str) -> Vec<String> {
        overflows_with(g, label, &|_| {})
    }

    fn overflows_with(
        g: &DreamscapeGame,
        label: &str,
        tweak: &dyn Fn(&mut hud::HudView),
    ) -> Vec<String> {
        let mut bad = Vec::new();
        let drawn = texts(&frame_with(g, 1280.0, 720.0, tweak)).len();
        eprintln!("audit {label}: {drawn} texts");
        if drawn == 0 {
            bad.push(format!("{label} drew no text at all"));
        }
        for (w, h) in AUDIT_SIZES {
            let shapes = frame_with(g, w, h, tweak);
            let k = hud::ui_scale(g.mode, w, h);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w / k, h / k));
            let panels = boxes(&shapes);
            for (text, r) in texts(&shapes) {
                if text.trim().is_empty() {
                    continue;
                }
                let tol = 1.5;
                if !screen.expand(tol).contains_rect(r) {
                    bad.push(format!("{label} {w}x{h} OFF-SCREEN {text:?} {r:?}"));
                    continue;
                }
                // The tightest panel that holds the text's centre and is
                // bigger than the text: the text belongs to it.
                let c = r.center();
                let home = panels
                    .iter()
                    .filter(|b| b.contains(c) && b.area() > r.area() * 1.05 && b.area() < screen.area() * 0.9)
                    .min_by(|a, b| a.area().total_cmp(&b.area()));
                if let Some(b) = home {
                    if !b.expand(tol).contains_rect(r) {
                        bad.push(format!("{label} {w}x{h} OUT-OF-BOX {text:?} {r:?} box {b:?}"));
                    }
                }
            }
        }
        bad
    }

    // === pacts ===
    #[test]
    fn pacts_bite_and_pay() {
        let mut g = game("pacts");
        g.director.depth = 5;
        let base_speed = g.player_speed();
        let base_mult = g.dust_multiplier();
        g.take_pact(pacts::Pact::HeavyDreams);
        assert!(g.player_speed() < base_speed);
        assert!(g.dust_multiplier() > base_mult);
        let shards = g.director.shards_to_wake;
        g.take_pact(pacts::Pact::ShortFuse);
        assert_eq!(g.director.shards_to_wake, shards + 1);
        g.take_pact(pacts::Pact::NoRegrets);
        g.open_upgrade_choice();
        let rerolls = g.run.rerolls_used;
        g.reroll_choice();
        assert_eq!(g.run.rerolls_used, rerolls);
        let dust = g.bonus_dust;
        g.skip_choice();
        assert!(g.choice.is_some() && g.bonus_dust == dust);
        let rows = g.summary_view();
        assert!(rows.rows.iter().any(|(k, _)| k == "pact"));
    }

    #[test]
    fn thin_air_removes_a_card_and_owed_costs_dust() {
        let mut g = game("pacts2");
        g.director.depth = 5;
        g.open_upgrade_choice();
        let n = g.choice_view.cards.len();
        g.take_pact(pacts::Pact::ThinAir);
        g.open_upgrade_choice();
        assert_eq!(g.choice_view.cards.len(), (n - 1).max(2));
        g.bonus_dust = 25;
        g.take_pact(pacts::Pact::Owed);
        assert_eq!(g.bonus_dust, 15);
    }
    // === end pacts ===

    // === shop ===
    /// A game one pick away from a nightmare reward, with a mergeable booklet
    /// (so the old merge popup would have fired).
    fn reward_game(name: &str) -> DreamscapeGame {
        let mut g = game(name);
        three_cards(&mut g);
        assert!(merge_ui::any_pair(&g.booklet.cards().cloned().collect::<Vec<_>>()));
        g.director.depth = 1;
        g.boss_reward = true;
        g.open_upgrade_choice();
        assert!(matches!(g.choice, Some(ChoiceKind::Upgrade(_))));
        g
    }

    #[test]
    fn a_nightmare_reward_opens_the_shop_not_the_merge_menu() {
        let mut g = reward_game("shop_reward");
        g.choose(0);
        assert_eq!(g.choice, Some(ChoiceKind::MothShop));
        assert_eq!(g.mode, hud::Mode::Choice);
        assert_ne!(g.mode, hud::Mode::Merge);
        assert!(!g.boss_reward, "the reward is spent");
        assert_eq!(g.choice_view.title, moth_shop::TITLE);
        // Shelf cards plus the reroll card.
        let shelf = g.moth_stock.clone().expect("stocked");
        // Shelf cards plus the reroll and the moth's coin.
        assert_eq!(g.choice_view.cards.len(), shelf.cards.len() + 2);
        g.leave_moth_shop();
        assert_eq!((g.mode, g.choice.clone()), (hud::Mode::Playing, None));
        // Once: a further pick-less call does nothing.
        g.moth_after_nightmare();
        assert_eq!(g.choice, None);
    }

    #[test]
    fn a_sold_out_shelf_keeps_the_shop_open_with_reroll_coin_and_leave() {
        let mut g = reward_game("shop_soldout");
        g.choose(0);
        g.bonus_dust = 5000;
        for _ in 0..40 {
            if g.moth_stock.as_ref().is_some_and(|s| s.cards.is_empty()) {
                break;
            }
            g.choose(0);
        }
        g.moth_stock.as_mut().unwrap().tools.clear();
        g.show_moth_shop(0);
        assert!(g.moth_stock.as_ref().unwrap().is_empty());
        assert_eq!(g.choice, Some(ChoiceKind::MothShop), "still open");
        let names: Vec<_> = g.choice_view.cards.iter().map(|c| c.name.clone()).collect();
        assert_eq!(names, ["EMPTY", "REROLL", gamble::ROW_NAME], "{names:?}");
        assert!(g.choice_view.cards[0].desc.contains("NOTHING LEFT"));
        // The EMPTY row is not for sale.
        let dust = g.bonus_dust;
        g.choose(0);
        assert_eq!((g.bonus_dust, g.choice.clone()), (dust, Some(ChoiceKind::MothShop)));
        // The reroll refills the shelf.
        g.choose(1);
        assert!(g.bonus_dust < dust, "the reroll was paid");
        assert!(!g.moth_stock.as_ref().unwrap().is_empty(), "refilled");
        // Sell it out again by hand: the coin row is reachable by [g].
        g.moth_stock.as_mut().unwrap().cards.clear();
        g.moth_stock.as_mut().unwrap().tools.clear();
        g.show_moth_shop(0);
        assert_eq!(g.gamble_row(), 2);
        g.choose(g.gamble_row());
        assert!(matches!(g.choice, Some(ChoiceKind::Gamble(_))), "{:?}", g.choice);
        g.gamble_back();
        assert_eq!(g.choice, Some(ChoiceKind::MothShop));
        g.leave_moth_shop();
        assert_eq!((g.mode, g.choice.clone()), (hud::Mode::Playing, None));
    }

    #[test]
    fn skipping_a_nightmare_reward_also_opens_the_shop_once() {
        let mut g = reward_game("shop_skip");
        let dust = g.bonus_dust;
        g.skip_choice();
        assert_eq!(g.choice, Some(ChoiceKind::MothShop));
        assert_eq!(g.bonus_dust, dust + upgrades::SKIP_DUST);
        assert!(!g.boss_reward);
        g.leave_moth_shop();
        assert_eq!(g.mode, hud::Mode::Playing);
        assert_eq!(g.choice, None);
    }

    #[test]
    fn no_shop_in_autopilot_coop_or_for_an_ordinary_pick() {
        let mut g = reward_game("shop_autopilot");
        g.autopilot = true;
        g.choose(0);
        assert_eq!(g.choice, None);
        let mut g = game("shop_plain");
        g.director.depth = 2;
        g.open_upgrade_choice();
        g.choose(0);
        assert_eq!((g.choice.clone(), g.mode), (None, hud::Mode::Playing));
    }

    // === risk ===
    fn coin_game(name: &str, dust: u32) -> DreamscapeGame {
        let mut g = game(name);
        g.director.depth = 6;
        g.booklet.stash.dust = 500;
        g.bonus_dust = dust;
        g.enter_moth_shop_for_test();
        g
    }

    /// Walk the three steps: row, stake, confirm.
    fn flip(g: &mut DreamscapeGame, stake_index: usize) {
        let row = g.gamble_row();
        g.choose(row);
        assert_eq!(g.choice, Some(ChoiceKind::Gamble(GambleStage::Stakes)));
        g.choose(stake_index);
        assert!(matches!(g.choice, Some(ChoiceKind::Gamble(GambleStage::Confirm(_)))));
        g.choose(0);
    }

    #[test]
    fn the_coin_costs_nothing_until_confirmed_and_odds_are_on_the_screen() {
        let mut g = coin_game("risk_confirm", 40);
        let row = g.gamble_row();
        g.choose(row);
        assert_eq!(g.bonus_dust, 40);
        assert!(g.choice_view.cards[0].desc.contains("50% double · 50% lose the stake"));
        assert_eq!(g.choice_view.cards.len(), 4, "5, 10, 20 and back");
        g.choose(1);
        assert_eq!(g.bonus_dust, 40, "choosing a stake stakes nothing");
        assert_eq!(g.choice_view.selected, 1, "the safe card is the default");
        assert!(g.choice_view.cards[0].desc.contains("50%"));
        g.choose(1); // NOT NOW
        assert_eq!(g.choice, Some(ChoiceKind::Gamble(GambleStage::Stakes)));
        assert_eq!((g.bonus_dust, g.gamble.flips), (40, 0));
        // [x] walks back out to the shelf.
        g.gamble_back();
        assert_eq!(g.choice, Some(ChoiceKind::MothShop));
        assert_eq!(g.bonus_dust, 40);
    }

    #[test]
    fn a_flip_moves_exactly_the_stake_on_run_dust_only() {
        let mut g = coin_game("risk_flip", 40);
        flip(&mut g, 1); // 10 dust
        assert_eq!(g.gamble.flips, 1);
        assert!(g.bonus_dust == 50 || g.bonus_dust == 30, "{}", g.bonus_dust);
        assert_eq!(g.booklet.stash.dust, 500, "the bank is untouched");
        assert_eq!(g.choice, Some(ChoiceKind::Gamble(GambleStage::Result)));
        g.choose(0);
        assert_eq!(g.choice, Some(ChoiceKind::MothShop));
    }

    #[test]
    fn stakes_never_exceed_half_the_dust_and_a_visit_allows_three() {
        let mut g = coin_game("risk_caps", 9);
        let row = g.gamble_row();
        g.choose(row);
        assert_eq!(g.choice, Some(ChoiceKind::MothShop), "9 dust: shut");
        let mut g = coin_game("risk_visit", 400);
        g.gamble.losses = gamble::GAMBLE_LOSS_CAP; // the first is a win, whatever
        for _ in 0..gamble::MAX_PER_VISIT {
            flip(&mut g, 0);
            g.choose(0);
        }
        assert_eq!(g.gamble_visit, 3);
        let row = g.gamble_row();
        g.choose(row);
        assert_eq!(g.choice, Some(ChoiceKind::MothShop), "a fourth is refused");
        assert_eq!(g.gamble.flips, 3);
        g.flip_coin(5);
        assert_eq!(g.gamble.flips, 3, "even called directly");
    }

    #[test]
    fn two_losses_then_the_moth_owes_you_one_and_says_so() {
        let mut g = coin_game("risk_owed", 400);
        g.gamble.losses = gamble::GAMBLE_LOSS_CAP;
        let row = g.gamble_row();
        g.choose(row);
        assert!(g.choice_view.cards[0].desc.contains("the moth owes you one"));
        g.choose(0);
        let dust = g.bonus_dust;
        g.choose(0);
        assert_eq!(g.bonus_dust, dust + 5, "a guaranteed win");
        assert_eq!(g.gamble.losses, 0);
    }

    #[test]
    fn quitting_cannot_reset_the_visit_count_or_the_wager_baseline() {
        let mut g = coin_game("risk_quit", 400);
        g.save_path = std::env::temp_dir().join("dreamscape_smoke_risk_quit_run.ron");
        let _ = std::fs::remove_file(&g.save_path);
        g.run_active = true;
        g.wager_dust_start = 123;
        for _ in 0..gamble::MAX_PER_VISIT {
            flip(&mut g, 0);
            g.choose(0);
        }
        let start = g.wager_dust_start;
        let saved = progress::load(&g.save_path).expect("saved");
        // What the continue path and load_dream do with it.
        g.gamble = saved.gamble.clone();
        g.bonus_dust = saved.bonus_dust;
        g.wager_dust_start = g.gamble.take_wager_start(g.bonus_dust);
        g.gamble_visit = g.gamble.take_visit_flips();
        assert_eq!(g.gamble_visit, gamble::MAX_PER_VISIT, "no extra flips after a reload");
        assert_eq!(g.wager_dust_start, start, "not the current dust");
        assert_ne!(g.wager_dust_start, g.bonus_dust);
        let _ = std::fs::remove_file(&g.save_path);
    }

    #[test]
    fn the_outcome_is_saved_before_it_shows_and_a_reload_cannot_reroll_it() {
        let mut g = coin_game("risk_save", 100);
        g.save_path = std::env::temp_dir().join("dreamscape_smoke_risk_save_run.ron");
        let _ = std::fs::remove_file(&g.save_path);
        g.run_active = true;
        flip(&mut g, 2);
        let saved = progress::load(&g.save_path).expect("saved at commit");
        assert_eq!(saved.bonus_dust, g.bonus_dust, "the dust and the counter were saved together");
        assert_eq!(saved.gamble.visit_flips, Some(1));
        assert_eq!(saved.gamble.wager_start, Some(g.wager_dust_start));
        assert_eq!(saved.gamble.flips, g.gamble.flips);
        assert_eq!(saved.gamble.flips, 1);
        // Same dream, reloaded state: the next flip is the same draw.
        let seed = g.director.dream_seed();
        assert_eq!(saved.gamble.roll(seed), g.gamble.roll(seed));
        let _ = std::fs::remove_file(&g.save_path);
    }

    #[test]
    fn the_shop_is_unchanged_when_you_do_not_gamble() {
        let mut g = coin_game("risk_unchanged", 100);
        let shelf = g.moth_stock.clone().unwrap();
        let cost = moth_shop::price_at(&shelf, 0, g.moth_terms()).unwrap();
        g.choose(0);
        assert_eq!(g.bonus_dust, 100 - cost);
        assert_eq!(g.gamble, gamble::RunState::default());
        g.leave_moth_shop();
        assert_eq!((g.mode, g.choice.clone()), (hud::Mode::Playing, None));
    }

    // === toolcards ===
    fn tool_shop_game(name: &str, dust: u32) -> DreamscapeGame {
        let mut g = deck_game(name);
        g.run.deck_slots = 5;
        g.director.depth = 6;
        g.bonus_dust = dust;
        g.save_path = std::env::temp_dir().join(format!("dreamscape_smoke_{name}_run.ron"));
        g.enter_moth_shop_for_test();
        g
    }

    #[test]
    fn the_shelf_sells_tool_cards_only_in_deck_runs() {
        let g = tool_shop_game("tool_offer", 100);
        let shelf = g.moth_stock.clone().unwrap();
        assert_eq!(shelf.tools.len(), moth_shop::TOOLS_OFFERED);
        assert_eq!(g.choice_view.cards.len(), shelf.rows() + 2, "cards, tools, reroll, coin");
        let at = shelf.cards.len();
        assert!(g.choice_view.cards[at].tag.contains("dust"), "{}", g.choice_view.cards[at].tag);
        assert_eq!(g.choice_view.cards[at].name, shelf.tools[0].name());
        let fixed = coin_game("tool_fixed", 100);
        assert!(fixed.moth_stock.as_ref().unwrap().tools.is_empty());
    }

    #[test]
    fn buying_tool_cards_costs_more_each_time_stops_at_two_and_keeps_counts() {
        let mut g = tool_shop_game("tool_buy", 200);
        let before = g.deck.as_ref().unwrap().all_cards().len();
        let shelf = g.moth_stock.clone().unwrap();
        let at = shelf.cards.len();
        let (a, b) = (shelf.tools[0], shelf.tools[1]);
        let first = moth_shop::tool_price(a, 0, g.moth_terms());
        g.choose(at);
        assert_eq!(g.bonus_dust, 200 - first);
        let second = moth_shop::tool_price(b, 1, g.moth_terms());
        assert_eq!(second, moth_shop::tool_base_price(b) + moth_shop::TOOL_PRICE_STEP);
        g.choose(at + 1);
        assert_eq!(g.bonus_dust, 200 - first - second);
        let d = g.deck.as_ref().unwrap();
        assert_eq!(d.tool_cards().len(), 2);
        assert_eq!(d.all_cards().len(), before + 2, "counts: two cards added, none lost");
        assert_eq!(g.tool_cards_held().len(), moth_shop::MAX_TOOL_CARDS);
        // A third (a fresh reroll of the shelf) shows no room and takes nothing.
        let mut shelf = g.moth_stock.clone().unwrap();
        shelf.tools = vec![Ability::Overload];
        g.moth_stock = Some(shelf);
        let dust = g.bonus_dust;
        g.show_moth_shop(0);
        let row = g.moth_stock.as_ref().unwrap().cards.len();
        assert_eq!(g.choice_view.cards[row].tag, moth_shop::TOOL_NO_ROOM);
        g.choose(row);
        assert_eq!(g.bonus_dust, dust);
        assert_eq!(g.deck.as_ref().unwrap().tool_cards().len(), 2);
        assert!(g.booklet.lore.deck.iter().all(|&n| !deck::is_tool(n)), "never in the booklet's deck");
    }

    #[test]
    fn a_bought_tool_card_is_shown_as_a_tool_in_the_hand_and_never_saved_in_the_booklet() {
        let mut g = deck_game("tool_hand");
        g.run.deck_slots = 5;
        let mut d = deck::Deck::new(&[], 3);
        d.insert_shuffled(deck::tool_id(Ability::Snare), 3);
        d.draw_to_fill(5);
        g.deck = Some(d);
        g.update_deck(0.0);
        let view = g.hand_view();
        let c = view.iter().find(|c| c.tool).expect("a tool card in hand");
        assert_eq!((c.ability, c.card.is_none(), c.passive.is_none(), c.edge.is_none()), ("SNARE", true, true, true));
        assert!(g.booklet.card(deck::tool_id(Ability::Snare)).is_none());
        // The card draws like any other: its face can be painted.
        let shapes = frame(&g, 1280.0, 720.0).len();
        assert!(shapes > 0);
        assert!(!view.iter().any(|c| !c.tool && c.ability == "SNARE"));
    }

    #[test]
    fn using_a_tool_card_discards_it_and_it_comes_back_into_the_draw_pile() {
        let mut g = deck_game("tool_use");
        g.run.deck_slots = 5;
        let id = deck::tool_id(Ability::Echo);
        let mut d = deck::Deck::new(&[], 3);
        d.insert_shuffled(id, 3);
        d.draw_to_fill(5);
        g.deck = Some(d);
        g.update_deck(0.0);
        g.run.abilities[0].cooldown = 0.0;
        g.deck_on_use(0);
        assert_eq!(g.deck.as_ref().unwrap().hand()[0], None);
        g.update_deck(deck::SHUFFLE_DELAY + deck::DRAW_DELAY + 0.1);
        assert_eq!(g.deck.as_ref().unwrap().tool_cards(), vec![id], "kept for the run");
        assert_eq!(g.deck.as_ref().unwrap().all_cards(), vec![id]);
    }

    #[test]
    fn tool_cards_survive_a_save_and_an_old_save_loads_without_them() {
        let mut g = tool_shop_game("tool_save", 100);
        g.run_active = true;
        g.choose(g.moth_stock.as_ref().unwrap().cards.len());
        let id = g.deck.as_ref().unwrap().tool_cards()[0];
        let saved = progress::load(&g.save_path).expect("saved when bought");
        assert_eq!(saved.tool_cards, vec![id]);
        // Resumed: the list is carried on (a resumed run has no deck yet) ...
        let mut r = game("tool_resume");
        r.deck = None;
        r.tool_cards = saved.tool_cards.clone();
        r.restore_tool_cards();
        assert_eq!(r.tool_cards_held(), vec![id]);
        // ... and goes back into a deck when there is one.
        r.deck = Some(deck::Deck::new(&[1, 2, 3], 4));
        r.restore_tool_cards();
        assert_eq!(r.deck.as_ref().unwrap().tool_cards(), vec![id]);
        // An old save without the field loads, with no tool cards.
        let mut bare = saved.clone();
        bare.tool_cards.clear();
        let text = ron::ser::to_string_pretty(&bare, ron::ser::PrettyConfig::default()).unwrap();
        let old: String = text.lines().filter(|l| !l.contains("tool_cards")).collect::<Vec<_>>().join("\n");
        let back: progress::SavedRun = ron::from_str(&old).expect("old save loads");
        assert!(back.tool_cards.is_empty());
    }
    // === end toolcards ===

    #[test]
    fn no_coin_in_autopilot_or_co_op() {
        let mut g = coin_game("risk_inert", 100);
        g.autopilot = true;
        g.open_gamble_stakes();
        assert_eq!(g.choice, Some(ChoiceKind::MothShop));
        g.flip_coin(5);
        assert_eq!((g.gamble.flips, g.bonus_dust), (0, 100));
    }

    #[test]
    fn the_wager_deal_takes_nothing_by_default_and_not_now_changes_nothing() {
        let mut g = game("risk_deal");
        g.open_wager_deal();
        assert_eq!(g.choice_view.selected, 1);
        assert_eq!(g.choice_view.title, gamble::WAGER_TITLE);
        assert_eq!(g.choice_view.subtitle, "harder dream, double reward");
        g.choose(1);
        assert!(!g.wager_enter && !g.gamble.wager_next);
        assert_eq!((g.mode, g.choice.clone()), (hud::Mode::Playing, None));
        g.open_wager_deal();
        g.choose(0);
        assert!(g.wager_enter, "the door is entered next frame");
    }

    #[test]
    fn a_wagered_dream_is_harder_pays_double_and_owes_a_rare_pick() {
        let mut g = game("risk_wagered");
        g.director.depth = 5;
        g.pressure_applies(); // normal dream
        let base = g.pressure_base().enemies;
        let plain = g.hazard_rule_state().map(|r| r.strength);
        g.gamble.take_wager();
        g.gamble.begin_dream();
        assert!(g.gamble.wager_active);
        assert!((g.pressure_base().enemies - base * 1.3).abs() < 1e-4);
        if let (Some(a), Some(b)) = (plain, g.hazard_rule_state().map(|r| r.strength)) {
            assert!(b > a, "stronger hazards");
        }
        // Earn 30 in it: leaving pays 30 more.
        g.wager_dust_start = 10;
        g.bonus_dust = 40;
        g.bonus_dust = gamble::doubled(g.wager_dust_start, g.bonus_dust);
        g.gamble.finish_dream();
        assert_eq!(g.bonus_dust, 70);
        assert!(g.gamble.pick_owed);
        // The next ordinary pick closes into the extra rare+ one.
        g.director.depth = 7;
        g.open_upgrade_choice();
        g.choose(0);
        let Some(ChoiceKind::Upgrade(opts)) = g.choice.clone() else {
            panic!("the extra pick");
        };
        assert!(opts.iter().all(|u| u.tier() >= upgrades::Tier::Rare), "rare or better: {opts:?}");
        assert!(!g.gamble.pick_owed);
        g.choose(0);
        assert!(!matches!(g.choice, Some(ChoiceKind::Upgrade(_))), "only one extra");
    }

    #[test]
    fn nothing_is_wagered_in_a_plain_dream() {
        let g = game("risk_plain");
        assert!(!g.gamble.wager_active && !g.gamble.pick_owed);
        assert_eq!(gamble::enemy_mult(g.gamble.wager_active), 1.0);
    }
    // === end risk ===

    #[test]
    fn every_third_depth_opens_the_shop_after_the_pick() {
        let mut g = game("shop_depth");
        g.director.depth = moth_shop::SHOP_EVERY;
        g.open_upgrade_choice();
        g.choose(0);
        assert_eq!(g.choice, Some(ChoiceKind::MothShop));
        assert_eq!(g.mode, hud::Mode::Choice);
    }

    #[test]
    fn buying_and_rerolling_spend_run_dust_only() {
        let mut g = game("shop_buy");
        g.director.depth = 6;
        g.booklet.stash.dust = 500;
        g.bonus_dust = 100;
        g.enter_moth_shop_for_test();
        let shelf = g.moth_stock.clone().unwrap();
        let cost = moth_shop::price_at(&shelf, 0, g.moth_terms()).unwrap();
        let card = shelf.cards[0];
        g.choose(0);
        assert_eq!(g.bonus_dust, 100 - cost);
        assert_eq!(g.booklet.stash.dust, 500, "the bank is untouched");
        assert_eq!(g.run.count(card), 1);
        assert_eq!(g.moth_stock.as_ref().unwrap().cards.len(), shelf.cards.len() - 1);
        // Reroll is the card after the last one (and the R key).
        let left = g.bonus_dust;
        let n = g.moth_stock.as_ref().unwrap().cards.len();
        g.choose(n);
        assert_eq!(g.bonus_dust, left - moth_shop::reroll_price(0));
        assert_eq!(g.moth_stock.as_ref().unwrap().rerolls, 1);
        assert_eq!(g.choice, Some(ChoiceKind::MothShop));
        // Too poor: nothing changes.
        g.bonus_dust = 0;
        let before = g.moth_stock.clone();
        let n = before.as_ref().unwrap().cards.len();
        g.choose(n);
        assert_eq!((g.bonus_dust, g.moth_stock.clone()), (0, before));
    }

    #[test]
    fn perks_change_the_run_the_way_they_say() {
        let mut g = game("shop_perks");
        g.perks = vec![store::Perk::Shrug, store::Perk::Haggler, store::Perk::DeepShelves];
        g.director.depth = 2;
        g.open_upgrade_choice();
        g.skip_choice();
        assert_eq!(g.bonus_dust, upgrades::SKIP_DUST + store::SHRUG_DUST);
        let t = g.moth_terms();
        assert_eq!((t.pct_off, t.extra_stock), (store::HAGGLER_PCT, 1));
        assert_eq!(g.perk_fx().nightmare_mult, 1);
        g.perks = vec![store::Perk::Bounty];
        assert_eq!(g.perk_fx().nightmare_mult, store::BOUNTY_MULT);
    }

    #[test]
    fn the_big_store_draws_every_shelf_at_every_cursor() {
        let mut g = game("shop_screens");
        g.mode = hud::Mode::Store;
        for shelf in 0..store::Shelf::ALL.len() {
            for col in 0..store::Shelf::ALL[shelf].items().len() {
                g.shop_shelf = shelf;
                g.shop_col = col;
                assert!(shapes(&g) > 0);
            }
        }
    }
    #[test]
    fn the_fullest_moth_shelf_fits_the_screen() {
        let mut g = game("shop_fit");
        g.director.depth = 9;
        g.bonus_dust = 40;
        g.perks = vec![store::Perk::DeepShelves];
        g.enter_moth_shop_for_test();
        assert_eq!(g.mode, hud::Mode::Choice);
        let bad = overflows(&g, "MothShop");
        assert!(bad.is_empty(), "{bad:?}");
    }
    // === toolcards ===
    #[test]
    fn the_fullest_deck_run_shelf_with_tool_cards_fits_the_screen() {
        for bought in [0, 1, 2] {
            let mut g = deck_game("shop_fit_tools");
            g.run.deck_slots = 5;
            g.director.depth = 9;
            g.bonus_dust = 80;
            g.perks = vec![store::Perk::DeepShelves];
            g.enter_moth_shop_for_test();
            let at = g.moth_stock.as_ref().unwrap().cards.len();
            for k in 0..bought {
                g.choose(at + k);
            }
            assert_eq!(g.tool_cards_held().len(), bought);
            let bad = overflows(&g, "MothShop+tools");
            assert!(bad.is_empty(), "{bought} bought: {bad:?}");
        }
    }
    // === end toolcards ===
    // === end shop ===

    #[test]
    fn audit_text_fits_every_screen() {
        use dream::DreamTheme;
        let mut g = game("audit");
        let records: Vec<_> = dream::ALL_THEMES
            .iter()
            .enumerate()
            .map(|(i, &t)| cards::tests::record(t, 40 + i as u64, 1 + i as u32, 0.3, i % 2 == 0))
            .collect();
        g.booklet.add_run(1, &records);
        g.booklet.stash.earn(5000);
        let _ = DreamTheme::Lobby;
        let mut bad = Vec::new();
        // === ascension === the title picker at every level
        g.booklet.codex.ascension_unlocked = progress::MAX_ASCENSION;
        g.mode = hud::Mode::Title;
        for l in 0..=progress::MAX_ASCENSION {
            g.ascension = l;
            bad.extend(overflows(&g, &format!("Title asc {l}")));
        }
        g.ascension = 0;
        // === end ascension ===
        for mode in [
            hud::Mode::Warning,
            hud::Mode::Title,
            hud::Mode::Summary,
            hud::Mode::Code,
            hud::Mode::Lobby,
            hud::Mode::Ending,
            hud::Mode::Paused,
            hud::Mode::Playing,
        ] {
            g.mode = mode;
            bad.extend(overflows(&g, &format!("{mode:?}")));
        }
        // === pressure ===
        g.pressure_collapsed = true;
        g.mode = hud::Mode::Summary;
        bad.extend(overflows(&g, "Summary-collapsed"));
        g.pressure_collapsed = false;
        g.mode = hud::Mode::Playing;
        for warn in [false, true] {
            bad.extend(overflows_with(&g, "HUD-pressure", &|v| {
                v.pressure_lid = Some((0.6, warn));
            }));
        }
        // === end pressure ===
        g.open_settings();
        bad.extend(overflows(&g, "Settings"));
        g.open_codex();
        bad.extend(overflows(&g, "Codex"));
        g.open_booklet();
        bad.extend(overflows(&g, "Booklet"));
        g.open_loadout();
        bad.extend(overflows(&g, "Loadout"));
        g.open_merge(hud::Mode::Title);
        bad.extend(overflows(&g, "Merge"));
        g.open_store();
        bad.extend(overflows(&g, "Store"));
        g.store_tab = 1;
        bad.extend(overflows(&g, "Lottery"));
        g.store_tab = 2; // === fortune ===
        bad.extend(overflows(&g, "Tickets"));
        g.ticket_confirm = true;
        bad.extend(overflows(&g, "Tickets-confirm"));
        g.store_tab = 1;
        g.begin_pick(offers(3));
        bad.extend(overflows(&g, "PackPick"));
        g.pick.age = 3.0;
        bad.extend(overflows(&g, "PackPick-late"));
        g.mode = hud::Mode::Memories;
        bad.extend(overflows(&g, "Memories"));
        g.mode = hud::Mode::Journal;
        bad.extend(overflows(&g, "Journal"));
        for id in events::ALL {
            g.show_event(id);
            bad.extend(overflows(&g, &format!("Event {id:?}")));
        }
        g.open_wake_choice();
        bad.extend(overflows(&g, "WakeChoice"));
        // === risk === the moth's coin and the second door
        g.bonus_dust = 80;
        g.gamble_visit = 0;
        g.director.depth = 6;
        g.enter_moth_shop_for_test();
        bad.extend(overflows(&g, "MothShop+coin"));
        g.open_gamble_stakes();
        bad.extend(overflows(&g, "Coin-stakes"));
        g.open_gamble_confirm(20);
        bad.extend(overflows(&g, "Coin-confirm"));
        g.flip_coin(20);
        bad.extend(overflows(&g, "Coin-result"));
        g.gamble.losses = gamble::GAMBLE_LOSS_CAP;
        g.open_gamble_stakes();
        bad.extend(overflows(&g, "Coin-owed"));
        g.open_wager_deal();
        bad.extend(overflows(&g, "Wager-deal"));
        g.choice = None;
        g.gamble = gamble::RunState::default();
        g.bonus_dust = 0;
        g.gamble_visit = 0;
        // === end risk ===
        for depth in [1, 6, 12] {
            g.director.depth = depth;
            g.open_upgrade_choice();
            bad.extend(overflows(&g, &format!("Upgrades@{depth}")));
        }
        // === pacts ===
        for p in pacts::ALL {
            g.pacts.clear();
            g.director.depth = 6;
            g.open_upgrade_choice();
            g.pact_offer = Some(p);
            let mut cards = g.choice_view.cards.clone();
            cards.pop();
            cards.push(upgrade_ui::ChoiceCard {
                name: p.name().into(),
                desc: p.desc().into(),
                icon: upgrades::Icon::Lid,
                color: [255, 90, 100],
                frame: [255, 60, 80],
                tag: format!("pact · +{:.2} dust", p.bonus()),
            });
            g.choice_view.cards = cards;
            bad.extend(overflows(&g, &format!("Pact {p:?}")));
        }
        g.pacts = pacts::ALL.to_vec();
        g.mode = hud::Mode::Summary;
        bad.extend(overflows(&g, "Summary-pacts"));
        g.pacts.clear();
        g.pact_offer = None;
        // === end pacts ===
        g.open_pack();
        g.pack.age = f32::MAX;
        bad.extend(overflows(&g, "Reveal"));
        g.begin_run();
        g.mode = hud::Mode::Playing;
        bad.extend(overflows(&g, "Run"));
        // The in-game HUD with long lines on it.
        let long = |n: usize| "the quick brown dream jumps over lazy sleepers ".repeat(4)[..n].to_string();
        g.mode = hud::Mode::Playing;
        bad.extend(overflows_with(&g, "HUD-long", &|v| {
            v.title = long(28);
            v.whisper = long(60);
            v.entrance_age = f32::MAX;
            v.eye_line = Some((long(70), 1.5));
            v.coop_line = Some(long(50));
            v.objective = Some(long(50));
            v.goal = long(60);
            v.twist = long(20);
            v.twist_blurb = long(50);
            v.hint = long(60);
            v.boss = Some((long(28), 3, 5, Some("A WARNING THAT IS QUITE LONG INDEED")));
            v.upgrades = (0..12).map(|i| format!("{} {i}", long(18))).collect();
        }));
        bad.extend(overflows_with(&g, "Toast-long", &|v| {
            v.entrance_age = f32::MAX;
            v.achievement = Some((long(32), 1.0));
        }));
        g.mode = hud::Mode::Note;
        bad.extend(overflows_with(&g, "Note-long", &|v| {
            v.note = Some((long(30), long(40), long(140)));
        }));
        g.mode = hud::Mode::Store;
        g.store_tab = 0;
        bad.extend(overflows_with(&g, "Store-long", &|v| {
            v.store_status = Some(long(70));
        }));
        g.mode = hud::Mode::Booklet;
        bad.extend(overflows_with(&g, "Booklet-long", &|v| {
            v.booklet_attune = long(80);
        }));
        g.mode = hud::Mode::Title;
        bad.extend(overflows_with(&g, "Title-long", &|v| {
            v.goal = long(60);
            v.perks = vec!["lantern oil", "second wind", "a long spare key", "fog", "chalk", "salt"];
            v.menu = (0..10)
                .map(|i| ("k".to_string(), format!("menu item number {i}"), long(40)))
                .collect();
        }));
        // Animated screens at other moments of their run.
        for t in [0.5, 2.0, 6.0, 15.0, 40.0, 90.0, 200.0] {
            g.time = t;
            g.ending_age = t;
            g.title_age = t;
            g.warning_age = t;
            for mode in [hud::Mode::Ending, hud::Mode::Title, hud::Mode::Warning, hud::Mode::Summary] {
                g.mode = mode;
                bad.extend(overflows(&g, &format!("{mode:?}@{t}")));
            }
        }
        bad.sort();
        bad.dedup();
        for b in &bad {
            eprintln!("{b}");
        }
        assert!(bad.is_empty(), "{} text overflows", bad.len());
    }

    #[test]
    fn the_pick_screen_shows_the_real_odds() {
        // Card counts: the same numbers `packs::open` rolls with.
        assert_eq!(reveal_ui::take_lines(1), vec!["1 CARD", "100%"]);
        assert_eq!(reveal_ui::take_lines(2), vec!["1-2 CARDS", "67% / 33%"]);
        assert_eq!(reveal_ui::take_lines(5), vec!["1-3 CARDS", "60% / 30% / 10%"]);
        // Memory chances, as a range of the real `memory_chance` values.
        assert_eq!(reveal_ui::memory_line(&[0.6, 0.4]).as_deref(), Some("REMEMBERED 40-60% EACH"));
        assert_eq!(reveal_ui::memory_line(&[0.43, 0.43]).as_deref(), Some("REMEMBERED 43% EACH"));
        assert_eq!(reveal_ui::memory_line(&[]), None);
    }
}
