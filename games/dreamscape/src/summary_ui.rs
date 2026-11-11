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

// === m4e ===
/// `typed`, but with Off / reduced motion (`amount` 0) every row is already there.
pub fn typed_for(row: usize, age: f32, len: usize, amount: f32) -> usize {
    if amount <= 0.0 {
        len
    } else {
        typed(row, age, len)
    }
}

/// When a typed row finishes, in seconds since the screen opened.
pub fn typed_done(row: usize, len: usize) -> f32 {
    row as f32 * ROW_DELAY + len as f32 / 60.0
}

/// A row value whose leading whole number counts up from 0 once the row has
/// finished typing (`delay` = when that is): "12 shards" -> "7 shards" -> "12
/// shards". Values that do not start with a plain integer ("x1.5", "3.2",
/// "1,204") and everything at `amount` 0 come back unchanged.
pub fn ticked(value: &str, age: f32, delay: f32, amount: f32) -> String {
    let digits = value.chars().take_while(|c| c.is_ascii_digit()).count();
    let rest = &value[digits..];
    if digits == 0 || digits > 9 || rest.starts_with(|c: char| c == '.' || c == ',' || c.is_ascii_digit()) {
        return value.to_string();
    }
    let to: i64 = value[..digits].parse().unwrap_or(0);
    format!("{}{rest}", crate::screen_anim::tick(to, age, delay, amount))
}

/// How far (px at unit scale) a panel still has to rise at entrance progress `e`.
pub fn rise(e: f32) -> f32 {
    (1.0 - e.clamp(0.0, 1.0)) * 16.0
}
// === end m4e ===

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
    // === m4e === eased entrance, rows appear whole at Off / reduced motion
    let amount = crate::ui_motion::effective(v.settings_snapshot.ui_juice, v.settings_snapshot.reduced_motion);
    let e_panel = crate::screen_anim::enter(0, age, amount);
    // === end m4e ===
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
    let panel_top = top + 70.0 * u + rise(e_panel) * u;
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
        let n = typed_for(row, age, text_len, amount);
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
            // === m4e === the number settles once the row has typed out
            if n >= text_len {
                ticked(value, age, typed_done(row, text_len), amount)
            } else {
                clip(value, n.saturating_sub(label.len()))
            },
            FontId::monospace(22.0 * u),
            rgba(ink(v), e_panel),
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
        let n = typed_for(row, age, line.len(), amount);
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
    let n = typed_for(row, age, s.dust.len(), amount);
    let e_dust = crate::screen_anim::enter(row.min(10) + 1, age, amount); // === m4e ===
    let dust_size = pixel_ui::fit(&s.dust, 22.0 * u, width - c * 8.0, 11.0);
    let ribbon = Rect::from_min_size(
        Pos2::new(cx - width * 0.5, panel.bottom() + 10.0 * u + rise(e_dust) * u),
        Vec2::new(width, dust_size + c * 5.0),
    );
    pixel_ui::panel(p, ribbon, [255, 210, 80], rgba([20, 14, 4], 0.9), c, 0.9);
    p.text(
        ribbon.left_center() + Vec2::new(c * 4.0, 0.0),
        Align2::LEFT_CENTER,
        clip(&s.dust, n),
        FontId::monospace(dust_size),
        rgba([255, 210, 80], e_dust),
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

    #[test]
    fn m4e_numbers_tick_up_then_settle_and_off_is_final() {
        // Counts from 0 to the value, monotone, bounded, ending exactly on it.
        let mut last = -1;
        for k in 0..=120 {
            let age = 1.0 + k as f32 * 0.02;
            let s = ticked("12 shards", age, 1.0, 1.0);
            let n: i64 = s.split(' ').next().unwrap().parse().unwrap();
            assert!((0..=12).contains(&n) && n >= last);
            assert!(s.ends_with(" shards"));
            last = n;
        }
        assert_eq!(last, 12);
        assert_eq!(ticked("12 shards", 0.0, 1.0, 0.0), "12 shards");
        // Non-integers are left alone.
        for v in ["x1.5", "3.2s", "1,204", "none", ""] {
            assert_eq!(ticked(v, 1.0, 1.0, 1.0), v);
        }
        assert_eq!(ticked("7", 100.0, 0.0, 1.4), "7");
    }

    #[test]
    fn m4e_off_shows_every_row_and_entrance_is_bounded() {
        assert_eq!(typed_for(4, 0.0, 30, 0.0), 30);
        assert_eq!(typed_for(4, 0.0, 30, 1.0), 0);
        assert_eq!(typed_for(0, 10.0, 30, 1.0), 30);
        assert!(typed_done(2, 60) > typed_done(1, 60));
        assert_eq!(rise(1.0), 0.0);
        assert!(rise(0.0) > 0.0 && rise(-3.0) == rise(0.0) && rise(9.0) == 0.0);
        assert_eq!(crate::screen_anim::enter(0, 0.0, 0.0), 1.0);
    }
}
