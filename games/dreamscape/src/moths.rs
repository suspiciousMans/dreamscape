//! MOTHS: four pale flyers that ring a pocket and dive one at a time.
//!
//! Pure rules, no GL and no clock. Every [`DIVE_EVERY`] seconds the next moth
//! in round-robin order brightens for [`TELEGRAPH`] (the dreamer sees where it
//! is looking: its dive aims at where you stood when the tell began), then
//! dives at your speed, and flies home. Never more than one telegraphs or
//! dives at a time. STILLNESS freezes the whole ring; GUST scatters it.

use crate::dream::DreamTheme;
use engine::glam::Vec3;

/// First depth they appear at.
pub const MIN_DEPTH: u32 = 6;
pub const GROUP: usize = 4;
pub const HP: f32 = 1.0;
/// Loose ring radius around the pocket, in cells (see `gameplay::CELL`).
pub const ORBIT_MIN_CELLS: f32 = 2.0;
pub const ORBIT_MAX_CELLS: f32 = 3.0;
/// Orbit height above the pocket's floor, and radians per second round it.
pub const ORBIT_HEIGHT: f32 = 1.6;
pub const ORBIT_SPEED: f32 = 0.6;
/// Seconds between the starts of two dives.
pub const DIVE_EVERY: f32 = 2.5;
/// The brighten before a dive.
pub const TELEGRAPH: f32 = 0.5;
/// Dive and return speed, as a multiple of the player's.
pub const DIVE_SPEED_MULT: f32 = 1.0;
/// A diving moth this close to you has touched you.
pub const TOUCH: f32 = 0.85;
/// A moth that touched you keeps counting as a touch this long (seconds), so
/// the game's fail-state check cannot miss it between two frames.
pub const CONTACT_HOLD: f32 = 0.1;
/// Gust: a scattered moth cannot act for this long, and is flung this far.
pub const SCATTER_SECS: f32 = 1.0;
pub const SCATTER_PUSH: f32 = 4.0;
/// Within this of its ring slot a returning moth rejoins the ring.
const HOME: f32 = 0.3;

/// One placement seed per moth group, or none: below [`MIN_DEPTH`], in
/// nightmares, and never more than one group per dream. Its own RNG stream.
pub fn spawn_plan(depth: u32, theme: DreamTheme, seed: u64, nightmare: bool) -> Vec<u64> {
    let _ = theme; // every theme has moths for now
    if nightmare || depth < MIN_DEPTH {
        return Vec::new();
    }
    vec![seed ^ STREAM]
}

pub const STREAM: u64 = 0x30_7A5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// On the ring.
    Orbit,
    /// Brightening: seconds left, and the spot it will dive at.
    Telegraph(f32, Vec3),
    /// Diving at that spot.
    Dive(Vec3),
    /// Flying back to the ring.
    Return,
    /// Flung by a gust: seconds left, and the drift (units / s).
    Scattered(f32, Vec3),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moth {
    pub id: usize,
    pub pos: Vec3,
    pub mode: Mode,
}

impl Moth {
    /// 0..1: how bright the tell is (for the tint and the scale pulse).
    pub fn glow(&self) -> f32 {
        match self.mode {
            Mode::Telegraph(left, _) => (1.0 - left / TELEGRAPH).clamp(0.0, 1.0),
            Mode::Dive(_) => 1.0,
            _ => 0.0,
        }
    }

    pub fn busy(&self) -> bool {
        matches!(self.mode, Mode::Telegraph(..) | Mode::Dive(_))
    }
}

#[derive(Clone, Debug)]
pub struct Swarm {
    pub center: Vec3,
    t: f32,
    timer: f32,
    cursor: usize,
    contact: f32,
    pub moths: Vec<Moth>,
    /// Ids that started a dive, in order (for tests and the log).
    pub dives: Vec<usize>,
}

impl Swarm {
    /// A group of [`GROUP`] moths ringing `center` (a pocket's floor cell).
    pub fn new(center: Vec3) -> Self {
        let mut s = Self {
            center,
            t: 0.0,
            timer: DIVE_EVERY,
            cursor: 0,
            contact: 0.0,
            moths: Vec::new(),
            dives: Vec::new(),
        };
        s.moths = (0..GROUP)
            .map(|id| Moth { id, pos: s.slot(id), mode: Mode::Orbit })
            .collect();
        s
    }

