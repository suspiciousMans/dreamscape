mod framing;
mod protocol;

use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub use protocol::{
    CharacterSnapshot, ClientMessage, NetId, ObjectSnapshot, PlayerSnapshot, ServerMessage,
    PROTOCOL_VERSION,
};

use framing::NetConnection;

pub const DEFAULT_PORT: u16 = 7777;
/// Total players including the host — a `NetHost` accepts at most
/// `MAX_PLAYERS - 1` remote connections.
pub const MAX_PLAYERS: usize = 4;
pub const SNAPSHOT_RATE_HZ: f32 = 20.0;
/// How long an accepted-but-not-yet-`Hello`'d connection may sit in
/// `NetHost::pending` before it's dropped. Without this, a peer that
/// connects and never speaks (a stray probe, a slowloris-style stall)
/// would occupy a pending slot and a file descriptor forever — it never
/// counts against `MAX_PLAYERS`, since that check only runs after `Hello`.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// One client's current input for a frame — built from `ctx.input` the
/// same way `update_player_input` already reads it locally, then either
/// applied directly (host, for its own player) or sent as
/// `ClientMessage::Input` (client) / received via `NetHost::poll_inputs`
/// (host, for remote players).
#[derive(Clone, Copy, Debug, Default)]
pub struct InputState {
    pub move_dir: [f32; 2],
    pub yaw_deg: f32,
    pub pitch_deg: f32,
    pub jump: bool,
}

struct PeerConnection {
    conn: NetConnection,
    name: String,
    appearance: Option<PathBuf>,
}

/// The authoritative side of a listen-server session — accepts
/// connections, receives `ClientMessage`s, and broadcasts
/// `ServerMessage`s. Every `poll_*`/`take_*` method does real socket I/O
/// on established connections and buffers what it finds; call
/// `poll_new_connections` first each frame (it's the one that also
/// accepts+pumps not-yet-`Hello`'d sockets), then the others to drain
/// what that pass filled — mirrors `HotReloadWatcher::poll_events`'s
/// "call once per frame, never blocks" shape.
pub struct NetHost {
    listener: TcpListener,
    /// Accepted but not yet past the `Hello` handshake, each stamped with
    /// the instant it was accepted so stale ones can be dropped after
    /// `HANDSHAKE_TIMEOUT`.
    pending: Vec<(Instant, NetConnection)>,
    connections: HashMap<NetId, PeerConnection>,
    next_id: u32,
    snapshot_accumulator: f32,
    pending_interacts: Vec<(NetId, NetId)>,
    pending_dialogue_choices: Vec<(NetId, NetId, usize)>,
    pending_appearance_changes: Vec<(NetId, Option<PathBuf>)>,
    pending_disconnected: Vec<NetId>,
}

