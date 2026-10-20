//! The codex: every dream type and strange thing you've met, and the best
//! depth you've reached in each dream. Unmet entries stay "???".

use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use crate::pixel_ui;
use engine::ui::egui::{self, Align2, Color32, FontId, Pos2, Rect, Vec2};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CodexView {
    /// (dream name, best depth) — `None` = never visited.
    pub dreams: Vec<(String, Option<u32>)>,
    /// (enemy name, what it does) — `None` = never met.
    pub enemies: Vec<(String, Option<String>)>,
    pub footer: String,
}

/// "MyceliumGrove" → "MYCELIUM GROVE".
pub fn spaced(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.extend(c.to_uppercase());
    }
    out
}

pub fn draw(p: &egui::Painter, screen: Rect, v: &HudView, c: &CodexView) {
    p.rect_filled(screen, 0.0, rgba([6, 3, 16], 0.96));
    let u = pixel_ui::scale(screen);
    let cell = pixel_ui::cell(u);
    let cx = screen.center().x;
    let top = screen.top() + screen.height() * 0.05;
    glitch_text(
        p,
        Pos2::new(cx, top),
        true,
        "CODEX",
        48.0 * u,
        ink(v),
        1.0,
        v,
    );
    let margin = screen.width() * 0.06;
    let gap = 24.0 * u;
    let col_w = (screen.width() - 2.0 * margin - gap) * 0.5;
    let area_top = top + 84.0 * u;
    let area_bottom = screen.bottom() - 120.0 * u;
    let left_panel = Rect::from_min_max(
        Pos2::new(screen.left() + margin, area_top),
        Pos2::new(screen.left() + margin + col_w, area_bottom),
    );
    let right_panel = Rect::from_min_max(
        Pos2::new(left_panel.right() + gap, area_top),
        Pos2::new(screen.right() - margin, area_bottom),
    );
    pixel_ui::titled_panel(
        p,
        left_panel,
        "DREAMS",
        [255, 200, 120],
        rgba([8, 4, 20], 0.85),
        cell,
        18.0 * u.max(0.8),
    );
    pixel_ui::titled_panel(
        p,
        right_panel,
        "STRANGE THINGS",
        [255, 120, 190],
        rgba([8, 4, 20], 0.85),
        cell,
        18.0 * u.max(0.8),
    );

    // Dreams: two columns inside the left panel, "???" rows hatched.
    let cols = 2;
    let per_col = c.dreams.len().div_ceil(cols).max(1);
    let row = ((left_panel.height() - cell * 6.0) / per_col as f32).min(30.0 * u.max(0.8));
    let cw = (left_panel.width() - cell * 6.0) / cols as f32;
    let longest = c
        .dreams
        .iter()
        .map(|d| d.0.chars().count())
        .max()
        .unwrap_or(10) as f32;
    let font = (row * 0.62)
        .clamp(10.0, 19.0)
        .min((cw - cell * 2.0) / ((longest + 4.0) * 0.6));
    for (i, (name, best)) in c.dreams.iter().enumerate() {
        let (col, r) = (i / per_col, i % per_col);
        let at = Pos2::new(
            left_panel.left() + cell * 3.0 + col as f32 * cw,
            left_panel.top() + cell * 4.0 + r as f32 * row,
        );
        let cell_rect = Rect::from_min_size(at, Vec2::new(cw - cell * 2.0, row - 2.0));
        match best {
            Some(d) => {
                p.text(
                    at + Vec2::new(cell, (row - font) * 0.4),
                    Align2::LEFT_TOP,
                    pixel_ui_clip(name, cw - cell * 2.0 - font * 3.5, font),
                    FontId::monospace(font),
                    rgba(ink(v), 0.95),
                );
                p.text(
                    cell_rect.right_top() + Vec2::new(-cell, (row - font) * 0.4),
                    Align2::RIGHT_TOP,
                    format!("{d}"),
                    FontId::monospace(font),
                    rgba([255, 210, 80], 0.95),
                );
            }
            None => {
                pixel_ui::hatch(p, cell_rect.shrink(1.0), cell, Color32::from_white_alpha(8));
                p.text(
                    cell_rect.center(),
                    Align2::CENTER_CENTER,
                    "???",
                    FontId::monospace(font),
                    rgba([120, 110, 150], 0.7),
                );
            }
        }
    }
    // Strange things: one small panel each.
    let n = c.enemies.len().max(1);
    let slot_h = ((right_panel.height() - cell * 6.0) / n as f32).min(70.0 * u.max(0.8));
    for (i, (name, hint)) in c.enemies.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(
                right_panel.left() + cell * 3.0,
                right_panel.top() + cell * 4.0 + i as f32 * slot_h,
            ),
            Vec2::new(right_panel.width() - cell * 6.0, slot_h - cell),
        );
        match hint {
            Some(h) => {
                let tone = hue(v.time * 0.1 + i as f32 * 0.2);
                pixel_ui::panel(p, r, tone, rgba([14, 8, 28], 0.8), cell.min(3.0), 0.8);
                p.text(
                    r.left_top() + Vec2::new(cell * 3.0, cell * 2.0),
                    Align2::LEFT_TOP,
                    name,
                    FontId::monospace(18.0 * u.max(0.8)),
                    rgba(tone, 1.0),
                );
                let g = p.layout(
                    h.clone(),
                    FontId::proportional(15.0 * u.max(0.8)),
                    rgba([200, 190, 230], 0.9),
                    r.width() - cell * 6.0,
                );
                p.galley(
                    r.left_top() + Vec2::new(cell * 3.0, cell * 2.0 + 22.0 * u.max(0.8)),
                    g,
                    rgba([200, 190, 230], 0.9),
                );
            }
            None => {
                pixel_ui::panel(
                    p,
                    r,
                    [70, 62, 95],
                    rgba([10, 6, 20], 0.7),
                    cell.min(3.0),
                    0.7,
                );
                pixel_ui::hatch(p, r.shrink(cell * 2.0), cell, Color32::from_white_alpha(7));
                p.text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    "???",
                    FontId::monospace(18.0 * u.max(0.8)),
                    rgba([120, 110, 150], 0.7),
                );
            }
        }
    }
    if !c.footer.is_empty() {
        p.text(
            Pos2::new(cx, screen.bottom() - 84.0 * u),
            Align2::CENTER_BOTTOM,
            &c.footer,
            FontId::monospace(18.0 * u.max(0.8)),
            rgba([255, 210, 80], 0.9),
        );
    }
    crate::loadout_ui::hint_bar(p, screen, v, "[esc] back");
    pixel_ui::scanlines(p, screen, cell, 8);
}

/// `name` cut short (with a tilde) to fit `width` pixels at monospace `font`.
fn pixel_ui_clip(name: &str, width: f32, font: f32) -> String {
    let fit = (width / (font * 0.6)).floor().max(1.0) as usize;
    if name.chars().count() <= fit {
        name.to_string()
    } else {
        name.chars().take(fit.saturating_sub(1)).collect::<String>() + "~"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_spaced_and_uppercased() {
        assert_eq!(spaced("MyceliumGrove"), "MYCELIUM GROVE");
        assert_eq!(spaced("Lobby"), "LOBBY");
    }
}
