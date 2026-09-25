//! The codex: every dream type and strange thing you've met, and the best
//! depth you've reached in each dream. Unmet entries stay "???".

use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect};

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
    let cx = screen.center().x;
    let top = screen.top() + screen.height() * 0.06;
    glitch_text(p, Pos2::new(cx, top), true, "CODEX", 48.0, ink(v), 1.0, v);
    let left = screen.left() + screen.width() * 0.08;
    let right = screen.left() + screen.width() * 0.52;
    let row = ((screen.height() * 0.7) / c.dreams.len().max(1) as f32).min(28.0);
    p.text(
        Pos2::new(left, top + 60.0),
        Align2::LEFT_TOP,
        "DREAMS",
        FontId::monospace(22.0),
        rgba([255, 200, 120], 1.0),
    );
    for (i, (name, best)) in c.dreams.iter().enumerate() {
        let y = top + 92.0 + i as f32 * row;
        let (text, col, a) = match best {
            Some(d) => (format!("{name:<20} deepest {d}"), ink(v), 0.95),
            None => ("???".to_string(), [120, 110, 150], 0.7),
        };
        p.text(
            Pos2::new(left, y),
            Align2::LEFT_TOP,
            text,
            FontId::monospace(19.0),
            rgba(col, a),
        );
    }
    p.text(
        Pos2::new(right, top + 60.0),
        Align2::LEFT_TOP,
        "STRANGE THINGS",
        FontId::monospace(22.0),
        rgba([255, 120, 190], 1.0),
    );
    for (i, (name, hint)) in c.enemies.iter().enumerate() {
        let y = top + 92.0 + i as f32 * 52.0;
        match hint {
            Some(h) => {
                p.text(
                    Pos2::new(right, y),
                    Align2::LEFT_TOP,
                    name,
                    FontId::monospace(20.0),
                    rgba(hue(v.time * 0.1 + i as f32 * 0.2), 1.0),
                );
                let g = p.layout(
                    h.clone(),
                    FontId::proportional(16.0),
                    rgba([200, 190, 230], 0.9),
                    screen.width() * 0.4,
                );
                p.galley(Pos2::new(right, y + 22.0), g, rgba([200, 190, 230], 0.9));
            }
            None => {
                p.text(
                    Pos2::new(right, y),
                    Align2::LEFT_TOP,
                    "???",
                    FontId::monospace(20.0),
                    rgba([120, 110, 150], 0.7),
                );
            }
        }
    }
    p.text(
        Pos2::new(cx, screen.bottom() - 70.0),
        Align2::CENTER_TOP,
        &c.footer,
        FontId::monospace(18.0),
        rgba([255, 210, 80], 0.9),
    );
    p.text(
        Pos2::new(cx, screen.bottom() - 40.0),
        Align2::CENTER_TOP,
        "[esc] back",
        FontId::monospace(18.0),
        rgba(ink(v), 0.7),
    );
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
