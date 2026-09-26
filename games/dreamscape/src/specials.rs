//! Special enemies and elite pacers. Each kind breaks the "time the pacer"
//! rhythm in its own way, and each is built so that a patient player is
//! still never walled in:
//! - the **stalker** only moves while out of sight, and a frozen one is
//!   harmless to touch (it shatters back to its lair);
//! - the **mimic** walks your own path a few seconds behind you;
//! - the **sentry** never touches you, it only calls the pacers;
//! - the **drifter** swings across a jump and is off the line half the time;
//! - the **jester** doesn't catch you, it throws you onto safe floor.
//!
//! Everything here is pure and unit-tested.

use crate::gameplay::CELL;
use engine::glam::Vec3;
use std::collections::VecDeque;

fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

fn step_toward(pos: &mut Vec3, target: Vec3, speed: f32, dt: f32) {
    let to = flat(target - *pos);
    let step = (speed * dt).min(to.length());
    *pos += to.normalize_or_zero() * step;
}

/// Stalkers creep at this fraction of your speed.
pub const STALKER_SPEED: f32 = 0.7;
/// Anything this far behind you (toward the camera, -Z) is out of view.
pub const BEHIND: f32 = 1.5;

/// In view: inside the clear part of the tunnel vision and not behind you.
pub fn in_view(pos: Vec3, player: Vec3, clear: f32) -> bool {
    flat(pos - player).length() < clear && pos.z >= player.z - BEHIND
}

#[derive(Clone, Copy, Debug)]
pub struct Stalker {
    pub lair: Vec3,
    speed: f32,
    /// It moved this frame (only a moving stalker can catch you).
    pub moving: bool,
}

impl Stalker {
    pub fn new(lair: Vec3, player_speed: f32) -> Self {
        Self {
            lair,
            speed: STALKER_SPEED * player_speed,
            moving: false,
        }
    }

    pub fn update(&mut self, pos: &mut Vec3, player: Vec3, clear: f32, dt: f32) {
        self.moving = !in_view(*pos, player, clear);
        if self.moving {
            step_toward(pos, player, self.speed, dt);
        }
    }
}

/// Timestamped positions of the player, oldest first.
#[derive(Clone, Debug, Default)]
pub struct Trail {
    samples: VecDeque<(f32, Vec3)>,
}

/// Seconds of trail kept (well past the mimic's delay).
pub const TRAIL_KEEP: f32 = 8.0;
pub const MIMIC_DELAY: f32 = 3.0;

impl Trail {
    pub fn clear(&mut self) {
        self.samples.clear();
    }

    pub fn push(&mut self, t: f32, pos: Vec3) {
        self.samples.push_back((t, pos));
        while self
            .samples
            .front()
            .is_some_and(|&(old, _)| old < t - TRAIL_KEEP)
        {
            self.samples.pop_front();
        }
    }

    /// Where the player was at time `t` (the last sample at or before it).
    pub fn at(&self, t: f32) -> Option<Vec3> {
        self.samples
            .iter()
            .rev()
            .find(|&&(st, _)| st <= t)
            .map(|&(_, p)| p)
    }
}

/// Where the mimic is at time `now`: exactly where you were `MIMIC_DELAY`
/// seconds ago, once awake.
pub fn mimic_pos(trail: &Trail, now: f32, awake_at: f32) -> Option<Vec3> {
    if now < awake_at {
        return None;
    }
    trail.at(now - MIMIC_DELAY)
}

pub const SENTRY_RANGE: f32 = 2.5 * CELL;
pub const SENTRY_HALF_ANGLE: f32 = 25.0 * std::f32::consts::PI / 180.0;
/// Radians per second.
pub const SENTRY_TURN: f32 = 0.9;
/// Pacers within this of a sentry answer its call.
pub const SENTRY_CALL: f32 = 2.0 * CELL;
/// A sentry can't call again for this long.
pub const SENTRY_COOLDOWN: f32 = 4.0;

#[derive(Clone, Copy, Debug)]
pub struct Sentry {
    pub angle: f32,
    pub cooldown: f32,
}

impl Sentry {
    pub fn new(phase: f32) -> Self {
        Self {
            angle: phase,
            cooldown: 0.0,
        }
    }

    pub fn facing(&self) -> Vec3 {
        Vec3::new(self.angle.sin(), 0.0, self.angle.cos())
    }

