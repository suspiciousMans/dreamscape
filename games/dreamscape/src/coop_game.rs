//! Co-op, as the game plays it: the lobby, the per-frame session tick
//! (poses, enemies, shards, catches, revives, the portal) and the hooks the
//! single-player code calls when `self.coop` is set. The rules and the wire
//! protocol live in `coop.rs`.

use super::*;
use crate::coop::{self, DreamPlan, HostEvent, PlayerId, Pose, ToHost, ToPeer};

pub(crate) enum CoopNet {
    /// Typing the host's address.
    Joining,
    Host(coop::Host),
    Client(coop::Client),
}

/// Pacers as (x, y, z, scale), and the hunter if any.
pub(crate) type EnemySnapshot = (Vec<[f32; 4]>, Option<[f32; 4]>);

pub(crate) struct CoopState {
    pub net: CoopNet,
    pub me: PlayerId,
    /// Dreamers when the run started (shards and enemies scale with it).
    pub players: usize,
    pub started: bool,
    /// The others' avatars in this dream.
    pub avatars: Vec<(PlayerId, Entity)>,
    /// Everyone's latest pose (the host's list is the truth).
    pub poses: Vec<Pose>,
    pub send_timer: f32,
    pub ghost: bool,
    pub in_portal: bool,
    pub revives: coop::Revives,
    /// Client: the next dream, from the host.
    pub plan: Option<DreamPlan>,
    /// Client: the host's latest enemy snapshot.
    pub enemies: Option<EnemySnapshot>,
    /// The join address being typed.
    pub address: String,
    pub status: Option<String>,
}

impl CoopState {
    fn new(net: CoopNet) -> Self {
        Self {
            net,
            me: 0,
            players: 1,
            started: false,
            avatars: Vec::new(),
            poses: Vec::new(),
            send_timer: 0.0,
            ghost: false,
            in_portal: false,
            revives: coop::Revives::default(),
            plan: None,
            enemies: None,
            address: "127.0.0.1".into(),
            status: None,
        }
    }

    pub fn is_host(&self) -> bool {
        matches!(self.net, CoopNet::Host(_))
    }

    pub fn is_client(&self) -> bool {
        matches!(self.net, CoopNet::Client(_))
    }

    /// Everyone in the lobby, as (id, name).
    pub fn roster(&self) -> Vec<(PlayerId, String)> {
        match &self.net {
            CoopNet::Host(h) => h.roster(),
            CoopNet::Client(c) => c.roster.clone(),
            CoopNet::Joining => Vec::new(),
        }
    }
}

/// Each dreamer's colour.
pub fn player_color(id: PlayerId) -> [u8; 3] {
    crate::hud::hue(id as f32 * 0.21 + 0.55)
}

/// The lobby screen.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LobbyView {
    pub hosting: bool,
    pub joining: bool,
    pub address: String,
    pub roster: Vec<(u8, String)>,
    pub status: Option<String>,
    pub port: u16,
}

pub fn draw_lobby(p: &egui::Painter, screen: egui::Rect, v: &hud::HudView) {
    use crate::hud::{glitch_text, ink, rgba};
    use egui::{Align2, FontId, Pos2};
    let m = &v.lobby;
    p.rect_filled(screen, 0.0, rgba([6, 3, 16], 0.95));
    let cx = screen.center().x;
    let top = screen.top() + 40.0;
    glitch_text(
        p,
        Pos2::new(cx, top),
        true,
        "A SHARED DREAM",
        44.0,
        ink(v),
        1.0,
        v,
    );
    let mut y = top + 80.0;
    let line = |y: f32, text: String, col: [u8; 3]| {
        p.text(
            Pos2::new(cx, y),
            Align2::CENTER_TOP,
            text,
            FontId::monospace(20.0),
            rgba(col, 0.95),
        );
    };
    if m.joining {
        line(y, "the host's address:".into(), [200, 190, 230]);
        y += 34.0;
        let caret = if (v.time * 2.0).fract() < 0.5 {
            "_"
        } else {
            " "
        };
        line(y, format!("{}{caret}", m.address), [255, 230, 160]);
        y += 50.0;
    } else {
        if m.hosting {
            line(
                y,
                format!(
                    "hosting on port {} · friends join with your address",
                    m.port
                ),
                [200, 190, 230],
            );
            y += 40.0;
        }
        for (id, name) in &m.roster {
            line(y, name.clone(), player_color(*id));
            y += 30.0;
        }
        y += 10.0;
        line(
            y,
            format!("{} / {} dreamers", m.roster.len(), coop::MAX_PLAYERS),
            [160, 150, 190],
        );
        y += 40.0;
    }
    if let Some(s) = &m.status {
        line(y, s.clone(), [255, 150, 170]);
    }
    let keys = if m.joining {
        "[0-9 . :] type   [enter] join   [esc] back"
    } else if m.hosting {
        "[space] fall asleep together   [esc] back"
    } else {
        "waiting for the host to fall asleep   [esc] leave"
    };
    p.text(
        Pos2::new(cx, screen.bottom() - 40.0),
        Align2::CENTER_TOP,
        v.k(keys),
        FontId::monospace(20.0),
        rgba(ink(v), 0.5 + 0.3 * (v.time * 2.0).sin()),
    );
}

