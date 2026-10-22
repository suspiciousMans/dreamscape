//! HAZARDS: each dream theme has one rule that bites (depth 3+, normal dreams).
//! Pure logic: the table, strength ramp, per-kind state machines and the
//! route-safe floor picks. Wiring lives in `main.rs` (`// === hazards ===`).
#![allow(dead_code)]

use crate::dream::DreamTheme;

/// First depth where a theme rule applies (depths 0..=2 stay gentle).
pub const MIN_DEPTH: u32 = 3;
/// Seconds since the last shard for a slow rule to reach full bite.
pub const FULL_BITE_SECS: f32 = 60.0;
/// Gust cycle: calm, then a push for `GUST_SECS`.
pub const GUST_PERIOD: f32 = 9.0;
pub const GUST_SECS: f32 = 1.6;
/// Push speed (units/s) of a gust at full strength.
pub const GUST_PUSH: f32 = 3.0;
/// Beat length (s) for the beat floor.
pub const BEAT_SECS: f32 = 1.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Slows more as the dream ages; a shard resets it.
    RisingTide,
    /// Outer cells crumble inward (never the route).
    ShrinkingFloor,
    /// Periodic push.
    GustWind,
    /// Enemies hear dashes and blinks from further away.
    Silence,
    /// The light fades as the dream ages; a shard resets it.
    DimmingLight,
    /// Existing fog pockets drift.
    DriftingFog,
    /// Constant drag on the floor.
    StickyFloor,
    /// Floor tiles burn on the beat.
    BeatFloor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeRule {
    pub kind: Kind,
    /// Short eye-voice line for the title card.
    pub line: &'static str,
}

const fn r(kind: Kind, line: &'static str) -> ThemeRule {
    ThemeRule { kind, line }
}

/// The one rule of every theme.
pub fn rule_for(theme: DreamTheme) -> ThemeRule {
    use DreamTheme::*;
    use Kind::*;
    match theme {
        Lobby => r(StickyFloor, "the carpet clings"),
        LiminalOffice => r(DimmingLight, "the lights are failing"),
        VoidPlatforms => r(ShrinkingFloor, "the edges are leaving"),
        Garden => r(DriftingFog, "the mist wanders"),
        NightmareFactory => r(BeatFloor, "the floor beats hot"),
        Awakening => r(DimmingLight, "the dream is thinning"),
        CursedForest => r(Silence, "the trees listen"),
        DrownedLibrary => r(RisingTide, "the water is rising"),
        SkyStairs => r(GustWind, "the wind shoves"),
        MirrorHall => r(Silence, "every step rings"),
        MyceliumGrove => r(DriftingFog, "the spores drift"),
        TheTunnel => r(DimmingLight, "the dark closes in"),
        FractalCathedral => r(ShrinkingFloor, "the nave is shrinking"),
        Elfworks => r(StickyFloor, "the floor is tacky"),
        AfterimageFields => r(Silence, "your echoes are heard"),
        SynesthesiaHall => r(BeatFloor, "the tiles keep time"),
        MeltingClockworks => r(RisingTide, "the hours pool up"),
        JellyfishSky => r(GustWind, "the current pushes"),
        WatchingWallpaper => r(Silence, "the walls hear you"),
        WhiteDissolve => r(ShrinkingFloor, "the white eats inward"),
    }
}

/// Does a rule apply here at all?
pub fn active(depth: u32, normal_dream: bool) -> bool {
    normal_dream && depth >= MIN_DEPTH
}

/// 0 below `MIN_DEPTH`; 0.25 at depth 3, +0.1 per depth, clamped to 1.
pub fn strength(depth: u32) -> f32 {
    if depth < MIN_DEPTH {
        return 0.0;
    }
    (0.25 + 0.1 * (depth - MIN_DEPTH) as f32).clamp(0.0, 1.0)
}

/// What the rule does this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effect {
    /// Multiplies walking speed.
    pub speed: f32,
    /// World push (x, z) in units/s.
    pub push: (f32, f32),
    /// Multiplies how bright/far you see (1 = normal).
    pub visibility: f32,
    /// Multiplies how far enemies hear dashes and blinks.
    pub hearing: f32,
    /// Fog drift speed (0 = still, as before).
    pub fog_drift: f32,
    /// A burning beat tile is hot right now.
    pub beat_hot: bool,
}

