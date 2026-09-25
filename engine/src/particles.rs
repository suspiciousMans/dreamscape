use glam::Vec3;
use hecs::Entity;
use serde::{Deserialize, Serialize};

use crate::ecs::Transform;

/// Everything about a burst/stream of particles except *where* it is
/// (that's the entity's `Transform::position`, read once per spawned
/// particle) — a plain data shape, savable as part of a level via
/// `engine::level::LevelParticleEmitter`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ParticleEmitterDef {
    /// Particles spawned per second for a continuous emitter. `0.0` means
    /// this emitter never auto-emits — it only produces particles via an
    /// explicit `spawn_burst` call (see `interact()`'s push-burst demo).
    pub rate_per_sec: f32,
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    /// Half-angle (degrees) of the cone around `+Y` new particles launch
    /// into — `0` is a straight vertical jet, `180` is a full sphere.
    pub spread_deg: f32,
    /// Multiplies a fixed downward acceleration; `0` for floaty sparkles,
    /// `1`-ish for falling debris.
    pub gravity_scale: f32,
    pub start_size: f32,
    pub end_size: f32,
    pub start_color: [f32; 4],
    pub end_color: [f32; 4],
    pub max_particles: u32,
}

impl Default for ParticleEmitterDef {
    fn default() -> Self {
        Self {
            rate_per_sec: 8.0,
            lifetime_min: 0.6,
            lifetime_max: 1.2,
            speed_min: 0.5,
            speed_max: 1.5,
            spread_deg: 25.0,
            gravity_scale: 0.3,
            start_size: 0.12,
            end_size: 0.02,
            start_color: [1.0, 0.9, 0.4, 1.0],
            end_color: [1.0, 0.4, 0.1, 0.0],
            max_particles: 64,
        }
    }
}

pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub age: f32,
    pub lifetime: f32,
}

/// A tiny xorshift32 generator — particle spawn direction/speed/lifetime
/// only need cheap visual variety, not statistical quality, so this avoids
/// pulling in a `rand` dependency for a handful of `next_f32()` calls.
struct Rng(u32);

impl Rng {
    fn seeded() -> Self {
        use std::hash::{BuildHasher, Hasher};
        // `RandomState` is seeded from the OS at process start — a free,
        // dependency-free source of a varying seed per emitter.
        let seed = std::hash::RandomState::new().build_hasher().finish() as u32;
        Self(seed | 1) // xorshift32 never recovers from a zero state
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        let unit = self.next_u32() as f32 / u32::MAX as f32;
        min + (max - min) * unit
    }
}

/// Runtime ECS component: an emitter's definition plus its live particle
/// pool. Lives on any entity with a `Transform` — particles spawn at that
/// `Transform::position` but then move independently in world space.
pub struct ParticleEmitter {
    pub def: ParticleEmitterDef,
    pub particles: Vec<Particle>,
    spawn_accumulator: f32,
    rng: Rng,
}

impl ParticleEmitter {
    pub fn new(def: ParticleEmitterDef) -> Self {
        Self {
            def,
            particles: Vec::new(),
            spawn_accumulator: 0.0,
            rng: Rng::seeded(),
        }
    }

    fn spawn_one(&mut self, origin: Vec3) -> Particle {
        let speed = self.rng.range(self.def.speed_min, self.def.speed_max);
        let spread = self.def.spread_deg.to_radians();
        let theta = self.rng.range(0.0, std::f32::consts::TAU);
        let phi = self.rng.range(0.0, spread.max(0.001));
        let direction = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
        Particle {
            position: origin,
            velocity: direction * speed,
            age: 0.0,
            lifetime: self.rng.range(
                self.def.lifetime_min.max(0.01),
                self.def.lifetime_max.max(0.01),
            ),
        }
    }

    /// Immediately emits `count` particles (up to `max_particles`),
    /// ignoring `rate_per_sec` — the burst path used by e.g. `interact()`.
    pub fn spawn_burst(&mut self, origin: Vec3, count: u32) {
        for _ in 0..count {
            if self.particles.len() >= self.def.max_particles as usize {
                break;
            }
            let particle = self.spawn_one(origin);
            self.particles.push(particle);
        }
    }

    /// The current size and RGBA color for a particle, linearly interpolated
    /// by its age/lifetime fraction — computed on the fly at render time
    /// rather than stored, since it's cheap and keeps `Particle` small.
    pub fn appearance(&self, particle: &Particle) -> (f32, [f32; 4]) {
        let t = (particle.age / particle.lifetime.max(0.001)).clamp(0.0, 1.0);
        let lerp = |a: f32, b: f32| a + (b - a) * t;
        let size = lerp(self.def.start_size, self.def.end_size);
        let color = [
            lerp(self.def.start_color[0], self.def.end_color[0]),
            lerp(self.def.start_color[1], self.def.end_color[1]),
            lerp(self.def.start_color[2], self.def.end_color[2]),
            lerp(self.def.start_color[3], self.def.end_color[3]),
        ];
        (size, color)
    }
}

