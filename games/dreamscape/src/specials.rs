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

/// Seen by the dreamer: the view cone in first person, `in_view` otherwise.
pub fn seen(pos: Vec3, player: Vec3, clear: f32, yaw: Option<f32>) -> bool {
    match yaw {
        Some(yaw) => crate::fpv::sees(player, yaw, pos, clear),
        None => in_view(pos, player, clear),
    }
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

    /// `yaw`: the dreamer's look direction in a first-person dream.
    pub fn update(&mut self, pos: &mut Vec3, player: Vec3, clear: f32, yaw: Option<f32>, dt: f32) {
        self.moving = !seen(*pos, player, clear, yaw);
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
    #[cfg(test)]
    pub fn update(&mut self, at: Vec3, player: Vec3, dt: f32) -> bool {
        self.update_scaled(at, player, dt, 1.0)
    }

    /// `update` with the turn rate scaled (tactics: x1.5 with a mimic in the beam).
    pub fn update_scaled(&mut self, at: Vec3, player: Vec3, dt: f32, turn: f32) -> bool {
        self.angle = (self.angle + SENTRY_TURN * turn * dt).rem_euclid(std::f32::consts::TAU);
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

/// How thick a shut gate is along the corridor (plus the player's radius).
pub const GATE_DEPTH: f32 = 0.35;

/// Does a shut gate at `at`, across a corridor running along `dir`, touch
/// `player`? It spans the full cell, wall to wall, so you can't sidestep it.
pub fn gate_blocks(at: Vec3, dir: Vec3, player: Vec3) -> bool {
    let d = flat(player - at);
    let along = flat(dir).normalize_or_zero();
    let across = Vec3::Y.cross(along);
    d.dot(along).abs() < GATE_DEPTH + crate::gameplay::PLAYER_RADIUS
        && d.dot(across).abs() <= CELL * 0.5
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

    /// Extra hit points an elite of this kind carries (BIG is the armoured one).
    pub fn hp_bonus(self) -> f32 {
        match self {
            Elite::Big => 2.0,
            _ => 0.0,
        }
    }

    /// How much larger it stands.
    pub fn scale_mult(self) -> f32 {
        match self {
            Elite::Big => 1.3,
            _ => 1.0,
        }
    }
}

/// Elite variants a new enemy can wear. A Splitter's split is pacer-only.
pub const FOE_ELITES: [Elite; 3] = [Elite::Fast, Elite::Big, Elite::Shade];
/// Elite variants the ENEMIES-B group can wear: their pace is authored inside
/// each module, so FAST is not offered there.
pub const GROUP_ELITES: [Elite; 2] = [Elite::Big, Elite::Shade];
/// A new enemy turns elite at this share of a pacer's chance. A headless sweep
/// over real spawn plans (see the tests) picks 1.0: parity with the pacer curve
/// is the share whose per-foe rate matches it, which is about one elite new foe
/// every other deep dream.
pub const FOE_ELITE_SHARE: f32 = 1.0;

/// Deterministic elite roll for new enemy `k` of a dream.
pub fn roll_foe_elite(seed: u64, k: usize, difficulty: f32) -> Option<Elite> {
    roll_foe_elite_with(seed, k, difficulty, FOE_ELITE_SHARE)
}

/// The same roll at another share: the balance sweep tries shares out.
pub fn roll_foe_elite_with(seed: u64, k: usize, difficulty: f32, share: f32) -> Option<Elite> {
    roll_foe_from(seed, k, difficulty, &FOE_ELITES, share)
}

/// Deterministic elite roll for a Moths/Mask/Leech group of a dream.
pub fn roll_group_elite(seed: u64, k: usize, difficulty: f32) -> Option<Elite> {
    roll_group_elite_with(seed, k, difficulty, FOE_ELITE_SHARE)
}

pub fn roll_group_elite_with(seed: u64, k: usize, difficulty: f32, share: f32) -> Option<Elite> {
    roll_foe_from(seed, k, difficulty, &GROUP_ELITES, share)
}

fn roll_foe_from(
    seed: u64,
    k: usize,
    difficulty: f32,
    list: &'static [Elite],
    share: f32,
) -> Option<Elite> {
    // Its own, well-mixed stream: one roll never turns another enemy elite, and
    // every dream gets its own draw (an XOR of a small seed would not move the
    // high bits the roll is read from, so every dream would draw the same one).
    let h = crate::worth::mix(
        seed ^ (k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
        0x5DEE_CE66_D1CE_5EED,
    );
    let roll = (h >> 40) as f32 / (1u64 << 24) as f32;
    (roll < elite_chance(difficulty) * share)
        .then(|| list[(h >> 3) as usize % list.len()])
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
pub const BEAT_HOT: f32 = 0.3;
pub fn beat_hot(t: f32, bpm: f32) -> bool {
    if bpm <= 0.0 {
        return false;
    }
    let bar = 120.0 / bpm;
    t.rem_euclid(bar) / bar < BEAT_HOT
}

/// Melting Clockworks: the dream rewinds every LOOP_SECS.
pub const LOOP_SECS: f32 = 20.0;
pub fn loop_index(age: f32) -> u32 {
    (age.max(0.0) / LOOP_SECS) as u32
}

/// Afterimage Fields: a ghost of you every ECHO_EVERY seconds, fading over ECHO_LIFE.
pub const ECHO_EVERY: f32 = 0.18;
pub const ECHO_LIFE: f32 = 1.4;
pub const MAX_ECHOES: usize = 10;
pub fn echo_scale(age: f32) -> f32 {
    (1.0 - age / ECHO_LIFE).clamp(0.0, 1.0)
}

/// Watching Wallpaper: the yaw that turns an eye at `at` toward `target`.
pub fn watcher_yaw(at: Vec3, target: Vec3) -> f32 {
    let d = target - at;
    d.x.atan2(d.z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay::MOVE_SPEED;

    /// A shut gate fills the whole corridor: walking through anywhere
    /// across its width touches it, only being clear of its plane doesn't.
    #[test]
    fn the_drawn_gate_is_the_gate_that_hits() {
        // main.rs draws a shut gate as a cube scaled (CELL, 2.6, 2*GATE_DEPTH)
        // and rotated local Z -> corridor direction. Its footprint must equal
        // the hit area (bar the player's radius along the corridor).
        for dir in [Vec3::X, Vec3::Z, Vec3::new(1.0, 0.0, -1.0).normalize()] {
            let rot = engine::glam::Quat::from_rotation_arc(Vec3::Z, dir);
            let half = rot * Vec3::new(CELL, 0.0, GATE_DEPTH * 2.0) * 0.5;
            let across = Vec3::Y.cross(dir);
            assert!(
                (half.dot(across).abs() - CELL * 0.5).abs() < 1e-4,
                "{dir:?}"
            );
            assert!((half.dot(dir).abs() - GATE_DEPTH).abs() < 1e-4, "{dir:?}");
        }
    }

    #[test]
    fn a_shut_gate_spans_the_whole_corridor() {
        for dir in [Vec3::X, -Vec3::Z, Vec3::new(1.0, 0.0, 1.0).normalize()] {
            let at = Vec3::new(3.0, 0.0, -6.0);
            let side = Vec3::Y.cross(dir);
            // Every lateral offset a player (radius 0.5) can reach in a 3-wide corridor.
            for k in -10..=10 {
                let lateral = side * (k as f32 / 10.0) * (CELL * 0.5 - 0.5);
                assert!(
                    gate_blocks(at, dir, at + lateral),
                    "walked round at lateral {k}/10"
                );
            }
            assert!(
                !gate_blocks(at, dir, at + dir * 1.5),
                "caught well in front"
            );
            assert!(!gate_blocks(at, dir, at - dir * 1.5), "caught well behind");
        }
    }

    #[test]
    fn in_first_person_a_stalker_moves_only_behind_your_back() {
        let mut s = Stalker::new(Vec3::ZERO, MOVE_SPEED);
        let mut pos = Vec3::new(0.0, 0.9, 5.0);
        s.update(&mut pos, Vec3::ZERO, 20.0, Some(0.0), 1.0 / 60.0); // looking straight at it
        assert!(!s.moving);
        s.update(
            &mut pos,
            Vec3::ZERO,
            20.0,
            Some(std::f32::consts::PI),
            1.0 / 60.0,
        ); // turned away
        assert!(s.moving);
    }

    #[test]
    fn a_stalker_never_moves_while_you_can_see_it() {
        let clear = 6.0;
        let player = Vec3::ZERO;
        let mut s = Stalker::new(Vec3::new(0.0, 0.9, 20.0), MOVE_SPEED);
        let mut pos = Vec3::new(0.0, 0.9, 20.0);
        // Far ahead: out of view, so it creeps in...
        for _ in 0..600 {
            let before = pos;
            s.update(&mut pos, player, clear, None, 1.0 / 60.0);
            if in_view(before, player, clear) {
                assert_eq!(pos, before, "moved while watched");
                assert!(!s.moving);
            }
        }
        // ...and stops at the edge of sight, never reaching you from the front.
        assert!(flat(pos - player).length() >= clear - STALKER_SPEED * MOVE_SPEED / 60.0 - 1e-3);
        // From behind (toward the camera) it keeps coming.
        let mut behind = Vec3::new(0.0, 0.9, -4.0);
        s.update(&mut behind, player, clear, None, 0.1);
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
    fn a_scaled_sentry_turns_faster() {
        let (mut a, mut b) = (Sentry::new(0.0), Sentry::new(0.0));
        a.update_scaled(Vec3::ZERO, Vec3::splat(99.0), 1.0, 1.5);
        b.update(Vec3::ZERO, Vec3::splat(99.0), 1.0);
        assert!((a.angle - 1.5 * b.angle).abs() < 1e-5);
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


    #[test]
    fn the_elite_share_is_the_one_a_dream_sweep_chooses() {
        // A headless sweep over real spawn plans: how many new enemies a dream
        // fields, and how often one of them turns elite at each candidate share.
        use crate::combat::Kind;
        use crate::dream::ALL_THEMES;
        use crate::gameplay;
        const SHARES: [f32; 10] = [0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3];
        let mut dreams: Vec<([usize; 3], f32)> = Vec::new(); // (kinds' counts, difficulty)
        let mut news = 0usize;
        for depth in 4..=12u32 {
            for seed in 0..200u64 {
                let theme = ALL_THEMES[(seed as usize) % ALL_THEMES.len()];
                let plan = crate::newfoes::plan(depth, theme, seed, false);
                let counts = [
                    plan.iter().filter(|(k, _)| *k == Kind::Tooth).map(|(_, n)| *n).sum(),
                    plan.iter().filter(|(k, _)| *k == Kind::Weeper).map(|(_, n)| *n).sum(),
                    plan.iter().filter(|(k, _)| *k == Kind::Lullaby).map(|(_, n)| *n).sum(),
                ];
                news += counts.iter().sum::<usize>();
                let difficulty = gameplay::difficulty(Some((depth, 0))) * 1.2; // a strong loadout
                dreams.push((counts, difficulty));
            }
        }
        // (dreams with an elite, elites per dream, group elites' dreams, per-foe rate)
        let measure = |share: f32| {
            let (mut with_one, mut elites, mut groups) = (0usize, 0usize, 0usize);
            for (seed, (counts, difficulty)) in dreams.iter().enumerate() {
                let mut here = 0usize;
                let total: usize = counts.iter().sum();
                for k in 0..total {
                    if roll_foe_elite_with(seed as u64, k, *difficulty, share).is_some() {
                        here += 1;
                    }
                }
                if roll_group_elite_with(seed as u64, 1, *difficulty, share).is_some() {
                    groups += 1;
                }
                elites += here;
                if here > 0 {
                    with_one += 1;
                }
            }
            let n = dreams.len() as f32;
            (
                with_one as f32 / n,
                elites as f32 / n,
                groups as f32 / n,
                elites as f32 / news as f32,
            )
        };
        // What a pacer gets in the same dreams: the curve the new enemies match.
        let pacers = dreams
            .iter()
            .map(|(_, difficulty)| elite_chance(*difficulty))
            .sum::<f32>()
            / dreams.len() as f32;
        let mut pick = SHARES[0];
        let mut best = f32::MAX;
        for share in SHARES {
            let (dreams_with, mean, groups, per_foe) = measure(share);
            println!(
                "share {share:.2}: {:.0}% of deep dreams have an elite new foe, {mean:.2} each,                  {:.0}% a group elite, {:.0}% of foes elite (pacer curve {:.0}%)",
                dreams_with * 100.0,
                groups * 100.0,
                per_foe * 100.0,
                pacers * 100.0
            );
            let cost = (per_foe - pacers).abs();
            if cost < best {
                best = cost;
                pick = share;
            }
        }
        assert_eq!(FOE_ELITE_SHARE, pick, "the shipped share is the sweep's pick");
        let (dreams_with, mean, groups, per_foe) = measure(pick);
        // It matches the pacer curve it was measured against...
        assert!((per_foe - pacers).abs() < 0.02, "{per_foe} vs {pacers}");
        // ...which is about one elite new foe every other deep dream.
        assert!((0.28..=0.45).contains(&dreams_with), "{dreams_with}");
        assert!((0.35..=0.6).contains(&mean), "{mean}");
        // The group roll shares the number, and stays a rarer sight.
        assert!((0.18..=0.35).contains(&groups), "{groups}");
        assert!(news > 0);
    }
}
