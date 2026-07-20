use std::path::PathBuf;

use glam::Vec3;
use hecs::Entity;
use serde::{Deserialize, Serialize};

use crate::ecs::Transform;
use crate::pathfinding::NavGrid;
use crate::physics::RigidBody;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Disposition {
    Passive,
    Hostile,
    Friendly,
}

/// Authored, unchanging tuning for a placed character. Fields with no
/// separate live counterpart (`damage`/`attack_range`/etc. never change
/// during play, unlike `Health.current` or `Dialogue`'s cycling index) live
/// directly here rather than behind a `LevelObjectMeta`-style mirror struct
/// — the same shape as `engine::particles::ParticleEmitter`'s `def` field.
/// Round-tripped by `build_level_from_ecs` into
/// `engine::level::CharacterInstance` alongside `Health`/`Dialogue`.
#[derive(Clone, Debug)]
pub struct CharacterMeta {
    pub name: String,
    pub color: [f32; 3],
    /// A texture applied to the character's cube in place of the flat
    /// `color` fill, if assigned — `color` stays meaningful even then, as
    /// the fallback used while the texture loads/if it fails to load.
    pub texture_path: Option<PathBuf>,
    pub disposition: Disposition,
    pub move_speed: f32,
    /// Radius (world units) a `Wandering`/idle character roams from its
    /// spawn point. `0.0` means stationary until perception moves it
    /// (chasing/fleeing still work; it just never wanders on its own).
    pub wander_radius: f32,
    pub sight_range: f32,
    /// `None` means this character never attacks — a `Passive` critter, or
    /// a `Hostile`/`Friendly` one authored without combat.
    pub damage: Option<f32>,
    pub attack_range: f32,
    pub attack_cooldown_secs: f32,
}