impl DreamscapeGame {
    fn my_name(&self) -> String {
        lore::dreamer(self.booklet.lore.save_seed, 0).name
    }

    pub(crate) fn lobby_view(&self) -> LobbyView {
        let Some(c) = &self.coop else {
            return LobbyView::default();
        };
        LobbyView {
            hosting: c.is_host(),
            joining: matches!(c.net, CoopNet::Joining),
            address: c.address.clone(),
            roster: c.roster(),
            status: c.status.clone(),
            port: match &c.net {
                CoopNet::Host(h) => h.port(),
                _ => 0,
            },
        }
    }

    pub(crate) fn open_host_lobby(&mut self) {
        match coop::Host::bind(coop::PORT, &self.my_name()) {
            Ok(h) => {
                log::info!("Co-op: hosting on port {}", h.port());
                self.coop = Some(CoopState::new(CoopNet::Host(h)));
            }
            Err(e) => {
                log::warn!("Co-op: couldn't host ({e})");
                let mut c = CoopState::new(CoopNet::Joining);
                c.status = Some(format!("couldn't host: {e}"));
                self.coop = Some(c);
            }
        }
        self.mode = hud::Mode::Lobby;
    }

    pub(crate) fn open_join_lobby(&mut self) {
        let mut c = CoopState::new(CoopNet::Joining);
        c.address = self.settings.coop_address.clone();
        self.coop = Some(c);
        self.mode = hud::Mode::Lobby;
    }

    pub(crate) fn leave_coop(&mut self) {
        if let Some(c) = &mut self.coop {
            if let CoopNet::Client(cl) = &mut c.net {
                cl.send(&ToHost::Leave);
                cl.poll();
            }
        }
        self.coop = None;
    }

    pub(crate) fn lobby_key(&mut self, key: Keycode) {
        let my_name = self.my_name();
        let Some(c) = &mut self.coop else {
            self.mode = hud::Mode::Title;
            return;
        };
        match (&c.net, key) {
            (_, Keycode::Escape) => {
                self.leave_coop();
                self.mode = hud::Mode::Title;
            }
            (CoopNet::Joining, Keycode::Backspace) => {
                c.address.pop();
            }
            (CoopNet::Joining, Keycode::Return) => {
                let addr = c.address.clone();
                match coop::Client::connect(&addr, &my_name) {
                    Ok(cl) => {
                        log::info!("Co-op: joining {addr}");
                        c.net = CoopNet::Client(cl);
                        c.status = None;
                        self.settings.coop_address = addr;
                        let _ = settings::save(&settings::settings_path(), &self.settings);
                    }
                    Err(e) => c.status = Some(format!("couldn't reach {addr}: {e}")),
                }
            }
            (CoopNet::Joining, k) => {
                if let Some(ch) = address_char(k) {
                    if c.address.len() < 40 {
                        c.address.push(ch);
                    }
                }
            }
            (CoopNet::Host(h), Keycode::Space | Keycode::Return) => {
                let players = h.player_count();
                let seed = gameplay::next_run_seed(self.run_seed ^ self.time.to_bits() as u64);
                let long = self.run_length == RunLength::Long;
                if let CoopNet::Host(h) = &mut c.net {
                    h.started = true;
                    h.broadcast(&ToPeer::Start {
                        run_seed: seed,
                        long,
                        players: players as u8,
                    });
                }
                self.pending_coop_start = Some((seed, long, players));
            }
            _ => {}
        }
    }