impl NetHost {
    pub fn bind(port: u16) -> io::Result<Self> {
        let listener = TcpListener::bind(("0.0.0.0", port))?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            pending: Vec::new(),
            connections: HashMap::new(),
            next_id: 1,
            snapshot_accumulator: 0.0,
            pending_interacts: Vec::new(),
            pending_dialogue_choices: Vec::new(),
            pending_appearance_changes: Vec::new(),
            pending_disconnected: Vec::new(),
        })
    }

    /// Allocates a fresh `NetId` — called internally when a new connection
    /// completes its handshake, and by game code for the host's own local
    /// player and any pre-existing replicated entities (characters,
    /// dynamic props) when transitioning into hosting.
    pub fn allocate_net_id(&mut self) -> NetId {
        let id = NetId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Accepts every ready incoming TCP connection, gives every
    /// not-yet-`Hello`'d one a chance to speak, and returns `(net_id,
    /// player_name, appearance)` for each one that just completed the
    /// handshake this call. Rejects (and drops) a connection whose
    /// `protocol_version` doesn't match or that would exceed `MAX_PLAYERS`.
    pub fn poll_new_connections(&mut self) -> Vec<(NetId, String, Option<PathBuf>)> {
        loop {
            match self.listener.accept() {
                Ok((stream, _addr)) => match NetConnection::wrap(stream) {
                    Ok(conn) => self.pending.push((Instant::now(), conn)),
                    Err(err) => log::error!("failed to prepare an accepted connection: {err}"),
                },
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(err) => {
                    log::error!("listener accept error: {err}");
                    break;
                }
            }
        }

        let mut newly_joined = Vec::new();
        let pending = std::mem::take(&mut self.pending);
        for (accepted_at, mut conn) in pending {
            let messages: Vec<ClientMessage> = conn.pump();
            let hello = messages.into_iter().find_map(|msg| match msg {
                ClientMessage::Hello { player_name, protocol_version, appearance } => {
                    Some((player_name, protocol_version, appearance))
                }
                _ => None,
            });

            let Some((player_name, protocol_version, appearance)) = hello else {
                // Keep waiting on a still-connecting socket, but only until
                // the handshake deadline — a peer that connects and stays
                // silent is dropped instead of held forever.
                if !conn.is_disconnected() && accepted_at.elapsed() < HANDSHAKE_TIMEOUT {
                    self.pending.push((accepted_at, conn));
                }
                continue;
            };

            if protocol_version != PROTOCOL_VERSION {
                conn.send(&ServerMessage::Reject {
                    reason: format!(
                        "protocol version mismatch (host is {PROTOCOL_VERSION}, you are {protocol_version})"
                    ),
                });
                let _: Vec<ClientMessage> = conn.pump();
                continue;
            }
            if self.connections.len() + 1 >= MAX_PLAYERS {
                conn.send(&ServerMessage::Reject { reason: "server is full".to_string() });
                let _: Vec<ClientMessage> = conn.pump();
                continue;
            }

            let net_id = self.allocate_net_id();
            log::info!("player '{player_name}' joined as {net_id:?}");
            self.connections.insert(net_id, PeerConnection { conn, name: player_name.clone(), appearance: appearance.clone() });
            newly_joined.push((net_id, player_name, appearance));
        }

        newly_joined
    }

    /// Pumps every established connection once and returns this frame's
    /// `Input` messages. `Interact`/`Disconnect` messages seen during the
    /// same pump are buffered for `poll_interacts`/`take_disconnected`.
    pub fn poll_inputs(&mut self) -> Vec<(NetId, InputState)> {
        let mut inputs = Vec::new();
        let mut newly_disconnected = Vec::new();
        for (&net_id, peer) in self.connections.iter_mut() {
            let messages: Vec<ClientMessage> = peer.conn.pump();
            for msg in messages {
                match msg {
                    ClientMessage::Input { move_dir, yaw_deg, pitch_deg, jump, .. } => {
                        inputs.push((net_id, InputState { move_dir, yaw_deg, pitch_deg, jump }));
                    }
                    ClientMessage::Interact { target } => self.pending_interacts.push((net_id, target)),
                    ClientMessage::DialogueChoice { speaker, index } => {
                        self.pending_dialogue_choices.push((net_id, speaker, index));
                    }
                    ClientMessage::SetAppearance { rig_path } => {
                        // Recorded here (not just buffered) so a later
                        // catch-up send (e.g. this peer's `PlayerJoined`
                        // being replayed to a *third* joiner) reflects the
                        // latest choice, not the one from `Hello`.
                        peer.appearance = rig_path.clone();
                        self.pending_appearance_changes.push((net_id, rig_path));
                    }
                    ClientMessage::Disconnect => newly_disconnected.push(net_id),
                    ClientMessage::Hello { .. } => {}
                }
            }
            if peer.conn.is_disconnected() && !newly_disconnected.contains(&net_id) {
                newly_disconnected.push(net_id);
            }
        }
        for net_id in newly_disconnected {
            self.connections.remove(&net_id);
            self.pending_disconnected.push(net_id);
        }
        inputs
    }

    /// Drains `(requester, target)` interact requests buffered by the
    /// last `poll_inputs` call.
    pub fn poll_interacts(&mut self) -> Vec<(NetId, NetId)> {
        std::mem::take(&mut self.pending_interacts)
    }

    /// Drains `(requester, speaker, choice_index)` dialogue-choice picks
    /// buffered by the last `poll_inputs` call.
    pub fn poll_dialogue_choices(&mut self) -> Vec<(NetId, NetId, usize)> {
        std::mem::take(&mut self.pending_dialogue_choices)
    }

    /// Drains `(net_id, new_appearance)` mid-session appearance changes
    /// buffered by the last `poll_inputs` call — the caller should rebuild
    /// that `NetId`'s proxy and broadcast `ServerMessage::PlayerAppearanceChanged`.
    pub fn poll_appearance_changes(&mut self) -> Vec<(NetId, Option<PathBuf>)> {
        std::mem::take(&mut self.pending_appearance_changes)
    }

    /// Drains the `NetId`s of connections that dropped since the last
    /// call (detected during `poll_inputs`, or an explicit
    /// `ClientMessage::Disconnect`) — the caller should despawn each and
    /// broadcast `ServerMessage::PlayerLeft`.
    pub fn take_disconnected(&mut self) -> Vec<NetId> {
        std::mem::take(&mut self.pending_disconnected)
    }

    /// Queues `msg` for every established connection — the bytes don't
    /// actually reach the socket until the next `poll_inputs()` call (that
    /// method is what pumps established connections' writes as well as
    /// reads), so callers must keep calling `poll_inputs()` every frame
    /// even on frames where they don't care about its returned inputs.
    pub fn broadcast(&mut self, msg: &ServerMessage) {
        for peer in self.connections.values_mut() {
            peer.conn.send(msg);
        }
    }

    /// Sibling to `broadcast` for a single connection — same "queued now,
    /// flushed on the next `poll_inputs()`" caveat applies.
    pub fn send_to(&mut self, net_id: NetId, msg: &ServerMessage) {
        if let Some(peer) = self.connections.get_mut(&net_id) {
            peer.conn.send(msg);
        }
    }

    /// Accumulates `dt` against `SNAPSHOT_RATE_HZ` and returns `true` on
    /// frames where a `Snapshot` should be built and broadcast — decouples
    /// the snapshot rate from the render frame rate.
    pub fn should_send_snapshot(&mut self, dt: f32) -> bool {
        self.snapshot_accumulator += dt;
        let interval = 1.0 / SNAPSHOT_RATE_HZ;
        if self.snapshot_accumulator >= interval {
            self.snapshot_accumulator -= interval;
            true
        } else {
            false
        }
    }

    /// Remote players currently connected — includes the host's own
    /// player, i.e. `player_count() == connections.len() + 1`.
    pub fn player_count(&self) -> usize {
        self.connections.len() + 1
    }

    pub fn player_name(&self, net_id: NetId) -> Option<&str> {
        self.connections.get(&net_id).map(|peer| peer.name.as_str())
    }

    /// The given connected player's currently-chosen player-model rig
    /// path, if any — used to catch a newly-joined client up on every
    /// already-connected peer's appearance via `PlayerJoined`.
    pub fn player_appearance(&self, net_id: NetId) -> Option<&Path> {
        self.connections.get(&net_id).and_then(|peer| peer.appearance.as_deref())
    }

    /// Every currently-connected remote player's `NetId` — used to catch a
    /// newly-joined client up on peers who were already connected (the
    /// host's own player is announced separately, since it isn't in this
    /// map at all).
    pub fn connected_ids(&self) -> Vec<NetId> {
        self.connections.keys().copied().collect()
    }
}

