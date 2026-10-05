//! Run-feel juice, kept soft: a brief ease-out time dip instead of a hard
//! freeze, a small decaying shake, a rising pitch when pickups chain, a ring
//! that ripples out from the centre of the screen, and a hum that swells as
//! you walk toward the wake door.
//!
//! Everything here is pure (no GL, no audio, no clock): the game feeds it
//! `dt` and asks what to do, so it is unit-tested. `reduced` (the reduced
//! motion setting) removes the time dip, the shake and the ring.

use glam::Vec3;

/// What just happened, for [`Juice::hit`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    /// Caught by an enemy.
    Caught,
    /// A lucidity shard.
    Shard,
    /// A smaller pickup (fragment, sigil).
    Pickup,
}

/// Slowest the world ever runs during a dip (it never fully freezes).
pub const DIP_FLOOR: f32 = 0.2;
/// Seconds a pickup must follow the last one to keep the chain going.
pub const CHAIN_WINDOW: f32 = 6.0;
/// Seconds the pickup ring takes to ripple out and fade.
pub const RING_SECS: f32 = 0.7;
/// How far from the wake door (world units) its hum starts.
pub const HUM_RANGE: f32 = 14.0;

/// Semitones above the base note for each link of a chain (major pentatonic,
/// so any run of pickups sounds like a tune and never a clash).
const CHAIN_STEPS: [i32; 8] = [0, 2, 4, 7, 9, 12, 14, 16];

#[derive(Default, Debug)]
pub struct Juice {
    clock: f32,
    dip_left: f32,
    dip_total: f32,
    dip_floor: f32,
    shake: f32,
    chain: u32,
    since_pickup: f32,
    ring_age: f32,
    hum_wait: f32,
    hum: f32,
}

impl Juice {
    pub fn new() -> Self {
        Self {
            ring_age: f32::INFINITY,
            since_pickup: f32::INFINITY,
            ..Default::default()
        }
    }

    /// Advance by real seconds; returns the (possibly slowed) dt the world
    /// should use this frame. Always `dt` when nothing is dipping.
    pub fn step(&mut self, dt: f32) -> f32 {
        self.clock += dt;
        self.since_pickup += dt;
        self.ring_age += dt;
        self.shake *= (-dt * 14.0).exp();
        if self.shake < 1e-4 {
            self.shake = 0.0;
        }
        let scale = self.time_scale();
        self.dip_left = (self.dip_left - dt).max(0.0);
        dt * scale
    }

    /// 1.0 normally; dips to the floor on a hit and eases back to 1.0.
    pub fn time_scale(&self) -> f32 {
        if self.dip_left <= 0.0 || self.dip_total <= 0.0 {
            return 1.0;
        }
        let k = 1.0 - self.dip_left / self.dip_total; // 0 at the hit -> 1 at the end
        self.dip_floor + (1.0 - self.dip_floor) * k * k
    }

    /// Something happened: dip time, shake, and (for pickups) ripple.
    pub fn hit(&mut self, what: Hit, reduced: bool) {
        if reduced {
            return;
        }
        if what != Hit::Caught {
            self.ring_age = 0.0;
        }
        let (secs, floor, shake) = match what {
            Hit::Caught => (0.12, DIP_FLOOR, 0.10),
            Hit::Shard => (0.10, 0.45, 0.04),
            Hit::Pickup => (0.0, 1.0, 0.015),
        };
        if secs > 0.0 && secs >= self.dip_left {
            self.dip_left = secs;
            self.dip_total = secs;
            self.dip_floor = floor;
        }
        self.shake = self.shake.max(shake);
    }

    /// Camera offset for this frame (world units, tiny).
    pub fn shake_offset(&self) -> Vec3 {
        if self.shake <= 0.0 {
            return Vec3::ZERO;
        }
        let t = self.clock;
        self.shake
            * Vec3::new(
                (t * 47.0).sin(),
                (t * 61.0 + 1.3).sin() * 0.6,
                (t * 53.0 + 2.1).sin(),
            )
    }

    /// A pickup: returns the pitch ratio to play its sound at. Pickups
    /// within [`CHAIN_WINDOW`] of the last climb a pentatonic step each.
    pub fn chain_pitch(&mut self) -> f32 {
        if self.since_pickup <= CHAIN_WINDOW {
            self.chain += 1;
        } else {
            self.chain = 0;
        }
        self.since_pickup = 0.0;
        let step = CHAIN_STEPS[(self.chain as usize).min(CHAIN_STEPS.len() - 1)];
        2f32.powf(step as f32 / 12.0)
    }

    /// Length of the current chain (0 for a lone pickup).
    pub fn chain_len(&self) -> u32 {
        self.chain
    }

    /// 0..1 progress of the pickup ring, or `None` when it is not showing.
    pub fn ring(&self) -> Option<f32> {
        (self.ring_age < RING_SECS).then(|| self.ring_age / RING_SECS)
    }

    /// 0 far from the door .. 1 right on it.
    pub fn door_closeness(distance: f32) -> f32 {
        (1.0 - distance / HUM_RANGE).clamp(0.0, 1.0)
    }

