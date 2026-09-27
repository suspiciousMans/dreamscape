//! What a dream asks of you. A dream with a shard rolls one objective;
//! finishing it collects the shard.

use engine::glam::Vec3;
use rand::{rngs::StdRng, Rng, SeedableRng};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Shard,
    Fragments,
    Chase,
    HoldOn,
    FindMemory,
}

/// `note_due`: this dream has a note (Find the memory hides the shard in it).
pub fn roll(seed: u64, depth: u32, note_due: bool) -> Kind {
    if depth == 0 {
        return Kind::Shard;
    }
    let r = StdRng::seed_from_u64(seed ^ 0x0B7E_C71E).gen::<f32>();
    match r {
        r if r < 0.40 => Kind::Shard,
        r if r < 0.60 => Kind::Fragments,
        r if r < 0.75 => Kind::Chase,
        r if r < 0.90 => Kind::HoldOn,
        _ if note_due => Kind::FindMemory,
        _ => Kind::Shard,
    }
}

pub const HOLD_CATCH_COST: f32 = 5.0;
/// Enemies are called to you this often while you hold on.
pub const HOLD_CALL_EVERY: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoldOn {
    pub left: f32,
    pub call_in: f32,
}

impl HoldOn {
    pub fn new(depth: u32) -> Self {
        Self {
            left: (20.0 + depth as f32).min(35.0),
            call_in: HOLD_CALL_EVERY,
        }
    }
    /// True when enemies should be called to you this tick.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.left = (self.left - dt).max(0.0);
        self.call_in -= dt;
        if self.call_in <= 0.0 {
            self.call_in += HOLD_CALL_EVERY;
            return true;
        }
        false
    }
    pub fn caught(&mut self) {
        self.left += HOLD_CATCH_COST;
    }
    pub fn done(&self) -> bool {
        self.left <= 0.0
    }
}

/// It runs this fraction of your speed when you're near, and waits when you're not.
pub const CHASE_SPEED: f32 = 0.8;
pub const CHASE_NOTICE: f32 = 7.0;
pub const CHASE_CATCH: f32 = 0.9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chase {
    /// Index of the route waypoint it's heading to.
    pub target: usize,
    pub pos: Vec3,
}

impl Chase {
    /// Starts at waypoint `start` (clamped to the route).
    pub fn new(route: &[Vec3], start: usize) -> Self {
        let i = start.min(route.len().saturating_sub(1));
        Self {
            target: i,
            pos: route[i],
        }
    }

    /// Flees along the route, away from the player: toward whichever
    /// neighbouring waypoint is further from you.
    pub fn step(&mut self, route: &[Vec3], player: Vec3, player_speed: f32, dt: f32) {
        let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
        if flat(player - self.pos).length() > CHASE_NOTICE || route.len() < 2 {
            return;
        }
        let mut left = CHASE_SPEED * player_speed * dt;
        while left > 0.0 {
            let to = route[self.target] - self.pos;
            let d = to.length();
            if d > left {
                self.pos += to / d * left;
                return;
            }
            self.pos = route[self.target];
            left -= d;
            let here = self.target;
            let options = [
                here.checked_sub(1),
                (here + 1 < route.len()).then_some(here + 1),
            ];
            self.target = options
                .into_iter()
                .flatten()
                .max_by(|&a, &b| {
                    flat(route[a] - player)
                        .length()
                        .total_cmp(&flat(route[b] - player).length())
                })
                .unwrap_or(here);
            if self.target == here {
                return; // cornered at an end: it waits there
            }
        }
    }

    pub fn caught(&self, player: Vec3) -> bool {
        Vec3::new(player.x - self.pos.x, 0.0, player.z - self.pos.z).length() < CHASE_CATCH
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fragments {
    pub total: u32,
    pub taken: u32,
}

impl Fragments {
    pub fn new(total: u32) -> Self {
        Self { total, taken: 0 }
    }
    pub fn take(&mut self) {
        self.taken = (self.taken + 1).min(self.total);
    }
    pub fn done(&self) -> bool {
        self.taken >= self.total
    }
    pub fn label(&self) -> String {
        format!("FRAGMENTS {}/{}", self.taken, self.total)
    }
}

/// The running objective of the current dream.
#[derive(Clone, Debug, PartialEq)]
pub enum Active {
    Shard,
    Fragments(Fragments),
    Chase(Chase),
    HoldOn(HoldOn),
    FindMemory,
}

impl Active {
    pub fn label(&self) -> String {
        match self {
            Active::Shard => "FIND THE SHARD".into(),
            Active::Fragments(f) => f.label(),
            Active::Chase(_) => "CATCH THE MEMORY".into(),
            Active::HoldOn(h) => format!("HOLD ON {:.0}s", h.left.ceil()),
            Active::FindMemory => "FIND THE MEMORY".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::glam::Vec3;
    use std::collections::HashMap;

    #[test]
    fn a_mix_of_objectives_and_find_the_memory_only_with_a_note() {
        let mut n: HashMap<Kind, u32> = HashMap::new();
        for s in 0..2000 {
            *n.entry(roll(s, 5, true)).or_default() += 1;
            assert_ne!(roll(s, 5, false), Kind::FindMemory);
        }
        assert!(n[&Kind::Shard] > 600 && n[&Kind::Shard] < 1000, "{n:?}");
        for k in [Kind::Fragments, Kind::Chase, Kind::HoldOn, Kind::FindMemory] {
            assert!(n.get(&k).copied().unwrap_or(0) > 100, "{k:?}: {n:?}");
        }
        assert!(
            (0..50).all(|s| roll(s, 0, true) == Kind::Shard),
            "depth 0 stays simple"
        );
    }

    #[test]
    fn hold_on_counts_down_and_a_catch_costs_time() {
        let mut h = HoldOn::new(4);
        let total = h.left;
        assert!((20.0..=35.0).contains(&total));
        h.tick(5.0);
        h.caught();
        assert!((h.left - (total - 5.0 + HOLD_CATCH_COST)).abs() < 1e-4);
        h.tick(100.0);
        assert!(h.done());
    }

    #[test]
    fn the_memory_flees_along_the_route_and_can_be_caught() {
        let route: Vec<Vec3> = (0..20).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
        let mut c = Chase::new(&route, 6);
        let mut player = Vec3::new(3.0, 0.0, 0.0);
        let speed = 6.0;
        for _ in 0..600 {
            c.step(&route, player, speed, 1.0 / 60.0);
            assert!(
                route.iter().any(|w| w.distance(c.pos) < 1.01),
                "left the route"
            );
            player += (c.pos - player).normalize_or_zero() * speed / 60.0;
            if c.caught(player) {
                return;
            }
        }
        panic!(
            "a player at full speed never caught it: {:?} vs {player:?}",
            c.pos
        );
    }

    #[test]
    fn the_memory_waits_when_you_are_far() {
        let route: Vec<Vec3> = (0..20).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
        let mut c = Chase::new(&route, 10);
        let start = c.pos;
        c.step(&route, Vec3::new(-50.0, 0.0, 0.0), 6.0, 1.0);
        assert_eq!(c.pos, start);
    }

    #[test]
    fn fragments_count_down() {
        let mut f = Fragments::new(3);
        assert_eq!(f.label(), "FRAGMENTS 0/3");
        f.take();
        f.take();
        f.take();
        assert!(f.done());
    }
}
