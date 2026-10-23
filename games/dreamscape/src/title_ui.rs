//! The title screen: where a run begins, and the way into the store and the
//! booklet between runs. A big wobbling DREAMSCAPE over a slowly breathing
//! field of pixel stars, the dreamer's eye half-open underneath.

use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use crate::pixel_ui;
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
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    let cx = screen.center().x;
    let body = "A very small percentage of people may experience seizures when exposed to certain visual images, including flashing lights or patterns that may appear in video games. Dreamscape contains flashing colours, strobing patterns and rapidly shifting visuals.\n\nIf you or anyone in your family has an epileptic condition or has had seizures, consult a doctor before playing. Stop playing immediately and consult a doctor if you experience dizziness, altered vision, eye or muscle twitching, loss of awareness, disorientation or involuntary movements.\n\nSettings: \"reduce flashing\" and \"reduced motion\" tone the effects down.";
    let width = (screen.width() * 0.8).min(900.0);
    let galley = p.layout(
        body.to_string(),
        FontId::monospace(22.0 * u),
        rgba(ink(v), 0.9),
        width - c * 12.0,
    );
    let panel = Rect::from_center_size(
        Pos2::new(cx, screen.center().y - 10.0 * u),
        Vec2::new(width, galley.size().y + c * 12.0 + 52.0 * u),
    );
    let amber = [255, 210, 80];
    pixel_ui::panel(p, panel, amber, rgba([14, 8, 6], 1.0), c, 1.0);
    // Hazard stripes along the top edge: a stepped diagonal in amber and black.
    let strip = Rect::from_min_size(
        panel.left_top() + Vec2::new(c * 2.0, c * 2.0),
        Vec2::new(panel.width() - c * 4.0, c * 2.0),
    );
    let mut x = strip.left();
    let mut k = 0;
    while x < strip.right() {
        let w = (c * 2.0).min(strip.right() - x);
        if k % 2 == 0 {
            p.rect_filled(
                Rect::from_min_size(Pos2::new(x, strip.top()), Vec2::new(w, strip.height())),
                0.0,
                rgba(amber, 0.9),
            );
        }
        x += c * 2.0;
        k += 1;
    }
    pixel_ui::split_text(
        p,
        Pos2::new(cx, panel.top() + c * 6.0),
        Align2::CENTER_TOP,
        "PHOTOSENSITIVITY WARNING",
        pixel_ui::fit("PHOTOSENSITIVITY WARNING", 40.0 * u, width - c * 8.0, 16.0),
        rgba(amber, 1.0),
        (c * 0.4).max(1.0),
        1.0,
    );
    p.galley(
        Pos2::new(panel.left() + c * 6.0, panel.top() + c * 6.0 + 52.0 * u),
        galley,
        rgba(ink(v), 0.9),
    );
    if v.warning_age >= WARNING_MIN {
        crate::loadout_ui::hint_bar(p, screen, v, "[enter] continue");
    }
}

/// Where the title menu sits, so drawing and the mouse agree.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuLayout {
    /// The header's top and the 0.5..1 shrink on short windows.
    pub top: f32,
    pub k: f32,
    pub menu_top: f32,
    pub step: f32,
    pub font: f32,
    pub width: f32,
    pub panel: Rect,
    pub caption_size: f32,
}

/// `rows` are the labels as drawn (`[key]  label`); `has_goal` is whether the
/// goal line is showing at the bottom.
pub fn menu_layout(screen: Rect, rows: &[String], has_goal: bool) -> MenuLayout {
    let cx = screen.center().x;
    // A long menu (host/join/memories...) needs the room: lift the header and keep
    // the menu clear of the goal/streak lines pinned to the bottom.
    let top = screen.top() + screen.height() * if rows.len() > 8 { 0.1 } else { 0.2 };
    // The header's fixed offsets shrink together on short windows.
    let k = (screen.height() / 720.0).clamp(0.5, 1.0);
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    let menu_top = top + 236.0 * k;
    // Keep the menu clear of the caption, the goal line, the stats strip and
    // the perks, all of which are centred on the same axis as the menu.
    let caption_size = 15.0 * u.max(0.8);
    let caption_room = caption_size + c * 4.0;
    let goal_room = if has_goal { 18.0 * u.max(0.8) + 14.0 * u } else { 0.0 };
    let bottom_room = 62.0 * u + 12.0 * u + 22.0 * u + goal_room + caption_room + 6.0;
    let n = rows.len().max(1) as f32;
    let step = ((screen.bottom() - bottom_room.max(105.0) - menu_top - c * 6.0) / n).min(36.0);
    let font = (step * 0.9).clamp(9.0, 26.0);
    // Rows are centred; the panel is as wide as its longest row needs, evenly
    // padded, and sits on the screen's centre line.
    let longest = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) as f32;
    let want = longest * font * 0.6 + c * 16.0;
    let width = want.max(340.0 * u).min(screen.width() - 20.0);
    let panel = Rect::from_min_max(
        Pos2::new(cx - width * 0.5, menu_top - c * 3.0),
        Pos2::new(cx + width * 0.5, menu_top + step * n - 2.0 + c * 3.0),
    );
    MenuLayout {
        top,
        k,
        menu_top,
        step,
        font,
        width,
        panel,
        caption_size,
    }
}