    /// Where moth `id` sits on the ring right now: a loose ring, so the
    /// radius breathes between 2 and 3 cells and each moth drifts on its own.
    pub fn slot(&self, id: usize) -> Vec3 {
        let ang = self.t * ORBIT_SPEED + id as f32 * std::f32::consts::TAU / GROUP as f32;
        let breathe = 0.5 + 0.5 * (self.t * 0.7 + id as f32 * 1.7).sin();
        let cells = ORBIT_MIN_CELLS + (ORBIT_MAX_CELLS - ORBIT_MIN_CELLS) * breathe;
        let r = cells * crate::gameplay::CELL;
        self.center + Vec3::new(ang.sin() * r, ORBIT_HEIGHT, ang.cos() * r)
    }

    /// Moths not telegraphing or diving right now.
    pub fn diving(&self) -> usize {
        self.moths.iter().filter(|m| m.busy()).count()
    }

    /// A dive just touched the dreamer (true for [`CONTACT_HOLD`] seconds).
    pub fn connected(&self) -> bool {
        self.contact > 0.0
    }

    pub fn alive(&self) -> usize {
        self.moths.len()
    }

    /// One STRIKE kills one: remove moth `id`.
    pub fn kill(&mut self, id: usize) {
        self.moths.retain(|m| m.id != id);
    }

    /// Gust: every moth within `radius` of `from` is flung away and cannot act
    /// for [`SCATTER_SECS`]. A scattered diver forgets its dive.
    pub fn scatter(&mut self, from: Vec3, radius: f32) {
        for m in &mut self.moths {
            let d = m.pos - from;
            let flat = Vec3::new(d.x, 0.0, d.z);
            if flat.length() < radius {
                let away = if flat.length() < 1e-3 { Vec3::X } else { flat.normalize() };
                m.mode = Mode::Scattered(SCATTER_SECS, away * SCATTER_PUSH);
            }
        }
    }

    /// Advances by `dt`. `frozen` (STILLNESS) stops everything, the ring too.
    pub fn update(&mut self, dt: f32, player: Vec3, player_speed: f32, frozen: bool) {
        if frozen {
            return;
        }
        self.contact = (self.contact - dt).max(0.0);
        self.t += dt;
        self.timer = (self.timer - dt).max(0.0);
        let speed = player_speed * DIVE_SPEED_MULT;
        for k in 0..self.moths.len() {
            let slot = self.slot(self.moths[k].id);
            let m = &mut self.moths[k];
            match m.mode {
                Mode::Orbit => m.pos = slot,
                Mode::Telegraph(left, target) => {
                    let left = left - dt;
                    m.mode = if left <= 0.0 { Mode::Dive(target) } else { Mode::Telegraph(left, target) };
                }
                Mode::Dive(target) => {
                    m.pos = step_toward(m.pos, target, speed * dt);
                    if m.pos.distance(player) < TOUCH {
                        self.contact = CONTACT_HOLD;
                        m.mode = Mode::Return;
                    } else if m.pos.distance(target) < 1e-3 {
                        m.mode = Mode::Return;
                    }
                }
                Mode::Return => {
                    m.pos = step_toward(m.pos, slot, speed * dt);
                    if m.pos.distance(slot) < HOME {
                        m.mode = Mode::Orbit;
                    }
                }
                Mode::Scattered(left, drift) => {
                    m.pos += drift * (left / SCATTER_SECS) * dt;
                    let left = left - dt;
                    m.mode = if left <= 0.0 { Mode::Return } else { Mode::Scattered(left, drift) };
                }
            }
        }
        // The next dive: nobody else telegraphing or diving, the timer out.
        if self.timer <= 0.0 && self.diving() == 0 {
            let n = self.moths.len();
            let next = (0..GROUP)
                .map(|o| (self.cursor + o) % GROUP)
                .find(|id| n > 0 && self.moths.iter().any(|m| m.id == *id && m.mode == Mode::Orbit));
            if let Some(id) = next {
                self.cursor = (id + 1) % GROUP;
                self.timer = DIVE_EVERY;
                self.dives.push(id);
                if let Some(m) = self.moths.iter_mut().find(|m| m.id == id) {
                    m.mode = Mode::Telegraph(TELEGRAPH, player);
                }
            }
        }
    }
}

