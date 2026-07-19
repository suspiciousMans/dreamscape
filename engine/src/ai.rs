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

/// Lines of dialogue a `Friendly` character cycles through on `interact()`.
/// `lines` is the authored part (round-tripped like `Health.max`); `index`
/// is live state that resets whenever the level is (re)loaded.
pub struct Dialogue {
    pub lines: Vec<String>,
    index: usize,
}

impl Dialogue {
    pub fn new(lines: Vec<String>) -> Self {
        Self { lines, index: 0 }
    }

    /// Returns the next line and advances, wrapping around. Empty string if
    /// there are no lines — shouldn't happen in practice, a `Dialogue` is
    /// only ever attached to a character authored with at least one.
    pub fn next(&mut self) -> &str {
        if self.lines.is_empty() {
            return "";
        }
        let line = self.lines[self.index].as_str();
        self.index = (self.index + 1) % self.lines.len();
        line
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