/// The non-authoritative side — connects to a `NetHost`, sends its own
/// input, and receives `ServerMessage`s to apply locally. Like `NetHost`,
/// `poll_messages` should be called once per frame.
pub struct NetClient {
    conn: NetConnection,
    my_net_id: Option<NetId>,
    next_seq: u32,
    connected: bool,
}

impl NetClient {
    /// Blocks for up to `timeout` while the TCP handshake completes (a
    /// one-shot hitch triggered by a "Join" button click, not a per-frame
    /// cost — see the plan's stated simplifications), then sends `Hello`
    /// (including the locally-chosen `appearance`, if any — see
    /// `ClientMessage::Hello`) and switches to non-blocking for the rest
    /// of the session.
    pub fn connect(addr: SocketAddr, name: String, appearance: Option<PathBuf>, timeout: Duration) -> io::Result<Self> {
        let stream = TcpStream::connect_timeout(&addr, timeout)?;
        let mut conn = NetConnection::wrap(stream)?;
        conn.send(&ClientMessage::Hello { player_name: name, protocol_version: PROTOCOL_VERSION, appearance });
        Ok(Self { conn, my_net_id: None, next_seq: 0, connected: true })
    }

    pub fn send_input(&mut self, input: &InputState) {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        self.conn.send(&ClientMessage::Input {
            seq,
            move_dir: input.move_dir,
            yaw_deg: input.yaw_deg,
            pitch_deg: input.pitch_deg,
            jump: input.jump,
        });
    }

    pub fn send_interact(&mut self, target: NetId) {
        self.conn.send(&ClientMessage::Interact { target });
    }

    pub fn send_dialogue_choice(&mut self, speaker: NetId, index: usize) {
        self.conn.send(&ClientMessage::DialogueChoice { speaker, index });
    }