    /// Everyone falls asleep together: the same seed and length.
    pub(crate) fn start_coop_run(
        &mut self,
        ctx: &mut Context,
        seed: u64,
        long: bool,
        players: usize,
    ) -> anyhow::Result<()> {
        log::info!("Co-op run: seed {seed}, {players} dreamers");
        self.prologue = None;
        self.daily = None;
        self.run_seed = seed;
        self.run_length = if long {
            RunLength::Long
        } else {
            RunLength::Short
        };
        self.director = DreamDirector::with_length(seed, self.run_length);
        if let Some(c) = &mut self.coop {
            c.players = players;
            c.started = true;
            c.ghost = false;
            c.in_portal = false;
            c.avatars.clear();
        }
        self.motif = None;
        self.run_log.clear();
        self.transition.cancel();
        self.begin_run();
        self.director.shards_to_wake = coop::shards_needed(self.director.shards_to_wake, players);
        self.mode = hud::Mode::Playing;
        self.load_dream(ctx)
    }

    /// The host's plan for the current dream (it just descended).
    pub(crate) fn coop_plan(&self) -> DreamPlan {
        let d = &self.director;
        let players = self.coop.as_ref().map_or(1, |c| c.players);
        let base = self.pressure_base();
        DreamPlan {
            depth: d.depth,
            theme: d.theme,
            next: d.next,
            nightmare: d.nightmare,
            has_shard: d.has_shard,
            blend: d.blend,
            hard_from: d.hard_from,
            overdrive: d.overdrive,
            lucidity: d.lucidity,
            shards_to_wake: d.shards_to_wake,
            enemies: base.enemies * coop::enemy_scale(players),
            growth_cap: base.growth_cap,
            waking: d.theme == DreamTheme::Awakening,
        }
    }

    /// A client takes the host's plan as its own director state.
    pub(crate) fn apply_plan(&mut self, p: &DreamPlan) {
        let d = &mut self.director;
        d.depth = p.depth;
        d.theme = p.theme;
        d.next = p.next;
        d.nightmare = p.nightmare;
        d.has_shard = p.has_shard;
        d.blend = p.blend;
        d.hard_from = p.hard_from;
        d.overdrive = p.overdrive;
        d.lucidity = p.lucidity;
        d.shards_to_wake = p.shards_to_wake;
        d.shard_this_dream = false;
        d.card_dream = false;
    }

    /// In co-op the dream's pressure is the host's (scaled for the party).
    pub(crate) fn coop_pressure(&self, base: Pressure) -> Pressure {
        match &self.coop {
            Some(c) if c.is_client() => match &c.plan {
                Some(p) => Pressure {
                    enemies: p.enemies,
                    growth_cap: p.growth_cap,
                    ..base
                },
                None => base,
            },
            Some(c) => Pressure {
                enemies: base.enemies * coop::enemy_scale(c.players),
                ..base
            },
            None => base,
        }
    }

    /// Who the host's enemies can hunt: every living dreamer not waiting
    /// in the portal (empty outside co-op).
    pub(crate) fn coop_targets(&self) -> Vec<Vec3> {
        let Some(c) = self.coop.as_ref().filter(|c| c.is_host() && c.started) else {
            return Vec::new();
        };
        let mut v = Vec::new();
        if !c.ghost && !c.in_portal {
            v.push(self.player_position);
        }
        v.extend(
            c.poses
                .iter()
                .filter(|p| p.id != c.me && !p.ghost && !p.in_portal)
                .map(|p| Vec3::from(p.pos)),
        );
        v
    }

    /// Enemies stand still for the host while anyone is still picking.
    pub(crate) fn coop_hold(&self) -> bool {
        match &self.coop {
            Some(c) => match &c.net {
                CoopNet::Host(h) => h.peers.iter().any(|p| !p.ready),
                _ => false,
            },
            None => false,
        }
    }

