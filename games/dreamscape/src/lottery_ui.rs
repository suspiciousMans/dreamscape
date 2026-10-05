//! The Dream Lottery: a tab of the Lucid Store where dust buys a pull at the
//! moth's deck. Odds are shown exactly as they are rolled, and the pity
//! counters are shown too, so a dry spell always has a visible end.

use crate::booklet_ui::{card, rarity_color, CardArt, CARD_SIZE};
use crate::cards::{Attribute, Card, Rarity, Recalled};
use crate::dream::DreamTheme;
use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect, Stroke, Vec2};

/// Seconds the deck shudders before the card turns over.
pub const SHUFFLE: f32 = 1.1;
/// Seconds the reveal flash and shake take to settle.
pub const SETTLE: f32 = 0.7;

/// What the last pull produced.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub card: Card,
    pub foil: bool,
    pub pity: bool,
    pub near_miss: Option<Rarity>,
    pub recurring: bool,
}

/// Everything the lottery screen needs for one frame.
#[derive(Clone, Debug, Default)]
pub struct LotteryView {
    pub odds: Vec<(Rarity, f32)>,
    pub since_lucid: u32,
    pub hard_lucid: u32,
    pub since_prophetic: u32,
    pub hard_prophetic: u32,
    pub sparks: u32,
    pub spark_goal: u32,
    pub featured: Option<DreamTheme>,
    pub featured_odds: f32,
    /// The theme a spark pick would choose.
    pub pick: Option<DreamTheme>,
    pub cost: u32,
    pub total_pulls: u32,
    pub outcome: Option<Outcome>,
    /// Seconds since the last pull was made.
    pub age: f32,
    pub status: Option<String>,
}

/// 0 while the deck shudders, then 1.0 -> 0.0 as the reveal settles.
pub fn settle(age: f32) -> f32 {
    if age < SHUFFLE {
        0.0
    } else {
        (1.0 - (age - SHUFFLE) / SETTLE).clamp(0.0, 1.0)
    }
}

pub fn revealed(age: f32) -> bool {
    age >= SHUFFLE
}

/// Words for what just happened, most exciting first.
pub fn headline(o: &Outcome) -> String {
    if o.foil {
        "FOIL! A CARD THAT SHINES BACK".to_string()
    } else if o.pity {
        "THE DECK KEEPS ITS PROMISE".to_string()
    } else if o.card.rarity >= Rarity::Prophetic {
        "PROPHETIC. IT KNEW YOU WERE COMING".to_string()
    } else if let Some(r) = o.near_miss {
        format!("SO CLOSE... A {} STIRRED", r.label())
    } else if o.recurring {
        "A DREAM YOU'VE HAD BEFORE".to_string()
    } else {
        "A NEW DREAM".to_string()
    }
}

fn theme_name(t: DreamTheme) -> &'static str {
    Attribute::of(t).label()
}

pub fn draw(ctx: &egui::Context, p: &egui::Painter, screen: Rect, v: &HudView, art: &mut CardArt) {
    let l = &v.lottery;
    p.rect_filled(screen, 0.0, rgba([6, 3, 16], 0.96));
    for k in 0..60u32 {
        let h1 = crate::hud::hash01(k, 7);
        let h2 = crate::hud::hash01(k, 8);
        let x = screen.left() + h1 * screen.width();
        let y = screen.bottom() - ((h2 * screen.height() + v.time * (8.0 + 10.0 * h1)) % screen.height());
        p.rect_filled(
            Rect::from_center_size(Pos2::new(x, y), Vec2::splat(2.0 + 2.0 * h2)),
            0.0,
            rgba(hue(0.7 + 0.2 * h1), 0.15 + 0.2 * h2),
        );
    }
    let cx = screen.center().x;
    glitch_text(
        p,
        Pos2::new(cx, screen.top() + 14.0),
        true,
        "THE MOTH'S LOTTERY",
        42.0,
        hue(0.75 + 0.03 * (v.time * 0.7).sin()),
        1.0,
        v,
    );
    p.text(
        Pos2::new(cx, screen.top() + 62.0),
        Align2::CENTER_TOP,
        format!("{} dream dust   ·   a pull costs {}   ·   {} pulls so far", v.dust, l.cost, l.total_pulls),
        FontId::monospace(22.0),
        rgba([255, 210, 80], 1.0),
    );

    odds_panel(p, screen, v);
    stage(ctx, p, screen, v, art);

    // Controls and status, bottom.
    let can_pull = v.dust >= l.cost;
    p.text(
        Pos2::new(cx, screen.bottom() - 62.0),
        Align2::CENTER_BOTTOM,
        v.k(&format!(
            "[enter] pull ({} dust)   [a/d] pick a dream   [p] spark pick ({}/{})",
            l.cost, l.sparks, l.spark_goal
        )),
        FontId::monospace(22.0),
        rgba(ink(v), if can_pull { 0.9 } else { 0.45 }),
    );
    p.text(
        Pos2::new(cx, screen.bottom() - 34.0),
        Align2::CENTER_BOTTOM,
        v.k("[tab] back to the store   [esc] leave"),
        FontId::monospace(20.0),
        rgba(ink(v), 0.5),
    );
    if let Some(s) = &l.status {
        p.text(
            Pos2::new(cx, screen.bottom() - 90.0),
            Align2::CENTER_BOTTOM,
            s,
            FontId::monospace(22.0),
            rgba([255, 210, 80], 0.95),
        );
    }
}

