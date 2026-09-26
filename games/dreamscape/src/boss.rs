//! Nightmare bosses. The hunter (hunter.rs) does the chasing; from the second
//! nightmare on it also attacks. What keeps it fair:
//! - every attack is announced for `TELEGRAPH` seconds before it lands,
//! - it only starts one while the hunter is resting (the tell you know),
//! - one attack at a time, at least `attack_gap(tier)` apart,
//! - every attack can be beaten: jump the ring, outwalk the wisps, wait out the dark.

use crate::gameplay::PLAYER_RADIUS;
use crate::hunter::{Hunter, Phase};
use engine::glam::Vec3;

pub const TELEGRAPH: f32 = 0.8;
pub const RING_SPEED: f32 = 7.0;
/// Top of the ring above the floor: your feet must clear it.
pub const RING_H: f32 = 0.45;
pub const RING_T: f32 = 0.6;
pub const RING_MAX: f32 = 30.0;
/// Fraction of your speed.
pub const WISP_SPEED: f32 = 0.5;
pub const WISP_LIFE: f32 = 6.0;
pub const WISP_TOUCH: f32 = 0.6;
pub const MAX_WISPS: usize = 4;
pub const ECLIPSE_TIME: f32 = 3.0;
pub const ECLIPSE_SIGHT: f32 = 0.55;

pub fn tier(depth: u32) -> u32 {
    depth / crate::dream::NIGHTMARE_EVERY
}

pub fn sigil_count(tier: u32) -> usize {
    crate::dream::SIGILS + usize::from(tier >= 4)
}

pub fn attack_gap(tier: u32) -> f32 {
    (4.4 - 0.4 * tier as f32).max(2.5)
}

