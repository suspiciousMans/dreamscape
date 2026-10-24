//! Combat edges: what some dreams teach a card to do with your STRIKE.
//!
//! Most cards bring an ability and a passive. A few dreams also give their
//! card an *edge*, a way of fighting that is on while the card is equipped.
//! (Fused cards carry both dreams' edges.) Pure rules: the game feeds in the
//! clock and asks what to do, so every number is tested.

use crate::cards::Attribute;
use crate::dream::DreamTheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Edge {
    /// Your strike cuts through armour: a Lullaby's front, a Mask's shield.
    Rending,
    /// Every strike carries you forward a step.
    Lunge,
    /// A strike thrown while you are still shaking off a hit cuts deeper.
    Reprisal,
    /// Three strikes in quick succession: the third cuts deeper.
    Flourish,
    /// Strikes shove twice as far and leave enemies reeling longer.
    Haymaker,
    /// Every third enemy you undo steadies you.
    Siphon,
    /// You recover from a strike much faster.
    Quickdraw,
    /// A strike marks its target: the next blow on it cuts deeper.
    Hexmark,
    // === edges2 ===
    /// The fan of a strike is much wider.
    Cleave,
    /// A strike reaches a quarter further.
    Longarm,
    /// A strike at something that has not noticed you cuts deeper.
    Ambush,
    /// The first blow on an untouched foe cuts deeper.
    Opener,
    /// A blow rattles the foes standing near its target.
    Ripple,
    /// Every enemy you undo leaves a calm mote.
    Lantern,
    /// A reeling foe takes a deeper cut.
    Opportunist,
    /// Every enemy you undo leaves extra dust.
    Windfall,
    // === end edges2 ===
}

#[cfg(test)]
pub const ALL_EDGES: [Edge; 16] = [
    Edge::Rending,
    Edge::Lunge,
    Edge::Reprisal,
    Edge::Flourish,
    Edge::Haymaker,
    Edge::Siphon,
    Edge::Quickdraw,
    Edge::Hexmark,
    Edge::Cleave,
    Edge::Longarm,
    Edge::Ambush,
    Edge::Opener,
    Edge::Ripple,
    Edge::Lantern,
    Edge::Opportunist,
    Edge::Windfall,
];

impl Edge {
    pub fn name(self) -> &'static str {
        match self {
            Edge::Rending => "RENDING",
            Edge::Lunge => "LUNGE",
            Edge::Reprisal => "REPRISAL",
            Edge::Flourish => "FLOURISH",
            Edge::Haymaker => "HAYMAKER",
            Edge::Siphon => "SIPHON",
            Edge::Quickdraw => "QUICKDRAW",
            Edge::Hexmark => "HEXMARK",
            Edge::Cleave => "CLEAVE",
            Edge::Longarm => "LONGARM",
            Edge::Ambush => "AMBUSH",
            Edge::Opener => "OPENER",
            Edge::Ripple => "RIPPLE",
            Edge::Lantern => "LANTERN",
            Edge::Opportunist => "PUNISHER",
            Edge::Windfall => "WINDFALL",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Edge::Rending => "strikes cut through armour and shields",
            Edge::Lunge => "every strike carries you forward",
            Edge::Reprisal => "strikes cut deeper while you shake off a hit",
            Edge::Flourish => "the third strike in a row cuts deeper",
            Edge::Haymaker => "strikes shove twice as far and stagger longer",
            Edge::Siphon => "every third enemy undone steadies you",
            Edge::Quickdraw => "recover from a strike much faster",
            Edge::Hexmark => "a strike marks it: the next blow cuts deeper",
            Edge::Cleave => "your strike fans out much wider",
            Edge::Longarm => "your strike reaches a quarter further",
            Edge::Ambush => "strikes cut deeper at the unaware",
            Edge::Opener => "the first blow on a fresh foe cuts deeper",
            Edge::Ripple => "a blow rattles the foes beside its target",
            Edge::Lantern => "every enemy undone leaves a calm mote",
            Edge::Opportunist => "a reeling foe takes a deeper cut",
            Edge::Windfall => "every enemy undone leaves extra dust",
        }
    }

    pub fn color(self) -> [u8; 3] {
        match self {
            Edge::Rending => [255, 120, 80],
            Edge::Lunge => [120, 220, 255],
            Edge::Reprisal => [255, 90, 140],
            Edge::Flourish => [255, 230, 120],
            Edge::Haymaker => [255, 170, 90],
            Edge::Siphon => [150, 255, 190],
            Edge::Quickdraw => [200, 170, 255],
            Edge::Hexmark => [190, 110, 255],
            Edge::Cleave => [255, 200, 150],
            Edge::Longarm => [130, 200, 255],
            Edge::Ambush => [170, 150, 220],
            Edge::Opener => [255, 255, 160],
            Edge::Ripple => [110, 210, 230],
            Edge::Lantern => [255, 240, 170],
            Edge::Opportunist => [255, 130, 110],
            Edge::Windfall => [255, 215, 90],
        }
    }
}

