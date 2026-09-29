//! Co-op for up to five dreamers over the engine's TCP framing. The host is
//! authoritative: it plans every dream (so all peers build the same one),
//! runs the enemies, and decides shards, catches, revives and the portal.
//! Each player keeps their own loadout and picks their own upgrades. There
//! is no story in co-op (no notes, no card dreams).
//!
//! This file is the wire protocol, the two session ends, and the pure
//! rules; `main.rs` drives them.

use crate::dream::DreamTheme;
use engine::net::NetConnection;
use serde::{Deserialize, Serialize};
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

pub const PORT: u16 = 7778;
/// Bumped whenever a message's shape changes (bincode is positional).
pub const VERSION: u32 = 1;
pub const MAX_PLAYERS: usize = engine::net::MAX_PLAYERS;
/// Poses per second each way.
pub const POSE_HZ: f32 = 20.0;
/// How close a living dreamer must stand to a ghost to bring them back.
pub const REVIVE_RADIUS: f32 = 1.4;
/// Seconds they must stay there.
pub const REVIVE_SECONDS: f32 = 2.0;

/// A player in the session: 0 is the host.
pub type PlayerId = u8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    pub id: PlayerId,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub ghost: bool,
    /// Waiting in the portal for the others.
    pub in_portal: bool,
}

/// Everything that decides how a dream is built, so every peer builds the
/// same one (the host's director is the only one that rolls).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DreamPlan {
    pub depth: u32,
    pub theme: DreamTheme,
    pub next: DreamTheme,
    pub nightmare: bool,
    pub has_shard: bool,
    pub blend: Option<DreamTheme>,
    pub hard_from: Option<u32>,
    pub overdrive: u32,
    pub lucidity: u32,
    pub shards_to_wake: u32,
    /// Enemy-count multiplier (already scaled for the party).
    pub enemies: f32,
    pub growth_cap: u32,
    /// The waking dream: the run ends when everyone reaches its portal.
    pub waking: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToHost {
    Hello {
        name: String,
        version: u32,
    },
    Pose(Pose),
    /// Back in the dream after the pick screen.
    Ready,
    /// Touched the shard of the dream at this depth.
    Shard {
        depth: u32,
    },
    /// Caught (by an enemy the host placed): now a ghost.
    Caught,
    Leave,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToPeer {
    Welcome {
        you: PlayerId,
        players: Vec<(PlayerId, String)>,
    },
    Reject(String),
    Joined(PlayerId, String),
    Left(PlayerId),
    /// The run starts: same seed and length for everyone.
    Start {
        run_seed: u64,
        long: bool,
        players: u8,
    },
    Dream(DreamPlan),
    Poses(Vec<Pose>),
    /// Pacers as (x, y, z, scale) in the dream's patrol order (splits
    /// appended), and the nightmare's hunter if there is one.
    Enemies {
        pacers: Vec<[f32; 4]>,
        hunter: Option<[f32; 4]>,
    },
    ShardTaken {
        by: PlayerId,
        lucidity: u32,
    },
    Caught(PlayerId),
    Revived(PlayerId),
    /// Everyone is a ghost: the run wakes.
    RunOver,
}

/// Shards to wake with `players` dreamers: one more per extra dreamer.
pub fn shards_needed(base: u32, players: usize) -> u32 {
    base + players.saturating_sub(1) as u32
}

/// Enemy count multiplier: a quarter more per extra dreamer.
pub fn enemy_scale(players: usize) -> f32 {
    1.0 + 0.25 * players.saturating_sub(1) as f32
}

/// The portal opens when every living dreamer is in it (and someone is).
pub fn all_in_portal(poses: &[Pose]) -> bool {
    let living: Vec<&Pose> = poses.iter().filter(|p| !p.ghost).collect();
    !living.is_empty() && living.iter().all(|p| p.in_portal)
}

/// Everyone caught: the run wakes.
pub fn all_ghosts(poses: &[Pose]) -> bool {
    !poses.is_empty() && poses.iter().all(|p| p.ghost)
}

fn near(a: [f32; 3], b: [f32; 3], r: f32) -> bool {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2] <= r * r
}

/// Revive timers, per ghost.
#[derive(Clone, Debug, Default)]
pub struct Revives {
    held: Vec<(PlayerId, f32)>,
}

