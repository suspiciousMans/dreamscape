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

/// Seconds before the warning can be dismissed.
pub const WARNING_MIN: f32 = 2.0;

pub fn draw_warning(p: &egui::Painter, screen: Rect, v: &HudView) {
    p.rect_filled(screen, 0.0, rgba([0, 0, 0], 1.0));
    let cx = screen.center().x;
    p.text(
        Pos2::new(cx, screen.top() + screen.height() * 0.16),
        Align2::CENTER_TOP,
        "PHOTOSENSITIVITY WARNING",
        FontId::monospace(40.0),
        rgba([255, 210, 80], 1.0),
    );
    let body = "A very small percentage of people may experience seizures when exposed to certain visual images, including flashing lights or patterns that may appear in video games. Dreamscape contains flashing colours, strobing patterns and rapidly shifting visuals.\n\nIf you or anyone in your family has an epileptic condition or has had seizures, consult a doctor before playing. Stop playing immediately and consult a doctor if you experience dizziness, altered vision, eye or muscle twitching, loss of awareness, disorientation or involuntary movements.\n\nSettings: \"reduce flashing\" and \"reduced motion\" tone the effects down.";
    let galley = p.layout(
        body.to_string(),
        FontId::monospace(22.0),
        rgba(ink(v), 0.9),
        screen.width().min(900.0),
    );
    let at = Pos2::new(
        cx - galley.size().x * 0.5,
        screen.top() + screen.height() * 0.28,
    );
    p.galley(at, galley, rgba(ink(v), 0.9));
    if v.warning_age >= WARNING_MIN {
        p.text(
            Pos2::new(cx, screen.bottom() - 60.0),
            Align2::CENTER_TOP,
            v.k("[enter] continue"),
            FontId::monospace(24.0),
            rgba(ink(v), 0.6 + 0.3 * (v.time * 2.0).sin()),
        );
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
    // A long menu (host/join/memories...) needs the room: lift the header and keep
    // the menu clear of the goal/streak lines pinned to the bottom.
    let top = screen.top() + screen.height() * if v.menu.len() > 8 { 0.1 } else { 0.2 };
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
    let step = ((screen.bottom() - 105.0 - menu_top) / v.menu.len().max(1) as f32).min(36.0);
    let font = (step * 0.9).clamp(16.0, 26.0);
    for (i, (key, label, note)) in v.menu.iter().enumerate() {
        let sel = i == v.shop_col.min(v.menu.len().saturating_sub(1));
        let y = menu_top + i as f32 * step;
        let col = if sel { hue(v.time * 0.2) } else { ink(v) };
        if sel {
            p.text(
                Pos2::new(cx - 230.0, y),
                Align2::LEFT_TOP,
                ">",
                FontId::monospace(font),
                rgba(col, 1.0),
            );
        }
        p.text(
            Pos2::new(cx - 200.0, y),
            Align2::LEFT_TOP,
            v.k(&format!("[{key}]  {label}")),
            FontId::monospace(font),
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
        if v.streak > 1 {
            format!(
                "{} dust   ·   {} cards   ·   deepest {}   ·   {} day streak",
                v.dust, v.cards_owned, v.best_depth, v.streak
            )
        } else {
            format!(
                "{} dust   ·   {} cards   ·   deepest {}",
                v.dust, v.cards_owned, v.best_depth
            )
        },
        FontId::monospace(20.0),
        rgba([255, 210, 80], 0.85),
    );
    if !v.goal.is_empty() {
        p.text(
            Pos2::new(cx, screen.bottom() - 88.0),
            Align2::CENTER_TOP,
            &v.goal,
            FontId::monospace(18.0),
            rgba(hue(v.time * 0.15), 0.85),
        );
    }
    if !v.perks.is_empty() {
        p.text(
            Pos2::new(cx, screen.bottom() - 32.0),
            Align2::CENTER_TOP,
            format!("stocked: {}", v.perks.join(", ")),
            FontId::monospace(16.0),
            rgba([80, 230, 200], 0.8),
        );
    }
    p.text(
        screen.right_bottom() + Vec2::new(-16.0, -12.0),
        Align2::RIGHT_BOTTOM,
        format!("v{} {}", crate::crash::VERSION, crate::crash::GIT),
        FontId::monospace(16.0),
        rgba(ink(v), 0.35),
    );
}