    /// Sent when the local player changes their player-model appearance
    /// mid-session — the host relays it to every peer as
    /// `ServerMessage::PlayerAppearanceChanged`.
    pub fn send_set_appearance(&mut self, rig_path: Option<PathBuf>) {
        self.conn.send(&ClientMessage::SetAppearance { rig_path });
    }

    /// Pumps the connection once and returns every `ServerMessage`
    /// received this call — including tracking `my_net_id` off the
    /// initial `Welcome` and flipping `is_connected()` false if the host
    /// went away.
    pub fn poll_messages(&mut self) -> Vec<ServerMessage> {
        if !self.connected {
            return Vec::new();
        }
        let messages: Vec<ServerMessage> = self.conn.pump();
        for msg in &messages {
            if let ServerMessage::Welcome { your_net_id, .. } = msg {
                self.my_net_id = Some(*your_net_id);
            }
        }
        if self.conn.is_disconnected() {
            self.connected = false;
        }
        messages
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn my_net_id(&self) -> Option<NetId> {
        self.my_net_id
    }
}

/// Deterministic small spread so 2-4 players landing on the same authored
/// spawn point (initial join, or a level transition) don't stack on top
/// of each other — a pure function of `net_id` so host and client always
/// derive the identical offset independently, with no extra data needing
/// to travel over the wire.
pub fn spawn_ring_offset(base: glam::Vec3, base_yaw_deg: f32, net_id: NetId) -> (glam::Vec3, f32) {
    const RING_RADIUS: f32 = 1.2;
    // Place each player by the golden angle (~137.5°) keyed on its raw
    // `NetId`. A naive `net_id % MAX_PLAYERS` scheme collides here: NetIds
    // come from a single counter shared with characters and dynamic props,
    // so a session's player ids are rarely a dense `1..=4` run, and two
    // players whose ids happen to be congruent mod 4 would stack at the
    // identical offset — defeating the whole point. The golden angle spreads
    // any set of distinct integer ids near-uniformly around the ring, so
    // whatever ids the 2-4 concurrent players actually got, they land at
    // distinct offsets. Still purely a function of `net_id`, so host and
    // client derive the identical position with no wire data.
    const GOLDEN_ANGLE: f32 = 2.399_963_2; // = PI * (3 - sqrt(5)) ≈ 137.507°
    let angle = net_id.0 as f32 * GOLDEN_ANGLE;
    let offset = glam::Vec3::new(angle.cos() * RING_RADIUS, 0.0, angle.sin() * RING_RADIUS);
    (base + offset, base_yaw_deg)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives a real `NetHost`/`NetClient` pair over `127.0.0.1` inside
    /// this one test process — no live SDL window needed — proving the
    /// non-blocking accept/connect/handshake path actually completes
    /// end-to-end, not just that the types compile.
    #[test]
    fn client_completes_handshake_with_host() {
        let mut host = NetHost::bind(0).expect("bind to an ephemeral port");
        let port = host.listener.local_addr().unwrap().port();

        let mut client = NetClient::connect(
            format!("127.0.0.1:{port}").parse().unwrap(),
            "Tester".to_string(),
            None,
            Duration::from_secs(2),
        )
        .expect("connect");

        // `NetHost` itself is transport-only and has no notion of `Level`
        // content — sending `Welcome` in response to a join is game-level
        // responsibility (see the plan's step 3 vs. step 4 split), so this
        // test plays that role itself, exactly like real game code would.
        let mut joined = Vec::new();
        let mut welcomed = false;
        for _ in 0..100 {
            for (net_id, name, _appearance) in host.poll_new_connections() {
                host.send_to(
                    net_id,
                    &ServerMessage::Welcome {
                        your_net_id: net_id,
                        level: crate::level::Level::default(),
                        named_net_ids: Vec::new(),
                        tick_rate: SNAPSHOT_RATE_HZ,
                    },
                );
                joined.push((net_id, name));
            }
            // `send_to` above only queues bytes — established connections
            // (as opposed to not-yet-`Hello`'d ones) only actually get
            // pumped by `poll_inputs`, so real game code calls both every
            // frame; this test does the same.
            let _ = host.poll_inputs();
            for msg in client.poll_messages() {
                if matches!(msg, ServerMessage::Welcome { .. }) {
                    welcomed = true;
                }
            }
            if !joined.is_empty() && welcomed {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        assert_eq!(joined.len(), 1);
        assert_eq!(joined[0].1, "Tester");
        assert!(welcomed);
        assert_eq!(client.my_net_id(), Some(joined[0].0));
        assert_eq!(host.player_count(), 2);
        assert!(client.is_connected());
    }

    #[test]
    fn spawn_ring_offset_is_deterministic_and_spreads_players() {
        let base = glam::Vec3::new(1.0, 2.0, 3.0);
        let (a1, _) = spawn_ring_offset(base, 0.0, NetId(1));
        let (a2, _) = spawn_ring_offset(base, 0.0, NetId(1));
        assert_eq!(a1, a2);

        let (b, _) = spawn_ring_offset(base, 0.0, NetId(2));
        assert_ne!(a1, b);

        // Sparse, non-dense ids that a real session produces (host = 1, three
        // characters take 2/3/4, remote players join as 5 and 9). The old
        // `net_id % MAX_PLAYERS` scheme mapped 1, 5, and 9 all to slot 1 and
        // stacked those players; every pair here must now be distinct.
        let player_ids = [1u32, 5, 9];
        for (i, &id_a) in player_ids.iter().enumerate() {
            for &id_b in &player_ids[i + 1..] {
                let (pa, _) = spawn_ring_offset(base, 0.0, NetId(id_a));
                let (pb, _) = spawn_ring_offset(base, 0.0, NetId(id_b));
                assert_ne!(pa, pb, "players {id_a} and {id_b} stacked on the same spawn offset");
            }
        }
    }

    /// A raw connection speaking `Hello` directly (bypassing `NetClient`,
    /// which always sends the correct `PROTOCOL_VERSION`) so a mismatch
    /// can actually be exercised.
    fn hello_and_wait_for_reply(
        host: &mut NetHost,
        addr: std::net::SocketAddr,
        hello: ClientMessage,
    ) -> (Vec<(NetId, String, Option<PathBuf>)>, Vec<ServerMessage>) {
        let stream = TcpStream::connect(addr).expect("connect");
        let mut raw = NetConnection::wrap(stream).expect("wrap");
        raw.send(&hello);

        let mut joined = Vec::new();
        let mut replies = Vec::new();
        for _ in 0..100 {
            joined.extend(host.poll_new_connections());
            replies.extend(raw.pump::<ServerMessage>());
            if !joined.is_empty() || !replies.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        (joined, replies)
    }

    #[test]
    fn rejects_protocol_version_mismatch() {
        let mut host = NetHost::bind(0).expect("bind to an ephemeral port");
        let addr: std::net::SocketAddr = format!("127.0.0.1:{}", host.listener.local_addr().unwrap().port())
            .parse()
            .unwrap();

        let (joined, replies) = hello_and_wait_for_reply(
            &mut host,
            addr,
            ClientMessage::Hello {
                player_name: "Mismatched".to_string(),
                protocol_version: PROTOCOL_VERSION + 1,
                appearance: None,
            },
        );

        assert!(joined.is_empty(), "a version-mismatched peer must never be treated as joined");
        assert!(matches!(replies.as_slice(), [ServerMessage::Reject { .. }]));
        assert_eq!(host.player_count(), 1, "only the host itself — the mismatched peer never counts");
    }

    #[test]
    fn rejects_connection_beyond_max_players() {
        let mut host = NetHost::bind(0).expect("bind to an ephemeral port");
        let addr: std::net::SocketAddr = format!("127.0.0.1:{}", host.listener.local_addr().unwrap().port())
            .parse()
            .unwrap();

        // Fill every remote slot — MAX_PLAYERS - 1, since the host itself
        // already occupies one.
        let mut clients = Vec::new();
        for i in 0..MAX_PLAYERS - 1 {
            let mut client =
                NetClient::connect(addr, format!("Player{i}"), None, Duration::from_secs(2)).expect("connect");
            for _ in 0..100 {
                let _ = host.poll_new_connections();
                client.poll_messages();
                if host.player_count() == i + 2 {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            clients.push(client);
        }
        assert_eq!(host.player_count(), MAX_PLAYERS, "lobby should now be full");

        let (joined, replies) = hello_and_wait_for_reply(
            &mut host,
            addr,
            ClientMessage::Hello {
                player_name: "Overflow".to_string(),
                protocol_version: PROTOCOL_VERSION,
                appearance: None,
            },
        );

        assert!(joined.is_empty(), "a peer arriving after the lobby is full must never be treated as joined");
        assert!(matches!(replies.as_slice(), [ServerMessage::Reject { .. }]));
        assert_eq!(host.player_count(), MAX_PLAYERS, "still full, not full+1");
    }
}