/// The honest numbers: exact odds, pity, sparks, tonight's featured dream.
fn odds_panel(p: &egui::Painter, screen: Rect, v: &HudView) {
    let l = &v.lottery;
    let left = screen.left() + 36.0;
    let mut y = screen.top() + 110.0;
    p.text(Pos2::new(left, y), Align2::LEFT_TOP, "ODDS RIGHT NOW", FontId::monospace(20.0), rgba(ink(v), 0.85));
    y += 28.0;
    let bar_w = 150.0;
    for &(r, share) in &l.odds {
        let col = rarity_color(r, v.time);
        p.text(Pos2::new(left, y), Align2::LEFT_TOP, r.label(), FontId::monospace(18.0), rgba(col, 1.0));
        let bar = Rect::from_min_size(Pos2::new(left + 110.0, y + 3.0), Vec2::new(bar_w, 12.0));
        p.rect_stroke(bar, 0.0, Stroke::new(1.0_f32, rgba(ink(v), 0.3)));
        p.rect_filled(
            Rect::from_min_size(bar.min, Vec2::new(bar_w * share.clamp(0.0, 1.0), 12.0)),
            0.0,
            rgba(col, 0.85),
        );
        p.text(
            Pos2::new(left + 110.0 + bar_w + 10.0, y),
            Align2::LEFT_TOP,
            format!("{:.1}%", share * 100.0),
            FontId::monospace(18.0),
            rgba(ink(v), 0.8),
        );
        y += 24.0;
    }
    y += 14.0;
    for (label, have, need, col) in [
        ("LUCID GUARANTEED", l.since_lucid, l.hard_lucid, rarity_color(Rarity::Lucid, v.time)),
        ("PROPHETIC GUARANTEED", l.since_prophetic, l.hard_prophetic, rarity_color(Rarity::Prophetic, v.time)),
        ("SPARKS", l.sparks, l.spark_goal, [255, 210, 80]),
    ] {
        let left_to_go = need.saturating_sub(have);
        let note = if label == "SPARKS" {
            format!("{have} / {need}")
        } else {
            format!("in {left_to_go} pulls")
        };
        p.text(Pos2::new(left, y), Align2::LEFT_TOP, label, FontId::monospace(16.0), rgba(ink(v), 0.7));
        p.text(Pos2::new(left + 270.0, y), Align2::RIGHT_TOP, note, FontId::monospace(16.0), rgba(ink(v), 0.7));
        let bar = Rect::from_min_size(Pos2::new(left, y + 18.0), Vec2::new(270.0, 8.0));
        p.rect_stroke(bar, 0.0, Stroke::new(1.0_f32, rgba(ink(v), 0.3)));
        let f = if need == 0 { 0.0 } else { (have as f32 / need as f32).clamp(0.0, 1.0) };
        p.rect_filled(Rect::from_min_size(bar.min, Vec2::new(270.0 * f, 8.0)), 0.0, rgba(col, 0.9));
        y += 38.0;
    }
    y += 6.0;
    if let Some(t) = l.featured {
        p.text(
            Pos2::new(left, y),
            Align2::LEFT_TOP,
            format!("TONIGHT'S DREAM: {}  ({:.0}% of pulls)", theme_name(t), l.featured_odds * 100.0),
            FontId::monospace(18.0),
            rgba(hue(v.time * 0.2), 0.95),
        );
        y += 24.0;
    }
    if let Some(t) = l.pick {
        p.text(
            Pos2::new(left, y),
            Align2::LEFT_TOP,
            format!("SPARK PICK WOULD CHOOSE: < {} >", theme_name(t)),
            FontId::monospace(18.0),
            rgba(ink(v), 0.8),
        );
    }
}

