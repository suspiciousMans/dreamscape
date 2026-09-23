use glam::{Vec3, Quat};
use rand::{Rng, SeedableRng};

#[derive(Debug, Clone)]
pub struct WorldVariant {
    pub seed: u64,
    pub difficulty: f32,  // 0.0 = safe dream, 1.0 = nightmare
}

pub struct WorldGenerator {
    seed: u64,
}

impl WorldGenerator {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// Mutate a spawned object's transform based on world variant
    pub fn mutate_object(
        &self,
        original_pos: Vec3,
        original_rot: Quat,
        variant: &WorldVariant,
    ) -> (Vec3, Quat) {
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed ^ variant.seed);

        // Add procedural offset (liminal: slightly off-grid). Guarded —
        // `gen_range` panics on an empty range, and difficulty 0.0 (safe
        // dream worlds) collapses the range to exactly zero width.
        let offset_scale = variant.difficulty * 0.5;
        let offset = if offset_scale > 0.0 {
            Vec3::new(
                rng.gen_range(-offset_scale..offset_scale),
                rng.gen_range(-offset_scale * 0.3..offset_scale * 0.3),
                rng.gen_range(-offset_scale..offset_scale),
            )
        } else {
            Vec3::ZERO
        };

        // Add procedural rotation (nightmare: more spin) — same empty-range
        // guard as the offset above.
        let rot_angle = variant.difficulty * std::f32::consts::PI * 0.2;
        let rot = if rot_angle > 0.0 {
            Quat::from_axis_angle(
                Vec3::new(
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(-1.0..1.0),
                )
                .normalize(),
                rng.gen_range(-rot_angle..rot_angle),
            )
        } else {
            Quat::IDENTITY
        };

        (original_pos + offset, original_rot * rot)
    }

    /// Should this hazard be active in this variant?
    pub fn should_spawn_hazard(&self, hazard_id: usize, variant: &WorldVariant) -> bool {
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed ^ variant.seed ^ hazard_id as u64);
        rng.gen_bool(variant.difficulty as f64)
    }
}
