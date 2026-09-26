//! The title screen: where a run begins, and the way into the store and the
//! booklet between runs. A big wobbling DREAMSCAPE over a slowly breathing
//! field of pixel stars, the dreamer's eye half-open underneath.

use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

/// What the run-length row says under each setting.
pub fn run_blurb(length: &str) -> &'static str {
    if length == "long" {
        "six shards · every dream harder than the last · richer rewards"
    } else {
        "three shards · the classic descent"
    }
}

pub fn draw(p: &egui::Painter, screen: Rect, v: &HudView) {
    p.rect_filled(screen, 0.0, rgba([5, 2, 14], 1.0));
    // Star field that breathes.
    for k in 0..140u32 {
        let h1 = crate::hud::hash01(k, 1);
        let h2 = crate::hud::hash01(k, 2);
        let pulse = 0.5 + 0.5 * (v.time * (0.5 + h1) + h2 * std::f32::consts::TAU).sin();
        let at = Pos2::new(
            screen.left() + h1 * screen.width(),
            screen.top() + h2 * screen.height(),
        );
        let size = if h1 > 0.93 { 4.0 } else { 2.0 };
        p.rect_filled(
            Rect::from_center_size(at, Vec2::splat(size)),
            0.0,
            rgba(hue(h2 + v.time * 0.02), 0.15 + 0.5 * pulse),
        );
    }
    let cx = screen.center().x;
    let top = screen.top() + screen.height() * 0.2;
    glitch_text(
        p,
        Pos2::new(cx, top),
        true,
        "DREAMSCAPE",
        96.0,
        ink(v),
        1.0,
        v,
    );
    p.text(
        Pos2::new(cx, top + 108.0),
        Align2::CENTER_TOP,
        "you are asleep. you know you are asleep. find a way out.",
        FontId::proportional(22.0),
        rgba([200, 190, 230], 0.8),
    );
    crate::hud::draw_eye_preview(p, Pos2::new(cx, top + 190.0), v.eye, v);

    let menu_top = top + 236.0;
    let step = ((screen.bottom() - 80.0 - menu_top) / v.menu.len().max(1) as f32).min(36.0);
    for (i, (key, label, note)) in v.menu.iter().enumerate() {
        let sel = i == v.shop_col.min(v.menu.len().saturating_sub(1));
        let y = menu_top + i as f32 * step;
        let col = if sel { hue(v.time * 0.2) } else { ink(v) };
        if sel {
            p.text(
                Pos2::new(cx - 230.0, y),
                Align2::LEFT_TOP,
                ">",
                FontId::monospace(26.0),
                rgba(col, 1.0),
            );
        }
        p.text(
            Pos2::new(cx - 200.0, y),
            Align2::LEFT_TOP,
            v.k(&format!("[{key}]  {label}")),
            FontId::monospace(26.0),
            rgba(col, if sel { 1.0 } else { 0.7 }),
        );
        if sel && !note.is_empty() {
            p.text(
                Pos2::new(cx + 170.0, y + 7.0),
                Align2::LEFT_TOP,
                note,
                FontId::monospace(15.0),
                rgba([255, 120, 150], 0.85),
            );
        }
    }
    p.text(
        Pos2::new(cx, screen.bottom() - 60.0),
        Align2::CENTER_TOP,
        format!(
            "{} dust   ·   {} cards   ·   deepest {}",
            v.dust, v.cards_owned, v.best_depth
        ),
        FontId::monospace(20.0),
        rgba([255, 210, 80], 0.85),
    );
    if !v.perks.is_empty() {
        p.text(
            Pos2::new(cx, screen.bottom() - 32.0),
            Align2::CENTER_TOP,
            format!("stocked: {}", v.perks.join(", ")),
            FontId::monospace(16.0),
            rgba([80, 230, 200], 0.8),
        );
    }
}