    /// The HUD's co-op line.
    pub(crate) fn coop_line(&self) -> Option<String> {
        let c = self.coop.as_ref().filter(|c| c.started)?;
        let living = c.poses.iter().filter(|p| !p.ghost).count();
        let waiting = c.poses.iter().filter(|p| !p.ghost && p.in_portal).count();
        let mut line = format!("{} dreamers", c.poses.len().max(1));
        if c.ghost {
            let pct = (c.revives.progress(c.me) * 100.0) as u32;
            line += &if c.is_host() && pct > 0 {
                format!(" · you're a ghost ({pct}%)")
            } else {
                " · you're a ghost: stand near someone".to_string()
            };
        }
        if waiting > 0 {
            line += &format!(" · {waiting}/{living} in the portal");
        }
        Some(line)
    }

    pub(crate) fn coop_active(&self) -> bool {
        self.coop.as_ref().is_some_and(|c| c.started)
    }

    pub(crate) fn coop_client(&self) -> bool {
        self.coop
            .as_ref()
            .is_some_and(|c| c.started && c.is_client())
    }

    pub(crate) fn coop_ghost(&self) -> bool {
        self.coop.as_ref().is_some_and(|c| c.started && c.ghost)
    }

    /// Back in the dream after a pick.
    pub(crate) fn coop_ready(&mut self) {
        if let Some(c) = &mut self.coop {
            if let CoopNet::Client(cl) = &mut c.net {
                cl.send(&ToHost::Ready);
            }
        }
    }

    /// The host just descended: tell everyone, and wait for their picks.
    pub(crate) fn coop_broadcast_plan(&mut self) {
        let plan = self.coop_plan();
        if let Some(CoopState {
            net: CoopNet::Host(h),
            in_portal,
            ..
        }) = &mut self.coop
        {
            h.broadcast(&ToPeer::Dream(plan));
            for p in &mut h.peers {
                p.ready = false;
                p.pose.in_portal = false;
            }
            *in_portal = false;
        }
    }

    /// Stepped into the portal: wait there for the others.
    pub(crate) fn coop_enter_portal(&mut self) {
        if let Some(c) = &mut self.coop {
            if !c.in_portal {
                log::info!("Co-op: waiting in the portal");
                c.in_portal = true;
                self.eye_line = Some(("waiting for the others...".into(), 0.0));
            }
        }
    }

    /// Touched the shard: the host decides.
    pub(crate) fn coop_touch_shard(&mut self, ctx: &mut Context) {
        let depth = self.director.depth;
        match &mut self.coop {
            Some(CoopState {
                net: CoopNet::Client(cl),
                ..
            }) => {
                cl.send(&ToHost::Shard { depth });
                // Gone locally at once; the host's word adds the lucidity.
                self.despawn_shard_slot();
            }
            Some(CoopState {
                net: CoopNet::Host(_),
                me,
                ..
            }) => {
                let me = *me;
                self.host_shard_taken(ctx, me);
            }
            _ => {}
        }
    }

    fn host_shard_taken(&mut self, ctx: &mut Context, by: PlayerId) {
        if self.director.shard_this_dream || !self.director.has_shard {
            return;
        }
        self.complete_objective(ctx);
        let lucidity = self.director.lucidity;
        if let Some(CoopState {
            net: CoopNet::Host(h),
            ..
        }) = &mut self.coop
        {
            h.broadcast(&ToPeer::ShardTaken { by, lucidity });
        }
        log::info!("Co-op: shard taken by {by}");
    }

    /// Caught: a ghost until someone stands with you.
    pub(crate) fn coop_caught(&mut self) {
        let Some(c) = &mut self.coop else { return };
        if c.ghost {
            return;
        }
        c.ghost = true;
        let me = c.me;
        match &mut c.net {
            CoopNet::Client(cl) => cl.send(&ToHost::Caught),
            CoopNet::Host(h) => h.broadcast(&ToPeer::Caught(me)),
            CoopNet::Joining => {}
        }
        log::info!("Co-op: you're a ghost until someone stands with you");
        self.eye_line = Some(("you're a ghost. someone has to stand with you.".into(), 0.0));
        self.set_player_ghost(true);
    }

    fn set_player_ghost(&mut self, ghost: bool) {
        if let Some(p) = self.player {
            if ghost {
                let _ = self.world.insert_one(p, Translucent);
            } else {
                let _ = self.world.remove_one::<Translucent>(p);
            }
        }
    }

    fn revived(&mut self) {
        if let Some(c) = &mut self.coop {
            c.ghost = false;
        }
        self.set_player_ghost(false);
        self.grace = gameplay::RESPAWN_GRACE;
        self.flash.trigger([0.6, 1.0, 0.8], 0.5);
        self.sfx(Sound::Pick);
        log::info!("Co-op: revived");
    }