    /// Turns the beam; true when it has just spotted the player.
    pub fn update(&mut self, at: Vec3, player: Vec3, dt: f32) -> bool {
        self.angle = (self.angle + SENTRY_TURN * dt).rem_euclid(std::f32::consts::TAU);
        self.cooldown = (self.cooldown - dt).max(0.0);
        if self.cooldown <= 0.0 && sees(at, self.facing(), player) {
            self.cooldown = SENTRY_COOLDOWN;
            return true;
        }
        false
    }
}

/// Is `target` inside the beam from `at` pointing along `facing`?
pub fn sees(at: Vec3, facing: Vec3, target: Vec3) -> bool {
    let to = flat(target - at);
    let d = to.length();
    d < SENTRY_RANGE && (d < 1e-3 || to.normalize().dot(facing) >= SENTRY_HALF_ANGLE.cos())
}

/// A jester touch throws you this many cells.
pub const JESTER_THROW: f32 = 2.0;
/// ...and then it can't do it again for a while.
pub const JESTER_COOLDOWN: f32 = 3.0;

/// The eight directions a jester can throw you, starting from `seed`.
pub fn throw_directions(seed: u32) -> Vec<Vec3> {
    (0..8)
        .map(|k| {
            let a = std::f32::consts::TAU * ((k + seed) % 8) as f32 / 8.0;
            Vec3::new(a.cos(), 0.0, a.sin())
        })
        .collect()
}

/// Tunnel gates: one full open-close cycle, and how long of it they're shut.
pub const GATE_PERIOD: f32 = 3.2;
pub const GATE_CLOSED: f32 = 1.2;

/// Is a gate with this phase shut at time `t`? Shut less than half the time.
pub fn gate_closed(t: f32, phase: f32) -> bool {
    (t + phase).rem_euclid(GATE_PERIOD) < GATE_CLOSED
}

/// How far a shifting tile has sunk at time `t` (0 = level, 1 = lowest).
pub fn shift_depth(t: f32, phase: f32) -> f32 {
    0.5 - 0.5 * (t * std::f32::consts::TAU / 4.0 + phase).cos()
}
/// The lowest a shifting tile sinks (you can always hop back up).
pub const SHIFT_DROP: f32 = 0.7;

/// Fog pockets slow you to this fraction of your speed.
pub const FOG_SLOW: f32 = 0.6;

/// Mycelium: walking on a vein speeds you up by this much.
pub const VEIN_BOOST: f32 = 1.2;

/// Distance on the floor plane from `p` to the segment `a`-`b`.
pub fn dist_to_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let (p, a, b) = (flat(p), flat(a), flat(b));
    let ab = b - a;
    let t = if ab.length_squared() < 1e-6 {
        0.0
    } else {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    };
    (p - (a + ab * t)).length()
}

/// Elite pacers, in long or deepened runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Elite {
    /// Faster (still slower than you).
    Fast,
    /// Bigger reach.
    Big,
    /// Splits in two the first time it lunges.
    Splitter,
    /// Invisible until it's close.
    Shade,
}

pub const ALL_ELITES: [Elite; 4] = [Elite::Fast, Elite::Big, Elite::Splitter, Elite::Shade];
pub const ELITE_FAST: f32 = 1.25;
pub const ELITE_BIG_REACH: f32 = 1.6;
pub const SHADE_SHOWS_WITHIN: f32 = 2.0 * CELL;

impl Elite {
    /// Tint for its eye texture (the tell).
    pub fn color(self) -> [u8; 3] {
        match self {
            Elite::Fast => [255, 200, 40],
            Elite::Big => [255, 60, 40],
            Elite::Splitter => [60, 255, 120],
            Elite::Shade => [120, 60, 255],
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Elite::Fast => "FAST",
            Elite::Big => "BIG",
            Elite::Splitter => "SPLITTER",
            Elite::Shade => "SHADE",
        }
    }
}

/// Chance a pacer is elite at this difficulty (none in gentle runs).
pub fn elite_chance(difficulty: f32) -> f32 {
    (0.12 * difficulty.max(1.0).log2()).clamp(0.0, 0.5)
}

