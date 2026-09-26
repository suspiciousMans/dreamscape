//! The run summary, shown on waking before the pack opens: how deep you
//! went, what you picked up, what the dream threw at you, and how your
//! dust will be worked out. Lines type themselves out one after another.

use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SummaryView {
    /// (label, value) rows.
    pub rows: Vec<(String, String)>,
    pub upgrades: Vec<String>,
    pub synergies: Vec<String>,
    /// How the dust multiplier is made up.
    pub dust: String,
}

/// Seconds between rows appearing.
pub const ROW_DELAY: f32 = 0.12;

/// Characters of a row shown at `age` (types out, then holds).
pub fn typed(row: usize, age: f32, len: usize) -> usize {
    let t = age - row as f32 * ROW_DELAY;
    if t <= 0.0 {
        0
    } else {
        ((t * 60.0) as usize).min(len)
    }
}

fn clip(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

pub fn draw(p: &egui::Painter, screen: Rect, v: &HudView, s: &SummaryView) {
    p.rect_filled(screen, 0.0, rgba([6, 3, 16], 1.0));
    let cx = screen.center().x;
    let top = screen.top() + screen.height() * 0.08;
    glitch_text(
        p,
        Pos2::new(cx, top),
        true,
        "YOU WAKE UP",
        52.0,
        ink(v),
        1.0,
        v,
    );
    let age = v.title_age;
    let left = cx - 330.0;
    let mut y = top + 80.0;
    let mut row = 0;
    for (label, value) in &s.rows {
        let text_len = label.len() + value.len();
        let n = typed(row, age, text_len);
        p.text(
            Pos2::new(left, y),
            Align2::LEFT_TOP,
            clip(label, n),
            FontId::monospace(22.0),
            rgba([180, 170, 210], 0.9),
        );
        p.text(
            Pos2::new(left + 300.0, y),
            Align2::LEFT_TOP,
            clip(value, n.saturating_sub(label.len())),
            FontId::monospace(22.0),
            rgba(ink(v), 1.0),
        );
        y += 28.0;
        row += 1;
    }
    y += 10.0;
    let list =
        |p: &egui::Painter, y: &mut f32, row: &mut usize, title: &str, items: &[String], rgb| {
            if items.is_empty() {
                return;
            }
            let line = format!("{title}: {}", items.join(", "));
            let n = typed(*row, age, line.len());
            let galley = p.layout(
                clip(&line, n),
                FontId::monospace(19.0),
                rgba(rgb, 0.95),
                660.0,
            );
            let h = galley.size().y;
            p.galley(Pos2::new(left, *y), galley, rgba(rgb, 0.95));
            *y += h + 8.0;
            *row += 1;
        };
    list(
        p,
        &mut y,
        &mut row,
        "upgrades",
        &s.upgrades,
        [255, 200, 120],
    );
    list(p, &mut y, &mut row, "combos", &s.synergies, [140, 255, 220]);
    let n = typed(row, age, s.dust.len());
    p.text(
        Pos2::new(left, y + 6.0),
        Align2::LEFT_TOP,
        clip(&s.dust, n),
        FontId::monospace(22.0),
        rgba([255, 210, 80], 1.0),
    );
    if n == s.dust.len() {
        p.text(
            Pos2::new(cx, screen.bottom() - 52.0),
            Align2::CENTER_TOP,
            v.k("[enter] see what you remember"),
            FontId::monospace(22.0),
            rgba(hue(v.time * 0.2), 0.9),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_type_out_in_order_and_finish() {
        assert_eq!(typed(0, 0.0, 10), 0);
        assert!(typed(0, 0.1, 30) > typed(1, 0.1, 30));
        assert_eq!(typed(3, 10.0, 25), 25);
        assert_eq!(clip("héllo", 2), "hé");
    }
}
