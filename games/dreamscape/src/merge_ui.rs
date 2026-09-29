//! The moth merges cards: pick the first card (its shape), then the second
//! (its colours and mood); the preview shows what they'll become.

use crate::booklet_ui::{card, CardArt, CARD_SIZE};
use crate::cards::{Card, Rarity};
use crate::fusion;
use crate::hud::{glitch_text, ink, rgba, HudView};
use crate::loadout_ui::{describe, draw_grid};
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MergeView {
    pub cards: Vec<Card>,
    pub selected: usize,
    /// The first card, once picked.
    pub first: Option<u32>,
    /// Shown after a merge.
    pub status: Option<String>,
}

/// What the two cards would make, if they can merge.
pub fn preview(m: &MergeView) -> Option<Card> {
    let a = m.cards.iter().find(|c| Some(c.number) == m.first)?;
    let b = m.cards.get(m.selected)?;
    fusion::can_merge(a, b).then(|| fusion::merge(a, b))
}

/// Any two cards in the booklet can merge.
pub fn any_pair(cards: &[Card]) -> bool {
    cards
        .iter()
        .any(|a| cards.iter().any(|b| fusion::can_merge(a, b)))
}

pub fn draw(ctx: &egui::Context, p: &egui::Painter, screen: Rect, v: &HudView, art: &mut CardArt) {
    let m = &v.merge;
    p.rect_filled(screen, 0.0, rgba([10, 4, 18], 0.95));
    let cx = screen.center().x;
    let top = screen.top() + 24.0;
    glitch_text(
        p,
        Pos2::new(cx, top),
        true,
        "THE MOTH MERGES",
        40.0,
        ink(v),
        1.0,
        v,
    );
    let ask = match m.first {
        None => "pick the first card: it keeps its shape and its ability",
        Some(_) => "pick the second card: it gives its colours and mood",
    };
    p.text(
        Pos2::new(cx, top + 56.0),
        Align2::CENTER_TOP,
        m.status.as_deref().unwrap_or(ask),
        FontId::monospace(18.0),
        rgba([220, 200, 255], 0.9),
    );
    let grid_top = top + 100.0;
    let marked: Vec<u32> = m.first.into_iter().collect();
    draw_grid(p, screen, v, &m.cards, m.selected, &marked, grid_top);

    let right = screen.right() - screen.width() * 0.05 - CARD_SIZE.x;
    let r = Rect::from_min_size(Pos2::new(right, grid_top), CARD_SIZE);
    let shown = preview(m).or_else(|| m.cards.get(m.selected).cloned());
    if let Some(c) = &shown {
        card(ctx, p, r, c, v, art, 1.0);
        let (active, passive) = describe(c, None);
        let label = match (m.first.is_some(), c.rarity) {
            (true, Rarity::Resonant) => "RESONANT: they were meant to meet",
            (true, Rarity::Fused) => "FUSED",
            _ => "",
        };
        let mut y = r.bottom() + 14.0;
        for (text, col) in [
            (label.to_string(), [255, 230, 160]),
            (active, [255, 200, 255]),
            (passive, [170, 255, 200]),
        ] {
            p.text(
                Pos2::new(r.left(), y),
                Align2::LEFT_TOP,
                text,
                FontId::monospace(16.0),
                rgba(col, 0.95),
            );
            y += 26.0;
        }
    }
    p.text(
        Pos2::new(cx, screen.bottom() - 40.0),
        Align2::CENTER_TOP,
        v.k("[a/d/w/s] browse   [enter] pick   [esc] back"),
        FontId::monospace(20.0),
        rgba(ink(v), 0.5 + 0.3 * (v.time * 2.0).sin()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{card_from, tests::record};
    use crate::dream::DreamTheme::{self, *};

    fn cards(themes: &[DreamTheme]) -> Vec<Card> {
        themes
            .iter()
            .enumerate()
            .map(|(i, &t)| {
                let mut c = card_from(&record(t, i as u64, 1, 0.2, false));
                c.number = i as u32 + 1;
                c
            })
            .collect()
    }

    #[test]
    fn the_preview_needs_two_different_dreams() {
        let mut m = MergeView {
            cards: cards(&[Garden, MyceliumGrove, Garden]),
            ..Default::default()
        };
        assert!(preview(&m).is_none(), "nothing picked yet");
        m.first = Some(1);
        m.selected = 1;
        assert_eq!(preview(&m).unwrap().rarity, Rarity::Resonant);
        m.selected = 2;
        assert!(preview(&m).is_none(), "same dream");
        assert!(any_pair(&m.cards));
        assert!(!any_pair(&cards(&[Garden, Garden])));
    }
}