/// The row under `at`, if any (the whole width of the panel counts).
pub fn menu_hit(screen: Rect, rows: &[String], has_goal: bool, at: Pos2) -> Option<usize> {
    let l = menu_layout(screen, rows, has_goal);
    if at.x < l.panel.left() || at.x > l.panel.right() {
        return None;
    }
    (0..rows.len()).find(|&i| {
        let y = l.menu_top + i as f32 * l.step;
        at.y >= y - 2.0 && at.y < y + l.step - 2.0
    })
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
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    let rows: Vec<String> = v
        .menu
        .iter()
        .map(|(key, label, _)| v.k(&format!("[{key}]  {label}")))
        .collect();
    let lay = menu_layout(screen, &rows, !v.goal.is_empty());
    let (top, k, menu_top, step, font, width, panel, caption_size) = (
        lay.top,
        lay.k,
        lay.menu_top,
        lay.step,
        lay.font,
        lay.width,
        lay.panel,
        lay.caption_size,
    );
    // The header's fixed offsets shrink together on short windows.
    glitch_text(
        p,
        Pos2::new(cx, top),
        true,
        "DREAMSCAPE",
        96.0 * k,
        ink(v),
        1.0,
        v,
    );
    p.text(
        Pos2::new(cx, top + 108.0 * k),
        Align2::CENTER_TOP,
        "you are asleep. you know you are asleep. find a way out.",
        FontId::proportional(pixel_ui::fit(
            "you are asleep. you know you are asleep. find a way out.",
            22.0,
            screen.width() - 20.0,
            10.0,
        )),
        rgba([200, 190, 230], 0.8),
    );
    crate::hud::draw_eye_preview(p, Pos2::new(cx, top + 190.0 * k), v.eye, v);
    pixel_ui::panel(p, panel, [120, 100, 180], rgba([8, 4, 20], 0.82), c, 0.7);
    let selected = v.shop_col.min(v.menu.len().saturating_sub(1));
    for (i, ((_, _, _), row)) in v.menu.iter().zip(&rows).enumerate() {
        let sel = i == selected;
        let y = menu_top + i as f32 * step;
        let col = if sel { hue(v.time * 0.2) } else { ink(v) };
        if sel {
            pixel_ui::selected_row(
                p,
                Rect::from_min_max(
                    Pos2::new(panel.left() + c * 2.0, y - 2.0),
                    Pos2::new(panel.right() - c * 2.0, y + step - 2.0),
                ),
                col,
                c,
            );
        }
        // Never wider than the panel's inside, whatever the label says.
        let size = pixel_ui::fit(row, font, width - c * 12.0, 8.0);
        p.text(
            Pos2::new(cx, y + step * 0.5 - 2.0),
            Align2::CENTER_CENTER,
            row,
            FontId::monospace(size),
            rgba(col, if sel { 1.0 } else { 0.7 }),
        );
    }
    // What the highlighted choice does, one centred line under the panel.
    if let Some((_, _, note)) = v.menu.get(selected) {
        if !note.is_empty() {
            p.text(
                Pos2::new(cx, panel.bottom() + c * 2.0),
                Align2::CENTER_TOP,
                note,
                FontId::monospace(pixel_ui::fit(note, caption_size, screen.width() - 40.0, 8.0)),
                rgba([255, 120, 150], 0.85),
            );
        }
    }
    let stats = if v.streak > 1 {
        format!(
            "{} dust   ·   {} cards   ·   deepest {}   ·   {} day streak",
            v.dust, v.cards_owned, v.best_depth, v.streak
        )
    } else {
        format!(
            "{} dust   ·   {} cards   ·   deepest {}",
            v.dust, v.cards_owned, v.best_depth
        )
    };
    let size = pixel_ui::fit(&stats, 20.0 * u, screen.width() - 80.0, 11.0);
    let strip = Rect::from_center_size(
        Pos2::new(cx, screen.bottom() - 62.0 * u),
        Vec2::new(
            (stats.chars().count() as f32 * size * 0.6 + c * 10.0).min(screen.width() - 20.0),
            size + c * 4.0,
        ),
    );
    pixel_ui::panel(p, strip, [255, 210, 80], rgba([14, 10, 4], 0.85), c, 0.7);
    p.text(
        strip.center(),
        Align2::CENTER_CENTER,
        stats,
        FontId::monospace(size),
        rgba([255, 210, 80], 0.95),
    );
    if !v.goal.is_empty() {
        p.text(
            Pos2::new(cx, strip.top() - 10.0 * u),
            Align2::CENTER_BOTTOM,
            &v.goal,
            FontId::monospace(18.0 * u.max(0.8)),
            rgba(hue(v.time * 0.15), 0.85),
        );
    }
    if !v.perks.is_empty() {
        p.text(
            Pos2::new(cx, screen.bottom() - 12.0 * u),
            Align2::CENTER_BOTTOM,
            format!("stocked: {}", v.perks.join(", ")),
            FontId::monospace(15.0 * u.max(0.8)),
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
