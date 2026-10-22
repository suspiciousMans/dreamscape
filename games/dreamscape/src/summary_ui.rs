//! The run summary, shown on waking before the pack opens: how deep you
//! went, what you picked up, what the dream threw at you, and how your
//! dust will be worked out. Lines type themselves out one after another.

use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use crate::pixel_ui;
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

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
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    let cx = screen.center().x;
    let top = screen.top() + screen.height() * 0.06;
    glitch_text(
        p,
        Pos2::new(cx, top),
        true,
        "YOU WAKE UP",
        52.0 * u,
        ink(v),
        1.0,
        v,
    );
    let age = v.title_age;
    let width = (screen.width() * 0.8).min(760.0 * u.max(0.8));
    let left = cx - width * 0.5 + c * 4.0;
    let inner_w = width - c * 8.0;
    let row_h = 28.0 * u;
    // Lay the lists out in full first, so the panel never resizes as they type.
    let lists: Vec<(&str, &[String], [u8; 3])> = vec![
        ("upgrades", &s.upgrades[..], [255, 200, 120]),
        ("combos", &s.synergies[..], [140, 255, 220]),
    ];
    let mut list_h = 0.0;
    for (title, items, _) in &lists {
        if items.is_empty() {
            continue;
        }
        let g = p.layout(
            format!("{title}: {}", items.join(", ")),
            FontId::monospace(19.0 * u),
            egui::Color32::WHITE,
            inner_w,
        );
        list_h += g.size().y + 8.0 * u;
    }
    let rows_h = s.rows.len() as f32 * row_h;
    let panel_top = top + 70.0 * u;
    let panel = Rect::from_min_size(
        Pos2::new(cx - width * 0.5, panel_top),
        Vec2::new(width, rows_h + list_h + c * 8.0 + 14.0 * u),
    );
    pixel_ui::panel(p, panel, [120, 100, 180], rgba([10, 5, 24], 0.9), c, 0.8);
    let mut y = panel_top + c * 4.0;
    let mut row = 0;
    for (k, (label, value)) in s.rows.iter().enumerate() {
        if k % 2 == 0 {
            p.rect_filled(
                Rect::from_min_size(
                    Pos2::new(panel.left() + c * 2.0, y - 2.0 * u),
                    Vec2::new(panel.width() - c * 4.0, row_h),
                ),
                0.0,
                rgba([255, 255, 255], 0.035),
            );
        }
        let text_len = label.len() + value.len();
        let n = typed(row, age, text_len);
        p.text(
            Pos2::new(left, y),
            Align2::LEFT_TOP,
            clip(label, n),
            FontId::monospace(22.0 * u),
            rgba([180, 170, 210], 0.9),
        );
        p.text(
            Pos2::new(left + inner_w * 0.45, y),
            Align2::LEFT_TOP,
            clip(value, n.saturating_sub(label.len())),
            FontId::monospace(22.0 * u),
            rgba(ink(v), 1.0),
        );
        y += row_h;
        row += 1;
    }
    y += 8.0 * u;
    for (title, items, rgb) in &lists {
        if items.is_empty() {
            continue;
        }
        let line = format!("{title}: {}", items.join(", "));
        let n = typed(row, age, line.len());
        let galley = p.layout(
            clip(&line, n),
            FontId::monospace(19.0 * u),
            rgba(*rgb, 0.95),
            inner_w,
        );
        let h = galley.size().y;
        p.galley(Pos2::new(left, y), galley, rgba(*rgb, 0.95));
        y += h + 8.0 * u;
        row += 1;
    }
    // The dust total: a gold ribbon under the panel.
    let n = typed(row, age, s.dust.len());
    let dust_size = pixel_ui::fit(&s.dust, 22.0 * u, width - c * 8.0, 11.0);
    let ribbon = Rect::from_min_size(
        Pos2::new(cx - width * 0.5, panel.bottom() + 10.0 * u),
        Vec2::new(width, dust_size + c * 5.0),
    );
    pixel_ui::panel(p, ribbon, [255, 210, 80], rgba([20, 14, 4], 0.9), c, 0.9);
    p.text(
        ribbon.left_center() + Vec2::new(c * 4.0, 0.0),
        Align2::LEFT_CENTER,
        clip(&s.dust, n),
        FontId::monospace(dust_size),
        rgba([255, 210, 80], 1.0),
    );
    if n == s.dust.len() {
        if !v.goal.is_empty() {
            p.text(
                Pos2::new(cx, screen.bottom() - 82.0 * u),
                Align2::CENTER_BOTTOM,
                &v.goal,
                FontId::monospace(20.0 * u.max(0.8)),
                rgba(hue(v.time * 0.15), 0.9),
            );
        }
        if !v.run_code.is_empty() {
            let line = if v.code_copied {
                format!("{}   copied", v.run_code)
            } else {
                format!("{}   [c] copy to share", v.run_code)
            };
            p.text(
                Pos2::new(cx, screen.bottom() - 124.0),
                Align2::CENTER_TOP,
                v.k(&line),
                FontId::monospace(20.0),
                rgba([140, 255, 220], 0.9),
            );
        }
        crate::loadout_ui::hint_bar(p, screen, v, "[enter] see what you remember");
    }
    pixel_ui::scanlines(p, screen, c, 8);
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