/// Which dreams teach an edge: sixteen attributes of the twenty.
pub fn edge_of(attribute: Attribute) -> Option<Edge> {
    Some(match attribute {
        Attribute::Rust => Edge::Rending,
        Attribute::Abyss => Edge::Lunge,
        Attribute::Glass => Edge::Reprisal,
        Attribute::Chord => Edge::Flourish,
        Attribute::Jest => Edge::Haymaker,
        Attribute::Spore => Edge::Siphon,
        Attribute::Hour => Edge::Quickdraw,
        Attribute::Hex => Edge::Hexmark,
        // === edges2 === only attributes that had none
        Attribute::Dawn => Edge::Cleave,
        Attribute::Aether => Edge::Longarm,
        Attribute::Blank => Edge::Ambush,
        Attribute::Static => Edge::Opener,
        Attribute::Tide => Edge::Ripple,
        Attribute::Trace => Edge::Lantern,
        Attribute::Gaze => Edge::Opportunist,
        Attribute::Velvet => Edge::Windfall,
        // === end edges2 ===
        _ => return None,
    })
}

pub fn edge(theme: DreamTheme) -> Option<Edge> {
    edge_of(Attribute::of(theme))
}

/// LUNGE: how far a strike carries you, how long that takes.
pub const LUNGE_DIST: f32 = 1.5;
pub const LUNGE_SECS: f32 = 0.12;
pub const LUNGE_SPEED: f32 = LUNGE_DIST / LUNGE_SECS;
/// QUICKDRAW: the strike's cooldown multiplier, and its recovery shortening.
pub const QUICKDRAW_MULT: f32 = 0.6;
/// HAYMAKER: shove and stagger multipliers.
pub const HAYMAKER_KNOCK: f32 = 2.0;
pub const HAYMAKER_STAGGER: f32 = 1.5;
/// FLOURISH: strikes within this many seconds of the last are one chain; the
/// third of a chain cuts deeper.
pub const FLOURISH_WINDOW: f32 = 1.2;
pub const FLOURISH_EVERY: u32 = 3;
/// SIPHON: every this-many undone enemies, one nerve.
pub const SIPHON_EVERY: u32 = 3;
/// HEXMARK: how long a mark lasts.
pub const HEXMARK_SECS: f32 = 3.0;
/// The extra damage from REPRISAL, FLOURISH and a HEXMARK.
pub const EDGE_BONUS: f32 = 1.0;

// === edges2 ===
/// CLEAVE: the fan's half-arc cosine (smaller = wider than WIDE EYES).
pub const CLEAVE_ARC_COS: f32 = 0.1;
/// LONGARM: reach multiplier.
pub const LONGARM_REACH: f32 = 1.25;
/// RIPPLE: how far from the target other foes are rattled, and for how long.
pub const RIPPLE_REACH: f32 = 2.5;
pub const RIPPLE_STAGGER: f32 = 0.6;
/// WINDFALL: extra dust per enemy undone.
pub const WINDFALL_DUST: u32 = 2;

/// The strike's fan, given its normal half-arc cosine.
pub fn arc_cos(base: f32, cleave: bool) -> f32 {
    if cleave {
        base.min(CLEAVE_ARC_COS)
    } else {
        base
    }
}

/// The strike's reach, given its normal reach.
pub fn reach(base: f32, longarm: bool) -> f32 {
    if longarm {
        base * LONGARM_REACH
    } else {
        base
    }
}

/// Extra damage from AMBUSH, OPENER and PUNISHER for one blow.
pub fn bonus(edges: &[Edge], unaware: bool, staggered: bool, fresh: bool) -> f32 {
    let on = |e| edges.contains(&e);
    let mut b = 0.0;
    if on(Edge::Ambush) && unaware {
        b += EDGE_BONUS;
    }
    if on(Edge::Opener) && fresh {
        b += EDGE_BONUS;
    }
    if on(Edge::Opportunist) && staggered {
        b += EDGE_BONUS;
    }
    b
}
// === end edges2 ===

/// FLOURISH's chain of strikes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Chain {
    count: u32,
    last: f32,
}

