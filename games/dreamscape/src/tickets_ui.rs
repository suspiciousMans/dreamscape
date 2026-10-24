//! The third tab of the Lucid Store: scratch a dream ticket. The full prize
//! table and its exact odds are always on screen; the prize is decided (and
//! saved) when the scratch is confirmed, so the animation only uncovers it.
//! Nothing here builds suspense: no near misses, no countdowns.

use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use crate::pixel_ui;
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect, Stroke, Vec2};

/// Seconds the foil takes to wear away (reduced motion: instant).
pub const SCRATCH: f32 = 0.9;
/// Cells across and down the foil.
const GRID: (u32, u32) = (10, 4);

/// What the last scratch revealed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Revealed {
    pub title: String,
    pub detail: String,
}

/// Everything the tickets screen needs for one frame.
#[derive(Clone, Debug, Default)]
pub struct TicketsView {
    pub tickets: u32,
    /// (prize label, exact chance), most likely first.
    pub odds: Vec<(String, f32)>,
    /// Scratches until the smallest prize leaves the table (0 = already gone).
    pub pity_left: u32,
    pub scratches: u32,
    pub revealed: Option<Revealed>,
    /// Seconds since the last scratch was confirmed.
    pub age: f32,
    /// The first of the two presses has been made.
    pub confirming: bool,
    pub status: Option<String>,
}

/// Is foil cell `i` of `n` gone `age` seconds after the scratch? Cells wear
/// away in a fixed shuffled order; with `instant` (reduced motion) all are.
pub fn cleared(age: f32, i: u32, n: u32, instant: bool) -> bool {
    if instant || age >= SCRATCH {
        return true;
    }
    let order = (i.wrapping_mul(7919).wrapping_add(13)) % n.max(1);
    (age / SCRATCH) * n as f32 > order as f32
}

