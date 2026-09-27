//! The notes you've found: one page when you pick a note up, and the
//! memories screen (every dreamer, their notes in order).

use crate::hud::{glitch_text, ink, rgba, HudView};
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect, Stroke, Vec2};

/// One dreamer's row on the memories screen.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DreamerRow {
    /// "???" until one of their notes is found.
    pub name: String,
    pub found: u32,
    pub total: u32,
    /// What they carry, once every note is found.
    pub weight: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MemoriesView {
    pub rows: Vec<DreamerRow>,
    pub selected: usize,
    /// The selected dreamer's notes, in order (`None` = not found yet).
    pub notes: Vec<Option<String>>,
}

pub const HELP_LINE: &str =
    "if you're carrying something heavy, you don't have to carry it alone · findahelpline.com";

const PAPER: [u8; 3] = [236, 226, 204];
const PAPER_INK: [u8; 3] = [48, 38, 58];

/// A note, just picked up: (object, title, body).
pub fn draw_note(p: &egui::Painter, screen: Rect, v: &HudView, note: &(String, String, String)) {
    let (object, title, body) = note;
    p.rect_filled(screen, 0.0, rgba([4, 2, 10], 0.72));
    let size = Vec2::new(640.0_f32.min(screen.width() * 0.9), 360.0);
    let r = Rect::from_center_size(screen.center(), size);
    p.rect_filled(r, 2.0, rgba(PAPER, 0.97));
    p.rect_stroke(r, 2.0, Stroke::new(3.0_f32, rgba(PAPER_INK, 0.8)));
    p.text(
        r.left_top() + Vec2::new(28.0, 22.0),
        Align2::LEFT_TOP,
        object.to_uppercase(),
        FontId::monospace(16.0),
        rgba(PAPER_INK, 0.6),
    );
    p.text(
        r.left_top() + Vec2::new(28.0, 46.0),
        Align2::LEFT_TOP,
        title,
        FontId::monospace(22.0),
        rgba(PAPER_INK, 0.95),
    );
    let g = p.layout(
        body.clone(),
        FontId::proportional(26.0),
        rgba(PAPER_INK, 1.0),
        size.x - 56.0,
    );
    p.galley(
        r.left_top() + Vec2::new(28.0, 96.0),
        g,
        rgba(PAPER_INK, 1.0),
    );
    p.text(
        Pos2::new(r.center().x, r.bottom() - 30.0),
        Align2::CENTER_CENTER,
        v.k("[enter] keep it"),
        FontId::monospace(18.0),
        rgba(PAPER_INK, 0.7),
    );
}

pub fn draw(p: &egui::Painter, screen: Rect, v: &HudView, m: &MemoriesView) {
    p.rect_filled(screen, 0.0, rgba([6, 3, 16], 0.96));
    let cx = screen.center().x;
    let top = screen.top() + screen.height() * 0.06;
    glitch_text(
        p,
        Pos2::new(cx, top),
        true,
        "MEMORIES",
        48.0,
        ink(v),
        1.0,
        v,
    );
    let left = screen.left() + screen.width() * 0.07;
    let right = screen.left() + screen.width() * 0.38;
    for (i, row) in m.rows.iter().enumerate() {
        let y = top + 70.0 + i as f32 * 34.0;
        let sel = i == m.selected;
        let text = format!(
            "{} {:<10} {}/{}",
            if sel { ">" } else { " " },
            row.name,
            row.found,
            row.total
        );
        p.text(
            Pos2::new(left, y),
            Align2::LEFT_TOP,
            text,
            FontId::monospace(22.0),
            rgba(
                if sel { [255, 220, 140] } else { ink(v) },
                if row.found > 0 { 0.95 } else { 0.5 },
            ),
        );
        if let Some(w) = &row.weight {
            p.text(
                Pos2::new(left + 24.0, y + 22.0),
                Align2::LEFT_TOP,
                w,
                FontId::monospace(13.0),
                rgba([200, 170, 230], 0.8),
            );
        }
    }
    let mut y = top + 70.0;
    let width = screen.right() - right - screen.width() * 0.05;
    for (k, n) in m.notes.iter().enumerate() {
        let (text, a) = match n {
            Some(t) => (format!("{}. {t}", k + 1), 0.92),
            None => (format!("{}. …", k + 1), 0.4),
        };
        let g = p.layout(text, FontId::proportional(19.0), rgba(ink(v), a), width);
        let h = g.size().y;
        p.galley(Pos2::new(right, y), g, rgba(ink(v), a));
        y += h + 10.0;
    }
    p.text(
        Pos2::new(cx, screen.bottom() - 70.0),
        Align2::CENTER_TOP,
        HELP_LINE,
        FontId::monospace(15.0),
        rgba([190, 180, 220], 0.75),
    );
    p.text(
        Pos2::new(cx, screen.bottom() - 40.0),
        Align2::CENTER_TOP,
        v.k("[w/s] choose  [esc] back"),
        FontId::monospace(18.0),
        rgba(ink(v), 0.7),
    );
}
