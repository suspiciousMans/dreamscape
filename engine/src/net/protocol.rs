use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::level::{CharacterInstance, Level};
use crate::particles::ParticleEmitterDef;

/// A host-assigned identity for a replicated entity — independent of
/// `hecs::Entity`, which is purely local to one process (index+generation,
/// recycled on despawn, not meaningfully comparable across peers). Every
/// player, `CharacterMeta` character, and dynamic physics prop gets one;
/// static level geometry never does, since `Level` (sent whole in
/// `ServerMessage::Welcome`/`LevelTransition`) plus the existing
/// `apply_level` already reconstructs it identically on every peer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetId(pub u32);

/// Bumped whenever `ClientMessage`/`ServerMessage`'s wire shape changes.
/// `bincode`'s encoding is positional (unlike `Level`/`SaveData`'s RON +
/// `#[serde(default)]`, which tolerate missing fields by name) — there is
/// no attempt at cross-version compatibility, so `Hello`/`Welcome` require
/// an exact match and reject otherwise.
pub const PROTOCOL_VERSION: u32 = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Hello {
        player_name: String,
        protocol_version: u32,
        /// The joining player's chosen player-model rig, as a path
        /// relative to `asset_root` (e.g. `Some("rigs/my_character.ron")`),
        /// or `None` for the default appearance. Carried at handshake time
        /// so the host and every already-connected peer can build this
        /// player's proxy correctly from the moment it appears, without a
        /// separate round-trip. See `ServerMessage::PlayerJoined` and
        /// `SetAppearance` for how this then propagates.
        appearance: Option<PathBuf>,
    },
    /// Sent every client frame (cheap, no throttling) — `seq` is currently
    /// unused by the host (no client-side prediction/reconciliation to
    /// reorder against, see the plan's stated simplifications) but is
    /// carried so a future revision can add that without a wire change.
    /// Interacting (the `E` key) is a one-shot edge, not continuous held
    /// state like movement, so it's its own message below rather than a
    /// field here.
    Input {
        seq: u32,
        move_dir: [f32; 2],
        yaw_deg: f32,
        pitch_deg: f32,
        jump: bool,
    },
    /// Sent once when the client presses `E` near something interactable
    /// — `target` is the nearest interactable `NetId` the client can see
    /// in its own locally-mirrored snapshot state, the same "nearest
    /// target in range" search `interact()` already does locally in
    /// single-player, just resolved against replicated positions instead.
    Interact {
        target: NetId,
    },
    /// Sent when the client presses a number key while its own
    /// `ServerMessage::Dialogue`-driven choice overlay is showing —
    /// `speaker` names which `Dialogue` (there can only sensibly be one
    /// active at a time client-side, but the host is stateless per
    /// request) to advance, `index` the 0-based choice picked.
    DialogueChoice {
        speaker: NetId,
        index: usize,
    },
    /// Sent when a connected client changes their player-model appearance
    /// mid-session (see `engine::ui::AppearanceAction`). The host relays
    /// this out to every peer as `ServerMessage::PlayerAppearanceChanged`
    /// so everyone rebuilds that player's proxy with the new model — see
    /// the same field's doc comment on `Hello` for the path convention.
    SetAppearance {
        rig_path: Option<PathBuf>,
    },
    Disconnect,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome {
        your_net_id: NetId,
        level: Level,
        /// Matched against `CharacterMeta`/`LevelObjectMeta` names after
        /// the client's own `apply_level` runs, to tag the entities it
        /// just spawned with the right `NetId` — see the plan's "Entity
        /// replication model" section for why this is name-keyed rather
        /// than positional.
        named_net_ids: Vec<(String, NetId)>,
        tick_rate: f32,
    },
    Reject {
        reason: String,
    },
    /// Sent in three situations, all covered by this one shape: to a
    /// newly-joined client about the host's own player, to a newly-joined
    /// client about every already-connected peer (a catch-up loop), and
    /// to every already-connected peer about the new joiner. `appearance`
    /// is that player's currently-chosen rig path (see `ClientMessage::
    /// Hello`), letting every recipient build the right proxy immediately
    /// instead of defaulting to the gray-cube fallback and correcting it
    /// later.
    PlayerJoined {
        net_id: NetId,
        name: String,
        appearance: Option<PathBuf>,
    },
    PlayerLeft {
        net_id: NetId,
    },
    /// One-shot broadcast for a player (client or host) changing their
    /// appearance mid-session — every peer, on receipt, despawns that
    /// `NetId`'s current proxy (root + any rig-part children) and rebuilds
    /// it in place with the new appearance, falling back to the default
    /// cube exactly as `PlayerJoined`/initial spawn would on any
    /// resolution failure.
    PlayerAppearanceChanged {
        net_id: NetId,
        appearance: Option<PathBuf>,
    },
    CharacterSpawned {
        net_id: NetId,
        instance: CharacterInstance,
    },
    Snapshot {
        tick: u32,
        players: Vec<PlayerSnapshot>,
        characters: Vec<CharacterSnapshot>,
        objects: Vec<ObjectSnapshot>,
        removed: Vec<NetId>,
    },
    Dialogue {
        speaker: NetId,
        text: String,
        choices: Vec<(String, usize)>,
    },
    DialogueClosed,
    LevelTransition {
        level: Level,
        spawn_position: [f32; 3],
        spawn_yaw_deg: f32,
        /// Same purpose as `Welcome`'s field of the same name — the new
        /// level's characters/objects need fresh `NetId`s too, since
        /// `apply_level` just replaced all of them.
        named_net_ids: Vec<(String, NetId)>,
    },
    /// A purely cosmetic particle burst (a pushed object, a hit
    /// character, a death, ...) — these have no `Networked` entity of
    /// their own to ride along on a `Snapshot`, so they're broadcast as
    /// their own one-shot event instead, mirroring `Dialogue`'s shape.
    ParticleBurst {
        position: [f32; 3],
        def: ParticleEmitterDef,
        count: u32,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub net_id: NetId,
    pub position: [f32; 3],
    pub yaw_deg: f32,
    pub health: Option<(f32, f32)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterSnapshot {
    pub net_id: NetId,
    pub position: [f32; 3],
    pub health: Option<(f32, f32)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectSnapshot {
    pub net_id: NetId,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
}
