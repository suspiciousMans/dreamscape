//! The moth merges cards: pick the first card (its shape and ability), then
//! the second (its colours and mood). The altar shows the two cards going in
//! and the one that comes out, live, before you commit.

use crate::booklet_ui::{card, card_frame, CardArt, CARD_SIZE};
use crate::cards::{Card, Rarity};
use crate::fusion;
use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use crate::loadout_ui::{draw_grid, hint_bar, power_lines, GridMarks};
use crate::pixel_ui;
use engine::ui::egui::{self, Align2, Color32, FontId, Pos2, Rect, Vec2};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MergeView {
    pub cards: std::sync::Arc<[Card]>, // === perf === shared, see viewcache
    pub selected: usize,
    /// The first card, once picked.
    pub first: Option<u32>,
    /// Shown after a merge.
    pub status: Option<String>,
    /// Which of the `fusion::RESONANT` pairs have been merged before, in order.
    pub resonant_found: Vec<bool>,
    /// The sort label shown in the hint bar (empty = none).
    pub sort: String,
}

/// What the two cards would make, if they can merge.
pub fn preview(m: &MergeView) -> Option<Card> {
    let a = m.cards.iter().find(|c| Some(c.number) == m.first)?;
    let b = m.cards.get(m.selected)?;
    fusion::can_merge(a, b).then(|| fusion::merge(a, b))
}

#[cfg(test)]
/// Any two cards in the booklet can merge.
pub fn any_pair(cards: &[Card]) -> bool {
    cards
        .iter()
        .any(|a| cards.iter().any(|b| fusion::can_merge(a, b)))
}

/// Where things sit, so drawing and tests agree.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub grid_top: f32,
    pub grid_w: f32,
    pub altar: Rect,
    /// The three card slots: first card, second card, result.
    pub slots: [Rect; 3],
    /// Card scale inside the slots.
    pub scale: f32,
    /// Where the "+" and the arrow go.
    pub signs: [Pos2; 2],
    /// Top of the lines under the cards.
    pub info_top: f32,
}

pub fn layout(screen: Rect) -> Layout {
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    let left = screen.left() + screen.width() * 0.05;
    let grid_top = screen.top() + 140.0 * u;
    let altar_w = (screen.width() * 0.46).clamp(360.0, 720.0);
    let altar_left = screen.right() - screen.width() * 0.04 - altar_w;
    let max_bottom = screen.bottom() - 76.0 * u;
    let pad = 14.0 * u;
    let gap = 8.0 * c;
    let label_h = 22.0 * u;
    let inner_w = altar_w - 2.0 * pad;
    let card_w = (inner_w - 2.0 * gap) / 3.0;
    let avail_h = max_bottom - (grid_top - 6.0 * u) - 2.0 * pad - label_h - 150.0 * u;
    let scale = (card_w / CARD_SIZE.x)
        .min(avail_h / CARD_SIZE.y)
        .clamp(0.25, 0.9);
    let size = CARD_SIZE * scale;
    let total = 3.0 * size.x + 2.0 * gap;
    let x0 = altar_left + altar_w * 0.5 - total * 0.5;
    let y = grid_top - 6.0 * u + pad + label_h;
    let slots =
        [0.0, 1.0, 2.0].map(|i| Rect::from_min_size(Pos2::new(x0 + i * (size.x + gap), y), size));
    let sign_y = y + size.y * 0.5;
    let info_top = y + size.y + 12.0 * u;
    let altar = Rect::from_min_max(
        Pos2::new(altar_left, grid_top - 6.0 * u),
        Pos2::new(altar_left + altar_w, (info_top + 150.0 * u).min(max_bottom)),
    );
    Layout {
        grid_top,
        grid_w: (altar_left - 24.0 * u - left).max(300.0),
        altar,
        slots,
        scale,
        signs: [
            Pos2::new(slots[0].right() + gap * 0.5, sign_y),
            Pos2::new(slots[1].right() + gap * 0.5, sign_y),
        ],
        info_top,
    }
}

/// What the altar is waiting for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Nothing picked: the cursor's card is what enter will take first.
    PickFirst,
    /// The first is in; the cursor's card can be merged into it.
    PickSecond,
    /// The first is in, but the cursor's card is the same dream.
    Blocked,
}

pub fn step(m: &MergeView) -> Step {
    let Some(a) = m.cards.iter().find(|c| Some(c.number) == m.first) else {
        return Step::PickFirst;
    };
    match m.cards.get(m.selected) {
        Some(b) if fusion::can_merge(a, b) => Step::PickSecond,
        _ => Step::Blocked,
    }
}

