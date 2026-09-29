//! The ending: once all twelve of your own notes are found, the next portal
//! leads to the bottom. The eye speaks six lines over thirty seconds, then
//! closes; credits and the helpline, then the run goes on (awake now).
//! Lines approved in `plans/2026-09-28-dreamscape-rest-of-project-plan.md`.

use crate::hud::{ink, rgba, HudView};
use crate::memories_ui::HELP_LINE;
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect};

pub const LINES: [&str; 6] = [
    "you came all this way down. i was never trying to keep you here.",
    "every nightmare was me, holding on too tight. i'm sorry it hurt.",
    "the notes were yours. the others' too. none of you were alone in it.",
    "you don't have to be fixed to wake up. you just have to wake up.",
    "i'll still be here. i'm the part of you that noticed.",
    "open your eyes. it's morning, or close enough.",
];

/// Seconds each line holds.
pub const LINE_SECONDS: f32 = 5.0;
pub const SPOKEN: f32 = LINE_SECONDS * LINES.len() as f32;
/// The lid closes over this long, after the last line.
pub const CLOSING: f32 = 3.0;
/// Credits start here; [enter] works from here on.
pub const CREDITS_AT: f32 = SPOKEN + CLOSING;

pub const CREDITS: [&str; 4] = [
    "DREAMSCAPE",
    "made on the Jame engine",
    "thank you for dreaming",
    "the others' notes still wait below",
];

/// The line being spoken `age` seconds in, and how long it has been up.
pub fn line_at(age: f32) -> Option<(usize, f32)> {
    if !(0.0..SPOKEN).contains(&age) {
        return None;
    }
    let i = (age / LINE_SECONDS) as usize;
    Some((i, age - i as f32 * LINE_SECONDS))
}

/// How open the eye is: wide while it talks, then the lid comes down.
pub fn openness(age: f32) -> f32 {
    if age < SPOKEN {
        1.0
    } else {
        (1.0 - (age - SPOKEN) / CLOSING).clamp(0.0, 1.0)
    }
}

pub fn can_continue(age: f32) -> bool {
    age >= CREDITS_AT
}

pub fn draw(p: &egui::Painter, screen: Rect, v: &HudView) {
    let age = v.ending_age;
    let dark = (age / 2.0).clamp(0.0, 1.0);
    p.rect_filled(screen, 0.0, rgba([4, 2, 10], 0.85 * dark + 0.1));
    let c = Pos2::new(screen.center().x, screen.center().y - 60.0);
    if openness(age) > 0.0 {
        crate::hud::draw_big_eye(p, c, openness(age), v);
    }
    if let Some((i, t)) = line_at(age) {
        let text = LINES[i];
        let a = ((LINE_SECONDS - t) / 0.8).clamp(0.0, 1.0);
        p.text(
            Pos2::new(c.x, c.y + 140.0),
            Align2::CENTER_TOP,
            crate::eye::shown(text, t),
            FontId::proportional(28.0),
            rgba([226, 214, 250], a),
        );
    }
    if age >= CREDITS_AT {
        let a = ((age - CREDITS_AT) / 2.0).clamp(0.0, 1.0);
        let mut y = screen.center().y - 80.0;
        for (i, line) in CREDITS.iter().enumerate() {
            p.text(
                Pos2::new(c.x, y),
                Align2::CENTER_TOP,
                *line,
                FontId::monospace(if i == 0 { 44.0 } else { 20.0 }),
                rgba(ink(v), a),
            );
            y += if i == 0 { 70.0 } else { 34.0 };
        }
        p.text(
            Pos2::new(c.x, screen.bottom() - 90.0),
            Align2::CENTER_TOP,
            HELP_LINE,
            FontId::monospace(16.0),
            rgba([190, 180, 220], 0.85 * a),
        );
        p.text(
            Pos2::new(c.x, screen.bottom() - 50.0),
            Align2::CENTER_TOP,
            v.k("[enter] wake up"),
            FontId::monospace(20.0),
            rgba(ink(v), a * (0.6 + 0.3 * (v.time * 2.0).sin())),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_short_lines_over_thirty_seconds() {
        assert_eq!(SPOKEN, 30.0);
        for l in LINES {
            assert!(!l.is_empty() && l.len() <= 90 && !l.contains('{'), "{l}");
        }
        assert_eq!(line_at(0.0), Some((0, 0.0)));
        assert_eq!(line_at(29.9).map(|l| l.0), Some(5));
        assert_eq!(line_at(30.0), None);
    }

    #[test]
    fn the_eye_closes_after_the_last_line_then_credits() {
        assert_eq!(openness(10.0), 1.0);
        assert!(openness(SPOKEN + 1.0) < 1.0);
        assert_eq!(openness(CREDITS_AT), 0.0);
        assert!(!can_continue(SPOKEN));
        assert!(can_continue(CREDITS_AT));
    }
}
