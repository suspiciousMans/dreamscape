//! Menu backdrops the player picks in Settings ("menu background").
//! `Stars` is the original look and draws nothing extra outside the title.
//! Pure drawing: only the time and the reduced-motion flag are read.

use crate::hud::{hash01, hue, rgba, HudView};
use engine::ui::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuBackdrop {
    #[default]
    Stars,
    Cards,
    Garden,
    MirrorHall,
    Clockworks,
}

const ORDER: [MenuBackdrop; 5] = [
    MenuBackdrop::Stars,
    MenuBackdrop::Cards,
    MenuBackdrop::Garden,
    MenuBackdrop::MirrorHall,
    MenuBackdrop::Clockworks,
];

impl MenuBackdrop {
    pub fn label(self) -> &'static str {
        match self {
            MenuBackdrop::Stars => "stars",
            MenuBackdrop::Cards => "drifting cards",
            MenuBackdrop::Garden => "garden",
            MenuBackdrop::MirrorHall => "mirror hall",
            MenuBackdrop::Clockworks => "clockworks",
        }
    }

    /// Left/right on the settings row; wraps around.
    pub fn step(self, dir: i32) -> Self {
        let i = ORDER.iter().position(|&b| b == self).unwrap_or(0) as i32;
        ORDER[(i + dir.signum()).rem_euclid(ORDER.len() as i32) as usize]
    }

    fn base(self) -> [u8; 3] {
        match self {
            MenuBackdrop::Stars => [5, 2, 14],
            MenuBackdrop::Cards => [10, 4, 22],
            MenuBackdrop::Garden => [4, 14, 10],
            MenuBackdrop::MirrorHall => [12, 12, 20],
            MenuBackdrop::Clockworks => [18, 10, 4],
        }
    }
}

/// Paints the chosen design over a menu's own dim fill. `Stars` is a no-op.
pub fn draw(p: &egui::Painter, screen: Rect, v: &HudView) {
    let kind = v.settings_snapshot.menu_backdrop;
    if kind == MenuBackdrop::Stars {
        return;
    }
    p.rect_filled(screen, 0.0, rgba(kind.base(), 0.9));
    paint(p, screen, v, kind);
}

/// The title's full backdrop (fill included), any design.
pub fn draw_title(p: &egui::Painter, screen: Rect, v: &HudView) {
    let kind = v.settings_snapshot.menu_backdrop;
    p.rect_filled(screen, 0.0, rgba(kind.base(), 1.0));
    paint(p, screen, v, kind);
}