/// A card slot with its label above.
#[allow(clippy::too_many_arguments)]
fn slot(
    ctx: &egui::Context,
    p: &egui::Painter,
    r: Rect,
    label: &str,
    shown: Option<&Card>,
    dimmed: bool,
    waiting: bool,
    v: &HudView,
    art: &mut CardArt,
    u: f32,
    scale: f32,
) {
    let c = pixel_ui::cell(u);
    p.text(
        Pos2::new(r.center().x, r.top() - 6.0 * u),
        Align2::CENTER_BOTTOM,
        label,
        FontId::monospace(pixel_ui::fit(label, 12.0 * u, r.width() + 16.0 * u, 7.0)),
        rgba(
            if waiting {
                [255, 240, 170]
            } else {
                [170, 155, 205]
            },
            0.95,
        ),
    );
    match shown {
        Some(card_ref) => {
            card(ctx, p, r, card_ref, v, art, scale);
            if dimmed {
                p.rect_filled(r, 0.0, rgba([8, 4, 20], 0.55));
                pixel_ui::hatch(p, r.shrink(c), c, Color32::from_black_alpha(60));
            }
        }
        None => {
            let pulse = if waiting {
                0.6 + 0.4 * (v.time * 3.0).sin().abs()
            } else {
                0.5
            };
            pixel_ui::panel(
                p,
                r,
                if waiting {
                    [255, 230, 150]
                } else {
                    [90, 80, 120]
                },
                rgba([14, 8, 28], 0.7),
                c,
                pulse,
            );
            pixel_ui::hatch(p, r.shrink(c * 3.0), c, Color32::from_white_alpha(8));
            pixel_ui::split_text(
                p,
                r.center(),
                Align2::CENTER_CENTER,
                "?",
                (r.width() * 0.45).max(16.0),
                rgba([200, 190, 230], 0.8),
                (c * 0.5).max(1.0),
                0.7,
            );
        }
    }
}

/// How many of the ten resonant pairs you've merged, as a row of pixel tokens:
/// a found pair shows both dreams' colours, an unfound one a question mark.
fn resonant_strip(p: &egui::Painter, screen: Rect, v: &HudView, y: f32, width: f32) {
    let m = &v.merge;
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    let left = screen.left() + screen.width() * 0.05;
    let found = m.resonant_found.iter().filter(|&&f| f).count();
    p.text(
        Pos2::new(left, y),
        Align2::LEFT_TOP,
        format!("RESONANT PAIRS  {found} / {}", fusion::RESONANT.len()),
        FontId::monospace(14.0 * u),
        rgba([255, 230, 160], 0.9),
    );
    let tok = Vec2::new(26.0 * u, 20.0 * u);
    let gap = ((width - tok.x * fusion::RESONANT.len() as f32)
        / (fusion::RESONANT.len() as f32 - 1.0))
        .clamp(3.0, 10.0 * u);
    for (i, &(a, b)) in fusion::RESONANT.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(left + i as f32 * (tok.x + gap), y + 24.0 * u),
            tok,
        );
        if m.resonant_found.get(i).copied().unwrap_or(false) {
            let (ca, cb) = (a.spec().accents[0], b.spec().accents[0]);
            pixel_ui::panel(
                p,
                r,
                hue(v.time * 0.3 + i as f32 * 0.1),
                rgba([14, 8, 28], 0.9),
                c.min(3.0),
                1.0,
            );
            let inner = r.shrink(c * 2.0);
            let half = inner.width() * 0.5;
            p.rect_filled(
                Rect::from_min_size(inner.min, Vec2::new(half, inner.height())),
                0.0,
                rgba(ca, 1.0),
            );
            p.rect_filled(
                Rect::from_min_size(
                    inner.min + Vec2::new(half, 0.0),
                    Vec2::new(half, inner.height()),
                ),
                0.0,
                rgba(cb, 1.0),
            );
        } else {
            pixel_ui::panel(p, r, [70, 62, 95], rgba([10, 6, 20], 0.7), c.min(3.0), 0.7);
            p.text(
                r.center(),
                Align2::CENTER_CENTER,
                "?",
                FontId::monospace(13.0 * u),
                rgba([110, 100, 140], 0.9),
            );
        }
    }
}

