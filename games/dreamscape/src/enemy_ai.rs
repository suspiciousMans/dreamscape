//! Dream enemies. They patrol a short segment of your route; deeper down they
//! notice you, give chase, and — held by a leash to their patrol — give up
//! and drift back when you get far enough away. They phase through walls
//! (it's a dream), which is exactly why the leash exists.

use engine::glam::Vec3;

/// Chasing is a little faster than patrolling, but never as fast as you.
pub const CHASE_BOOST: f32 = 1.25;

#[derive(Debug, Clone, Copy)]
pub struct EnemyAI {
    a: Vec3,
    b: Vec3,
    speed: f32,
    /// 0 = never chases.
    alert_radius: f32,
    /// How far from the middle of its patrol it will follow you.
    leash: f32,
    toward_b: bool,
    pub chasing: bool,
}

fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

impl EnemyAI {
    pub fn new(a: Vec3, b: Vec3, speed: f32, alert_radius: f32, leash: f32) -> Self {
        Self {
            a,
            b,
            speed,
            alert_radius,
            leash,
            toward_b: true,
            chasing: false,
        }
    }

    fn home(&self) -> Vec3 {
        (self.a + self.b) * 0.5
    }

    /// Moves the enemy one step. Height never changes.
    pub fn update(&mut self, pos: &mut Vec3, player: Vec3, dt: f32) {
        let near = flat(player - *pos).length() < self.alert_radius;
        let in_leash = flat(player - self.home()).length() < self.leash;
        self.chasing = self.alert_radius > 0.0 && near && in_leash;
        let (target, speed) = if self.chasing {
            (player, self.speed * CHASE_BOOST)
        } else {
            let wp = if self.toward_b { self.b } else { self.a };
            // Movement snaps onto the waypoint, so this is an exact arrival.
            if flat(wp - *pos).length() < 1e-4 {
                self.toward_b = !self.toward_b;
            }
            (if self.toward_b { self.b } else { self.a }, self.speed)
        };
        let to = flat(target - *pos);
        let step = speed * dt;
        let delta = if to.length() <= step {
            to
        } else {
            to.normalize() * step
        };
        *pos += delta;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay::MOVE_SPEED;

    const FAR: Vec3 = Vec3::new(100.0, 0.5, 100.0);

    fn run(ai: &mut EnemyAI, pos: &mut Vec3, player: Vec3, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            ai.update(pos, player, 1.0 / 60.0);
        }
    }

    #[test]
    fn patrols_back_and_forth_between_its_points() {
        let (a, b) = (Vec3::new(0.0, 0.5, 0.0), Vec3::new(3.0, 0.5, 0.0));
        let mut ai = EnemyAI::new(a, b, 3.0, 0.0, 0.0);
        let mut pos = a;
        let mut max_x: f32 = 0.0;
        let mut min_x_after_turn = f32::MAX;
        for i in 0..240 {
            ai.update(&mut pos, FAR, 1.0 / 60.0);
            assert!((pos.y - 0.5).abs() < 1e-6, "height changed");
            assert!(
                (-1e-4..=3.0 + 1e-4).contains(&pos.x),
                "left its segment: {pos}"
            );
            max_x = max_x.max(pos.x);
            if i > 80 {
                min_x_after_turn = min_x_after_turn.min(pos.x);
            }
        }
        assert!(max_x > 2.99, "never reached b");
        assert!(min_x_after_turn < 0.01, "never came back to a");
    }

    #[test]
    fn never_chases_when_alert_radius_is_zero() {
        let (a, b) = (Vec3::ZERO, Vec3::X * 3.0);
        let mut ai = EnemyAI::new(a, b, 3.0, 0.0, 10.0);
        let mut pos = a;
        run(&mut ai, &mut pos, Vec3::new(1.5, 0.0, 1.0), 2.0);
        assert!(!ai.chasing && pos.z.abs() < 1e-6);
    }

    #[test]
    fn chases_a_nearby_player_but_stays_slower_than_them() {
        let (a, b) = (Vec3::ZERO, Vec3::X * 3.0);
        let mut ai = EnemyAI::new(a, b, 4.0, 5.0, 8.0);
        let mut pos = Vec3::new(1.5, 0.5, 0.0);
        let player = Vec3::new(1.5, 1.0, 3.0);
        let before = pos;
        ai.update(&mut pos, player, 0.1);
        assert!(ai.chasing);
        assert!(pos.z > before.z, "didn't move toward the player");
        assert!(
            (pos - before).length() / 0.1 < MOVE_SPEED,
            "faster than the player"
        );
        assert!((pos.y - 0.5).abs() < 1e-6);
    }

    #[test]
    fn gives_up_past_the_leash_and_goes_home() {
        let (a, b) = (Vec3::ZERO, Vec3::X * 3.0);
        let mut ai = EnemyAI::new(a, b, 4.0, 5.0, 6.0);
        let mut pos = Vec3::new(1.5, 0.5, 0.0);
        // Player is near the enemy but outside the leash around home.
        let player = Vec3::new(1.5, 0.0, 9.0);
        pos.z = 7.0;
        run(&mut ai, &mut pos, player, 4.0);
        assert!(!ai.chasing);
        assert!(pos.z.abs() < 0.2, "didn't return to its patrol: {pos}");
    }

    #[test]
    fn chase_boost_keeps_the_fastest_enemy_slower_than_you() {
        let fastest = crate::gameplay::enemy_speed(u32::MAX);
        assert!(fastest * CHASE_BOOST < MOVE_SPEED);
    }
}