fn step_toward(from: Vec3, to: Vec3, by: f32) -> Vec3 {
    let d = to - from;
    if d.length() <= by {
        to
    } else {
        from + d.normalize() * by
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::ALL_THEMES;
    use crate::gameplay::{CELL, MOVE_SPEED};

    const DT: f32 = 1.0 / 60.0;

    fn run(s: &mut Swarm, secs: f32, player: Vec3) {
        let mut t = 0.0;
        while t < secs {
            s.update(DT, player, MOVE_SPEED, false);
            t += DT;
        }
    }

    #[test]
    fn the_constants_are_the_briefs() {
        assert_eq!(MIN_DEPTH, 6);
        assert_eq!(GROUP, 4);
        assert_eq!(HP, 1.0);
        assert_eq!((ORBIT_MIN_CELLS, ORBIT_MAX_CELLS), (2.0, 3.0));
        assert_eq!(DIVE_EVERY, 2.5);
        assert_eq!(TELEGRAPH, 0.5);
        assert_eq!(DIVE_SPEED_MULT, 1.0);
        assert_eq!(SCATTER_SECS, 1.0);
        assert_eq!(crate::combat::base_hp(crate::combat::Kind::Moth), 1.0);
    }

    #[test]
    fn the_ring_stays_between_two_and_three_cells() {
        let s0 = Swarm::new(Vec3::ZERO);
        let mut s = s0.clone();
        for _ in 0..600 {
            s.update(DT, Vec3::new(500.0, 0.0, 0.0), MOVE_SPEED, false);
            for m in s.moths.iter().filter(|m| m.mode == Mode::Orbit) {
                let r = Vec3::new(m.pos.x, 0.0, m.pos.z).length();
                assert!(r >= 2.0 * CELL - 0.01 && r <= 3.0 * CELL + 0.01, "radius {r}");
                assert!((m.pos.y - ORBIT_HEIGHT).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn dives_go_round_robin_every_2_5_seconds() {
        let mut s = Swarm::new(Vec3::ZERO);
        // The player stands at the pocket: each dive is short, so the 2.5s
        // clock (not the dive) sets the pace.
        run(&mut s, 2.5 * 5.0 + 0.2, Vec3::new(0.0, 0.5, 0.0));
        assert_eq!(&s.dives[..5], &[0, 1, 2, 3, 0], "{:?}", s.dives);
    }

    #[test]
    fn the_first_dive_waits_2_5_seconds_and_telegraphs_half_a_second() {
        let mut s = Swarm::new(Vec3::ZERO);
        run(&mut s, 2.4, Vec3::new(20.0, 0.0, 0.0));
        assert!(s.dives.is_empty());
        run(&mut s, 0.2, Vec3::new(20.0, 0.0, 0.0));
        assert_eq!(s.dives, vec![0]);
        let m = s.moths[0];
        assert!(matches!(m.mode, Mode::Telegraph(..)), "{:?}", m.mode);
        assert!(m.glow() < 0.5);
        run(&mut s, 0.5, Vec3::new(20.0, 0.0, 0.0));
        assert!(matches!(s.moths[0].mode, Mode::Dive(_)), "{:?}", s.moths[0].mode);
        assert_eq!(s.moths[0].glow(), 1.0);
    }

    #[test]
    fn a_dead_moth_is_skipped_in_the_order() {
        let mut s = Swarm::new(Vec3::ZERO);
        s.kill(1);
        assert_eq!(s.alive(), 3);
        run(&mut s, 2.5 * 4.0 + 0.2, Vec3::new(0.0, 0.5, 0.0));
        assert_eq!(&s.dives[..4], &[0, 2, 3, 0], "{:?}", s.dives);
    }

    #[test]
    fn never_more_than_one_moth_telegraphs_or_dives() {
        let mut s = Swarm::new(Vec3::ZERO);
        // A far target makes dives long, which is when overlap would happen.
        let far = Vec3::new(40.0, 0.0, 0.0);
        let mut seen = 0;
        for _ in 0..(60 * 60) {
            s.update(DT, far, MOVE_SPEED, false);
            assert!(s.diving() <= 1, "{:?}", s.moths);
            seen = seen.max(s.diving());
        }
        assert_eq!(seen, 1, "it did dive");
        assert!(s.dives.len() >= 8);
    }

    #[test]
    fn a_dive_aims_where_you_stood_at_the_start_of_the_tell() {
        let mut s = Swarm::new(Vec3::ZERO);
        let at = Vec3::new(5.0, 0.5, 0.0);
        run(&mut s, 2.55, at);
        let Mode::Telegraph(_, target) = s.moths[0].mode else { panic!("{:?}", s.moths[0].mode) };
        assert_eq!(target, at);
        // The player walks off: the moth still goes for the old spot.
        let away = Vec3::new(30.0, 0.5, 30.0);
        run(&mut s, 0.5 + 0.05, away);
        let Mode::Dive(t) = s.moths[0].mode else { panic!("{:?}", s.moths[0].mode) };
        assert_eq!(t, at);
    }

    #[test]
    fn a_dive_moves_at_exactly_the_players_speed() {
        let mut s = Swarm::new(Vec3::ZERO);
        let far = Vec3::new(60.0, 0.0, 0.0);
        run(&mut s, 3.1, far);
        let a = s.moths[0].pos;
        assert!(matches!(s.moths[0].mode, Mode::Dive(_)));
        s.update(0.1, far, MOVE_SPEED, false);
        let moved = s.moths[0].pos.distance(a);
        assert!((moved - MOVE_SPEED * 0.1).abs() < 1e-3, "moved {moved}");
    }

    #[test]
    fn a_dive_that_reaches_you_connects_and_the_moth_heads_home() {
        let mut s = Swarm::new(Vec3::ZERO);
        let me = Vec3::new(4.0, 1.0, 0.0);
        let mut touched = false;
        for _ in 0..(60 * 6) {
            s.update(DT, me, MOVE_SPEED, false);
            if s.connected() {
                touched = true;
                assert_eq!(s.moths[0].mode, Mode::Return);
                break;
            }
        }
        assert!(touched);
        run(&mut s, CONTACT_HOLD + 0.05, me);
        assert!(!s.connected(), "the touch is brief");
    }

    #[test]
    fn stillness_freezes_the_ring_and_the_timers() {
        let mut s = Swarm::new(Vec3::ZERO);
        run(&mut s, 1.0, Vec3::new(20.0, 0.0, 0.0));
        let before = s.clone().moths;
        for _ in 0..600 {
            s.update(DT, Vec3::new(20.0, 0.0, 0.0), MOVE_SPEED, true);
        }
        assert_eq!(s.moths, before);
        assert!(s.dives.is_empty());
        // Thawed, the 2.5s clock carries on from where it stopped.
        run(&mut s, 1.6, Vec3::new(20.0, 0.0, 0.0));
        assert_eq!(s.dives, vec![0]);
    }

    #[test]
    fn a_gust_scatters_cancels_dives_and_the_moths_come_back() {
        let mut s = Swarm::new(Vec3::ZERO);
        run(&mut s, 2.8, Vec3::new(20.0, 0.0, 0.0));
        assert_eq!(s.diving(), 1);
        s.scatter(Vec3::ZERO, 50.0);
        assert_eq!(s.diving(), 0, "the diver forgot");
        assert!(s.moths.iter().all(|m| matches!(m.mode, Mode::Scattered(..))));
        let flung = s.moths[2].pos;
        run(&mut s, 0.5, Vec3::new(20.0, 0.0, 0.0));
        assert_ne!(s.moths[2].pos, flung);
        assert!(s.moths.iter().all(|m| matches!(m.mode, Mode::Scattered(..))), "still reeling at 0.5s");
        assert_eq!(s.diving(), 0, "no dives while reeling");
        run(&mut s, 0.6, Vec3::new(20.0, 0.0, 0.0));
        assert!(s.moths.iter().all(|m| !matches!(m.mode, Mode::Scattered(..))));
        run(&mut s, 6.0, Vec3::new(20.0, 0.0, 0.0));
        assert!(s.moths.iter().filter(|m| m.mode == Mode::Orbit).count() >= 3, "back on the ring");
    }

    #[test]
    fn a_gust_only_reaches_moths_within_its_radius() {
        let mut s = Swarm::new(Vec3::ZERO);
        s.scatter(Vec3::new(100.0, 0.0, 0.0), 5.0);
        assert!(s.moths.iter().all(|m| m.mode == Mode::Orbit));
    }

    #[test]
    fn spawn_plan_gates_by_depth_nightmare_and_caps_at_one_group() {
        let theme = ALL_THEMES[0];
        assert!(spawn_plan(5, theme, 7, false).is_empty());
        assert!(spawn_plan(0, theme, 7, false).is_empty());
        assert!(spawn_plan(40, theme, 7, true).is_empty(), "nightmares have none");
        for depth in 6..30 {
            for &t in &ALL_THEMES {
                let plan = spawn_plan(depth, t, depth as u64, false);
                assert_eq!(plan.len(), 1);
                assert_eq!(plan, spawn_plan(depth, t, depth as u64, false));
            }
        }
        assert_ne!(spawn_plan(6, theme, 1, false), spawn_plan(6, theme, 2, false));
    }
}