/// Ages/culls every emitter's particles, integrates survivors (gravity +
/// linear motion), and auto-emits for continuous (`rate_per_sec > 0`)
/// emitters. Runs unconditionally every frame (Edit and Play mode) — the
/// same "always-on live preview" philosophy as `engine::animation::step`.
///
/// Returns every entity whose emitter is a **spent one-shot burst**
/// (`rate_per_sec <= 0` and no particles left) — transient burst emitters
/// (see `spawn_burst`) aren't level data, so the caller is expected to
/// despawn them rather than leave empty emitters accumulating forever.
pub fn step(world: &mut hecs::World, dt: f32) -> Vec<Entity> {
    const GRAVITY: f32 = 9.8;
    let mut spent = Vec::new();

    for (entity, (transform, emitter)) in world.query::<(&Transform, &mut ParticleEmitter)>().iter()
    {
        emitter.particles.retain_mut(|particle| {
            particle.age += dt;
            particle.velocity.y -= GRAVITY * emitter.def.gravity_scale * dt;
            particle.position += particle.velocity * dt;
            particle.age < particle.lifetime
        });

        if emitter.def.rate_per_sec > 0.0 {
            emitter.spawn_accumulator += emitter.def.rate_per_sec * dt;
            while emitter.spawn_accumulator >= 1.0
                && emitter.particles.len() < emitter.def.max_particles as usize
            {
                emitter.spawn_accumulator -= 1.0;
                let particle = emitter.spawn_one(transform.position);
                emitter.particles.push(particle);
            }
            // While the pool is saturated the `while` above can't drain the
            // accumulator, so a high-rate emitter would bank an ever-growing
            // backlog and then dump a catch-up burst the instant slots free
            // up (a visible "pop" instead of a steady stream). Cap it so no
            // more than one spawn's worth of credit is ever carried over.
            // A no-op whenever slots are free (the loop already drains below
            // 1.0 then).
            emitter.spawn_accumulator = emitter.spawn_accumulator.min(1.0);
        } else if emitter.particles.is_empty() {
            spent.push(entity);
        }
    }

    spent
}

#[cfg(test)]
mod tests {
    use super::*;

    fn burst_def() -> ParticleEmitterDef {
        ParticleEmitterDef {
            rate_per_sec: 0.0,
            lifetime_min: 0.2,
            lifetime_max: 0.2,
            max_particles: 10,
            ..ParticleEmitterDef::default()
        }
    }

    #[test]
    fn burst_spawns_up_to_max_particles() {
        let mut emitter = ParticleEmitter::new(burst_def());
        emitter.spawn_burst(Vec3::ZERO, 25);
        assert_eq!(emitter.particles.len(), 10);
    }

    #[test]
    fn particles_age_and_cull_after_lifetime() {
        let mut world = hecs::World::new();
        let mut emitter = ParticleEmitter::new(burst_def());
        emitter.spawn_burst(Vec3::ZERO, 3);
        let entity = world.spawn((Transform::default(), emitter));

        step(&mut world, 0.1);
        assert_eq!(
            world
                .get::<&ParticleEmitter>(entity)
                .unwrap()
                .particles
                .len(),
            3,
            "not yet past lifetime"
        );

        step(&mut world, 0.2);
        assert_eq!(
            world
                .get::<&ParticleEmitter>(entity)
                .unwrap()
                .particles
                .len(),
            0,
            "should have aged out"
        );
    }

    #[test]
    fn spent_one_shot_emitter_is_reported_once_empty() {
        let mut world = hecs::World::new();
        let mut emitter = ParticleEmitter::new(burst_def());
        emitter.spawn_burst(Vec3::ZERO, 1);
        let entity = world.spawn((Transform::default(), emitter));

        let spent_immediately = step(&mut world, 0.05);
        assert!(spent_immediately.is_empty(), "still has a live particle");

        let spent_after_lifetime = step(&mut world, 0.3);
        assert_eq!(spent_after_lifetime, vec![entity]);
    }

    #[test]
    fn continuous_emitter_never_reported_as_spent() {
        let mut world = hecs::World::new();
        let def = ParticleEmitterDef {
            rate_per_sec: 5.0,
            ..burst_def()
        };
        world.spawn((Transform::default(), ParticleEmitter::new(def)));

        let spent = step(&mut world, 1.0);
        assert!(
            spent.is_empty(),
            "a continuous emitter with rate > 0 is never 'spent'"
        );
    }

    #[test]
    fn gravity_pulls_particles_downward() {
        let mut world = hecs::World::new();
        let mut emitter = ParticleEmitter::new(ParticleEmitterDef {
            rate_per_sec: 0.0,
            lifetime_min: 10.0,
            lifetime_max: 10.0,
            speed_min: 0.0,
            speed_max: 0.0,
            gravity_scale: 1.0,
            ..ParticleEmitterDef::default()
        });
        emitter.spawn_burst(Vec3::ZERO, 1);
        let entity = world.spawn((Transform::default(), emitter));

        step(&mut world, 0.5);
        let emitter = world.get::<&ParticleEmitter>(entity).unwrap();
        assert!(
            emitter.particles[0].position.y < 0.0,
            "gravity should have pulled the particle down"
        );
    }
}