/// The card area: the idle deck, the shudder, then the card itself.
fn stage(ctx: &egui::Context, p: &egui::Painter, screen: Rect, v: &HudView, art: &mut CardArt) {
    let l = &v.lottery;
    let scale = ((screen.height() - 300.0) / CARD_SIZE.y).clamp(0.4, 0.9);
    let size = CARD_SIZE * scale;
    let centre = Pos2::new(screen.center().x + 150.0, screen.top() + 110.0 + size.y * 0.5 + 10.0);
    let mut rect = Rect::from_center_size(centre, size);

    let Some(o) = &l.outcome else {
        deck(p, rect, v, 0.0);
        p.text(
            Pos2::new(centre.x, rect.bottom() + 12.0),
            Align2::CENTER_TOP,
            "the deck is waiting",
            FontId::monospace(20.0),
            rgba(ink(v), 0.55),
        );
        return;
    };

    if !revealed(l.age) {
        // The deck shudders harder as the card nears.
        let k = (l.age / SHUFFLE).clamp(0.0, 1.0);
        let jitter = Vec2::new(
            (v.time * 61.0).sin() * 6.0 * k,
            (v.time * 47.0).cos() * 4.0 * k,
        );
        rect = rect.translate(jitter);
        deck(p, rect, v, k);
        return;
    }

    let r = Recalled { card: o.card.clone(), remembered: true };
    let beat = crate::reveal_ui::beat(&r);
    let calm = settle(l.age);
    let shake = beat.shake * calm * if v.settings_snapshot.motion() < 1.0 { 0.0 } else { 1.0 };
    rect = rect.translate(Vec2::new((v.time * 83.0).sin(), (v.time * 71.0).cos()) * shake);

    if beat.glow > 0.0 {
        let col = rarity_color(o.card.rarity, v.time);
        for k in 1..=6 {
            p.rect_stroke(
                rect.expand(6.0 * k as f32),
                0.0,
                Stroke::new(2.0_f32, rgba(col, beat.glow * 0.5 / k as f32)),
            );
        }
    }
    card(ctx, p, rect, &o.card, v, art, scale);
    if beat.flash * calm > 0.0 && v.settings_snapshot.motion() >= 1.0 {
        p.rect_filled(screen, 0.0, rgba([255, 255, 255], (beat.flash * calm * 0.8).min(0.8)));
    }
    p.text(
        Pos2::new(centre.x, rect.bottom() + 12.0),
        Align2::CENTER_TOP,
        headline(o),
        FontId::monospace(22.0),
        rgba(if o.foil { hue(v.time * 0.6) } else { rarity_color(o.card.rarity, v.time) }, 1.0),
    );
}

/// A face-down card.
fn deck(p: &egui::Painter, rect: Rect, v: &HudView, excite: f32) {
    let frame = hue(v.time * (0.1 + 0.8 * excite));
    p.rect_filled(rect, 0.0, rgba([16, 9, 30], 1.0));
    p.rect_stroke(rect.shrink(2.0), 0.0, Stroke::new(4.0_f32, rgba(frame, 0.6 + 0.4 * excite)));
    p.rect_stroke(rect.shrink(12.0), 0.0, Stroke::new(1.0_f32, rgba(frame, 0.35)));
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        "?",
        FontId::monospace(rect.width() * 0.5),
        rgba(frame, 0.55 + 0.4 * (v.time * 3.0).sin().abs() * (0.4 + excite)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(rarity: Rarity, foil: bool, pity: bool, near: Option<Rarity>, recurring: bool) -> Outcome {
        let rec = crate::cards::DreamRecord {
            theme: DreamTheme::Garden,
            seed: 1,
            depth: 1,
            name: "X".into(),
            whisper: "y".into(),
            strangeness: 0.0,
            enemies: 0,
            shard_taken: false,
            art: crate::dream::portal_surface(DreamTheme::Garden, 1),
            blend: None,
        };
        let mut card = crate::cards::card_from(&rec);
        card.rarity = rarity;
        Outcome { card, foil, pity, near_miss: near, recurring }
    }

    #[test]
    fn the_reveal_waits_then_settles() {
        assert!(!revealed(0.0));
        assert!(!revealed(SHUFFLE - 0.01));
        assert!(revealed(SHUFFLE));
        assert_eq!(settle(0.5), 0.0, "nothing to settle while shuffling");
        assert!((settle(SHUFFLE) - 1.0).abs() < 1e-6);
        assert_eq!(settle(SHUFFLE + SETTLE + 1.0), 0.0);
        let mut last = 1.0;
        for i in 0..=20 {
            let s = settle(SHUFFLE + SETTLE * i as f32 / 20.0);
            assert!(s <= last + 1e-6, "settle only falls");
            last = s;
        }
    }

    #[test]
    fn the_headline_names_the_most_exciting_thing_first() {
        assert!(headline(&outcome(Rarity::Prophetic, true, true, None, false)).starts_with("FOIL"));
        assert!(headline(&outcome(Rarity::Lucid, false, true, None, false)).contains("PROMISE"));
        assert!(headline(&outcome(Rarity::Prophetic, false, false, None, false)).starts_with("PROPHETIC"));
        assert!(headline(&outcome(Rarity::Hazy, false, false, Some(Rarity::Vivid), false)).contains("VIVID"));
        assert!(headline(&outcome(Rarity::Hazy, false, false, None, true)).contains("BEFORE"));
        assert_eq!(headline(&outcome(Rarity::Hazy, false, false, None, false)), "A NEW DREAM");
    }
}