fn paint(p: &egui::Painter, screen: Rect, v: &HudView, kind: MenuBackdrop) {
    let t = if v.settings_snapshot.reduced_motion { 0.0 } else { v.time };
    let (w, h) = (screen.width(), screen.height());
    match kind {
        MenuBackdrop::Stars => {
            for k in 0..140u32 {
                let (h1, h2) = (hash01(k, 1), hash01(k, 2));
                let pulse = 0.5 + 0.5 * (t * (0.5 + h1) + h2 * TAU).sin();
                let at = Pos2::new(screen.left() + h1 * w, screen.top() + h2 * h);
                let size = if h1 > 0.93 { 4.0 } else { 2.0 };
                p.rect_filled(
                    Rect::from_center_size(at, Vec2::splat(size)),
                    0.0,
                    rgba(hue(h2 + t * 0.02), 0.15 + 0.5 * pulse),
                );
            }
        }
        MenuBackdrop::Cards => {
            // Slow tilted card outlines drifting upward, wrapping round.
            let cw = (h * 0.16).clamp(40.0, 120.0);
            for k in 0..22u32 {
                let (h1, h2, h3) = (hash01(k, 11), hash01(k, 12), hash01(k, 13));
                let speed = 0.015 + 0.03 * h3;
                let y = (h2 - t * speed).rem_euclid(1.0) * (h + cw * 2.0) - cw;
                let x = screen.left() + h1 * w + (t * 0.3 + h3 * TAU).sin() * 12.0;
                let ang = (h1 - 0.5) * 0.8 + (t * 0.2 + h2 * TAU).sin() * 0.08;
                let size = Vec2::new(cw, cw * 1.4) * (0.6 + 0.6 * h3);
                let col = hue(h3 + k as f32 * 0.05);
                let (s, c) = ang.sin_cos();
                let corner = |dx: f32, dy: f32| {
                    Pos2::new(x + dx * c - dy * s, screen.top() + y + dx * s + dy * c)
                };
                let (hx, hy) = (size.x * 0.5, size.y * 0.5);
                let pts = vec![corner(-hx, -hy), corner(hx, -hy), corner(hx, hy), corner(-hx, hy)];
                p.add(egui::Shape::convex_polygon(
                    pts.clone(),
                    rgba(col, 0.07),
                    Stroke::new(1.5, rgba(col, 0.35)),
                ));
                // The card's art window.
                let inner: Vec<Pos2> = pts
                    .iter()
                    .map(|q| Pos2::new(x + (q.x - x) * 0.7, screen.top() + y + (q.y - screen.top() - y) * 0.6))
                    .collect();
                p.add(egui::Shape::closed_line(inner, Stroke::new(1.0, rgba(col, 0.18))));
            }
        }
        MenuBackdrop::Garden => {
            // Rising green motes and soft swaying stems from the bottom.
            for k in 0..60u32 {
                let (h1, h2) = (hash01(k, 21), hash01(k, 22));
                let y = (h2 - t * (0.02 + 0.03 * h1)).rem_euclid(1.0);
                let x = h1 * w + (t * 0.5 + h2 * TAU).sin() * 10.0;
                p.circle_filled(
                    Pos2::new(screen.left() + x, screen.top() + y * h),
                    1.5 + 2.5 * h1,
                    rgba([120, 255, 160], 0.1 + 0.35 * (1.0 - y)),
                );
            }
            for k in 0..14u32 {
                let x = screen.left() + (k as f32 + 0.5) / 14.0 * w;
                let len = h * (0.15 + 0.2 * hash01(k, 23));
                let sway = (t * 0.6 + k as f32).sin() * 14.0;
                p.line_segment(
                    [Pos2::new(x, screen.bottom()), Pos2::new(x + sway, screen.bottom() - len)],
                    Stroke::new(2.0, rgba([60, 200, 110], 0.3)),
                );
            }
        }
        MenuBackdrop::MirrorHall => {
            // Diagonal panes that shimmer past each other.
            let n = 18;
            for k in 0..n {
                let f = k as f32 / n as f32;
                let x = screen.left() + ((f + t * 0.01).rem_euclid(1.0)) * (w + h) - h;
                let a = 0.04 + 0.05 * (t * 0.8 + k as f32 * 1.7).sin().abs();
                p.line_segment(
                    [Pos2::new(x, screen.bottom()), Pos2::new(x + h * 0.6, screen.top())],
                    Stroke::new(10.0 + 14.0 * hash01(k as u32, 31), rgba([190, 200, 255], a)),
                );
            }
        }
        MenuBackdrop::Clockworks => {
            // Slowly turning ringed gears with ticking teeth.
            for k in 0..7u32 {
                let (h1, h2) = (hash01(k, 41), hash01(k, 42));
                let c = Pos2::new(screen.left() + h1 * w, screen.top() + h2 * h);
                let r = h * (0.08 + 0.14 * hash01(k, 43));
                let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
                let spin = t * 0.15 * dir + h1 * TAU;
                let col: Color32 = rgba([230, 150, 60], 0.22);
                p.circle_stroke(c, r, Stroke::new(2.0, col));
                p.circle_stroke(c, r * 0.55, Stroke::new(1.0, rgba([230, 150, 60], 0.12)));
                for i in 0..12 {
                    let a = spin + i as f32 / 12.0 * TAU;
                    let d = Vec2::new(a.cos(), a.sin());
                    p.line_segment([c + d * r, c + d * (r * 1.18)], Stroke::new(3.0, col));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepping_wraps_through_every_design() {
        let mut b = MenuBackdrop::Stars;
        for _ in 0..ORDER.len() {
            b = b.step(1);
        }
        assert_eq!(b, MenuBackdrop::Stars);
        assert_eq!(MenuBackdrop::Stars.step(-1), MenuBackdrop::Clockworks);
    }
}
