//! The settings screen: a list of rows, the selected one glowing.

use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SettingsView {
    /// (label, value) per row.
    pub rows: Vec<(String, String)>,
    pub selected: usize,
    /// Waiting for a key to bind.
    pub capturing: bool,
}

pub fn draw(p: &egui::Painter, screen: Rect, v: &HudView, s: &SettingsView) {
    p.rect_filled(screen, 0.0, rgba([6, 3, 16], 0.94));
    let cx = screen.center().x;
    let top = screen.top() + screen.height() * 0.08;
    glitch_text(
        p,
        Pos2::new(cx, top),
        true,
        "SETTINGS",
        48.0,
        ink(v),
        1.0,
        v,
    );
    let row_h = ((screen.height() * 0.7) / s.rows.len().max(1) as f32).min(32.0);
    let y0 = top + 70.0;
    for (i, (label, value)) in s.rows.iter().enumerate() {
        let sel = i == s.selected;
        let y = y0 + i as f32 * row_h;
        let col = if sel { hue(v.time * 0.2) } else { ink(v) };
        if sel {
            p.text(
                Pos2::new(cx - 300.0, y),
                Align2::LEFT_TOP,
                ">",
                FontId::monospace(24.0),
                rgba(col, 1.0),
            );
        }
        p.text(
            Pos2::new(cx - 270.0, y),
            Align2::LEFT_TOP,
            label,
            FontId::monospace(24.0),
            rgba(col, if sel { 1.0 } else { 0.75 }),
        );
        let shown = if sel && s.capturing {
            "press a key…".to_string()
        } else {
            value.clone()
        };
        p.text(
            Pos2::new(cx + 270.0, y),
            Align2::RIGHT_TOP,
            shown,
            FontId::monospace(24.0),
            rgba(col, 0.95),
        );
    }
    p.text(
        Pos2::new(cx, screen.bottom() - 44.0),
        Align2::CENTER_TOP,
        "[w/s] choose   [a/d] change   [enter] rebind / reset   [esc] back",
        FontId::monospace(18.0),
        rgba(ink(v), 0.7),
    );
}
