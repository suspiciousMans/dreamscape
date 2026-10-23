//! The hand: the equipped cards that hold abilities, fanned down the right
//! edge of the screen. A card pops toward the centre when its ability is
//! used, flashes in its tint, and sends out a ring.
//!
//! Layout and timing are pure and unit-tested; `draw` only paints them.

use engine::ui::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};

/// Seconds a card takes to leap out, hold, and settle back.
pub const POP_RISE: f32 = 0.08;
pub const POP_HOLD: f32 = 0.18;
pub const POP_FALL: f32 = 0.35;
pub const POP_TOTAL: f32 = POP_RISE + POP_HOLD + POP_FALL;
/// Seconds the expanding ring lives.
pub const RING_SECS: f32 = 0.5;

/// `since_use` for a card that has not been used (this dream).
pub const NEVER: f32 = f32::MAX;

/// One card in the hand, as the HUD sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct HandCard {
    pub ability: &'static str,
    pub card_name: String,
    pub key: &'static str,
    /// 0..1 recharge.
    pub charge: f32,
    /// The ability is running right now.
    pub active: bool,
    /// The card's tint and second colour.
    pub tint: [u8; 3],
    pub tint2: [u8; 3],
    /// Seconds since the ability was used; [`NEVER`] if it has not been.
    pub since_use: f32,
}

/// 0 at rest, 1 at the top of the pop: quick out, short hold, slow back.
pub fn pop(since: f32) -> f32 {
    if !(0.0..POP_TOTAL).contains(&since) {
        return 0.0;
    }
    if since < POP_RISE {
        let t = since / POP_RISE;
        return 1.0 - (1.0 - t) * (1.0 - t);
    }
    let held = since - POP_RISE;
    if held < POP_HOLD {
        return 1.0;
    }
    let t = (held - POP_HOLD) / POP_FALL;
    let e = t * t * (3.0 - 2.0 * t);
    1.0 - e
}

/// 0..1 progress of the ring, or `None` when it is not showing.
pub fn ring(since: f32) -> Option<f32> {
    (0.0..RING_SECS).contains(&since).then(|| since / RING_SECS)
}

/// Where each of `n` cards rests: stacked down the right edge, the middle
/// ones bowed a little toward the centre, like a hand held out.
pub fn layout(screen: Rect, n: usize) -> Vec<Rect> {
    if n == 0 {
        return Vec::new();
    }
    let u = crate::pixel_ui::scale(screen);
    let (w, h) = (88.0 * u, 104.0 * u);
    let top_min = screen.top() + 150.0 * u;
    let bottom_max = screen.bottom() - 40.0 * u;
    let room = (bottom_max - top_min).max(h);
    let mut gap = 10.0 * u;
    let mut total = n as f32 * h + (n as f32 - 1.0) * gap;
    if total > room && n > 1 {
        gap = ((room - n as f32 * h) / (n as f32 - 1.0)).max(-h * 0.45);
        total = n as f32 * h + (n as f32 - 1.0) * gap;
    }
    let top = (screen.center().y - total / 2.0).clamp(top_min, (bottom_max - total).max(top_min));
    let mid = (n as f32 - 1.0) / 2.0;
    (0..n)
        .map(|i| {
            let t = if mid > 0.0 { (i as f32 - mid) / mid } else { 0.0 };
            let bow = -14.0 * u * (1.0 - t * t);
            Rect::from_min_size(
                Pos2::new(screen.right() - 20.0 * u - w + bow, top + i as f32 * (h + gap)),
                Vec2::new(w, h),
            )
        })
        .collect()
}

/// The card's rect while popped: toward the centre and a little larger.
/// Reduced motion keeps it in place.
pub fn popped(rest: Rect, amount: f32, u: f32, reduced: bool) -> Rect {
    if reduced || amount <= 0.0 {
        return rest;
    }
    let grown = Rect::from_center_size(rest.center(), rest.size() * (1.0 + 0.14 * amount));
    grown.translate(Vec2::new(-110.0 * u * amount, 0.0))
}