pub fn draw(p: &egui::Painter, screen: Rect, v: &HudView) {
    let t = &v.tickets;
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    let cx = screen.center().x;
    p.rect_filled(screen, 0.0, rgba([6, 3, 16], 0.96));
    glitch_text(
        p,
        Pos2::new(cx, screen.top() + 14.0),
        true,
        "DREAM TICKETS",
        42.0,
        hue(0.55 + 0.03 * (v.time * 0.7).sin()),
        1.0,
        v,
    );
    p.text(
        Pos2::new(cx, screen.top() + 62.0),
        Align2::CENTER_TOP,
        format!(
            "{} tickets   ·   {} dream dust   ·   {} scratched so far",
            t.tickets, v.dust, t.scratches
        ),
        FontId::monospace(22.0),
        rgba([255, 210, 80], 1.0),
    );

    // The prize table, with exact odds.
    let left = screen.left() + 36.0 * u;
    let top = screen.top() + 120.0 * u;
    let width = (screen.width() * 0.42).clamp(320.0, 440.0);
    let panel = Rect::from_min_size(
        Pos2::new(left - c * 2.0, top - c * 2.0),
        Vec2::new(width, 280.0 * u),
    );
    pixel_ui::titled_panel(
        p,
        panel,
        "WHAT A TICKET CAN BE",
        [200, 170, 230],
        rgba([8, 4, 20], 0.85),
        c,
        16.0 * u.max(0.8),
    );
    let inner_l = panel.left() + c * 3.0;
    let inner_r = panel.right() - c * 3.0;
    let mut y = top + 14.0 * u;
    for (label, share) in &t.odds {
        let size = pixel_ui::fit(label, 16.0 * u, width * 0.58, 9.0);
        p.text(
            Pos2::new(inner_l, y),
            Align2::LEFT_TOP,
            label,
            FontId::monospace(size),
            rgba(ink(v), 0.9),
        );
        p.text(
            Pos2::new(inner_r, y),
            Align2::RIGHT_TOP,
            format!("{:.0}%", share * 100.0),
            FontId::monospace(16.0 * u.max(0.8)),
            rgba([255, 210, 80], 0.95),
        );
        y += 24.0 * u;
    }
    y += 8.0 * u;
    let pity = if t.pity_left == 0 {
        "THE SMALLEST PRIZE IS OFF THE TABLE FOR THE NEXT SCRATCH".to_string()
    } else {
        format!(
            "THE SMALLEST PRIZE LEAVES THE TABLE IF IT COMES {} MORE TIMES",
            t.pity_left
        )
    };
    p.text(
        Pos2::new(inner_l, y),
        Align2::LEFT_TOP,
        &pity,
        FontId::monospace(pixel_ui::fit(&pity, 14.0 * u, inner_r - inner_l, 8.0)),
        rgba(ink(v), 0.7),
    );

    // The ticket.
    let ticket = Rect::from_center_size(
        Pos2::new(screen.right() - width.min(screen.width() * 0.5) * 0.55 - 20.0 * u, top + 100.0 * u),
        Vec2::new((screen.width() * 0.36).clamp(240.0, 380.0), 150.0 * u),
    );
    pixel_ui::panel(p, ticket, hue(v.time * 0.1), rgba([20, 12, 36], 1.0), c, 0.9);
    let body = ticket.shrink(c * 3.0);
    match &t.revealed {
        Some(r) => {
            let instant = v.settings_snapshot.motion() < 1.0;
            p.text(
                body.center() - Vec2::new(0.0, 14.0 * u),
                Align2::CENTER_CENTER,
                &r.title,
                FontId::monospace(pixel_ui::fit(&r.title, 24.0 * u, body.width(), 10.0)),
                rgba(hue(v.time * 0.3), 1.0),
            );
            p.text(
                body.center() + Vec2::new(0.0, 16.0 * u),
                Align2::CENTER_CENTER,
                &r.detail,
                FontId::monospace(pixel_ui::fit(&r.detail, 16.0 * u, body.width(), 9.0)),
                rgba(ink(v), 0.85),
            );
            let (gx, gy) = GRID;
            let cw = body.width() / gx as f32;
            let ch = body.height() / gy as f32;
            for j in 0..gy {
                for i in 0..gx {
                    if cleared(t.age, j * gx + i, gx * gy, instant) {
                        continue;
                    }
                    let cell = Rect::from_min_size(
                        body.min + Vec2::new(i as f32 * cw, j as f32 * ch),
                        Vec2::new(cw, ch),
                    );
                    p.rect_filled(cell, 0.0, rgba([150, 150, 165], 1.0));
                    p.rect_stroke(cell, 0.0, Stroke::new(1.0_f32, rgba([90, 90, 105], 1.0)));
                }
            }
        }
        None => {
            let msg = if t.tickets > 0 { "a ticket, unscratched" } else { "no tickets yet" };
            p.text(
                body.center(),
                Align2::CENTER_CENTER,
                msg,
                FontId::monospace(pixel_ui::fit(msg, 20.0 * u, body.width(), 10.0)),
                rgba(ink(v), 0.55),
            );
        }
    }
    p.text(
        Pos2::new(ticket.center().x, ticket.bottom() + 14.0 * u),
        Align2::CENTER_TOP,
        "earned: a clean dream, a new achievement, a weekly challenge",
        FontId::monospace(pixel_ui::fit(
            "earned: a clean dream, a new achievement, a weekly challenge",
            14.0 * u,
            ticket.width() + 40.0 * u,
            8.0,
        )),
        rgba(ink(v), 0.55),
    );

    if let Some(s) = &t.status {
        p.text(
            Pos2::new(cx, screen.bottom() - 84.0 * u),
            Align2::CENTER_BOTTOM,
            s,
            FontId::monospace(pixel_ui::fit(s, 22.0 * u, screen.width() - 40.0, 10.0)),
            rgba([255, 210, 80], 0.95),
        );
    }
    let hint = if t.confirming {
        "[enter] yes, scratch it   [any other key] not now   [tab] the store"
    } else {
        "[enter] scratch a ticket   [tab] the store   [esc] leave"
    };
    crate::loadout_ui::hint_bar(p, screen, v, hint);
    pixel_ui::scanlines(p, screen, c, 8);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_foil_wears_away_and_reduced_motion_is_instant() {
        let n = GRID.0 * GRID.1;
        assert!((0..n).all(|i| !cleared(0.0, i, n, false)));
        assert!((0..n).all(|i| cleared(SCRATCH, i, n, false)));
        assert!((0..n).all(|i| cleared(0.0, i, n, true)));
        let half = (0..n).filter(|&i| cleared(SCRATCH * 0.5, i, n, false)).count() as u32;
        assert!(half > 0 && half < n, "{half}");
    }
}
