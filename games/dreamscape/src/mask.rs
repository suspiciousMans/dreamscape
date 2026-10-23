//! MASK: a slow patroller with a shield on its face.
//!
//! Pure rules, no GL and no clock. Strikes from the front glance off; from
//! the side or back they land. It turns at only [`TURN_RATE`], so Blinking
//! behind it works. When you stand in front within [`SLAM_TRIGGER`] it
//! telegraphs [`SLAM_TELEGRAPH`] and slams its forehead. A Gust spins it (the
//! shield is down while it reels), and a Slam cracks the shield for a while.
//!
//! Yaw convention (same as the sentry): the facing vector is
//! `(sin yaw, 0, cos yaw)`.

use crate::dream::DreamTheme;
use crate::gameplay::CELL;
use engine::glam::Vec3;

pub const MIN_DEPTH: u32 = 7;
pub const HP: f32 = 3.0;
/// Walking speed, as a multiple of the player's.
pub const SPEED_MULT: f32 = 0.5;
/// Radians per second (90 degrees).
pub const TURN_RATE: f32 = std::f32::consts::FRAC_PI_2;
/// The shield covers this half-angle either side of straight ahead (a 120
/// degree face); expressed as its cosine.
pub const SHIELD_HALF_ANGLE: f32 = std::f32::consts::FRAC_PI_3;
/// Stand in front within this many cells and it winds up.
pub const SLAM_TRIGGER_CELLS: f32 = 3.0;
pub const SLAM_TRIGGER: f32 = SLAM_TRIGGER_CELLS * CELL;
/// The wind-up before the slam, how far the slam reaches (cells), how long it
/// stays dangerous, and how long it needs to recover.
pub const SLAM_TELEGRAPH: f32 = 0.7;
pub const SLAM_REACH_CELLS: f32 = 1.5;
pub const SLAM_REACH: f32 = SLAM_REACH_CELLS * CELL;
pub const SLAM_ACTIVE: f32 = 0.2;
pub const SLAM_RECOVER: f32 = 0.8;
/// Gust: a spin with the shield down. Slam: the shield cracks.
pub const GUST_STAGGER: f32 = 1.5;
pub const SLAM_SHIELD_DOWN: f32 = 2.0;
/// It notices you (and turns to face you) within this many cells.
pub const SENSE_CELLS: f32 = 4.0;
/// Spin rate while reeling from a gust (radians per second).
const SPIN: f32 = 12.0;

pub const STREAM: u64 = 0x4D_A5C;

/// One placement seed for the dream's mask, or none: below [`MIN_DEPTH`], in
/// nightmares; never more than one.
pub fn spawn_plan(depth: u32, theme: DreamTheme, seed: u64, nightmare: bool) -> Vec<u64> {
    let _ = theme;
    if nightmare || depth < MIN_DEPTH {
        return Vec::new();
    }
    vec![seed ^ STREAM]
}

/// The unit vector a mask with this yaw faces.
pub fn facing(yaw: f32) -> Vec3 {
    Vec3::new(yaw.sin(), 0.0, yaw.cos())
}

/// Yaw that faces from `from` toward `to`.
pub fn yaw_toward(from: Vec3, to: Vec3) -> f32 {
    (to.x - from.x).atan2(to.z - from.z)
}

/// Smallest signed angle from `a` to `b`, in -PI..=PI.
pub fn angle_diff(a: f32, b: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut d = (b - a) % tau;
    if d > std::f32::consts::PI {
        d -= tau;
    } else if d < -std::f32::consts::PI {
        d += tau;
    }
    d
}

/// Is `target` in front of a mask at `pos` facing `yaw`, within `half` radians?
pub fn in_front(pos: Vec3, yaw: f32, target: Vec3, half: f32) -> bool {
    let flat = Vec3::new(target.x - pos.x, 0.0, target.z - pos.z);
    if flat.length() < 1e-3 {
        return true;
    }
    angle_diff(yaw, yaw_toward(pos, target)).abs() <= half
}

