use engine::glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub enum EnemyBehavior {
    Patrol {
        waypoint_a: Vec3,
        waypoint_b: Vec3,
        speed: f32,
    },
    Chase {
        target_pos: Vec3,
        speed: f32,
        alert_radius: f32,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct EnemyAI {
    pub behavior: EnemyBehavior,
    pub is_active: bool,
}

impl EnemyAI {
    pub fn new_patrol(a: Vec3, b: Vec3, speed: f32) -> Self {
        Self {
            behavior: EnemyBehavior::Patrol {
                waypoint_a: a,
                waypoint_b: b,
                speed,
            },
            is_active: true,
        }
    }

    pub fn update(&mut self, current_pos: &mut Vec3, dt: f32) {
        if !self.is_active {
            return;
        }

        match self.behavior {
            EnemyBehavior::Patrol {
                waypoint_a,
                waypoint_b,
                speed,
            } => {
                // Simple oscillation between waypoints
                let target = if current_pos.distance(waypoint_a) < 0.5 {
                    waypoint_b
                } else if current_pos.distance(waypoint_b) < 0.5 {
                    waypoint_a
                } else {
                    // Pick closer waypoint
                    if current_pos.distance(waypoint_a) < current_pos.distance(waypoint_b) {
                        waypoint_b
                    } else {
                        waypoint_a
                    }
                };

                let dir = (target - *current_pos).normalize_or_zero();
                *current_pos += dir * speed * dt;
            }
            EnemyBehavior::Chase {
                target_pos, speed, ..
            } => {
                let dir = (target_pos - *current_pos).normalize_or_zero();
                *current_pos += dir * speed * dt;
            }
        }
    }
}