impl Chain {
    /// A strike at `now`. True when it is the one that cuts deeper.
    pub fn strike(&mut self, now: f32) -> bool {
        if self.count == 0 || now - self.last > FLOURISH_WINDOW {
            self.count = 0;
        }
        self.count += 1;
        self.last = now;
        if self.count >= FLOURISH_EVERY {
            self.count = 0;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// SIPHON's count of enemies undone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally(u32);

impl Tally {
    /// One more undone. True when this one steadies you.
    pub fn undone(&mut self) -> bool {
        self.0 += 1;
        if self.0 >= SIPHON_EVERY {
            self.0 = 0;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.0 = 0;
    }
}

/// The forward speed of a LUNGE `left` seconds from its end (0 when over).
pub fn lunge_speed(left: f32) -> f32 {
    if left > 0.0 {
        LUNGE_SPEED
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::ALL_THEMES;

    #[test]
    fn sixteen_attributes_teach_sixteen_different_edges_and_the_rest_none() {
        let with: Vec<Edge> = ALL_THEMES.iter().filter_map(|&t| edge(t)).collect();
        assert_eq!(with.len(), 16, "{with:?}");
        for e in ALL_EDGES {
            assert_eq!(with.iter().filter(|&&x| x == e).count(), 1, "{e:?}");
        }
        assert!(ALL_THEMES.iter().filter(|&&t| edge(t).is_none()).count() == 4);
    }

    #[test]
    fn every_edge_has_a_name_a_short_line_and_a_colour() {
        for e in ALL_EDGES {
            assert!(!e.name().is_empty() && e.name().len() <= 10, "{e:?}");
            assert!(e.desc().len() <= 50, "{e:?}: {}", e.desc());
            assert!(e.color().iter().any(|&c| c > 100));
        }
        let names: std::collections::HashSet<_> = ALL_EDGES.iter().map(|e| e.name()).collect();
        assert_eq!(names.len(), ALL_EDGES.len());
    }

    #[test]
    fn flourish_rewards_the_third_quick_strike_and_not_a_slow_one() {
        let mut c = Chain::default();
        assert!(!c.strike(0.0));
        assert!(!c.strike(0.5));
        assert!(c.strike(1.0), "third within the window");
        assert!(!c.strike(1.5), "a fresh chain");
        assert!(!c.strike(1.9));
        assert!(c.strike(2.3));
        // Too slow: the chain breaks.
        let mut s = Chain::default();
        assert!(!s.strike(0.0));
        assert!(!s.strike(0.5));
        assert!(!s.strike(2.5), "a long pause broke it");
        assert!(!s.strike(2.9));
        assert!(s.strike(3.3));
    }

    #[test]
    fn siphon_steadies_on_every_third_undone() {
        let mut t = Tally::default();
        let got: Vec<bool> = (0..7).map(|_| t.undone()).collect();
        assert_eq!(got, vec![false, false, true, false, false, true, false]);
        t.reset();
        assert!(!t.undone());
    }

    #[test]
    fn a_lunge_covers_its_distance_in_its_time_and_stops() {
        assert!((LUNGE_SPEED * LUNGE_SECS - LUNGE_DIST).abs() < 1e-4);
        assert_eq!(lunge_speed(0.05), LUNGE_SPEED);
        assert_eq!(lunge_speed(0.0), 0.0);
        assert!(LUNGE_DIST < crate::combat::REACH, "a step, not a dash");
    }

    #[test]
    fn the_original_eight_edges_keep_their_attributes() {
        let old = [
            (Attribute::Rust, Edge::Rending),
            (Attribute::Abyss, Edge::Lunge),
            (Attribute::Glass, Edge::Reprisal),
            (Attribute::Chord, Edge::Flourish),
            (Attribute::Jest, Edge::Haymaker),
            (Attribute::Spore, Edge::Siphon),
            (Attribute::Hour, Edge::Quickdraw),
            (Attribute::Hex, Edge::Hexmark),
        ];
        for (a, e) in old {
            assert_eq!(edge_of(a), Some(e));
        }
    }

    #[test]
    fn cleave_widens_the_fan_and_longarm_stretches_it() {
        assert!(arc_cos(crate::combat::ARC_COS, true) < crate::combat::ARC_COS_WIDE);
        assert_eq!(arc_cos(crate::combat::ARC_COS, false), crate::combat::ARC_COS);
        assert_eq!(arc_cos(0.0, true), 0.0, "never narrows");
        assert!((reach(2.0, true) - 2.5).abs() < 1e-4);
        assert_eq!(reach(2.0, false), 2.0);
    }

    #[test]
    fn ambush_opener_and_punisher_each_add_a_point_only_when_their_condition_holds() {
        let all = [Edge::Ambush, Edge::Opener, Edge::Opportunist];
        assert_eq!(bonus(&all, false, false, false), 0.0);
        assert_eq!(bonus(&[Edge::Ambush], true, true, true), EDGE_BONUS);
        assert_eq!(bonus(&[Edge::Opener], true, true, true), EDGE_BONUS);
        assert_eq!(bonus(&[Edge::Opportunist], true, true, true), EDGE_BONUS);
        assert_eq!(bonus(&all, true, true, true), 3.0 * EDGE_BONUS);
        assert_eq!(bonus(&[], true, true, true), 0.0, "no edge, no bonus");
        assert_eq!(bonus(&[Edge::Ambush], false, true, true), 0.0);
    }

    #[test]
    fn the_edges_are_balanced_by_their_numbers() {
        assert!(QUICKDRAW_MULT < 1.0 && QUICKDRAW_MULT > 0.3);
        assert!(HAYMAKER_KNOCK > 1.0 && HAYMAKER_STAGGER > 1.0);
        assert!(EDGE_BONUS >= 1.0);
        assert!(HEXMARK_SECS > crate::combat::COOLDOWN, "a mark outlasts a strike");
    }
}
