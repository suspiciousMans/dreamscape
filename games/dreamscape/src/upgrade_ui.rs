//! The choice screen, drawn over the frozen dream: pick one of three run
//! upgrades after each dream, or (at the wake door) WAKE or GO DEEPER.
//! Cards are pixel-framed with a 9x9 icon, and the chosen one breathes.

use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use crate::pixels::icons;
use crate::upgrades::Icon;
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect, Stroke, Vec2};

#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceCard {
    pub name: String,
    pub desc: String,
    pub icon: Icon,
    pub color: [u8; 3],
    /// Frame colour (the card's rarity).
    pub frame: [u8; 3],
    /// Small line under the name ("ability · shift", "x2", ...).
    pub tag: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChoiceView {
    pub title: String,
    pub subtitle: String,
    pub cards: Vec<ChoiceCard>,
    pub selected: usize,
    /// Seconds the screen has been up (cards deal in one by one).
    pub age: f32,
    /// Key hints along the bottom.
    pub hint: String,
}

pub fn icon_rows(icon: Icon) -> &'static [&'static str; icons::SIZE] {
    match icon {
        Icon::Boot => &icons::BOOT,
        Icon::Wind => &icons::WIND,
        Icon::Eye => &icons::EYE,
        Icon::Heart => &icons::HEART,
        Icon::Shard => &icons::SHARD,
        Icon::Lid => &icons::LID,
        Icon::Magnet => &icons::MAGNET,
        Icon::Brain => &icons::BRAIN,
        Icon::Gem => &icons::GEM,
    }
}

/// Card rectangles, centred on the screen, `n` side by side.
pub fn card_rects(screen: Rect, n: usize) -> Vec<Rect> {
    let n = n.max(1);
    let w = (screen.width() * 0.86 / n as f32).min(300.0);
    let h = (w * 1.3).min(screen.height() * 0.55);
    let gap = w * 0.12;
    let total = n as f32 * w + (n - 1) as f32 * gap;
    let x0 = screen.center().x - total * 0.5;
    let y0 = screen.center().y - h * 0.4;
    (0..n)
        .map(|i| Rect::from_min_size(Pos2::new(x0 + i as f32 * (w + gap), y0), Vec2::new(w, h)))
        .collect()
}

/// 0 → 1 as card `i` deals in.
pub fn deal(age: f32, i: usize) -> f32 {
    ((age - 0.12 * i as f32) / 0.25).clamp(0.0, 1.0)
}

pub fn draw(p: &egui::Painter, screen: Rect, v: &HudView, c: &ChoiceView) {
    p.rect_filled(screen, 0.0, rgba([8, 4, 20], 0.72));
    let top = screen.top() + screen.height() * 0.1;
    glitch_text(
        p,
        Pos2::new(screen.center().x, top),
        true,
        &c.title,
        44.0,
        ink(v),
        1.0,
        v,
    );
    p.text(
        Pos2::new(screen.center().x, top + 54.0),
        Align2::CENTER_TOP,
        &c.subtitle,
        FontId::proportional(22.0),
        rgba([200, 190, 230], 0.85),
    );
    let rects = card_rects(screen, c.cards.len());
    for (i, (card, r)) in c.cards.iter().zip(&rects).enumerate() {
        let t = deal(c.age, i);
        if t <= 0.0 {
            continue;
        }
        let sel = i == c.selected;
        let lift = if sel {
            -10.0 - 3.0 * (v.time * 3.0).sin()
        } else {
            0.0
        };
        let r = r.translate(Vec2::new(0.0, (1.0 - t) * 60.0 + lift));
        let frame = if sel { hue(v.time * 0.2) } else { card.frame };
        p.rect_filled(r, 6.0, rgba([18, 10, 40], 0.95 * t));
        p.rect_stroke(
            r,
            6.0,
            Stroke::new(if sel { 4.0_f32 } else { 2.0 }, rgba(frame, t)),
        );
        // Key hint in the corner.
        p.text(
            r.left_top() + Vec2::new(10.0, 6.0),
            Align2::LEFT_TOP,
            format!("[{}]", i + 1),
            FontId::monospace(20.0),
            rgba(ink(v), 0.7 * t),
        );
        // Icon.
        let px = (r.width() / 30.0).round().max(3.0);
        let centre = Pos2::new(r.center().x, r.top() + r.height() * 0.28);
        let accent = [
            card.color[0] / 2 + 60,
            card.color[1] / 2 + 60,
            card.color[2] / 2 + 60,
        ];
        for (x, y, acc) in icons::cells(icon_rows(card.icon)) {
            let at = centre + Vec2::new(x as f32 * px, y as f32 * px);
            p.rect_filled(
                Rect::from_center_size(at, Vec2::splat(px)),
                0.0,
                rgba(if acc { accent } else { card.color }, t),
            );
        }
        let name_y = r.top() + r.height() * 0.52;
        p.text(
            Pos2::new(r.center().x, name_y),
            Align2::CENTER_TOP,
            &card.name,
            FontId::monospace(28.0),
            rgba(if sel { [255, 255, 255] } else { ink(v) }, t),
        );
        p.text(
            Pos2::new(r.center().x, name_y + 32.0),
            Align2::CENTER_TOP,
            &card.tag,
            FontId::monospace(16.0),
            rgba(card.color, 0.9 * t),
        );
        let galley = p.layout(
            card.desc.clone(),
            FontId::proportional(19.0),
            rgba([215, 205, 240], t),
            r.width() - 28.0,
        );
        let at = Pos2::new(r.center().x - galley.size().x * 0.5, name_y + 58.0);
        p.galley(at, galley, rgba([215, 205, 240], t));
    }
    p.text(
        Pos2::new(screen.center().x, screen.bottom() - 48.0),
        Align2::CENTER_TOP,
        &c.hint,
        FontId::monospace(20.0),
        rgba(ink(v), 0.75),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_fit_the_screen_and_do_not_overlap() {
        for (w, h) in [(1280.0, 720.0), (800.0, 600.0), (1920.0, 1080.0)] {
            let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h));
            for n in 1..=5 {
                let rs = card_rects(screen, n);
                assert_eq!(rs.len(), n);
                for r in &rs {
                    assert!(screen.contains_rect(*r), "{r:?} off {w}x{h}");
                }
                for pair in rs.windows(2) {
                    assert!(pair[0].right() < pair[1].left());
                }
            }
        }
    }

    #[test]
    fn cards_deal_in_order() {
        assert_eq!(deal(0.0, 0), 0.0);
        assert!(deal(0.2, 0) > deal(0.2, 1));
        assert_eq!(deal(5.0, 2), 1.0);
    }
}