/// Does the shield stop a strike from `striker`? Pure: the angle test only
/// (the fan of the swing is the caller's). Down shields stop nothing.
pub fn shield_blocks(pos: Vec3, yaw: f32, striker: Vec3, shield_down: bool) -> bool {
    !shield_down && in_front(pos, yaw, striker, SHIELD_HALF_ANGLE)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Patrol,
    /// Winding up: seconds left.
    Telegraph(f32),
    /// The slam is live: seconds left.
    Slamming(f32),
    /// Getting its forehead out of the floor: seconds left.
    Recover(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mask {
    pub pos: Vec3,
    pub yaw: f32,
    pub state: State,
    a: Vec3,
    b: Vec3,
    to_b: bool,
    /// Seconds the shield is down.
    pub shield_down: f32,
    /// Seconds spent spinning from a gust.
    pub reeling: f32,
}

impl Mask {
    pub fn new(a: Vec3, b: Vec3) -> Self {
        Self {
            pos: a,
            yaw: yaw_toward(a, b),
            state: State::Patrol,
            a,
            b,
            to_b: true,
            shield_down: 0.0,
            reeling: 0.0,
        }
    }

    pub fn facing(&self) -> Vec3 {
        facing(self.yaw)
    }

    pub fn shield_up(&self) -> bool {
        self.shield_down <= 0.0
    }

    /// Would a strike from `striker` glance off right now?
    pub fn blocks(&self, striker: Vec3) -> bool {
        shield_blocks(self.pos, self.yaw, striker, !self.shield_up())
    }

    /// Gust: spin for [`GUST_STAGGER`] with the shield down; the slam is off.
    pub fn gust(&mut self) {
        self.reeling = GUST_STAGGER;
        self.shield_down = self.shield_down.max(GUST_STAGGER);
        self.state = State::Patrol;
    }

    /// The dreamer's Slam: the shield cracks for [`SLAM_SHIELD_DOWN`].
    pub fn crack(&mut self) {
        self.shield_down = self.shield_down.max(SLAM_SHIELD_DOWN);
    }

    /// 0..1 through the wind-up (the tell: lean in and flush).
    pub fn glow(&self) -> f32 {
        match self.state {
            State::Telegraph(left) => (1.0 - left / SLAM_TELEGRAPH).clamp(0.0, 1.0),
            State::Slamming(_) => 1.0,
            _ => 0.0,
        }
    }

    /// The slam is live and `player` is in front within reach.
    pub fn slam_hits(&self, player: Vec3) -> bool {
        let flat = Vec3::new(player.x - self.pos.x, 0.0, player.z - self.pos.z);
        matches!(self.state, State::Slamming(_))
            && flat.length() <= SLAM_REACH
            && (player.y - self.pos.y).abs() < 2.0
            && in_front(self.pos, self.yaw, player, SHIELD_HALF_ANGLE)
    }

    pub fn update(&mut self, dt: f32, player: Vec3, player_speed: f32) {
        self.shield_down = (self.shield_down - dt).max(0.0);
        if self.reeling > 0.0 {
            self.reeling = (self.reeling - dt).max(0.0);
            self.yaw += SPIN * dt;
            return;
        }
        let flat = Vec3::new(player.x - self.pos.x, 0.0, player.z - self.pos.z);
        let dist = flat.length();
        match self.state {
            State::Telegraph(left) => {
                let left = left - dt;
                self.state = if left <= 0.0 { State::Slamming(SLAM_ACTIVE) } else { State::Telegraph(left) };
            }
            State::Slamming(left) => {
                let left = left - dt;
                self.state = if left <= 0.0 { State::Recover(SLAM_RECOVER) } else { State::Slamming(left) };
            }
            State::Recover(left) => {
                let left = left - dt;
                self.state = if left <= 0.0 { State::Patrol } else { State::Recover(left) };
            }
            State::Patrol => {
                let sensed = dist < SENSE_CELLS * CELL && (player.y - self.pos.y).abs() < 3.0;
                let goal = if sensed {
                    player
                } else if self.to_b {
                    self.b
                } else {
                    self.a
                };
                let want = yaw_toward(self.pos, goal);
                let diff = angle_diff(self.yaw, want);
                let turn = TURN_RATE * dt;
                self.yaw += diff.clamp(-turn, turn);
                let here = Vec3::new(goal.x - self.pos.x, 0.0, goal.z - self.pos.z).length();
                if !sensed && here < 0.3 {
                    self.to_b = !self.to_b;
                } else if diff.abs() < std::f32::consts::FRAC_PI_4 && (!sensed || dist > 1.2) {
                    let step = (player_speed * SPEED_MULT * dt).min(here);
                    self.pos += self.facing() * step;
                }
                if dist <= SLAM_TRIGGER && in_front(self.pos, self.yaw, player, SHIELD_HALF_ANGLE) && (player.y - self.pos.y).abs() < 2.0 {
                    self.state = State::Telegraph(SLAM_TELEGRAPH);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::ALL_THEMES;
    use crate::gameplay::MOVE_SPEED;
    use std::f32::consts::{FRAC_PI_2, PI};

    const DT: f32 = 1.0 / 60.0;
    const FAR: Vec3 = Vec3::new(500.0, 0.0, 500.0);

    #[test]
    fn the_constants_are_the_briefs() {
        assert_eq!(MIN_DEPTH, 7);
        assert_eq!(HP, 3.0);
        assert_eq!(SPEED_MULT, 0.5);
        assert!((TURN_RATE.to_degrees() - 90.0).abs() < 1e-3);
        assert_eq!(SLAM_TELEGRAPH, 0.7);
        assert_eq!(SLAM_REACH, 1.5 * CELL);
        assert_eq!(SLAM_TRIGGER, 3.0 * CELL);
        assert_eq!(GUST_STAGGER, 1.5);
        assert_eq!(SLAM_SHIELD_DOWN, 2.0);
        assert_eq!(crate::combat::base_hp(crate::combat::Kind::Mask), 3.0);
        assert!(MOVE_SPEED * SPEED_MULT < MOVE_SPEED);
    }

    /// Strike from a given bearing (degrees clockwise from its facing).
    #[test]
    fn the_shield_angle_table() {
        let pos = Vec3::ZERO;
        let yaw = 0.7; // any heading
        let at = |deg: f32| {
            let a = yaw + deg.to_radians();
            Vec3::new(a.sin() * 2.0, 0.0, a.cos() * 2.0)
        };
        let table = [
            (0.0, true, "dead ahead"),
            (30.0, true, "front quarter"),
            (-30.0, true, "front quarter, other side"),
            (59.0, true, "inside the edge"),
            (-59.0, true, "inside the edge, other side"),
            (61.0, false, "just past the edge"),
            (-61.0, false, "just past the edge, other side"),
            (90.0, false, "side"),
            (-90.0, false, "side"),
            (135.0, false, "back quarter"),
            (180.0, false, "behind"),
        ];
        for (deg, blocked, what) in table {
            assert_eq!(shield_blocks(pos, yaw, at(deg), false), blocked, "{what} ({deg} deg)");
        }
        assert!(!shield_blocks(pos, yaw, at(0.0), true), "a down shield stops nothing");
    }

    #[test]
    fn angle_diff_wraps() {
        assert!((angle_diff(0.1, 0.3) - 0.2).abs() < 1e-5);
        assert!((angle_diff(3.0, -3.0) - (2.0 * PI - 6.0)).abs() < 1e-4);
        assert!(angle_diff(-3.0, 3.0) < 0.0);
    }

    #[test]
    fn it_walks_at_half_the_players_speed() {
        let mut m = Mask::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 30.0));
        for _ in 0..60 {
            m.update(DT, FAR, MOVE_SPEED);
        }
        // Heading is +Z, so one second covers half a player-second.
        assert!((m.pos.z - MOVE_SPEED * 0.5).abs() < 0.1, "{:?}", m.pos);
        assert!(m.pos.x.abs() < 0.1);
    }

    #[test]
    fn it_turns_at_ninety_degrees_per_second() {
        // The player is straight behind it, outside the trigger range: it
        // must swing round, 180 degrees, in two seconds.
        let mut m = Mask::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 30.0));
        let behind = Vec3::new(0.0, 0.0, -SENSE_CELLS * CELL * 0.9);
        let mut t = 0.0;
        for _ in 0..60 {
            m.update(DT, behind, MOVE_SPEED);
            t += DT;
        }
        let turned = angle_diff(0.0, m.yaw).abs();
        assert!((turned - FRAC_PI_2).abs() < 0.05, "{turned} after {t}s");
    }

    #[test]
    fn standing_in_front_within_three_cells_telegraphs_then_slams() {
        let mut m = Mask::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 30.0));
        let me = Vec3::new(0.0, 0.0, 2.0 * CELL);
        m.update(DT, me, MOVE_SPEED);
        assert!(matches!(m.state, State::Telegraph(_)));
        let mut t = 0.0;
        while matches!(m.state, State::Telegraph(_)) {
            assert!(!m.slam_hits(me), "no damage during the tell");
            m.update(DT, me, MOVE_SPEED);
            t += DT;
            assert!(t < 1.0);
        }
        assert!((t - SLAM_TELEGRAPH).abs() < 0.05, "tell lasted {t}");
        assert!(matches!(m.state, State::Slamming(_)));
        // 2 cells away: out of the 1.5-cell reach.
        assert!(!m.slam_hits(me));
        let close = Vec3::new(0.0, 0.0, 1.0 * CELL);
        assert!(m.slam_hits(close));
        // The slam is behind it: nothing there to hit.
        assert!(!m.slam_hits(Vec3::new(0.0, 0.0, -1.0 * CELL)));
    }

    #[test]
    fn the_slam_is_brief_then_it_recovers_then_patrols() {
        let mut m = Mask::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 30.0));
        m.state = State::Slamming(SLAM_ACTIVE);
        let mut live = 0.0;
        while matches!(m.state, State::Slamming(_)) {
            m.update(DT, FAR, MOVE_SPEED);
            live += DT;
        }
        assert!((live - SLAM_ACTIVE).abs() < 0.03);
        assert!(matches!(m.state, State::Recover(_)));
        for _ in 0..((SLAM_RECOVER + 0.1) * 60.0) as u32 {
            m.update(DT, FAR, MOVE_SPEED);
        }
        assert_eq!(m.state, State::Patrol);
    }

    #[test]
    fn standing_to_the_side_or_behind_does_not_provoke_a_slam() {
        let mut m = Mask::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 30.0));
        m.update(DT, Vec3::new(CELL, 0.0, 0.0), MOVE_SPEED);
        assert_eq!(m.state, State::Patrol, "beside it");
        let mut n = Mask::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 30.0));
        n.update(DT, Vec3::new(0.0, 0.0, -CELL), MOVE_SPEED);
        assert_eq!(n.state, State::Patrol, "behind it");
        let mut far = Mask::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 30.0));
        far.update(DT, Vec3::new(0.0, 0.0, 3.5 * CELL), MOVE_SPEED);
        assert_eq!(far.state, State::Patrol, "beyond three cells");
    }

    #[test]
    fn a_gust_spins_it_for_a_second_and_a_half_with_the_shield_down() {
        let mut m = Mask::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 30.0));
        m.state = State::Telegraph(0.3);
        m.gust();
        assert_eq!(m.state, State::Patrol, "the wind-up is broken");
        let front = Vec3::new(0.0, 0.0, 2.0);
        assert!(!m.blocks(front), "shield down at once");
        let at = m.pos;
        let yaw = m.yaw;
        for _ in 0..60 {
            m.update(DT, front, MOVE_SPEED);
        }
        assert_eq!(m.pos, at, "reeling, not walking");
        assert_ne!(m.yaw, yaw, "spinning");
        assert!(!m.shield_up());
        for _ in 0..40 {
            m.update(DT, FAR, MOVE_SPEED);
        }
        assert!(m.shield_up(), "up again after 1.5s");
        assert_eq!(m.reeling, 0.0);
    }

    #[test]
    fn a_slam_cracks_the_shield_for_two_seconds() {
        let mut m = Mask::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 30.0));
        let front = Vec3::new(0.0, 0.0, 2.0);
        assert!(m.blocks(front));
        m.crack();
        assert!(!m.blocks(front));
        for _ in 0..110 {
            m.update(DT, FAR, MOVE_SPEED);
        }
        assert!(!m.shield_up(), "still down at 1.83s");
        for _ in 0..20 {
            m.update(DT, FAR, MOVE_SPEED);
        }
        assert!(m.shield_up());
        // (It has been walking: ahead of it is wherever it now faces.)
        assert!(m.blocks(m.pos + m.facing() * 2.0));
    }

    #[test]
    fn it_patrols_between_its_two_ends() {
        let mut m = Mask::new(Vec3::ZERO, Vec3::new(0.0, 0.0, CELL));
        let mut max_z: f32 = 0.0;
        let mut came_back = false;
        for _ in 0..(60 * 30) {
            m.update(DT, FAR, MOVE_SPEED);
            max_z = max_z.max(m.pos.z);
            came_back |= max_z > 2.0 && m.pos.z < 1.0;
            assert!(m.pos.z > -0.6 && m.pos.z < CELL + 0.6, "{:?}", m.pos);
        }
        assert!(came_back);
    }

    #[test]
    fn spawn_plan_gates_by_depth_nightmare_and_caps_at_one() {
        let theme = ALL_THEMES[0];
        assert!(spawn_plan(6, theme, 7, false).is_empty());
        assert!(spawn_plan(30, theme, 7, true).is_empty());
        for depth in 7..30 {
            assert_eq!(spawn_plan(depth, theme, 7, false).len(), 1);
        }
        assert_ne!(spawn_plan(7, theme, 1, false), spawn_plan(7, theme, 2, false));
        assert_ne!(spawn_plan(7, theme, 1, false), crate::moths::spawn_plan(7, theme, 1, false), "own streams");
    }
}