impl Revives {
    /// Advances every ghost's timer while a living dreamer stands close;
    /// returns the ghosts brought back this tick.
    pub fn tick(&mut self, poses: &[Pose], dt: f32) -> Vec<PlayerId> {
        let mut back = Vec::new();
        for g in poses.iter().filter(|p| p.ghost) {
            let helped = poses
                .iter()
                .any(|p| !p.ghost && p.id != g.id && near(p.pos, g.pos, REVIVE_RADIUS));
            let t = match self.held.iter_mut().find(|(id, _)| *id == g.id) {
                Some((_, t)) => t,
                None => {
                    self.held.push((g.id, 0.0));
                    &mut self.held.last_mut().unwrap().1
                }
            };
            *t = if helped { *t + dt } else { 0.0 };
            if *t >= REVIVE_SECONDS {
                *t = 0.0;
                back.push(g.id);
            }
        }
        self.held
            .retain(|(id, _)| poses.iter().any(|p| p.id == *id && p.ghost));
        back
    }

    /// How far along a ghost's revive is, 0..1 (for the HUD).
    pub fn progress(&self, id: PlayerId) -> f32 {
        self.held
            .iter()
            .find(|(g, _)| *g == id)
            .map_or(0.0, |(_, t)| (t / REVIVE_SECONDS).min(1.0))
    }
}

/// One joined dreamer, as the host sees them.
pub struct Peer {
    pub id: PlayerId,
    pub name: String,
    pub conn: NetConnection,
    pub pose: Pose,
    pub ready: bool,
}

/// What the host heard this frame.
#[derive(Clone, Debug, PartialEq)]
pub enum HostEvent {
    Joined(PlayerId, String),
    Left(PlayerId),
    Pose(Pose),
    Ready(PlayerId),
    Shard { by: PlayerId, depth: u32 },
    Caught(PlayerId),
}

pub struct Host {
    listener: TcpListener,
    /// Accepted, waiting for Hello.
    pending: Vec<NetConnection>,
    pub peers: Vec<Peer>,
    pub name: String,
    /// The run has started: nobody else may join.
    pub started: bool,
}

impl Host {
    pub fn bind(port: u16, name: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(("0.0.0.0", port))?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            pending: Vec::new(),
            peers: Vec::new(),
            name: name.to_string(),
            started: false,
        })
    }

    pub fn port(&self) -> u16 {
        self.listener.local_addr().map_or(0, |a| a.port())
    }

    /// Everyone, host first.
    pub fn roster(&self) -> Vec<(PlayerId, String)> {
        std::iter::once((0, self.name.clone()))
            .chain(self.peers.iter().map(|p| (p.id, p.name.clone())))
            .collect()
    }

    pub fn player_count(&self) -> usize {
        1 + self.peers.len()
    }

    fn next_id(&self) -> PlayerId {
        (1..MAX_PLAYERS as PlayerId)
            .find(|id| !self.peers.iter().any(|p| p.id == *id))
            .unwrap_or(MAX_PLAYERS as PlayerId)
    }

    pub fn broadcast(&mut self, msg: &ToPeer) {
        for p in &mut self.peers {
            p.conn.send(msg);
        }
    }

    /// Accepts, handshakes and reads everything that's arrived.
    pub fn poll(&mut self) -> Vec<HostEvent> {
        let mut events = Vec::new();
        while let Ok((stream, _)) = self.listener.accept() {
            if let Ok(c) = NetConnection::wrap(stream) {
                self.pending.push(c);
            }
        }
        let mut still = Vec::new();
        for mut c in std::mem::take(&mut self.pending) {
            let msgs: Vec<ToHost> = c.pump();
            match msgs.into_iter().next() {
                Some(ToHost::Hello { name, version }) => {
                    let why = if version != VERSION {
                        Some(format!("version mismatch (host {VERSION}, you {version})"))
                    } else if self.started {
                        Some("the dream has already started".to_string())
                    } else if self.player_count() >= MAX_PLAYERS {
                        Some(format!("full ({MAX_PLAYERS} dreamers)"))
                    } else {
                        None
                    };
                    if let Some(why) = why {
                        c.send(&ToPeer::Reject(why));
                        c.pump::<ToHost>();
                        continue;
                    }
                    let id = self.next_id();
                    let name: String = name.chars().take(16).collect();
                    self.broadcast(&ToPeer::Joined(id, name.clone()));
                    let mut players = self.roster();
                    players.push((id, name.clone()));
                    c.send(&ToPeer::Welcome { you: id, players });
                    self.peers.push(Peer {
                        id,
                        name: name.clone(),
                        conn: c,
                        pose: Pose {
                            id,
                            ..Default::default()
                        },
                        ready: true,
                    });
                    events.push(HostEvent::Joined(id, name));
                }
                Some(_) => {} // must say hello first
                None if !c.is_disconnected() => still.push(c),
                None => {}
            }
        }
        self.pending = still;
        let mut gone = Vec::new();
        for p in &mut self.peers {
            for m in p.conn.pump::<ToHost>() {
                match m {
                    ToHost::Pose(mut pose) => {
                        pose.id = p.id;
                        pose.ghost = p.pose.ghost; // the host decides who's a ghost
                        p.pose = pose;
                        events.push(HostEvent::Pose(pose));
                    }
                    ToHost::Ready => {
                        p.ready = true;
                        events.push(HostEvent::Ready(p.id));
                    }
                    ToHost::Shard { depth } => events.push(HostEvent::Shard { by: p.id, depth }),
                    ToHost::Caught => events.push(HostEvent::Caught(p.id)),
                    ToHost::Leave => gone.push(p.id),
                    ToHost::Hello { .. } => {}
                }
            }
            if p.conn.is_disconnected() {
                gone.push(p.id);
            }
        }
        for id in gone {
            self.peers.retain(|p| p.id != id);
            self.broadcast(&ToPeer::Left(id));
            events.push(HostEvent::Left(id));
        }
        events
    }
}