impl Effect {
    pub const NONE: Effect = Effect {
        speed: 1.0,
        push: (0.0, 0.0),
        visibility: 1.0,
        hearing: 1.0,
        fog_drift: 0.0,
        beat_hot: false,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuleState {
    pub kind: Kind,
    pub strength: f32,
    /// Seconds into the dream.
    pub age: f32,
    /// Seconds since the last shard (or dream start).
    pub since_shard: f32,
    /// Seed for gust directions.
    pub seed: u64,
}

impl RuleState {
    pub fn new(theme: DreamTheme, depth: u32, normal_dream: bool, seed: u64) -> Option<RuleState> {
        if !active(depth, normal_dream) {
            return None;
        }
        Some(RuleState {
            kind: rule_for(theme).kind,
            strength: strength(depth),
            age: 0.0,
            since_shard: 0.0,
            seed,
        })
    }

    pub fn tick(&mut self, dt: f32) {
        self.age += dt;
        self.since_shard += dt;
    }

    /// Reaching a shard resets the tide and the dimming.
    pub fn on_shard(&mut self) {
        self.since_shard = 0.0;
    }

    fn ramp(&self) -> f32 {
        (self.since_shard / FULL_BITE_SECS).clamp(0.0, 1.0)
    }

    /// Gust direction angle of the `n`th gust (own hash, no shared RNG).
    fn gust_angle(&self, n: u64) -> f32 {
        let mut h = self.seed ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 29;
        h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h ^= h >> 32;
        (h % 6283) as f32 / 1000.0
    }

    pub fn effect(&self) -> Effect {
        let s = self.strength;
        let mut e = Effect::NONE;
        match self.kind {
            Kind::RisingTide => e.speed = 1.0 - 0.4 * s * self.ramp(),
            Kind::StickyFloor => e.speed = 1.0 - 0.2 * s,
            Kind::DimmingLight => e.visibility = 1.0 - 0.6 * s * self.ramp(),
            Kind::Silence => e.hearing = 1.0 + 0.8 * s,
            Kind::DriftingFog => e.fog_drift = 0.4 + 0.6 * s,
            Kind::GustWind => {
                let n = (self.age / GUST_PERIOD) as u64;
                if self.age % GUST_PERIOD < GUST_SECS {
                    let a = self.gust_angle(n);
                    let m = GUST_PUSH * s;
                    e.push = (a.cos() * m, a.sin() * m);
                }
            }
            Kind::BeatFloor => e.beat_hot = (self.age / BEAT_SECS) as u64 % 2 == 0,
            Kind::ShrinkingFloor => {}
        }
        e
    }
}

/// Floor cells that are gone after `age` seconds of a shrinking floor.
/// `cells` are every floor cell, `route` the shard/portal route (it and a
/// one-cell margin are kept safe), `center` where the collapse closes in on.
/// Outer cells go first; route cells never go. Pure and deterministic.
pub fn shrunk_cells(
    cells: &[(i32, i32)],
    route: &[(i32, i32)],
    center: (i32, i32),
    age: f32,
    strength: f32,
) -> Vec<(i32, i32)> {
    let safe = |c: &(i32, i32)| {
        route
            .iter()
            .any(|r| (r.0 - c.0).abs() <= 1 && (r.1 - c.1).abs() <= 1)
    };
    let mut cand: Vec<(i32, i32)> = cells.iter().copied().filter(|c| !safe(c)).collect();
    let d = |c: &(i32, i32)| (c.0 - center.0).abs().max((c.1 - center.1).abs());
    cand.sort_by(|a, b| d(b).cmp(&d(a)).then(a.cmp(b)));
    // Full strength: a cell every 2s; 0.25: every ~5s. Never more than half.
    let per_sec = 0.125 + 0.375 * strength;
    let n = ((age.max(0.0) * per_sec) as usize).min(cand.len() / 2);
    cand.truncate(n);
    cand
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::ALL_THEMES;

    #[test]
    fn every_theme_has_a_rule_with_a_short_line() {
        let mut kinds = std::collections::HashSet::new();
        for &t in ALL_THEMES.iter() {
            let rule = rule_for(t);
            assert!(!rule.line.is_empty() && rule.line.len() <= 28, "{t:?}");
            assert_eq!(rule.line, rule.line.to_lowercase());
            kinds.insert(std::mem::discriminant(&rule.kind));
        }
        assert!(kinds.len() >= 6, "rules should reuse a small set of kinds");
    }

    #[test]
    fn gate_and_strength_ramp() {
        assert!(!active(2, true));
        assert!(active(3, true));
        assert!(!active(5, false));
        assert_eq!(strength(0), 0.0);
        assert_eq!(strength(2), 0.0);
        assert!(strength(3) > 0.0);
        for d in 3..40 {
            assert!(strength(d + 1) >= strength(d));
            assert!(strength(d) <= 1.0);
        }
        assert_eq!(strength(200), 1.0);
        assert!(RuleState::new(DreamTheme::SkyStairs, 2, true, 1).is_none());
        assert!(RuleState::new(DreamTheme::SkyStairs, 3, false, 1).is_none());
        assert!(RuleState::new(DreamTheme::SkyStairs, 3, true, 1).is_some());
    }

    fn st(kind: Kind, depth: u32) -> RuleState {
        RuleState {
            kind,
            strength: strength(depth),
            age: 0.0,
            since_shard: 0.0,
            seed: 7,
        }
    }

    #[test]
    fn tide_slows_with_age_and_resets_on_shard() {
        let mut s = st(Kind::RisingTide, 6);
        assert_eq!(s.effect().speed, 1.0);
        s.tick(30.0);
        let mid = s.effect().speed;
        s.tick(60.0);
        let late = s.effect().speed;
        assert!(mid < 1.0 && late < mid && late > 0.4);
        s.on_shard();
        assert_eq!(s.effect().speed, 1.0);
    }

    #[test]
    fn dimming_fades_and_resets() {
        let mut s = st(Kind::DimmingLight, 8);
        s.tick(100.0);
        assert!(s.effect().visibility < 0.6);
        s.on_shard();
        assert_eq!(s.effect().visibility, 1.0);
    }

    #[test]
    fn gusts_are_periodic_and_deterministic() {
        let mut s = st(Kind::GustWind, 5);
        assert_ne!(s.effect().push, (0.0, 0.0));
        s.tick(GUST_SECS + 0.1);
        assert_eq!(s.effect().push, (0.0, 0.0));
        s.tick(GUST_PERIOD - GUST_SECS); // age = period + 0.1
        let p = s.effect().push;
        assert_ne!(p, (0.0, 0.0));
        let mut t = st(Kind::GustWind, 5);
        t.tick(GUST_PERIOD + 0.1);
        assert_eq!(t.effect().push, p);
        let m = (p.0 * p.0 + p.1 * p.1).sqrt();
        assert!((m - GUST_PUSH * strength(5)).abs() < 1e-3);
    }

    #[test]
    fn other_kinds_bite_a_little() {
        assert!(st(Kind::StickyFloor, 3).effect().speed < 1.0);
        assert!(st(Kind::Silence, 3).effect().hearing > 1.0);
        assert!(st(Kind::DriftingFog, 3).effect().fog_drift > 0.0);
        let mut b = st(Kind::BeatFloor, 3);
        let first = b.effect().beat_hot;
        b.tick(BEAT_SECS);
        assert_ne!(first, b.effect().beat_hot);
        assert!(st(Kind::StickyFloor, 99).effect().speed >= 0.7);
    }

    #[test]
    fn shrinking_never_removes_the_route() {
        let cells: Vec<(i32, i32)> = (0..15).flat_map(|x| (0..15).map(move |z| (x, z))).collect();
        let route: Vec<(i32, i32)> = (0..15).map(|x| (x, 7)).collect();
        let mut last = 0;
        for age in [0.0, 10.0, 60.0, 600.0, 1e6] {
            let gone = shrunk_cells(&cells, &route, (7, 7), age, 1.0);
            assert!(gone.len() >= last);
            last = gone.len();
            for g in &gone {
                assert!(!route
                    .iter()
                    .any(|r| (r.0 - g.0).abs() <= 1 && (r.1 - g.1).abs() <= 1));
            }
        }
        assert!(last > 0 && last <= cells.len() / 2);
        let early = shrunk_cells(&cells, &route, (7, 7), 20.0, 1.0);
        let near = early
            .iter()
            .map(|c| (c.0 - 7).abs().max((c.1 - 7).abs()))
            .min()
            .unwrap();
        assert!(near >= 6);
    }
}