fn rgba(rgb: [u8; 3], a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// Paints the hand. `time` drives the ready-pulse only.
pub fn draw(p: &egui::Painter, screen: Rect, cards: &[HandCard], time: f32, reduced: bool) {
    let rests = layout(screen, cards.len());
    let u = crate::pixel_ui::scale(screen);
    // Rest order, then whichever is mid-pop on top.
    let mut order: Vec<usize> = (0..cards.len()).collect();
    order.sort_by(|&a, &b| {
        pop(cards[a].since_use)
            .partial_cmp(&pop(cards[b].since_use))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for i in order {
        let c = &cards[i];
        let amount = pop(c.since_use);
        let r = popped(rests[i], amount, u, reduced);
        if let Some(t) = ring(c.since_use) {
            let grow = if reduced { 0.0 } else { 26.0 * u * t };
            p.rect_stroke(
                r.expand(grow),
                0.0,
                Stroke::new(2.0 * u, rgba(c.tint, 0.8 * (1.0 - t))),
            );
        }
        draw_card(p, r, c, amount, time, u);
    }
}

fn draw_card(p: &egui::Painter, r: Rect, c: &HandCard, amount: f32, time: f32, u: f32) {
    let ready = c.charge >= 1.0;
    let lit = ready || c.active || amount > 0.0;
    let edge = if lit { c.tint } else { [120, 100, 150] };
    let cell = (3.0 * u).round().max(2.0);
    crate::pixel_ui::panel(
        p,
        r,
        edge,
        rgba([12, 6, 28], 0.88),
        cell,
        if lit { 1.0 } else { 0.7 },
    );
    // The art window: bands from the card's tint to its second colour.
    let inner = r.shrink(cell * 2.5);
    let art = Rect::from_min_size(inner.min, Vec2::new(inner.width(), inner.height() * 0.58));
    let bands = 6;
    for b in 0..bands {
        let f = b as f32 / (bands - 1) as f32;
        let mix = |k: usize| (c.tint[k] as f32 * (1.0 - f) + c.tint2[k] as f32 * f) as u8;
        let band = Rect::from_min_size(
            Pos2::new(art.left(), art.top() + art.height() * b as f32 / bands as f32),
            Vec2::new(art.width(), art.height() / bands as f32 + 0.5),
        );
        p.rect_filled(band, 0.0, rgba([mix(0), mix(1), mix(2)], if lit { 0.45 } else { 0.22 }));
    }
    // The ability's icon, large, in the window.
    if let Some(&a) = crate::upgrades::ALL_ABILITIES
        .iter()
        .find(|a| a.name() == c.ability)
    {
        let tone = if lit { [255, 245, 255] } else { [160, 140, 190] };
        crate::pixel_ui::icon(
            p,
            art.center(),
            crate::pixels::icons::ability(a),
            (3.0 * u).max(2.0),
            tone,
            crate::pixel_ui::lighter(c.tint),
            if lit { 1.0 } else { 0.65 },
        );
    }
    // Name, then the key on a cap.
    let name_y = art.bottom() + 4.0 * u;
    let size = crate::pixel_ui::fit(c.ability, 14.0 * u, inner.width(), 7.0);
    p.text(
        Pos2::new(inner.center().x, name_y),
        Align2::CENTER_TOP,
        c.ability,
        FontId::monospace(size),
        rgba(edge, 1.0),
    );
    crate::pixel_ui::keycap(
        p,
        Pos2::new(inner.left() + 12.0 * u, art.top() + 11.0 * u),
        &format!("{}", c.key),
        (11.0 * u).max(8.0),
        2.0,
        edge,
        if lit { 1.0 } else { 0.7 },
    );
    // Recharge: a bar along the bottom; ready cells pulse.
    let cells = 8;
    let filled = (c.charge.clamp(0.0, 1.0) * cells as f32).floor() as i32;
    let cw = inner.width() / cells as f32;
    for k in 0..cells {
        let cellr = Rect::from_min_size(
            Pos2::new(inner.left() + k as f32 * cw, inner.bottom() - 5.0 * u),
            Vec2::new(cw - 1.0, 5.0 * u),
        );
        let a = if k < filled || ready {
            if ready { 0.7 + 0.3 * (time * 5.0 + k as f32 * 0.5).sin() } else { 0.9 }
        } else {
            0.15
        };
        p.rect_filled(cellr, 0.0, rgba(edge, a));
    }
    // The flash: a white wash that fades with the pop.
    if amount > 0.0 {
        p.rect_filled(r, 0.0, rgba(c.tint.map(|v| v / 2 + 127), 0.35 * amount));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0))
    }

    #[test]
    fn pop_rises_holds_and_settles() {
        assert_eq!(pop(NEVER), 0.0);
        assert_eq!(pop(-1.0), 0.0);
        assert_eq!(pop(0.0), 0.0);
        assert!((pop(POP_RISE) - 1.0).abs() < 1e-5);
        assert!((pop(POP_RISE + POP_HOLD * 0.5) - 1.0).abs() < 1e-5);
        let mid = pop(POP_RISE + POP_HOLD + POP_FALL * 0.5);
        assert!(mid > 0.0 && mid < 1.0);
        assert_eq!(pop(POP_TOTAL), 0.0);
        assert_eq!(pop(POP_TOTAL + 5.0), 0.0);
    }

    #[test]
    fn pop_never_leaves_zero_to_one() {
        let mut t = 0.0;
        while t < POP_TOTAL + 0.1 {
            let v = pop(t);
            assert!((0.0..=1.0).contains(&v), "pop({t}) = {v}");
            t += 0.005;
        }
    }

    #[test]
    fn ring_runs_once() {
        assert_eq!(ring(NEVER), None);
        assert_eq!(ring(-0.1), None);
        assert!(ring(0.25).is_some_and(|t| (t - 0.5).abs() < 1e-5));
        assert_eq!(ring(RING_SECS), None);
    }

    #[test]
    fn an_empty_hand_has_no_layout() {
        assert!(layout(screen(), 0).is_empty());
    }

    #[test]
    fn the_hand_stays_on_screen_and_apart_for_every_size() {
        for (w, h) in [(1280.0, 720.0), (1280.0, 800.0), (1920.0, 1080.0), (960.0, 540.0)] {
            let s = Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h));
            for n in 1..=crate::lore::MAX_SLOTS {
                let rects = layout(s, n);
                assert_eq!(rects.len(), n);
                for r in &rects {
                    assert!(r.right() <= s.right(), "{n} cards at {w}x{h}: off the right");
                    assert!(r.left() > s.center().x, "{n} cards at {w}x{h}: past the middle");
                    assert!(r.top() >= s.top() && r.bottom() <= s.bottom(), "{n} at {w}x{h}");
                }
                // Cards never swap order, and may overlap only when the
                // screen is too short for them to sit apart.
                for pair in rects.windows(2) {
                    assert!(pair[1].top() > pair[0].top());
                }
            }
        }
    }

    #[test]
    fn on_a_720p_screen_five_cards_do_not_overlap_much() {
        let rects = layout(screen(), crate::lore::MAX_SLOTS);
        for pair in rects.windows(2) {
            let overlap = pair[0].bottom() - pair[1].top();
            assert!(overlap < pair[0].height() * 0.5, "overlap {overlap}");
        }
    }

    #[test]
    fn the_middle_card_bows_toward_the_centre() {
        let rects = layout(screen(), 3);
        assert!(rects[1].left() < rects[0].left());
        assert!(rects[1].left() < rects[2].left());
        assert_eq!(rects[0].left(), rects[2].left());
    }

    #[test]
    fn popping_moves_toward_the_centre_and_grows() {
        let rest = layout(screen(), 2)[0];
        let up = popped(rest, 1.0, 1.0, false);
        assert!(up.center().x < rest.center().x);
        assert!(up.width() > rest.width());
        assert_eq!(popped(rest, 0.0, 1.0, false), rest);
    }

    #[test]
    fn reduced_motion_keeps_the_card_still() {
        let rest = layout(screen(), 2)[0];
        assert_eq!(popped(rest, 1.0, 1.0, true), rest);
    }
}
