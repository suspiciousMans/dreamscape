//! The deck builder: the loadout screen, now choosing the cards you draw
//! from in a run. Reuses the loadout screen's grid, card and hint bar; the
//! slot row becomes a strip of up to [`DECK_MAX`] pips with a DECK n/12 meter.

use crate::booklet_ui::{card, card_frame, CardArt, CARD_SIZE};
use crate::deck::{self, DECK_MAX, DECK_MIN};
use crate::hud::{glitch_text, ink, rgba, HudView};
use crate::loadout_ui::{self, GridMarks};
use crate::pixel_ui;
use crate::pixels::icons;
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

/// How the meter reads for a deck of `len` cards out of `owned`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Meter {
    /// Nothing chosen: the newest cards are used.
    Automatic,
    /// Too few cards to start.
    Short,
    Ready,
    Full,
}

pub fn meter(len: usize, owned: usize) -> Meter {
    if len == 0 {
        Meter::Automatic
    } else if len < deck::needed(owned) {
        Meter::Short
    } else if len >= DECK_MAX {
        Meter::Full
    } else {
        Meter::Ready
    }
}

/// The meter's label: "DECK 7/12".
pub fn label(len: usize) -> String {
    format!("DECK {len}/{DECK_MAX}")
}

/// The meter's note under the label.
pub fn note(len: usize, owned: usize) -> String {
    match meter(len, owned) {
        Meter::Automatic => "nothing chosen: your newest cards come along".into(),
        Meter::Short => format!("needs {} cards to start", deck::needed(owned)),
        Meter::Ready => format!("you hold {} at a time", crate::upgrades::HAND_SIZE.min(len)),
        Meter::Full => "the deck is full".into(),
    }
}

// === decklife ===
/// "7 of 10 passives active", plus a word on the highlighted card when the
/// cap left its passive out (`selected_benched`: `None` = not in the deck).
pub fn passives_note(active: usize, selected_benched: Option<bool>) -> String {
    let base = format!("{active} of {} passives active", crate::upgrades::KIT_PASSIVES);
    match selected_benched {
        Some(true) => format!("{base} · this card's is benched"),
        _ => base,
    }
}
// === end decklife ===

/// The pips for `DECK_MAX` cards, in a row across the top of the screen.
pub fn pips(screen: Rect) -> Vec<Rect> {
    let lay = loadout_ui::layout(screen);
    let u = pixel_ui::scale(screen);
    let gap = 6.0 * u;
    let w = ((screen.width() * 0.9 - gap * (DECK_MAX as f32 - 1.0)) / DECK_MAX as f32).min(96.0 * u);
    let total = DECK_MAX as f32 * (w + gap) - gap;
    let h = (lay.slot_h * 0.62).max(20.0);
    (0..DECK_MAX)
        .map(|i| {
            Rect::from_min_size(
                Pos2::new(screen.center().x - total / 2.0 + i as f32 * (w + gap), lay.row_y),
                Vec2::new(w, h),
            )
        })
        .collect()
}

/// Where the meter bar goes: under the pips, as wide as the row.
pub fn meter_rect(screen: Rect) -> Rect {
    let p = pips(screen);
    let u = pixel_ui::scale(screen);
    let (l, r) = (p[0].left(), p[DECK_MAX - 1].right());
    Rect::from_min_size(Pos2::new(l, p[0].bottom() + 8.0 * u), Vec2::new(r - l, 8.0 * u))
}

/// Meter fill, 0..1, and where the minimum sits on the bar.
pub fn meter_fill(len: usize) -> (f32, f32) {
    (len as f32 / DECK_MAX as f32, DECK_MIN as f32 / DECK_MAX as f32)
}