    /// Per frame while a wake door exists: how bright its glow is (0..1),
    /// and `Some(pitch ratio)` on the frames its hum should sound. The hum
    /// beats faster and rises in pitch the closer you get.
    pub fn door(&mut self, dt: f32, distance: Option<f32>) -> Option<f32> {
        let Some(d) = distance else {
            self.hum = 0.0;
            return None;
        };
        let p = Self::door_closeness(d);
        self.hum = p * p;
        if p <= 0.0 {
            self.hum_wait = 0.0;
            return None;
        }
        self.hum_wait -= dt;
        if self.hum_wait > 0.0 {
            return None;
        }
        self.hum_wait = 1.6 - 1.15 * p;
        Some(1.0 + 0.5 * p)
    }

    /// Soft warm glow for the HUD to draw (0..1).
    pub fn door_glow(&self) -> f32 {
        self.hum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dip_slows_then_returns_to_normal_and_never_freezes() {
        let mut j = Juice::new();
        assert_eq!(j.step(0.016), 0.016);
        j.hit(Hit::Caught, false);
        let mut prev = 0.0;
        let mut total_real = 0.0;
        let mut min = f32::MAX;
        while total_real < 0.5 {
            let dt = j.step(0.01);
            assert!(dt > 0.0, "never a full freeze");
            assert!(dt >= prev - 1e-6, "eases back monotonically");
            min = min.min(dt / 0.01);
            prev = dt;
            total_real += 0.01;
        }
        assert!(min <= DIP_FLOOR + 0.05, "it did slow down: {min}");
        assert_eq!(j.time_scale(), 1.0);
        assert!((j.step(0.01) - 0.01).abs() < 1e-7);
    }

    #[test]
    fn reduced_motion_removes_dip_shake_and_ring() {
        let mut j = Juice::new();
        j.hit(Hit::Caught, true);
        j.hit(Hit::Shard, true);
        assert_eq!(j.ring(), None);
        assert_eq!(j.step(0.016), 0.016);
        assert_eq!(j.shake_offset(), Vec3::ZERO);
    }

    #[test]
    fn shake_is_small_and_dies_away() {
        let mut j = Juice::new();
        j.hit(Hit::Caught, false);
        j.step(0.001);
        assert!(j.shake_offset().length() > 0.0);
        assert!(j.shake_offset().length() < 0.3, "gentle, not arcade");
        for _ in 0..120 {
            j.step(0.016);
        }
        assert_eq!(j.shake_offset(), Vec3::ZERO);
    }

    #[test]
    fn a_chain_climbs_a_pentatonic_scale_and_resets_after_the_window() {
        let mut j = Juice::new();
        let mut last = 0.0;
        for i in 0..8 {
            let r = j.chain_pitch();
            assert!(r > last, "step {i} must rise: {r} vs {last}");
            last = r;
            j.step(1.0);
        }
        assert_eq!(j.chain_len(), 7);
        // The top holds instead of running off the scale.
        assert_eq!(j.chain_pitch(), j.chain_pitch().max(last));
        // Wait too long: starts over at the base note.
        j.step(CHAIN_WINDOW + 1.0);
        assert_eq!(j.chain_pitch(), 1.0);
        assert_eq!(j.chain_len(), 0);
    }

    #[test]
    fn the_first_pickup_ever_is_the_base_note() {
        assert_eq!(Juice::new().chain_pitch(), 1.0);
    }

    #[test]
    fn the_ring_ripples_for_a_moment_on_pickups_only() {
        let mut j = Juice::new();
        assert_eq!(j.ring(), None);
        j.hit(Hit::Caught, false);
        assert_eq!(j.ring(), None);
        j.hit(Hit::Pickup, false);
        assert_eq!(j.ring(), Some(0.0));
        j.step(RING_SECS * 0.5);
        assert!((j.ring().unwrap() - 0.5).abs() < 1e-5);
        j.step(RING_SECS);
        assert_eq!(j.ring(), None);
    }

    #[test]
    fn the_door_hum_is_silent_far_away_and_quickens_and_rises_when_close() {
        let mut j = Juice::new();
        assert_eq!(j.door(0.016, Some(HUM_RANGE + 5.0)), None);
        assert_eq!(j.door_glow(), 0.0);
        assert_eq!(j.door(0.016, None), None);

        // Count hums over 10 simulated seconds at two distances.
        let count = |dist: f32| {
            let mut j = Juice::new();
            let mut n = 0;
            let mut pitch = 0.0;
            for _ in 0..1000 {
                if let Some(p) = j.door(0.01, Some(dist)) {
                    n += 1;
                    pitch = p;
                }
            }
            (n, pitch)
        };
        let (far_n, far_p) = count(HUM_RANGE * 0.9);
        let (near_n, near_p) = count(1.0);
        assert!(near_n > far_n * 2, "{near_n} vs {far_n}");
        assert!(near_p > far_p);
        assert!(near_p <= 1.5 + 1e-5);
    }

    #[test]
    fn the_door_glow_swells_toward_the_door() {
        let mut j = Juice::new();
        j.door(0.01, Some(10.0));
        let far = j.door_glow();
        j.door(0.01, Some(2.0));
        let near = j.door_glow();
        assert!(near > far && near <= 1.0);
    }
}
