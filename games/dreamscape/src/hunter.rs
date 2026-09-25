//! The nightmare's hunter: one big thing that always knows where you are.
//! It stalks slowly, then lunges (still a little slower than you), then has
//! to stop and catch its breath: it shrinks while it rests, which is the
//! tell that it's safe to dash past. It phases through walls like every
//! dream enemy, so the pillars are for breaking its line, not hiding.

use engine::glam::Vec3;

/// Touch radius (it's big).
pub const HUNTER_TOUCH: f32 = 1.4;
pub const STALK_TIME: f32 = 2.4;
pub const LUNGE_TIME: f32 = 1.1;
pub const REST_TIME: f32 = 1.6;
/// Body size while hunting / resting.
pub const SIZE: f32 = 2.0;
pub const REST_SIZE: f32 = 1.4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Stalk,
    Lunge,
    Rest,
}

#[derive(Clone, Copy, Debug)]
pub struct Hunter {
    /// Fractions of the player's speed.
    stalk: f32,
    lunge: f32,
    player_speed: f32,
    pub phase: Phase,
    left: f32,
}

impl Hunter {
    /// Deeper nightmares hunt harder (but a lunge never matches you).
    pub fn new(depth: u32, player_speed: f32) -> Self {
        let tier = (depth / 5).saturating_sub(1).min(6) as f32;
        Self {
            stalk: 0.42 + 0.03 * tier,
            lunge: (0.8 + 0.02 * tier).min(0.92),
            player_speed,
            phase: Phase::Rest,
            left: REST_TIME,
        }
    }

    pub fn speed(&self) -> f32 {
        self.player_speed
            * match self.phase {
                Phase::Stalk => self.stalk,
                Phase::Lunge => self.lunge,
                Phase::Rest => 0.0,
            }
    }

    pub fn size(&self) -> f32 {
        if self.phase == Phase::Rest {
            REST_SIZE
        } else {
            SIZE
        }
    }

    /// Catch your breath now (after it catches you, so you can get away).
    pub fn rest(&mut self) {
        self.phase = Phase::Rest;
        self.left = REST_TIME;
    }

    pub fn update(&mut self, pos: &mut Vec3, player: Vec3, dt: f32) {
        self.left -= dt;
        if self.left <= 0.0 {
            (self.phase, self.left) = match self.phase {
                Phase::Stalk => (Phase::Lunge, LUNGE_TIME),
                Phase::Lunge => (Phase::Rest, REST_TIME),
                Phase::Rest => (Phase::Stalk, STALK_TIME),
            };
        }
        let to = Vec3::new(player.x - pos.x, 0.0, player.z - pos.z);
        let step = (self.speed() * dt).min(to.length());
        *pos += to.normalize_or_zero() * step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay::MOVE_SPEED;

    #[test]
    fn it_is_never_as_fast_as_you() {
        for depth in [5, 10, 50, 1000] {
            let mut h = Hunter::new(depth, MOVE_SPEED);
            for _ in 0..500 {
                let mut pos = Vec3::ZERO;
                h.update(&mut pos, Vec3::X * 100.0, 0.05);
                assert!(h.speed() < MOVE_SPEED, "depth {depth}: {:?}", h.phase);
            }
        }
        assert!(Hunter::new(30, MOVE_SPEED).lunge > Hunter::new(5, MOVE_SPEED).lunge);
    }

    #[test]
    fn it_cycles_stalk_lunge_rest_and_shrinks_to_rest() {
        let mut h = Hunter::new(5, MOVE_SPEED);
        let mut pos = Vec3::ZERO;
        let mut seen = vec![h.phase];
        for _ in 0..200 {
            h.update(&mut pos, Vec3::X * 100.0, 0.05);
            if *seen.last().unwrap() != h.phase {
                seen.push(h.phase);
            }
        }
        assert_eq!(
            &seen[..4],
            &[Phase::Rest, Phase::Stalk, Phase::Lunge, Phase::Rest]
        );
        h.rest();
        assert_eq!(h.speed(), 0.0);
        assert!(h.size() < SIZE);
    }

    #[test]
    fn it_heads_for_you_on_the_ground_plane_and_never_overshoots() {
        let mut h = Hunter::new(5, MOVE_SPEED);
        let mut pos = Vec3::new(0.0, 0.9, 0.0);
        let target = Vec3::new(3.0, 0.0, 4.0);
        for _ in 0..400 {
            h.update(&mut pos, target, 0.05);
        }
        assert!((pos.y - 0.9).abs() < 1e-6, "height never changes");
        assert!(Vec3::new(pos.x - 3.0, 0.0, pos.z - 4.0).length() < 1e-3);
    }
}