    /// Dev: DREAMSCAPE_COOP=host (starts once DREAMSCAPE_COOP_PLAYERS have
    /// joined, default 2) or DREAMSCAPE_COOP=<address> joins it. For
    /// headless two-process tests with the autopilot.
    pub(crate) fn coop_dev_start(&mut self) {
        let Ok(how) = crate::dev::var("DREAMSCAPE_COOP") else {
            return;
        };
        if how == "host" {
            self.open_host_lobby();
        } else {
            self.open_join_lobby();
            if let Some(c) = &mut self.coop {
                c.address = how;
            }
            self.lobby_key(Keycode::Return);
        }
    }

    fn coop_dev_autostart(&mut self) {
        let want: usize = crate::dev::var("DREAMSCAPE_COOP_PLAYERS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2);
        let ready = self.coop.as_ref().is_some_and(|c| {
            !c.started
                && matches!(&c.net, CoopNet::Host(h) if h.player_count() >= want && !h.started)
        });
        if ready && crate::dev::var("DREAMSCAPE_COOP").is_ok() {
            self.lobby_key(Keycode::Space);
        }
    }

    /// Once a frame, whatever the mode.
    pub(crate) fn coop_tick(&mut self, ctx: &mut Context, dt: f32) -> anyhow::Result<()> {
        self.coop_dev_autostart();
        let Some(c) = &mut self.coop else {
            return Ok(());
        };
        c.send_timer -= dt;
        let send_now = c.send_timer <= 0.0;
        if send_now {
            c.send_timer = 1.0 / coop::POSE_HZ;
        }
        let mine = Pose {
            id: c.me,
            pos: self.player_position.into(),
            yaw: self.yaw,
            ghost: c.ghost,
            in_portal: c.in_portal,
        };
        let mut todo: Vec<ToPeer> = Vec::new();
        let mut host_events = Vec::new();
        match &mut c.net {
            CoopNet::Joining => return Ok(()),
            CoopNet::Client(cl) => {
                todo = cl.poll();
                if let Some(me) = cl.me {
                    c.me = me;
                }
                if !cl.is_connected() {
                    let why = cl
                        .rejected
                        .clone()
                        .unwrap_or_else(|| "the host woke up".into());
                    log::warn!("Co-op: {why}");
                    let started = c.started;
                    self.coop = None;
                    if started {
                        self.eye_line = Some((format!("{why}. you dream on alone."), 0.0));
                    } else {
                        self.open_join_lobby();
                        if let Some(c) = &mut self.coop {
                            c.status = Some(why);
                        }
                    }
                    return Ok(());
                }
                if c.started && send_now {
                    cl.send(&ToHost::Pose(mine));
                }
            }
            CoopNet::Host(h) => {
                host_events = h.poll();
            }
        }
        // The host: its own pose joins the list; poses and enemies go out.
        if c.is_host() {
            for e in &host_events {
                match e {
                    HostEvent::Joined(id, name) => log::info!("Co-op: {name} joined ({id})"),
                    HostEvent::Left(id) => {
                        log::info!("Co-op: dreamer {id} left");
                        if let Some(i) = c.avatars.iter().position(|(a, _)| a == id) {
                            let (_, e) = c.avatars.remove(i);
                            let _ = self.world.despawn(e);
                        }
                    }
                    _ => {}
                }
            }
            let CoopNet::Host(h) = &mut c.net else {
                unreachable!()
            };
            let mut poses: Vec<Pose> = vec![mine];
            poses.extend(h.peers.iter().map(|p| p.pose));
            if c.started {
                for id in c.revives.tick(&poses, dt) {
                    if let Some(p) = h.peers.iter_mut().find(|p| p.id == id) {
                        p.pose.ghost = false;
                    }
                    h.broadcast(&ToPeer::Revived(id));
                    if id == c.me {
                        todo.push(ToPeer::Revived(id));
                    }
                }
                for e in &host_events {
                    if let HostEvent::Caught(id) = e {
                        if let Some(p) = h.peers.iter_mut().find(|p| p.id == *id) {
                            p.pose.ghost = true;
                        }
                        h.broadcast(&ToPeer::Caught(*id));
                    }
                }
                let mut poses: Vec<Pose> = vec![mine];
                poses.extend(h.peers.iter().map(|p| p.pose));
                if send_now {
                    h.broadcast(&ToPeer::Poses(poses.clone()));
                }
                c.poses = poses;
            }
        } else if let Some(ToPeer::Poses(p)) =
            todo.iter().rev().find(|m| matches!(m, ToPeer::Poses(_)))
        {
            c.poses = p.clone();
        }
        let shards: Vec<(PlayerId, u32)> = host_events
            .iter()
            .filter_map(|e| match e {
                HostEvent::Shard { by, depth } => Some((*by, *depth)),
                _ => None,
            })
            .collect();
        let started = c.started;
        let is_host = c.is_host();
        for (by, depth) in shards {
            if depth == self.director.depth {
                self.host_shard_taken(ctx, by);
            }
        }
        for m in todo {
            self.coop_message(ctx, m)?;
        }
        if !started {
            return Ok(());
        }
        self.sync_avatars(ctx)?;
        if is_host && self.mode == hud::Mode::Playing && !self.transition.active() {
            self.host_rules(send_now);
        }
        Ok(())
    }

    fn coop_message(&mut self, ctx: &mut Context, m: ToPeer) -> anyhow::Result<()> {
        match m {
            ToPeer::Start {
                run_seed,
                long,
                players,
            } => self.pending_coop_start = Some((run_seed, long, players as usize)),
            ToPeer::Dream(plan) => {
                let waking = plan.waking;
                if let Some(c) = &mut self.coop {
                    c.plan = Some(plan);
                    c.in_portal = false;
                }
                self.begin_melt(if waking {
                    transition::Pending::Wake
                } else {
                    transition::Pending::Descend
                });
            }
            ToPeer::Enemies { pacers, hunter } => {
                if let Some(c) = &mut self.coop {
                    c.enemies = Some((pacers, hunter));
                }
            }
            ToPeer::ShardTaken { lucidity, .. } => {
                if !self.director.shard_this_dream && self.director.has_shard {
                    self.complete_objective(ctx);
                }
                self.director.lucidity = lucidity;
            }
            ToPeer::Caught(id) | ToPeer::Revived(id)
                if Some(id) != self.coop.as_ref().map(|c| c.me) => {}
            ToPeer::Caught(_) => {
                if !self.coop_ghost() {
                    self.coop_caught();
                }
            }
            ToPeer::Revived(_) => self.revived(),
            ToPeer::RunOver => {
                log::info!("Co-op: the run is over");
                if let Some(c) = &mut self.coop {
                    c.started = false;
                }
                self.begin_melt(transition::Pending::Finish);
            }
            _ => {}
        }
        Ok(())
    }

    /// The host: the portal, the end, and the enemy snapshot.
    fn host_rules(&mut self, send_now: bool) {
        let Some(c) = &self.coop else { return };
        if coop::all_ghosts(&c.poses) {
            self.coop_run_over();
            return;
        }
        if coop::all_in_portal(&c.poses) {
            log::info!("Co-op: everyone's in the portal");
            if self.director.theme == DreamTheme::Awakening {
                self.coop_run_over();
            } else {
                self.begin_melt(transition::Pending::Descend);
            }
            if let Some(c) = &mut self.coop {
                c.in_portal = false;
            }
            return;
        }
        if send_now {
            let pacers: Vec<[f32; 4]> = self
                .enemies
                .iter()
                .map(|p| {
                    self.world
                        .get::<&Transform>(p.entity)
                        .map_or([0.0; 4], |t| {
                            [t.position.x, t.position.y, t.position.z, t.scale.x]
                        })
                })
                .collect();
            let hunter = self.hunter.as_ref().and_then(|(e, _, _)| {
                self.world
                    .get::<&Transform>(*e)
                    .ok()
                    .map(|t| [t.position.x, t.position.y, t.position.z, t.scale.x])
            });
            if let Some(CoopState {
                net: CoopNet::Host(h),
                ..
            }) = &mut self.coop
            {
                h.broadcast(&ToPeer::Enemies { pacers, hunter });
            }
        }
    }

    fn coop_run_over(&mut self) {
        if let Some(CoopState {
            net: CoopNet::Host(h),
            started,
            ..
        }) = &mut self.coop
        {
            h.broadcast(&ToPeer::RunOver);
            *started = false;
        }
        self.begin_melt(transition::Pending::Finish);
    }

    /// A client puts the host's enemies where the host says.
    pub(crate) fn coop_apply_enemies(&mut self) {
        let Some((pacers, hunter)) = self.coop.as_mut().and_then(|c| c.enemies.take()) else {
            return;
        };
        for (k, e) in pacers.iter().enumerate() {
            let Some(p) = self.enemies.get(k) else { break };
            if let Ok(mut t) = self.world.get::<&mut Transform>(p.entity) {
                t.position = Vec3::new(e[0], e[1], e[2]);
                t.scale = Vec3::splat(e[3]);
            }
        }
        if let (Some(h), Some((e, _, _))) = (hunter, self.hunter.as_ref()) {
            if let Ok(mut t) = self.world.get::<&mut Transform>(*e) {
                t.position = Vec3::new(h[0], h[1], h[2]);
                t.scale = Vec3::splat(h[3]);
            }
        }
    }

    /// The others, drawn where they stand (see-through when they're ghosts).
    fn sync_avatars(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        let Some(c) = &self.coop else { return Ok(()) };
        let me = c.me;
        let others: Vec<Pose> = c.poses.iter().filter(|p| p.id != me).copied().collect();
        let mut avatars = std::mem::take(&mut self.coop.as_mut().unwrap().avatars);
        avatars.retain(|(id, e)| {
            let keep = self.world.contains(*e) && others.iter().any(|p| p.id == *id);
            if !keep {
                let _ = self.world.despawn(*e);
            }
            keep
        });
        for p in &others {
            let e = match avatars.iter().find(|(id, _)| *id == p.id) {
                Some(&(_, e)) => e,
                None => {
                    let [r, g, b] = player_color(p.id);
                    let tex = self.texture(ctx.gl(), [r, g, b, 255]);
                    let e = self.world.spawn((
                        Transform {
                            position: Vec3::from(p.pos),
                            rotation: Quat::IDENTITY,
                            scale: Vec3::new(0.9, 1.25, 0.9),
                        },
                        MeshRenderer {
                            mesh: self.mesh(Shape::Octahedron)?,
                            texture: Some(tex),
                        },
                        Spin(1.5),
                        Lit,
                        Hero,
                    ));
                    avatars.push((p.id, e));
                    e
                }
            };
            if let Ok(mut t) = self.world.get::<&mut Transform>(e) {
                t.position = t.position.lerp(Vec3::from(p.pos), 0.5);
                t.scale = if p.in_portal {
                    Vec3::ZERO
                } else {
                    Vec3::new(0.9, 1.25, 0.9)
                };
            }
            let ghostly = self.world.get::<&Translucent>(e).is_ok();
            if p.ghost && !ghostly {
                let _ = self.world.insert_one(e, Translucent);
            } else if !p.ghost && ghostly {
                let _ = self.world.remove_one::<Translucent>(e);
            }
        }
        if let Some(c) = &mut self.coop {
            c.avatars = avatars;
        }
        Ok(())
    }
}

/// The character a key types into the address field.
fn address_char(k: Keycode) -> Option<char> {
    use Keycode as K;
    Some(match k {
        K::Num0 | K::Kp0 => '0',
        K::Num1 | K::Kp1 => '1',
        K::Num2 | K::Kp2 => '2',
        K::Num3 | K::Kp3 => '3',
        K::Num4 | K::Kp4 => '4',
        K::Num5 | K::Kp5 => '5',
        K::Num6 | K::Kp6 => '6',
        K::Num7 | K::Kp7 => '7',
        K::Num8 | K::Kp8 => '8',
        K::Num9 | K::Kp9 => '9',
        K::Period | K::KpPeriod => '.',
        K::Semicolon => ':',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_type_digits_dots_and_a_colon() {
        assert_eq!(address_char(Keycode::Num7), Some('7'));
        assert_eq!(address_char(Keycode::Period), Some('.'));
        assert_eq!(address_char(Keycode::Semicolon), Some(':'));
        assert_eq!(address_char(Keycode::A), None);
    }

    #[test]
    fn every_dreamer_has_their_own_colour() {
        let cs: Vec<_> = (0..coop::MAX_PLAYERS as u8).map(player_color).collect();
        for (i, a) in cs.iter().enumerate() {
            for b in &cs[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