pub fn draw(ctx: &egui::Context, p: &egui::Painter, screen: Rect, v: &HudView, art: &mut CardArt) {
    let m = &v.merge;
    let u = pixel_ui::scale(screen);
    let c = pixel_ui::cell(u);
    let lay = layout(screen);
    p.rect_filled(screen, 0.0, rgba([10, 4, 18], 0.95));
    let cx = screen.center().x;
    let top = screen.top() + 20.0 * u;
    let state = step(m);

    // Title, with the moth that does the work.
    glitch_text(
        p,
        Pos2::new(cx, top),
        true,
        "THE MOTH MERGES",
        40.0 * u,
        ink(v),
        1.0,
        v,
    );
    let mpx = (3.0 * u).round().max(2.0);
    let moth_at = Pos2::new(
        cx - 40.0 * u * 6.0 * 0.6 - 18.0 * mpx - 20.0 * u,
        top - 2.0 * u,
    );
    let wing = if state == Step::Blocked {
        [150, 120, 170]
    } else {
        [255, 215, 120]
    };
    crate::shop_ui::paint_moth(p, moth_at, mpx, [40, 24, 70], wing, v.time, 1.0);

    // What to do next, in a ribbon.
    let ask = match (m.first, state) {
        (None, _) => "pick the first card: it keeps its shape and its ability",
        (Some(_), Step::Blocked) => "that's the same dream: pick a different card",
        (Some(_), _) => "pick the second card: it gives its colours and mood",
    };
    let text = m.status.as_deref().unwrap_or(ask);
    let size = pixel_ui::fit(text, 18.0 * u, screen.width() - 80.0 * u, 9.0);
    let rw = (text.chars().count() as f32 * size * 0.6 + c * 10.0).min(screen.width() - 30.0);
    let ribbon =
        Rect::from_center_size(Pos2::new(cx, top + 70.0 * u), Vec2::new(rw, size + c * 5.0));
    let (edge, tone) = if m.status.is_some() {
        ([255, 215, 100], [255, 240, 180])
    } else if state == Step::Blocked {
        ([255, 90, 120], [255, 170, 180])
    } else {
        ([150, 130, 210], [225, 210, 255])
    };
    pixel_ui::panel(p, ribbon, edge, rgba([14, 8, 30], 0.9), c, 0.9);
    p.text(
        ribbon.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::monospace(size),
        rgba(tone, 0.95),
    );

    // The booklet on the left: the first card wears an A, same-dream cards dim.
    let first_card = m.cards.iter().find(|c| Some(c.number) == m.first);
    let first_number: Vec<u32> = first_card.map(|c| c.number).into_iter().collect();
    let marks = GridMarks {
        marked: &first_number,
        badge: m.first.map(|n| (n, "A")),
        dim_theme: first_card.map(|c| c.theme),
    };
    let grid_bottom = draw_grid(
        p,
        screen,
        v,
        &m.cards,
        m.selected,
        &marks,
        lay.grid_top,
        lay.grid_w,
    );
    resonant_strip(p, screen, v, grid_bottom + 34.0 * u, lay.grid_w);

    // The altar.
    pixel_ui::panel(
        p,
        lay.altar,
        [130, 105, 190],
        rgba([12, 6, 26], 0.85),
        c,
        0.7,
    );
    let hovered = m.cards.get(m.selected);
    let result = preview(m);
    let (a_card, a_dim) = match first_card {
        Some(card) => (Some(card), false),
        None => (hovered, true),
    };
    let (b_card, b_dim) = match first_card {
        Some(_) => (hovered, state == Step::Blocked),
        None => (None, false),
    };
    slot(
        ctx,
        p,
        lay.slots[0],
        "KEEPS SHAPE + ABILITY",
        a_card,
        a_dim,
        m.first.is_none(),
        v,
        art,
        u,
        lay.scale,
    );
    slot(
        ctx,
        p,
        lay.slots[1],
        "GIVES COLOUR + MOOD",
        b_card,
        b_dim,
        m.first.is_some() && state != Step::Blocked,
        v,
        art,
        u,
        lay.scale,
    );
    // The result: a glow of its own colour, and sparkles orbiting when it's real.
    if let Some(res) = &result {
        let col = card_frame(res, v.time);
        let rr = lay.slots[2];
        for ring in 1..=3 {
            p.rect_filled(
                rr.expand(ring as f32 * c * 1.5),
                0.0,
                rgba(col, 0.09 / ring as f32),
            );
        }
        for k in 0..4 {
            let t = (v.time * 0.3 + k as f32 / 4.0).rem_euclid(1.0);
            let ring = rr.expand(c * 4.0);
            let per = ring.width() * 2.0 + ring.height() * 2.0;
            let mut d = t * per;
            let at = if d < ring.width() {
                Pos2::new(ring.left() + d, ring.top())
            } else {
                d -= ring.width();
                if d < ring.height() {
                    Pos2::new(ring.right(), ring.top() + d)
                } else {
                    d -= ring.height();
                    if d < ring.width() {
                        Pos2::new(ring.right() - d, ring.bottom())
                    } else {
                        Pos2::new(ring.left(), ring.bottom() - (d - ring.width()))
                    }
                }
            };
            let snapped = Pos2::new((at.x / c).round() * c, (at.y / c).round() * c);
            pixel_ui::sparkle(p, snapped, c, rgba(col, 0.9));
        }
    }
    slot(
        ctx,
        p,
        lay.slots[2],
        "BECOMES",
        result.as_ref(),
        false,
        false,
        v,
        art,
        u,
        lay.scale,
    );
    // The signs between: they light up once there's something to make.
    let ready = result.is_some();
    let sign = if ready {
        rgba([255, 230, 140], 0.7 + 0.3 * (v.time * 4.0).sin().abs())
    } else {
        rgba([120, 105, 160], 0.7)
    };
    pixel_ui::plus(p, lay.signs[0], c, sign);
    pixel_ui::arrow(p, lay.signs[1], c, sign);

    // Under the cards: what you'd get.
    let info_w = lay.altar.width() - 28.0 * u;
    let info_at = Pos2::new(lay.altar.left() + 14.0 * u, lay.info_top);
    match (&result, state) {
        (Some(res), _) => {
            let (label, tone) = match res.rarity {
                Rarity::Resonant => ("RESONANT: they were meant to meet", hue(v.time * 0.4)),
                _ => ("FUSED: two dreams, one card", [255, 225, 150]),
            };
            pixel_ui::split_text(
                p,
                info_at,
                Align2::LEFT_TOP,
                label,
                pixel_ui::fit(label, 16.0 * u, info_w, 9.0),
                rgba(tone, 1.0),
                (c * 0.5).max(1.0),
                0.8,
            );
            power_lines(
                p,
                info_at + Vec2::new(0.0, 24.0 * u),
                info_w,
                res,
                None,
                u,
                v.time,
            );
        }
        (None, Step::Blocked) => {
            p.text(
                info_at,
                Align2::LEFT_TOP,
                "same dream: nothing to make",
                FontId::monospace(15.0 * u),
                rgba([255, 150, 170], 0.9),
            );
        }
        (None, _) => {
            if let Some(h) = hovered {
                p.text(
                    info_at,
                    Align2::LEFT_TOP,
                    if m.first.is_none() {
                        "this one will keep its shape:"
                    } else {
                        ""
                    },
                    FontId::monospace(13.0 * u),
                    rgba([180, 170, 210], 0.8),
                );
                power_lines(
                    p,
                    info_at + Vec2::new(0.0, 22.0 * u),
                    info_w,
                    h,
                    None,
                    u,
                    v.time,
                );
            }
        }
    }

    let hint = if m.first.is_some() {
        format!("[enter] merge  [tab] use as first  [backspace] clear  [t] sort: {}  [r] flip  [esc] back", m.sort)
    } else {
        format!("[enter] pick the first card  [a/d/w/s] browse  [t] sort: {}  [r] flip  [esc] back", m.sort)
    };
    hint_bar(p, screen, v, &hint);
    pixel_ui::scanlines(p, screen, c, 10);
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
            cards: cards(&[Garden, MyceliumGrove, Garden]).into(),
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

    #[test]
    fn the_altar_follows_the_picks() {
        let mut m = MergeView {
            cards: cards(&[Garden, MyceliumGrove, Garden]).into(),
            ..Default::default()
        };
        assert_eq!(step(&m), Step::PickFirst);
        m.first = Some(1);
        m.selected = 1;
        assert_eq!(step(&m), Step::PickSecond);
        m.selected = 2;
        assert_eq!(step(&m), Step::Blocked, "same dream");
        m.selected = 0;
        assert_eq!(step(&m), Step::Blocked, "itself");
    }

    #[test]
    fn the_layout_fits_every_screen_without_overlap() {
        for (w, h) in [
            (800.0, 600.0),
            (1280.0, 720.0),
            (1920.0, 1080.0),
            (1280.0, 800.0),
        ] {
            let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h));
            let l = layout(screen);
            assert!(screen.contains_rect(l.altar), "{w}x{h} altar");
            for s in &l.slots {
                assert!(
                    l.altar.contains_rect(*s),
                    "{w}x{h}: {s:?} outside {:?}",
                    l.altar
                );
            }
            for pair in l.slots.windows(2) {
                assert!(pair[0].right() < pair[1].left(), "{w}x{h} overlap");
            }
            // The signs sit in the gaps.
            assert!(l.signs[0].x > l.slots[0].right() && l.signs[0].x < l.slots[1].left());
            assert!(l.signs[1].x > l.slots[1].right() && l.signs[1].x < l.slots[2].left());
            // Room under the cards for the lines, and the grid doesn't run into the altar.
            assert!(
                l.altar.bottom() - l.info_top >= 100.0 * pixel_ui::scale(screen) * 0.9 - 0.01,
                "{w}x{h} info room"
            );
            let grid_right = screen.left() + screen.width() * 0.05 + l.grid_w;
            assert!(grid_right < l.altar.left(), "{w}x{h} grid hits altar");
            assert!(l.scale >= 0.25);
        }
    }
}