pub fn title(tier: u32) -> &'static str {
    match tier {
        0 | 1 => "THE HUNTER",
        2 => "THE QUAKE",
        3 => "THE SWARM MOTHER",
        _ => "THE ECLIPSE",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackKind {
    Shockwave,
    Summon,
    Eclipse,
}

pub fn attacks(tier: u32) -> &'static [AttackKind] {
    use AttackKind::*;
    match tier {
        0 | 1 => &[],
        2 => &[Shockwave],
        3 => &[Shockwave, Summon],
        _ => &[Shockwave, Summon, Eclipse],
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Attack {
    Telegraph { kind: AttackKind, left: f32 },
    Ring { centre: Vec3, radius: f32 },
    Eclipse { left: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Telegraph(AttackKind),
    Ring,
    Summon,
    Eclipse,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wisp {
    pub pos: Vec3,
    pub life: f32,
}

#[derive(Clone, Debug)]
pub struct Boss {
    pub hunter: Hunter,
    pub tier: u32,
    pub attack: Option<Attack>,
    pub wisps: Vec<Wisp>,
    gap: f32,
    next: usize,
    player_speed: f32,
}

fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

/// Touching the ring: inside its band with your feet below its top.
pub fn ring_hits(centre: Vec3, radius: f32, player: Vec3) -> bool {
    let d = flat(player - centre).length();
    (d - radius).abs() < RING_T * 0.5 + PLAYER_RADIUS && player.y - PLAYER_RADIUS < RING_H
}

impl Boss {
    pub fn new(depth: u32, player_speed: f32) -> Self {
        let tier = tier(depth);
        Self {
            hunter: Hunter::new(depth, player_speed),
            tier,
            attack: None,
            wisps: Vec::new(),
            gap: attack_gap(tier),
            next: 0,
            player_speed,
        }
    }

    pub fn update(&mut self, pos: &mut Vec3, player: Vec3, dt: f32) -> Vec<Event> {
        let mut events = Vec::new();
        self.hunter.update(pos, player, dt);
        let step = WISP_SPEED * self.player_speed * dt;
        for w in &mut self.wisps {
            let to = flat(player - w.pos);
            w.pos += to.normalize_or_zero() * step.min(to.length());
            w.life -= dt;
        }
        self.wisps.retain(|w| w.life > 0.0);
        let gap = attack_gap(self.tier);
        let current = self.attack;
        self.attack = match current {
            None => {
                self.gap -= dt;
                let list = attacks(self.tier);
                if self.gap <= 0.0 && !list.is_empty() && self.hunter.phase == Phase::Rest {
                    let kind = list[self.next % list.len()];
                    self.next += 1;
                    events.push(Event::Telegraph(kind));
                    Some(Attack::Telegraph {
                        kind,
                        left: TELEGRAPH,
                    })
                } else {
                    None
                }
            }
            Some(Attack::Telegraph { kind, left }) if left - dt > 0.0 => Some(Attack::Telegraph {
                kind,
                left: left - dt,
            }),
            Some(Attack::Telegraph { kind, .. }) => match kind {
                AttackKind::Shockwave => {
                    events.push(Event::Ring);
                    Some(Attack::Ring {
                        centre: flat(*pos),
                        radius: 0.0,
                    })
                }
                AttackKind::Summon => {
                    for side in [-1.0_f32, 1.0] {
                        if self.wisps.len() < MAX_WISPS {
                            self.wisps.push(Wisp {
                                pos: flat(*pos) + Vec3::X * 2.0 * side,
                                life: WISP_LIFE,
                            });
                        }
                    }
                    events.push(Event::Summon);
                    self.gap = gap;
                    None
                }
                AttackKind::Eclipse => {
                    events.push(Event::Eclipse);
                    Some(Attack::Eclipse { left: ECLIPSE_TIME })
                }
            },
            Some(Attack::Ring { centre, radius }) => {
                let radius = radius + RING_SPEED * dt;
                if radius > RING_MAX {
                    self.gap = gap;
                    None
                } else {
                    Some(Attack::Ring { centre, radius })
                }
            }
            Some(Attack::Eclipse { left }) => {
                if left - dt > 0.0 {
                    Some(Attack::Eclipse { left: left - dt })
                } else {
                    self.gap = gap;
                    None
                }
            }
        };
        events
    }

    pub fn hits(&self, player: Vec3) -> bool {
        let ring = matches!(self.attack, Some(Attack::Ring { centre, radius }) if ring_hits(centre, radius, player));
        ring || self
            .wisps
            .iter()
            .any(|w| flat(w.pos - player).length() < WISP_TOUCH + PLAYER_RADIUS * 0.5)
    }

    pub fn sight_scale(&self) -> f32 {
        if matches!(self.attack, Some(Attack::Eclipse { .. })) {
            ECLIPSE_SIGHT
        } else {
            1.0
        }
    }

    pub fn warning(&self) -> Option<&'static str> {
        match self.attack {
            Some(Attack::Telegraph { kind, .. }) => Some(match kind {
                AttackKind::Shockwave => "IT GATHERS ITSELF - JUMP",
                AttackKind::Summon => "IT CALLS FOR HELP",
                AttackKind::Eclipse => "THE LIGHTS ARE GOING OUT",
            }),
            _ => None,
        }
    }

    /// It caught you: it backs off and everything in flight is dropped.
    pub fn after_catch(&mut self) {
        self.hunter.rest();
        self.attack = None;
        self.wisps.clear();
        self.gap = attack_gap(self.tier);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay::{JUMP_SPEED, MOVE_SPEED};
    const DT: f32 = 1.0 / 60.0;

    fn run(depth: u32, secs: f32) -> Vec<(f32, Event)> {
        let mut b = Boss::new(depth, MOVE_SPEED);
        let mut pos = Vec3::new(0.0, 0.9, 0.0);
        let player = Vec3::new(0.0, 0.5, 60.0);
        let (mut t, mut out) = (0.0, Vec::new());
        while t < secs {
            t += DT;
            out.extend(b.update(&mut pos, player, DT).into_iter().map(|e| (t, e)));
        }
        out
    }

    #[test]
    fn the_first_nightmare_never_attacks() {
        assert!(run(5, 90.0).is_empty());
    }

    #[test]
    fn attacks_unlock_in_order_and_deep_nightmares_want_four_sigils() {
        for t in 0..12 {
            let (a, b) = (attacks(t), attacks(t + 1));
            assert!(b.len() >= a.len() && b[..a.len()] == *a, "tier {t}");
        }
        assert_eq!(attacks(9).len(), 3);
        assert_eq!((sigil_count(3), sigil_count(4)), (3, 4));
    }

    #[test]
    fn every_attack_is_announced_and_spaced_out() {
        for depth in [10, 15, 20, 40] {
            let ev = run(depth, 120.0);
            assert!(
                ev.iter().any(|(_, e)| *e == Event::Ring),
                "depth {depth}: never attacked"
            );
            let mut pending: Option<(f32, AttackKind)> = None;
            let mut last_telegraph: Option<f32> = None;
            for (t, e) in ev {
                match e {
                    Event::Telegraph(k) => {
                        assert!(pending.is_none(), "two attacks at once");
                        if let Some(l) = last_telegraph {
                            assert!(
                                t - l >= attack_gap(tier(depth)),
                                "depth {depth}: attacks bunched"
                            );
                        }
                        last_telegraph = Some(t);
                        pending = Some((t, k));
                    }
                    effect => {
                        let (t0, k) = pending.take().expect("unannounced attack");
                        assert!(
                            t - t0 >= TELEGRAPH - 2.0 * DT,
                            "announced only {}s ahead",
                            t - t0
                        );
                        let want = match k {
                            AttackKind::Shockwave => Event::Ring,
                            AttackKind::Summon => Event::Summon,
                            AttackKind::Eclipse => Event::Eclipse,
                        };
                        assert_eq!(effect, want);
                    }
                }
            }
        }
    }

    /// Uses the game's own jump speed and the engine's gravity.
    #[test]
    fn the_ring_hits_you_standing_and_a_jump_two_steps_early_clears_it() {
        let g = engine::physics::PhysicsParams::default().gravity;
        for dist in [3.0_f32, 6.0, 12.0, 20.0] {
            let mut r = 0.0;
            let mut hit = false;
            while r < RING_MAX {
                hit |= ring_hits(Vec3::ZERO, r, Vec3::new(0.0, PLAYER_RADIUS, dist));
                r += RING_SPEED * DT;
            }
            assert!(hit, "standing at {dist}: ring missed");
            let (mut r, mut y, mut vy, mut jumped) = (0.0_f32, PLAYER_RADIUS, 0.0_f32, false);
            while r < RING_MAX {
                if !jumped && dist - r < 2.0 {
                    vy = JUMP_SPEED;
                    jumped = true;
                }
                vy -= g * DT;
                y += vy * DT;
                if y < PLAYER_RADIUS {
                    y = PLAYER_RADIUS;
                    vy = 0.0;
                }
                assert!(
                    !ring_hits(Vec3::ZERO, r, Vec3::new(0.0, y, dist)),
                    "jumped at {dist}: hit at r={r:.2} y={y:.2}"
                );
                r += RING_SPEED * DT;
            }
        }
    }

    #[test]
    fn wisps_are_slower_than_you_few_and_fade() {
        let mut b = Boss::new(15, MOVE_SPEED);
        let mut pos = Vec3::new(0.0, 0.9, 0.0);
        let player = Vec3::new(0.0, 0.5, 30.0);
        let mut seen = false;
        for _ in 0..(60.0 / DT) as usize {
            let before: Vec<Vec3> = b.wisps.iter().map(|w| w.pos).collect();
            b.update(&mut pos, player, DT);
            assert!(b.wisps.len() <= MAX_WISPS);
            for (w, p) in b.wisps.iter().zip(before) {
                assert!(w.pos.distance(p) <= WISP_SPEED * MOVE_SPEED * DT + 1e-4);
                assert!(w.life <= WISP_LIFE);
            }
            seen |= !b.wisps.is_empty();
        }
        assert!(seen, "tier 3 never summoned");
    }

    #[test]
    fn a_catch_clears_the_air_and_the_eclipse_never_blinds() {
        let mut b = Boss::new(40, MOVE_SPEED);
        b.attack = Some(Attack::Ring {
            centre: Vec3::ZERO,
            radius: 3.0,
        });
        b.wisps.push(Wisp {
            pos: Vec3::ZERO,
            life: 3.0,
        });
        b.after_catch();
        assert!(b.attack.is_none() && b.wisps.is_empty() && b.hunter.speed() == 0.0);
        assert!(ECLIPSE_SIGHT >= 0.5);
    }
}