/// Deterministic elite roll for pacer `k` of a dream.
pub fn roll_elite(seed: u64, k: usize, difficulty: f32) -> Option<Elite> {
    let h = seed
        .wrapping_add(k as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .rotate_left(17);
    let roll = (h >> 40) as f32 / (1u64 << 24) as f32;
    (roll < elite_chance(difficulty)).then(|| ALL_ELITES[(h % 4) as usize])
}

/// Synesthesia Hall: route tiles burn on the first beat of every two.
#[allow(dead_code)]
pub const BEAT_HOT: f32 = 0.3;
#[allow(dead_code)]
pub fn beat_hot(t: f32, bpm: f32) -> bool {
    if bpm <= 0.0 {
        return false;
    }
    let bar = 120.0 / bpm;
    t.rem_euclid(bar) / bar < BEAT_HOT
}

/// Melting Clockworks: the dream rewinds every LOOP_SECS.
#[allow(dead_code)]
pub const LOOP_SECS: f32 = 20.0;
#[allow(dead_code)]
pub fn loop_index(age: f32) -> u32 {
    (age.max(0.0) / LOOP_SECS) as u32
}

/// Afterimage Fields: a ghost of you every ECHO_EVERY seconds, fading over ECHO_LIFE.
#[allow(dead_code)]
pub const ECHO_EVERY: f32 = 0.18;
#[allow(dead_code)]
pub const ECHO_LIFE: f32 = 1.4;
#[allow(dead_code)]
pub const MAX_ECHOES: usize = 10;
#[allow(dead_code)]
pub fn echo_scale(age: f32) -> f32 {
    (1.0 - age / ECHO_LIFE).clamp(0.0, 1.0)
}

/// Watching Wallpaper: the yaw that turns an eye at `at` toward `target`.
#[allow(dead_code)]
pub fn watcher_yaw(at: Vec3, target: Vec3) -> f32 {
    let d = target - at;
    d.x.atan2(d.z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay::MOVE_SPEED;

    #[test]
    fn a_stalker_never_moves_while_you_can_see_it() {
        let clear = 6.0;
        let player = Vec3::ZERO;
        let mut s = Stalker::new(Vec3::new(0.0, 0.9, 20.0), MOVE_SPEED);
        let mut pos = Vec3::new(0.0, 0.9, 20.0);
        // Far ahead: out of view, so it creeps in...
        for _ in 0..600 {
            let before = pos;
            s.update(&mut pos, player, clear, 1.0 / 60.0);
            if in_view(before, player, clear) {
                assert_eq!(pos, before, "moved while watched");
                assert!(!s.moving);
            }
        }
        // ...and stops at the edge of sight, never reaching you from the front.
        assert!(flat(pos - player).length() >= clear - STALKER_SPEED * MOVE_SPEED / 60.0 - 1e-3);
        // From behind (toward the camera) it keeps coming.
        let mut behind = Vec3::new(0.0, 0.9, -4.0);
        s.update(&mut behind, player, clear, 0.1);
        assert!(s.moving && behind.z > -4.0);
        assert!(STALKER_SPEED < 1.0);
    }

    #[test]
    fn the_mimic_is_always_exactly_where_you_were() {
        let mut trail = Trail::default();
        for i in 0..600 {
            let t = i as f32 / 60.0;
            trail.push(t, Vec3::new(t, 0.0, 0.0));
            let now = t;
            match mimic_pos(&trail, now, MIMIC_DELAY) {
                None => assert!(now < MIMIC_DELAY),
                Some(p) => {
                    assert!(p.x <= now - MIMIC_DELAY + 1e-4, "ahead of its delay");
                    assert!(p.x > now - MIMIC_DELAY - 0.02);
                }
            }
        }
        // Old samples are dropped.
        assert!(trail.at(0.0).is_none());
    }

    #[test]
    fn standing_still_lets_the_mimic_catch_up_and_moving_keeps_it_away() {
        let mut trail = Trail::default();
        for i in 0..300 {
            trail.push(i as f32 / 60.0, Vec3::new(i as f32 * 0.1, 0.0, 0.0));
        }
        let moving_gap =
            (Vec3::new(29.9, 0.0, 0.0) - mimic_pos(&trail, 299.0 / 60.0, 0.0).unwrap()).length();
        assert!(moving_gap > 10.0);
        let stop = Vec3::new(29.9, 0.0, 0.0);
        for i in 300..600 {
            trail.push(i as f32 / 60.0, stop);
        }
        assert_eq!(mimic_pos(&trail, 599.0 / 60.0, 0.0), Some(stop));
    }

    #[test]
    fn a_sentry_beam_sweeps_and_rests_between_calls() {
        let at = Vec3::ZERO;
        assert!(sees(at, Vec3::Z, Vec3::new(0.0, 0.0, 3.0)));
        assert!(
            !sees(at, Vec3::Z, Vec3::new(3.0, 0.0, 0.5)),
            "outside the cone"
        );
        assert!(
            !sees(at, Vec3::Z, Vec3::new(0.0, 0.0, SENTRY_RANGE + 0.1)),
            "out of range"
        );
        let mut s = Sentry::new(0.0);
        let player = Vec3::new(0.0, 0.0, 4.0);
        let mut calls = 0;
        for _ in 0..(20.0 * 60.0) as usize {
            calls += s.update(at, player, 1.0 / 60.0) as u32;
        }
        let full_turns = 20.0 * SENTRY_TURN / std::f32::consts::TAU;
        assert!(
            calls >= 1 && calls as f32 <= full_turns.ceil() + 1.0,
            "{calls} calls"
        );
    }

    #[test]
    fn jesters_try_every_direction() {
        let dirs = throw_directions(3);
        assert_eq!(dirs.len(), 8);
        for (i, a) in dirs.iter().enumerate() {
            assert!((a.length() - 1.0).abs() < 1e-5);
            for b in &dirs[i + 1..] {
                assert!(a.distance(*b) > 0.5);
            }
        }
    }

    #[test]
    fn gates_are_open_more_than_half_the_time() {
        let open = (0..3200)
            .filter(|&i| !gate_closed(i as f32 / 1000.0, 0.4))
            .count();
        assert!(open as f32 / 3200.0 > 0.55, "{open}");
        assert!(gate_closed(0.0, 0.0) && !gate_closed(GATE_CLOSED + 0.1, 0.0));
    }

    #[test]
    fn shifting_tiles_stay_within_a_hop() {
        for i in 0..400 {
            let d = shift_depth(i as f32 * 0.05, 1.3);
            assert!((-1e-6..=1.0 + 1e-6).contains(&d));
        }
        let hop = crate::gameplay::JUMP_SPEED.powi(2) / (2.0 * 9.8);
        assert!(SHIFT_DROP < hop * 0.6, "you couldn't hop back up");
    }

    #[test]
    fn segment_distance() {
        let (a, b) = (Vec3::ZERO, Vec3::X * 3.0);
        assert!((dist_to_segment(Vec3::new(1.5, 5.0, 1.0), a, b) - 1.0).abs() < 1e-5);
        assert!((dist_to_segment(Vec3::new(-2.0, 0.0, 0.0), a, b) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn elites_only_in_hard_runs_and_capped() {
        assert_eq!(elite_chance(1.0), 0.0);
        assert!(elite_chance(4.0) > 0.0);
        assert_eq!(elite_chance(1e6), 0.5);
        assert!((0..1000).all(|k| roll_elite(9, k, 1.0).is_none()));
        let n = (0..4000)
            .filter(|&k| roll_elite(9, k, 1e6).is_some())
            .count();
        assert!((n as f32 / 4000.0 - 0.5).abs() < 0.05, "{n}");
        let kinds: std::collections::HashSet<_> =
            (0..4000).filter_map(|k| roll_elite(9, k, 1e6)).collect();
        assert_eq!(kinds.len(), ALL_ELITES.len());
    }

    #[test]
    fn beat_tiles_always_leave_time_to_cross() {
        for bpm in [90.0_f32, 100.0, 110.0, 120.0, 130.0] {
            let bar = 120.0 / bpm;
            assert!(
                bar * (1.0 - BEAT_HOT) >= CELL / MOVE_SPEED + 0.05,
                "{bpm} bpm"
            );
            let hot = (0..1000)
                .filter(|&k| beat_hot(k as f32 * bar / 1000.0, bpm))
                .count();
            assert!((250..=350).contains(&hot), "{bpm}: {hot}/1000 hot");
        }
        assert!(!beat_hot(0.0, 0.0));
    }

    #[test]
    fn loops_count_up_and_echoes_fade() {
        assert_eq!(
            (loop_index(19.9), loop_index(20.0), loop_index(45.0)),
            (0, 1, 2)
        );
        assert_eq!((echo_scale(0.0), echo_scale(ECHO_LIFE)), (1.0, 0.0));
        assert!(echo_scale(0.3) > echo_scale(0.9));
    }

    #[test]
    fn watchers_look_at_their_target() {
        for t in [Vec3::new(5.0, 0.0, 1.0), Vec3::new(-3.0, 2.0, -8.0)] {
            let at = Vec3::new(1.0, 1.7, 1.0);
            let dir = Vec3::new(t.x - at.x, 0.0, t.z - at.z).normalize();
            let y = watcher_yaw(at, t);
            let fwd = Vec3::new(y.sin(), 0.0, y.cos());
            assert!(fwd.dot(dir) > 0.9999);
        }
    }
}