pub fn draw(ctx: &egui::Context, p: &egui::Painter, screen: Rect, v: &HudView, art: &mut CardArt) {
    let m = &v.loadout;
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    p.rect_filled(screen, 0.0, rgba([8, 4, 20], 0.95));
    let cx = screen.center().x;
    let top = screen.top() + 22.0 * u;
    glitch_text(p, Pos2::new(cx, top), true, &label(m.chosen.len()), 44.0 * u, ink(v), 1.0, v);
    let owned = m.cards.len();
    let kind = meter(m.chosen.len(), owned);

    // The pips: each shows the card chosen at that place in the deck.
    for (i, r) in pips(screen).into_iter().enumerate() {
        match m.chosen.get(i).and_then(|&n| m.cards.iter().find(|c| c.number == n)) {
            Some(cd) => {
                let col = card_frame(cd, v.time);
                let off = m.benched.contains(&cd.number); // === decklife ===
                pixel_ui::panel(p, r, col, rgba(col, 0.16), c * 0.7, if off { 0.45 } else { 1.0 });
                let pw = crate::powers::power_of(cd);
                pixel_ui::icon(p, r.center(), icons::ability(pw.active), (c * 0.5).max(1.5), pw.tint, pixel_ui::lighter(pw.tint), 1.0);
                p.text(
                    r.left_top() + Vec2::new(c, c * 0.6),
                    Align2::LEFT_TOP,
                    format!("{}", cd.number),
                    FontId::monospace((8.0 * u).max(6.0)),
                    rgba(col, 0.9),
                );
            }
            None => pixel_ui::panel(p, r, [70, 62, 100], rgba([12, 7, 24], 0.6), c * 0.7, 0.6),
        }
    }

    // The meter: filled to the count, a notch at the minimum.
    let bar = meter_rect(screen);
    let (fill, min) = meter_fill(m.chosen.len());
    let col = match kind {
        Meter::Short => [230, 110, 120],
        Meter::Full => [240, 200, 110],
        Meter::Ready => [130, 220, 170],
        Meter::Automatic => [150, 140, 190],
    };
    p.rect_filled(bar, 0.0, rgba([20, 12, 40], 0.9));
    p.rect_filled(Rect::from_min_size(bar.min, Vec2::new(bar.width() * fill, bar.height())), 0.0, rgba(col, 0.9));
    let notch = bar.left() + bar.width() * min;
    p.line_segment(
        [Pos2::new(notch, bar.top() - 2.0 * u), Pos2::new(notch, bar.bottom() + 2.0 * u)],
        engine::ui::egui::Stroke::new(1.5 * u, rgba([255, 255, 255], 0.6)),
    );
    p.text(
        Pos2::new(cx, bar.bottom() + 6.0 * u),
        Align2::CENTER_TOP,
        note(m.chosen.len(), owned),
        FontId::monospace(13.0 * u),
        rgba(col, 0.95),
    );
    if !m.passives.is_empty() {
        p.text(
            Pos2::new(cx, bar.bottom() + 20.0 * u),
            Align2::CENTER_TOP,
            &m.passives,
            FontId::monospace(11.0 * u),
            rgba([190, 180, 230], 0.9),
        );
    }
    if let Some(line) = &m.companion {
        p.text(Pos2::new(cx, bar.bottom() + 36.0 * u), Align2::CENTER_TOP, v.k(line), FontId::monospace(14.0 * u), rgba([200, 170, 230], 0.9));
    }

    let lay = loadout_ui::layout(screen);
    let (grid_top, grid_w) = (lay.grid_top, lay.grid_w);
    let marks = GridMarks { marked: &m.chosen, ..Default::default() };
    loadout_ui::draw_grid(p, screen, v, &m.cards, m.selected, &marks, grid_top, grid_w);

    if let Some(sel) = m.cards.get(m.selected) {
        let avail = screen.bottom() - 64.0 * u - grid_top;
        let sc = ((avail - 150.0 * u) / CARD_SIZE.y).clamp(0.45, 1.0);
        let size = CARD_SIZE * sc;
        let right = screen.right() - screen.width() * 0.05 - size.x;
        let r = Rect::from_min_size(Pos2::new(right, grid_top), size);
        let col = card_frame(sel, v.time);
        let back = r.expand2(Vec2::new(10.0 * u, 10.0 * u));
        pixel_ui::panel(p, back, col, rgba([12, 6, 26], 0.8), c, 0.5);
        card(ctx, p, r, sel, v, art, sc);
        let at = Pos2::new(r.left(), back.bottom() + 12.0 * u);
        pixel_ui::split_text(p, at, Align2::LEFT_TOP, &sel.name, pixel_ui::fit(&sel.name, 15.0 * u, size.x, 8.0), rgba(col, 1.0), 1.0, 0.5);
        loadout_ui::power_lines(p, at + Vec2::new(0.0, 22.0 * u), size.x, sel, None, u, v.time);
    }

    loadout_ui::hint_bar(
        p,
        screen,
        v,
        &format!(
            "[a/d/w/s] browse  [enter] add/remove  [t] sort: {}  [r] flip  [space] fall asleep  [esc] back",
            m.sort
        ),
    );
    pixel_ui::scanlines(p, screen, c, 10);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0))
    }

    #[test]
    fn the_meter_reads_empty_short_ready_and_full() {
        assert_eq!(meter(0, 20), Meter::Automatic);
        assert_eq!(meter(2, 20), Meter::Short);
        assert_eq!(meter(3, 20), Meter::Ready);
        assert_eq!(meter(DECK_MAX, 20), Meter::Full);
        assert_eq!(meter(1, 1), Meter::Ready, "a one-card booklet is ready");
    }

    #[test]
    fn the_passive_note_counts_and_flags_a_benched_card() {
        assert_eq!(passives_note(7, None), "7 of 10 passives active");
        assert_eq!(passives_note(10, Some(false)), "10 of 10 passives active");
        assert!(passives_note(10, Some(true)).contains("benched"));
    }

    #[test]
    fn the_label_shows_count_over_max() {
        assert_eq!(label(7), "DECK 7/12");
        assert_eq!(label(0), "DECK 0/12");
    }

    #[test]
    fn notes_say_what_is_missing() {
        assert!(note(2, 20).contains("3"));
        assert!(note(0, 20).contains("newest"));
        assert!(note(12, 20).contains("full"));
        assert!(note(7, 20).contains(&crate::upgrades::HAND_SIZE.to_string()), "a full hand at a time");
        assert!(note(4, 20).contains("4"), "a small deck holds what it has");
    }

    #[test]
    fn twelve_pips_sit_in_order_on_screen_above_the_grid() {
        for (w, h) in [(1280.0, 720.0), (960.0, 540.0), (1920.0, 1080.0)] {
            let s = Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h));
            let ps = pips(s);
            assert_eq!(ps.len(), DECK_MAX);
            for pair in ps.windows(2) {
                assert!(pair[1].left() > pair[0].right() - 0.01);
            }
            assert!(s.contains_rect(ps[0].union(ps[DECK_MAX - 1])), "{w}x{h}");
            let lay = loadout_ui::layout(s);
            let bar = meter_rect(s);
            assert!(bar.bottom() + 30.0 < lay.grid_top + lay.slot_h, "meter clears the grid");
        }
        let (fill, min) = meter_fill(6);
        assert_eq!(fill, 0.5);
        assert_eq!(min, 0.25);
        let _ = screen();
    }
}