/// Current/max hit points — used for the player as well as characters (see
/// `Sandbox::enter_play_mode`). `max` is the authored value; `current` is
/// live state, so `build_level_from_ecs` only ever saves `max` (a
/// (re)loaded character always starts at full health).
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    pub fn damage(&mut self, amount: f32) {
        self.current = (self.current - amount).max(0.0);
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }

    pub fn fraction(&self) -> f32 {
        if self.max > 0.0 {
            (self.current / self.max).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// One line of dialogue plus optional branches: `choices` is a list of
/// `(button label, target node index)` pairs. An empty `choices` list
/// means "linear" — the line just auto-advances to the next node on the
/// next `Dialogue::advance()` call, so a flat conversation (the common
/// case) is simply every node having no choices, with no special-casing
/// needed anywhere that reads a `Dialogue`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DialogueNode {
    pub text: String,
    #[serde(default)]
    pub choices: Vec<(String, usize)>,
}

/// A branching conversation a `Friendly` character cycles/navigates
/// through on `interact()`. `nodes` is the authored part (round-tripped
/// like `Health.max`); `current` is live state that resets whenever the
/// level is (re)loaded.
pub struct Dialogue {
    pub nodes: Vec<DialogueNode>,
    current: usize,
}

impl Dialogue {
    pub fn new(nodes: Vec<DialogueNode>) -> Self {
        Self { nodes, current: 0 }
    }

    /// The line to show for wherever the conversation currently is.
    /// Empty string if there are no nodes — shouldn't happen in practice,
    /// a `Dialogue` is only ever attached to a character authored with at
    /// least one.
    pub fn current_text(&self) -> &str {
        self.nodes.get(self.current).map(|node| node.text.as_str()).unwrap_or("")
    }

    /// The current node's choices, if any — an empty slice means the
    /// conversation is linear at this point (`advance()` is what moves it
    /// forward, not `choose()`).
    pub fn current_choices(&self) -> &[(String, usize)] {
        self.nodes.get(self.current).map(|node| node.choices.as_slice()).unwrap_or(&[])
    }

    /// Moves to the next node, wrapping around. A no-op if the current
    /// node has choices — the caller is expected to call `choose` instead
    /// once the player has picked one (checked via `current_choices`).
    pub fn advance(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        if self.nodes[self.current].choices.is_empty() {
            self.current = (self.current + 1) % self.nodes.len();
        }
    }

    /// Jumps to the target node of the current node's `index`-th choice.
    /// Returns `false` (no-op) if `index` or the target is out of range.
    pub fn choose(&mut self, index: usize) -> bool {
        let Some(node) = self.nodes.get(self.current) else { return false };
        let Some(&(_, target)) = node.choices.get(index) else { return false };
        if target >= self.nodes.len() {
            return false;
        }
        self.current = target;
        true
    }
}

#[derive(Clone, Debug)]
enum BrainState {
    Idle { timer: f32 },
    Wandering { path: Vec<Vec3>, index: usize },
    Chasing { path: Vec<Vec3>, index: usize },
    Fleeing { path: Vec<Vec3>, index: usize },
    Attacking { cooldown: f32 },
}

#[derive(Clone, Copy, PartialEq)]
enum BrainKind {
    Idle,
    Wandering,
    Chasing,
    Fleeing,
    Attacking,
}

impl BrainState {
    fn kind(&self) -> BrainKind {
        match self {
            BrainState::Idle { .. } => BrainKind::Idle,
            BrainState::Wandering { .. } => BrainKind::Wandering,
            BrainState::Chasing { .. } => BrainKind::Chasing,
            BrainState::Fleeing { .. } => BrainKind::Fleeing,
            BrainState::Attacking { .. } => BrainKind::Attacking,
        }
    }
}

/// A tiny xorshift32 generator for wander-target variety — the same
/// dependency-free approach as `engine::particles`'s private `Rng`, kept as
/// its own copy since that one isn't exported.
struct Rng(u32);

impl Rng {
    fn seeded() -> Self {
        use std::hash::{BuildHasher, Hasher};
        let seed = std::hash::RandomState::new().build_hasher().finish() as u32;
        Self(seed | 1) // xorshift32 never recovers from a zero state
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        let unit = self.next_u32() as f32 / u32::MAX as f32;
        min + (max - min) * unit
    }
}

/// Runtime AI state — a small state machine driven by perception each
/// `step`. `Idle`/`Wandering` apply to every disposition (ambient life when
/// the player isn't nearby); `Chasing`/`Attacking` are only ever entered by
/// `Hostile` characters, `Fleeing` only by `Passive` ones. `Friendly`
/// characters notice the player but never move toward/away from them —
/// talking is driven by `interact()`, not perception. Kept separate from
/// `CharacterMeta` so saving a level never freezes a copy of "currently
/// mid-chase" state.
pub struct CharacterBrain {
    home: Vec3,
    state: BrainState,
    repath_timer: f32,
    rng: Rng,
}

impl CharacterBrain {
    pub fn new(home: Vec3) -> Self {
        Self { home, state: BrainState::Idle { timer: 0.0 }, repath_timer: 0.0, rng: Rng::seeded() }
    }
}

/// Something `step` noticed this frame that the game needs to react to —
/// mirrors `engine::physics::step`'s "return events, let the caller handle
/// them" contract rather than reaching into `HudState`/`ScreenEffectState`/
/// etc. directly from inside this module.
pub enum AiEvent {
    AttackedPlayer { entity: Entity, damage: f32 },
    CharacterDied { entity: Entity, position: Vec3 },
}

/// How much farther than `sight_range` a character keeps tracking the
/// player before giving up — avoids state flicker for a target sitting
/// right at the perception boundary.
const LOSE_TRACK_MULTIPLIER: f32 = 1.5;
const REPATH_INTERVAL_SECS: f32 = 1.0;
const WAYPOINT_ARRIVAL_RADIUS: f32 = 0.35;
const IDLE_DWELL_SECS: f32 = 2.0;
const FLEE_DISTANCE: f32 = 4.0;

/// Advances every character's perception/state machine/steering by `dt`
/// (writing horizontal `RigidBody.velocity` — physics' existing gravity/
/// collision resolution handles the rest, same as the player), then scans
/// every `Health` (character or player) for `is_dead()`. Actual despawn is
/// the caller's job, mirroring `engine::particles::step`'s "returns
/// entities to despawn" contract, since the caller also needs the death
/// position for feedback before the entity is gone.
///
/// Perception is a plain distance check against `player_position`, not a
/// line-of-sight raycast — a stated simplification, matching the
/// Glow-mushroom-darkness precedent elsewhere in this codebase.
pub fn step(world: &mut hecs::World, dt: f32, player_position: Vec3, nav_grid: &NavGrid) -> Vec<AiEvent> {
    let mut events = Vec::new();

    for (entity, (transform, body, meta, brain)) in
        world.query::<(&Transform, &mut RigidBody, &CharacterMeta, &mut CharacterBrain)>().iter()
    {
        let position = transform.position;
        let distance_to_player = position.distance(player_position);
        let lose_track_range = meta.sight_range * LOSE_TRACK_MULTIPLIER;
        let current_kind = brain.state.kind();

        brain.repath_timer -= dt;

        match meta.disposition {
            Disposition::Hostile => {
                let in_attack_range = meta.damage.is_some() && distance_to_player <= meta.attack_range;
                if current_kind == BrainKind::Attacking {
                    if distance_to_player > meta.attack_range {
                        brain.state = BrainState::Chasing { path: Vec::new(), index: 0 };
                        brain.repath_timer = 0.0;
                    }
                } else if in_attack_range {
                    brain.state = BrainState::Attacking { cooldown: 0.0 };
                } else if current_kind == BrainKind::Chasing {
                    if distance_to_player > lose_track_range {
                        brain.state = BrainState::Idle { timer: 0.0 };
                    }
                } else if distance_to_player <= meta.sight_range {
                    brain.state = BrainState::Chasing { path: Vec::new(), index: 0 };
                    brain.repath_timer = 0.0;
                }
            }
            Disposition::Passive => {
                if current_kind == BrainKind::Fleeing {
                    if distance_to_player > lose_track_range {
                        brain.state = BrainState::Idle { timer: 0.0 };
                    }
                } else if distance_to_player <= meta.sight_range {
                    brain.state = BrainState::Fleeing { path: Vec::new(), index: 0 };
                    brain.repath_timer = 0.0;
                }
            }
            Disposition::Friendly => {}
        }

        if let BrainState::Idle { timer } = &brain.state {
            let remaining = *timer - dt;
            if remaining <= 0.0 && meta.wander_radius > 0.0 {
                let angle = brain.rng.range(0.0, std::f32::consts::TAU);
                let radius = brain.rng.range(0.0, meta.wander_radius);
                let target = brain.home + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
                brain.state = match nav_grid.find_path(position, target) {
                    Some(path) => BrainState::Wandering { path, index: 0 },
                    None => BrainState::Idle { timer: IDLE_DWELL_SECS },
                };
            } else {
                brain.state = BrainState::Idle { timer: remaining };
            }
        }

        if brain.repath_timer <= 0.0 {
            match brain.state.kind() {
                BrainKind::Chasing => {
                    brain.repath_timer = REPATH_INTERVAL_SECS;
                    if let Some(path) = nav_grid.find_path(position, player_position) {
                        brain.state = BrainState::Chasing { path, index: 0 };
                    }
                }
                BrainKind::Fleeing => {
                    brain.repath_timer = REPATH_INTERVAL_SECS;
                    let away = (position - player_position).normalize_or_zero();
                    let away = if away == Vec3::ZERO { Vec3::X } else { away };
                    let target = position + away * FLEE_DISTANCE;
                    if let Some(path) = nav_grid.find_path(position, target) {
                        brain.state = BrainState::Fleeing { path, index: 0 };
                    }
                }
                _ => {}
            }
        }

        let mut horizontal_velocity = Vec3::ZERO;
        match &mut brain.state {
            BrainState::Wandering { path, index }
            | BrainState::Chasing { path, index }
            | BrainState::Fleeing { path, index } => {
                if let Some(&waypoint) = path.get(*index) {
                    let mut to_waypoint = waypoint - position;
                    to_waypoint.y = 0.0;
                    if to_waypoint.length() < WAYPOINT_ARRIVAL_RADIUS {
                        *index += 1;
                    } else {
                        horizontal_velocity = to_waypoint.normalize_or_zero() * meta.move_speed;
                    }
                }
            }
            BrainState::Idle { .. } | BrainState::Attacking { .. } => {}
        }
        body.velocity.x = horizontal_velocity.x;
        body.velocity.z = horizontal_velocity.z;

        let wandering_finished =
            matches!(&brain.state, BrainState::Wandering { path, index } if *index >= path.len());
        if wandering_finished {
            brain.state = BrainState::Idle { timer: IDLE_DWELL_SECS };
        }

        if let BrainState::Attacking { cooldown } = &mut brain.state {
            *cooldown -= dt;
            if *cooldown <= 0.0 {
                if let Some(damage) = meta.damage {
                    events.push(AiEvent::AttackedPlayer { entity, damage });
                }
                *cooldown = meta.attack_cooldown_secs.max(0.1);
            }
        }
    }

    for (entity, (transform, health)) in world.query::<(&Transform, &Health)>().iter() {
        if health.is_dead() {
            events.push(AiEvent::CharacterDied { entity, position: transform.position });
        }
    }

    events
}

/// Authored config for a periodic character spawner — a `Transform`-only
/// entity (like `LevelLight`/`LevelParticleEmitter`) that emits
/// `SpawnRequest`s for the game to fulfill via `Sandbox::spawn_character`.
/// This module has no `gl` access, so — like `step`'s own `AiEvent` —
/// spawning is something the caller does, not this system itself.
#[derive(Clone, Debug)]
pub struct SpawnerConfig {
    pub name: String,
    /// Only `disposition`/tuning/combat/dialogue fields are used; the
    /// template's own `position` is ignored — each spawn's position is the
    /// spawner's own `Transform.position` plus a random offset within
    /// `spawn_radius`.
    pub template: crate::level::CharacterInstance,
    pub spawn_interval_secs: f32,
    pub max_alive: u32,
    /// `None` means unlimited (bounded only by `max_alive` at any one time).
    pub total_to_spawn: Option<u32>,
    pub spawn_radius: f32,
}

/// Runtime state — never serialized (see `engine::level::SpawnerInstance`
/// for the authored/round-trippable half `Sandbox::spawn_spawner` builds
/// this alongside).
pub struct SpawnerState {
    elapsed_since_last: f32,
    spawned_count: u32,
    alive: Vec<Entity>,
}

impl SpawnerState {
    pub fn new() -> Self {
        Self { elapsed_since_last: 0.0, spawned_count: 0, alive: Vec::new() }
    }

    /// Called by the game right after `spawn_character` fulfills a
    /// `SpawnRequest` from this spawner, so `step_spawners` can track it
    /// against `max_alive`.
    pub fn track_spawned(&mut self, entity: Entity) {
        self.alive.push(entity);
    }
}

impl Default for SpawnerState {
    fn default() -> Self {
        Self::new()
    }
}

/// A spawner's ask, for the frame's caller to fulfill (mirrors `AiEvent` —
/// see `step`'s doc comment for why this system can't spawn directly).
pub struct SpawnRequest {
    pub spawner: Entity,
    pub instance: crate::level::CharacterInstance,
}

/// Ticks every `SpawnerConfig`, emitting a `SpawnRequest` once its interval
/// elapses and it's still under both `max_alive` (checked against
/// `SpawnerState::alive`, pruned of anything since despawned) and
/// `total_to_spawn`. The timer doesn't accumulate while capped, so a
/// spawner that's been at its cap for a while doesn't burst out a pile of
/// spawns the instant a slot frees up.
pub fn step_spawners(world: &mut hecs::World, dt: f32) -> Vec<SpawnRequest> {
    let mut requests = Vec::new();
    let mut rng = Rng::seeded();

    for (spawner_entity, (transform, config, state)) in
        world.query::<(&Transform, &SpawnerConfig, &mut SpawnerState)>().iter()
    {
        state.alive.retain(|&entity| world.contains(entity));

        if state.alive.len() as u32 >= config.max_alive {
            continue;
        }
        if let Some(total) = config.total_to_spawn {
            if state.spawned_count >= total {
                continue;
            }
        }

        state.elapsed_since_last += dt;
        if state.elapsed_since_last < config.spawn_interval_secs {
            continue;
        }
        state.elapsed_since_last = 0.0;
        state.spawned_count += 1;

        let angle = rng.range(0.0, std::f32::consts::TAU);
        let radius = rng.range(0.0, config.spawn_radius.max(0.0));
        let offset = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        let position = transform.position + offset;

        let mut instance = config.template.clone();
        instance.position = position.to_array();
        requests.push(SpawnRequest { spawner: spawner_entity, instance });
    }

    requests
}
