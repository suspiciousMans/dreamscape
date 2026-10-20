//! The notes you've found: one page when you pick a note up, and the
//! memories screen (every dreamer, their notes in order).

use crate::hud::{glitch_text, ink, rgba, HudView};
use crate::pixel_ui;
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

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
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    let size = Vec2::new(640.0_f32.min(screen.width() * 0.9), 360.0 * u.max(0.8));
    let r = Rect::from_center_size(screen.center(), size);
    // The page: paper, a hard shadow, chipped corners, a pixel pin at the top.
    p.rect_filled(r.translate(Vec2::splat(c)), 0.0, rgba([2, 1, 6], 0.7));
    pixel_ui::panel(p, r, PAPER_INK, rgba(PAPER, 0.98), c, 0.85);
    let pin = Rect::from_center_size(
        Pos2::new(r.center().x, r.top() + c * 1.5),
        Vec2::splat(c * 2.0),
    );
    p.rect_filled(pin, 0.0, rgba([200, 70, 90], 1.0));
    p.rect_filled(
        Rect::from_min_size(pin.min, Vec2::splat(c * 0.7)),
        0.0,
        rgba([255, 190, 200], 1.0),
    );
    // Faint ruled lines, like a page torn from a pad.
    let mut y = r.top() + 96.0 * u;
    while y < r.bottom() - 56.0 * u {
        p.line_segment(
            [
                Pos2::new(r.left() + c * 3.0, y + 30.0 * u),
                Pos2::new(r.right() - c * 3.0, y + 30.0 * u),
            ],
            egui::Stroke::new(1.0_f32, rgba(PAPER_INK, 0.08)),
        );
        y += 34.0 * u;
    }
    p.text(
        r.left_top() + Vec2::new(28.0, 22.0 * u),
        Align2::LEFT_TOP,
        object.to_uppercase(),
        FontId::monospace(16.0 * u.max(0.8)),
        rgba(PAPER_INK, 0.6),
    );
    p.text(
        r.left_top() + Vec2::new(28.0, 46.0 * u),
        Align2::LEFT_TOP,
        title,
        FontId::monospace(22.0 * u.max(0.8)),
        rgba(PAPER_INK, 0.95),
    );
    let g = p.layout(
        body.clone(),
        FontId::proportional(26.0 * u.max(0.8)),
        rgba(PAPER_INK, 1.0),
        size.x - 56.0,
    );
    p.galley(
        r.left_top() + Vec2::new(28.0, 96.0 * u),
        g,
        rgba(PAPER_INK, 1.0),
    );
    p.text(
        Pos2::new(r.center().x, r.bottom() - 30.0 * u),
        Align2::CENTER_CENTER,
        v.k("[enter] keep it"),
        FontId::monospace(18.0 * u.max(0.8)),
        rgba(PAPER_INK, 0.7),
    );
}

pub fn draw(p: &egui::Painter, screen: Rect, v: &HudView, m: &MemoriesView) {
    p.rect_filled(screen, 0.0, rgba([6, 3, 16], 0.96));
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    let cx = screen.center().x;
    let top = screen.top() + screen.height() * 0.05;
    glitch_text(
        p,
        Pos2::new(cx, top),
        true,
        "MEMORIES",
        48.0 * u,
        ink(v),
        1.0,
        v,
    );
    let margin = screen.width() * 0.06;
    let gap = 24.0 * u;
    let area_top = top + 84.0 * u;
    let area_bottom = screen.bottom() - 118.0 * u;
    let left_w = (screen.width() * 0.34).min(420.0);
    let left_panel = Rect::from_min_max(
        Pos2::new(screen.left() + margin, area_top),
        Pos2::new(screen.left() + margin + left_w, area_bottom),
    );
    let right_panel = Rect::from_min_max(
        Pos2::new(left_panel.right() + gap, area_top),
        Pos2::new(screen.right() - margin, area_bottom),
    );
    pixel_ui::titled_panel(
        p,
        left_panel,
        "DREAMERS",
        [200, 170, 230],
        rgba([8, 4, 20], 0.85),
        c,
        16.0 * u.max(0.8),
    );
    pixel_ui::titled_panel(
        p,
        right_panel,
        "THEIR NOTES",
        [255, 220, 140],
        rgba([8, 4, 20], 0.85),
        c,
        16.0 * u.max(0.8),
    );
    let row_h =
        ((left_panel.height() - c * 6.0) / m.rows.len().max(1) as f32).min(48.0 * u.max(0.8));
    for (i, row) in m.rows.iter().enumerate() {
        let y = left_panel.top() + c * 4.0 + i as f32 * row_h;
        let sel = i == m.selected;
        let tone = if sel { [255, 220, 140] } else { ink(v) };
        if sel {
            pixel_ui::selected_row(
                p,
                Rect::from_min_size(
                    Pos2::new(left_panel.left() + c * 2.0, y),
                    Vec2::new(left_panel.width() - c * 4.0, row_h - 2.0),
                ),
                tone,
                c,
            );
        }
        let a = if row.found > 0 { 0.95 } else { 0.5 };
        let font = (row_h * 0.42).clamp(11.0, 21.0);
        p.text(
            Pos2::new(left_panel.left() + c * 9.0, y + 3.0 * u),
            Align2::LEFT_TOP,
            &row.name,
            FontId::monospace(font),
            rgba(tone, a),
        );
        // Found notes as a row of pixel pips.
        let pip = (c * 0.8).max(3.0);
        for k in 0..row.total {
            let r = Rect::from_min_size(
                Pos2::new(
                    left_panel.right() - c * 4.0 - (row.total - k) as f32 * (pip + 2.0),
                    y + 5.0 * u,
                ),
                egui::Vec2::splat(pip),
            );
            if k < row.found {
                p.rect_filled(r, 0.0, rgba(tone, a));
            } else {
                p.rect_stroke(r, 0.0, egui::Stroke::new(1.0_f32, rgba(tone, 0.3)));
            }
        }
        if let Some(w) = &row.weight {
            p.text(
                Pos2::new(left_panel.left() + c * 9.0, y + font + 5.0 * u),
                Align2::LEFT_TOP,
                w,
                FontId::monospace((font * 0.62).max(9.0)),
                rgba([200, 170, 230], 0.8),
            );
        }
    }
    let mut y = right_panel.top() + c * 4.0;
    let width = right_panel.width() - c * 8.0;
    for (k, n) in m.notes.iter().enumerate() {
        let (text, a) = match n {
            Some(t) => (format!("{}. {t}", k + 1), 0.92),
            None => (format!("{}. …", k + 1), 0.4),
        };
        let g = p.layout(
            text,
            FontId::proportional(19.0 * u.max(0.8)),
            rgba(ink(v), a),
            width,
        );
        let h = g.size().y;
        p.galley(
            Pos2::new(right_panel.left() + c * 4.0, y),
            g,
            rgba(ink(v), a),
        );
        y += h + 10.0 * u;
    }
    p.text(
        Pos2::new(cx, screen.bottom() - 72.0 * u),
        Align2::CENTER_BOTTOM,
        HELP_LINE,
        FontId::monospace(15.0 * u.max(0.8)),
        rgba([190, 180, 220], 0.75),
    );
    crate::loadout_ui::hint_bar(p, screen, v, "[w/s] choose  [esc] back");
    pixel_ui::scanlines(p, screen, c, 8);
}
