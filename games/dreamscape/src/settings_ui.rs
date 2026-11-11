//! The settings screen: a list of rows, the selected one glowing.

use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use crate::pixel_ui;
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

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
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    let cx = screen.center().x;
    let top = screen.top() + screen.height() * 0.06;
    glitch_text(
        p,
        Pos2::new(cx, top),
        true,
        "SETTINGS",
        48.0 * u,
        ink(v),
        1.0,
        v,
    );
    let row_h = ((screen.height() * 0.7) / s.rows.len().max(1) as f32).min(34.0 * u.max(0.8));
    let width = (screen.width() * 0.8).min(700.0);
    let y0 = top + 70.0 * u;
    let panel = Rect::from_min_size(
        Pos2::new(cx - width * 0.5, y0 - c * 2.0),
        Vec2::new(width, row_h * s.rows.len() as f32 + c * 4.0),
    );
    pixel_ui::panel(p, panel, [120, 100, 180], rgba([8, 4, 20], 0.85), c, 0.7);
    let font = (row_h * 0.72).clamp((row_h * 0.85).min(12.0), 24.0);
    // === m4a === rows ease in, the focused one springs up
    let set = &v.settings_snapshot;
    let mo = crate::screen_anim::list_motion(p, "m4a_settings", v.time, Some(s.selected), set);
    // === end m4a ===
    for (i, (label, value)) in s.rows.iter().enumerate() {
        let sel = i == s.selected;
        let e = crate::screen_anim::enter(i, mo.age, mo.amount); // === m4a ===
        let y = y0 + i as f32 * row_h + (1.0 - e) * 5.0 * u;
        let font = font * mo.scale_of(i as u32);
        let col = if sel { hue(v.time * 0.2) } else { ink(v) };
        if sel {
            pixel_ui::selected_row(
                p,
                Rect::from_min_size(
                    Pos2::new(panel.left() + c * 2.0, y - 1.0),
                    Vec2::new(panel.width() - c * 4.0, row_h),
                ),
                col,
                c,
            );
        }
        p.text(
            Pos2::new(panel.left() + c * 9.0, y + (row_h - font) * 0.35),
            Align2::LEFT_TOP,
            label,
            FontId::monospace(font),
            rgba(col, (if sel { 1.0 } else { 0.75 }) * e),
        );
        let shown = if sel && s.capturing {
            "press a key…".to_string()
        } else if sel {
            format!("< {value} >")
        } else {
            value.clone()
        };
        p.text(
            Pos2::new(panel.right() - c * 4.0, y + (row_h - font) * 0.35),
            Align2::RIGHT_TOP,
            shown,
            FontId::monospace(font),
            rgba(col, 0.95 * e),
        );
    }
    // === m4a === the chip on the focused row (rebinding says its own thing)
    if !s.capturing && !s.rows.is_empty() {
        let key = crate::prompts::label(crate::prompts::ActionId::Confirm, v.mode, set, v.pad);
        let y = y0 + s.selected as f32 * row_h + row_h * 0.5;
        if let Some(at) = crate::screen_anim::chip_beside(panel.right(), y, &key, screen.right()) {
            crate::hud::prompt_chip(p, at, &key, "", crate::prompts::State::Available, v.time, set);
        }
    }
    // === end m4a ===
    crate::loadout_ui::hint_bar(
        p,
        screen,
        v,
        "[w/s] choose   [a/d] change   [enter] rebind / reset   [esc] back",
    );
    pixel_ui::scanlines(p, screen, c, 8);
}