pub struct Client {
    conn: NetConnection,
    pub me: Option<PlayerId>,
    pub roster: Vec<(PlayerId, String)>,
    pub rejected: Option<String>,
}

impl Client {
    pub fn connect(addr: &str, name: &str) -> io::Result<Self> {
        let addr: SocketAddr = if addr.contains(':') {
            addr.parse()
        } else {
            format!("{addr}:{PORT}").parse()
        }
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))?;
        let mut conn = NetConnection::wrap(stream)?;
        conn.send(&ToHost::Hello {
            name: name.to_string(),
            version: VERSION,
        });
        Ok(Self {
            conn,
            me: None,
            roster: Vec::new(),
            rejected: None,
        })
    }

    pub fn send(&mut self, msg: &ToHost) {
        self.conn.send(msg);
    }

    pub fn is_connected(&self) -> bool {
        !self.conn.is_disconnected() && self.rejected.is_none()
    }

    /// Everything from the host this frame (the roster is kept up to date).
    pub fn poll(&mut self) -> Vec<ToPeer> {
        let msgs: Vec<ToPeer> = self.conn.pump();
        for m in &msgs {
            match m {
                ToPeer::Welcome { you, players } => {
                    self.me = Some(*you);
                    self.roster = players.clone();
                }
                ToPeer::Joined(id, name) => {
                    if !self.roster.iter().any(|(i, _)| i == id) {
                        self.roster.push((*id, name.clone()));
                    }
                }
                ToPeer::Left(id) => self.roster.retain(|(i, _)| i != id),
                ToPeer::Reject(why) => self.rejected = Some(why.clone()),
                _ => {}
            }
        }
        msgs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose(id: PlayerId, x: f32, ghost: bool, in_portal: bool) -> Pose {
        Pose {
            id,
            pos: [x, 0.0, 0.0],
            yaw: 0.0,
            ghost,
            in_portal,
        }
    }

    /// Pumps both ends until `done` or a second passes.
    fn settle(host: &mut Host, clients: &mut [Client], mut done: impl FnMut(&[HostEvent]) -> bool) {
        let mut all = Vec::new();
        for _ in 0..200 {
            all.extend(host.poll());
            for c in clients.iter_mut() {
                c.poll();
            }
            if done(&all) {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn dreamers_join_see_each_other_and_leave() {
        let mut host = Host::bind(0, "host").unwrap();
        let addr = format!("127.0.0.1:{}", host.port());
        let mut a = vec![Client::connect(&addr, "ada").unwrap()];
        settle(&mut host, &mut a, |e| {
            e.iter().any(|e| matches!(e, HostEvent::Joined(..)))
        });
        a.push(Client::connect(&addr, "ben").unwrap());
        settle(&mut host, &mut a, |_| false);
        assert_eq!(host.player_count(), 3);
        assert_eq!(a[0].me, Some(1));
        assert_eq!(a[1].me, Some(2));
        assert_eq!(a[0].roster.len(), 3, "ada heard about ben");
        a[1].send(&ToHost::Pose(pose(9, 4.0, true, false)));
        let mut got = None;
        settle(&mut host, &mut a, |e| {
            got = e.iter().find_map(|e| match e {
                HostEvent::Pose(p) => Some(*p),
                _ => None,
            });
            got.is_some()
        });
        let p = got.expect("pose arrives");
        assert_eq!(p.id, 2, "the host stamps who sent it");
        assert!(!p.ghost, "clients can't declare themselves ghosts");
        a[1].send(&ToHost::Leave);
        settle(&mut host, &mut a, |e| {
            e.iter().any(|e| matches!(e, HostEvent::Left(2)))
        });
        assert_eq!(host.player_count(), 2);
        settle(&mut host, &mut a[..1], |_| false);
        assert_eq!(a[0].roster.len(), 2, "ada heard ben leave");
    }

    #[test]
    fn a_full_or_started_dream_turns_people_away() {
        let mut host = Host::bind(0, "host").unwrap();
        host.started = true;
        let addr = format!("127.0.0.1:{}", host.port());
        let mut c = vec![Client::connect(&addr, "late").unwrap()];
        settle(&mut host, &mut c, |_| false);
        assert!(c[0].rejected.is_some());
        assert!(!c[0].is_connected());
    }

    #[test]
    fn the_portal_waits_for_every_living_dreamer() {
        assert!(!all_in_portal(&[
            pose(0, 0.0, false, true),
            pose(1, 0.0, false, false)
        ]));
        assert!(all_in_portal(&[
            pose(0, 0.0, false, true),
            pose(1, 0.0, true, false)
        ]));
        assert!(!all_in_portal(&[pose(0, 0.0, true, true)]), "nobody living");
        assert!(all_ghosts(&[
            pose(0, 0.0, true, false),
            pose(1, 0.0, true, false)
        ]));
        assert!(!all_ghosts(&[]));
    }

    #[test]
    fn a_ghost_comes_back_after_two_seconds_of_company() {
        let mut r = Revives::default();
        let far = [pose(0, 0.0, false, false), pose(1, 5.0, true, false)];
        assert!(r.tick(&far, 3.0).is_empty());
        let close = [pose(0, 0.0, false, false), pose(1, 1.0, true, false)];
        assert!(r.tick(&close, 1.0).is_empty());
        assert!(r.progress(1) > 0.4);
        assert!(r.tick(&far, 0.1).is_empty(), "stepping away resets it");
        assert_eq!(r.progress(1), 0.0);
        r.tick(&close, 1.0);
        assert_eq!(r.tick(&close, 1.0), vec![1]);
        let two_ghosts = [pose(0, 0.0, true, false), pose(1, 0.5, true, false)];
        assert!(
            r.tick(&two_ghosts, 5.0).is_empty(),
            "ghosts can't revive ghosts"
        );
    }

    #[test]
    fn more_dreamers_need_more_shards_and_meet_more_enemies() {
        assert_eq!(shards_needed(3, 1), 3);
        assert_eq!(shards_needed(3, 5), 7);
        assert_eq!(enemy_scale(1), 1.0);
        assert_eq!(enemy_scale(5), 2.0);
    }

    #[test]
    fn plans_survive_the_wire() {
        let p = DreamPlan {
            depth: 4,
            theme: DreamTheme::Garden,
            next: DreamTheme::Lobby,
            nightmare: false,
            has_shard: true,
            blend: Some(DreamTheme::TheTunnel),
            hard_from: None,
            overdrive: 0,
            lucidity: 1,
            shards_to_wake: 4,
            enemies: 1.25,
            growth_cap: 12,
            waking: false,
        };
        let mut host = Host::bind(0, "host").unwrap();
        let addr = format!("127.0.0.1:{}", host.port());
        let mut c = vec![Client::connect(&addr, "ada").unwrap()];
        settle(&mut host, &mut c, |e| {
            e.iter().any(|e| matches!(e, HostEvent::Joined(..)))
        });
        host.broadcast(&ToPeer::Dream(p.clone()));
        let mut got = None;
        for _ in 0..200 {
            host.poll();
            if let Some(ToPeer::Dream(d)) = c[0]
                .poll()
                .into_iter()
                .find(|m| matches!(m, ToPeer::Dream(_)))
            {
                got = Some(d);
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(got, Some(p));
    }
}
