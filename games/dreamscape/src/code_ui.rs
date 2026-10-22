//! The "enter a dream code" screen: type or paste `DS-XXXXX-XXXXX-XXXXX` and
//! dream the same run as a friend. The code logic lives in `seedcode.rs`.

use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect};

#[derive(Clone, Debug, Default)]
pub struct CodeView {
    /// The digits typed so far (no prefix, no dashes).
    pub digits: String,
    /// Why the last try failed, if it did.
    pub error: String,
}

/// The text shown in the box for `digits`: grouped, padded with dots so the
/// box keeps its width as you type.
pub fn shown(digits: &str) -> String {
    let mut g = crate::seedcode::group(digits);
    // A completed code needs no cursor; otherwise pad out to full width.
    let want = crate::seedcode::group(&"·".repeat(crate::seedcode::DIGITS)).chars().count();
    while g.chars().count() < want {
        let n = g.chars().count();
        // Keep the dashes where they will fall.
        g.push(if (n - 2) % 6 == 0 { '-' } else { '·' });
    }
    g
}

pub fn draw(p: &egui::Painter, screen: Rect, v: &HudView, c: &CodeView) {
    p.rect_filled(screen, 0.0, rgba([5, 2, 14], 1.0));
    let cx = screen.center().x;
    let top = screen.top() + screen.height() * 0.2;
    glitch_text(
        p,
        Pos2::new(cx, top),
        true,
        "DREAM CODE",
        64.0,
        ink(v),
        1.0,
        v,
    );
    p.text(
        Pos2::new(cx, top + 84.0),
        Align2::CENTER_TOP,
        v.k("a friend's code opens the very same dream"),
        FontId::proportional(22.0),
        rgba([200, 190, 230], 0.85),
    );
    let mut line = shown(&c.digits);
    if (v.time * 2.0) as i32 % 2 == 0 && c.digits.len() < crate::seedcode::DIGITS {
        // A blinking cursor on the next digit.
        let mut at = crate::seedcode::group(&c.digits).chars().count();
        if line.chars().nth(at) == Some('-') {
            at += 1; // sit on the digit, not on the dash before it
        }
        if let Some((i, _)) = line.char_indices().nth(at) {
            line.replace_range(i..i + line[i..].chars().next().map_or(0, char::len_utf8), "_");
        }
    }
    p.text(
        Pos2::new(cx, top + 170.0),
        Align2::CENTER_TOP,
        line,
        FontId::monospace(46.0),
        rgba(hue(v.time * 0.1), 1.0),
    );
    if !c.error.is_empty() {
        p.text(
            Pos2::new(cx, top + 250.0),
            Align2::CENTER_TOP,
            &c.error,
            FontId::monospace(22.0),
            rgba([255, 120, 150], 0.95),
        );
    }
    p.text(
        Pos2::new(cx, screen.bottom() - 80.0),
        Align2::CENTER_TOP,
        v.k("[type or ctrl+v]   [enter] dream it   [backspace]   [esc] back"),
        FontId::monospace(20.0),
        rgba(ink(v), 0.7),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_box_keeps_one_width_while_typing() {
        let full = shown("");
        for n in 0..=crate::seedcode::DIGITS {
            let s = shown(&"A".repeat(n));
            assert_eq!(s.chars().count(), full.chars().count(), "{n} digits: {s}");
        }
        assert_eq!(shown("ABC"), "DS-ABC··-·····-·····");
    }

    #[test]
    fn a_full_code_shows_as_itself() {
        let code = crate::seedcode::encode(77, true);
        let digits = crate::seedcode::clean(&code);
        assert_eq!(shown(&digits), code);
    }
}
