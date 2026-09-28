//! Pack opening and the Lucid Store: the two screens between waking up and
//! dreaming again. Same pixel + VT323 language as the rest of the HUD.

use crate::booklet_ui::{card, CardArt, CARD_SIZE};
use crate::cards::Recalled;
use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use engine::ui::egui::{self, Align2, FontId, Pos2, Rect, Stroke, Vec2};

/// Seconds per card in the reveal.
pub const REVEAL_STEP: f32 = 0.9;

/// However long the run, the whole pack flips within this many seconds.
pub const REVEAL_BUDGET: f32 = 10.0;

/// Seconds between one card flipping and the next.
pub fn reveal_step(n: usize) -> f32 {
    REVEAL_STEP.min(REVEAL_BUDGET / n.max(1) as f32).max(0.05)
}

/// Which reveal stage card `i` of `n` is in at time `t`:
/// 0 = face down, 1 = flipping, 2 = revealed.
pub fn reveal_stage(i: usize, n: usize, t: f32) -> u8 {
    let start = 0.6 + i as f32 * reveal_step(n);
    if t < start {
        0
    } else if t < start + 0.35 {
        1
    } else {
        2
    }
}

pub fn reveal_done(n: usize, t: f32) -> bool {
    n == 0 || reveal_stage(n - 1, n, t) == 2
}

/// Where the cards may go: under the title, above the two message lines.
pub fn pack_area(screen: Vec2) -> Vec2 {
    Vec2::new(screen.x - 80.0, screen.y - 80.0 - 110.0)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PackLayout {
    pub size: Vec2,
    pub gap: f32,
    pub per_row: usize,
    /// Cards drawn; anything past this is summed up as "+N".
    pub shown: usize,
    /// Tiny rarity chips instead of cards.
    pub compact: bool,
}

const FULL_SCALE: f32 = 0.62;
/// Below this the card art stops being readable: switch to chips.
const MIN_SCALE: f32 = 0.34;
const CHIP: Vec2 = Vec2::new(58.0, 30.0);

/// The biggest cards that fit all `n` dreams; if even the smallest don't,
/// chips; if even chips don't, as many chips as fit (the last says "+N").
pub fn pack_layout(n: usize, area: Vec2) -> PackLayout {
    let n = n.max(1);
    let fits = |size: Vec2, gap: f32| {
        let per_row = (((area.x + gap) / (size.x + gap)).floor() as usize).max(1);
        let rows = ((area.y + gap) / (size.y + gap)).floor() as usize;
        (per_row, rows)
    };
    let mut scale = FULL_SCALE;
    while scale >= MIN_SCALE {
        let size = CARD_SIZE * scale;
        let gap = 14.0 * scale / FULL_SCALE;
        let (per_row, rows) = fits(size, gap);
        let per_row = if scale == FULL_SCALE {
            per_row.min(6)
        } else {
            per_row
        };
        if per_row * rows >= n {
            return PackLayout {
                size,
                gap,
                per_row,
                shown: n,
                compact: false,
            };
        }
        scale -= 0.02;
    }
    let gap = 6.0;
    let (per_row, rows) = fits(CHIP, gap);
    let room = per_row * rows.max(1);
    PackLayout {
        size: CHIP,
        gap,
        per_row,
        shown: n.min(room),
        compact: true,
    }
}

#[derive(Clone, Debug, Default)]
pub struct PackView {
    pub pack: Vec<Recalled>,
    /// Seconds since the pack began opening; `f32::MAX` = skipped.
    pub age: f32,
    pub pressed: bool,
    pub dust_earned: u32,
    pub cards_added: usize,
}

pub fn draw_reveal(
    ctx: &egui::Context,
    p: &egui::Painter,
    screen: Rect,
    v: &HudView,
    pk: &PackView,
    art: &mut CardArt,
) {
    p.rect_filled(screen, 0.0, rgba([6, 3, 16], 0.95));
    let cx = screen.center().x;
    glitch_text(
        p,
        Pos2::new(cx, screen.top() + 22.0),
        true,
        "WHAT DO YOU REMEMBER?",
        40.0,
        hue(v.time * 0.1),
        1.0,
        v,
    );
    // As many dreams as the run had: full cards, smaller cards, or chips.
    let n = pk.pack.len();
    let l = pack_layout(n, pack_area(screen.size()));
    let scale = l.size.x / CARD_SIZE.x;
    let step = reveal_step(n);
    let overflow = n - l.shown;
    for (i, r) in pk.pack.iter().enumerate().take(l.shown) {
        let row = i / l.per_row;
        let col = i % l.per_row;
        let in_row = (l.shown - row * l.per_row).min(l.per_row) as f32;
        let row_w = in_row * l.size.x + (in_row - 1.0) * l.gap;
        let min = Pos2::new(
            cx - row_w * 0.5 + col as f32 * (l.size.x + l.gap),
            screen.top() + 80.0 + row as f32 * (l.size.y + l.gap),
        );
        let rect = Rect::from_min_size(min, l.size);
        // The last chip stands for everything that didn't fit.
        if l.compact && overflow > 0 && i + 1 == l.shown {
            p.rect_stroke(rect, 0.0, Stroke::new(2.0_f32, rgba(ink(v), 0.7)));
            p.text(
                rect.center(),
                Align2::CENTER_CENTER,
                format!("+{}", overflow + 1),
                FontId::monospace(20.0),
                rgba(ink(v), 0.9),
            );
            continue;
        }
        match reveal_stage(i, n, pk.age) {
            0 => card_back(p, rect, v, false),
            1 => {
                // Flip: squash horizontally around the centre.
                let t = ((pk.age - 0.6 - i as f32 * step) / 0.35).clamp(0.0, 1.0);
                let w = (1.0 - 2.0 * t).abs();
                let squashed = Rect::from_center_size(
                    rect.center(),
                    Vec2::new(l.size.x * w.max(0.04), l.size.y),
                );
                if t < 0.5 {
                    card_back(p, squashed, v, true);
                } else {
                    p.rect_filled(
                        squashed,
                        0.0,
                        rgba(crate::booklet_ui::card_frame(&r.card, v.time), 0.9),
                    );
                }
            }
            _ if l.compact => chip(p, rect, r, v),
            _ => {
                if r.remembered {
                    let clip = p.with_clip_rect(rect);
                    card(
                        ctx,
                        &clip.with_clip_rect(rect),
                        rect,
                        &r.card,
                        v,
                        art,
                        scale,
                    );
                } else {
                    faded(p, rect, r, v);
                }
            }
        }
    }
    let done = reveal_done(pk.pack.len(), pk.age);
    let kept = pk.pack.iter().filter(|r| r.remembered).count();
    let msg = if !done {
        v.k("remembering...    [space] skip")
    } else if pk.pressed {
        format!(
            "pressed {} cards · +{} dust · you hold {} dust",
            pk.cards_added, pk.dust_earned, v.dust
        )
    } else {
        format!("{kept} of {} dreams stayed with you", pk.pack.len())
    };
    p.text(
        Pos2::new(cx, screen.bottom() - 92.0),
        Align2::CENTER_TOP,
        msg,
        FontId::monospace(24.0),
        rgba([255, 210, 80], 0.95),
    );
    if done {
        let keys = if pk.pressed {
            "[b] booklet   [l] lucid store   [r] dream again   [esc] wake"
        } else {
            "[s] press into booklet   [r] dream again   [esc] wake"
        };
        let keys = v.k(keys);
        p.text(
            Pos2::new(cx, screen.bottom() - 54.0),
            Align2::CENTER_TOP,
            keys,
            FontId::monospace(22.0),
            rgba(ink(v), 0.5 + 0.3 * (v.time * 2.0).sin()),
        );
    }
}

/// A dream too many to show as a card: its rarity colour and depth.
/// Forgotten ones are hollow and grey.
fn chip(p: &egui::Painter, r: Rect, rec: &Recalled, v: &HudView) {
    let rgb = crate::booklet_ui::card_frame(&rec.card, v.time);
    if rec.remembered {
        p.rect_filled(r, 0.0, rgba(rgb, 0.85));
    } else {
        p.rect_stroke(r, 0.0, Stroke::new(2.0_f32, rgba([110, 100, 130], 0.6)));
    }
    p.text(
        r.center(),
        Align2::CENTER_CENTER,
        format!("{}", rec.card.depth),
        FontId::monospace(18.0),
        rgba(
            if rec.remembered {
                [10, 5, 20]
            } else {
                [150, 140, 170]
            },
            1.0,
        ),
    );
}

/// The face-down pack card: a pixel eye, the same one on every back.
fn card_back(p: &egui::Painter, r: Rect, v: &HudView, glowing: bool) {
    p.rect_filled(r, 0.0, rgba([24, 12, 44], 1.0));
    p.rect_stroke(
        r.shrink(3.0),
        0.0,
        Stroke::new(
            3.0_f32,
            rgba(hue(v.time * 0.2), if glowing { 1.0 } else { 0.55 }),
        ),
    );
    if r.width() < 40.0 {
        return;
    }
    let c = r.center();
    let px = (r.width() / 22.0).round().max(2.0);
    for (x, y, k) in crate::pixels::eye_pixels(0.8, None, false) {
        let col = match k {
            crate::pixels::EyePx::Lid | crate::pixels::EyePx::Lash => rgba(ink(v), 0.6),
            crate::pixels::EyePx::Pupil => rgba([6, 0, 12], 1.0),
            crate::pixels::EyePx::Sclera => continue,
            _ => rgba(hue(0.75), 0.8),
        };
        p.rect_filled(
            Rect::from_center_size(c + Vec2::new(x as f32 * px, y as f32 * px), Vec2::splat(px)),
            0.0,
            col,
        );
    }
}

/// A dream that didn't stay: a greyed-out ghost of its name, and the dust it left.
fn faded(p: &egui::Painter, r: Rect, rec: &Recalled, v: &HudView) {
    // Text scales with the card (long runs shrink it).
    let f = (r.width() / (CARD_SIZE.x * 0.62)).min(1.0);
    p.rect_filled(r, 0.0, rgba([14, 10, 22], 0.9));
    p.rect_stroke(r.shrink(3.0), 0.0, Stroke::new(1.0_f32, rgba(ink(v), 0.15)));
    // Dissolving pixel dust.
    for k in 0..40u32 {
        let h = crate::hud::hash01(k, rec.card.seed as u32);
        let h2 = crate::hud::hash01(k, (rec.card.seed >> 32) as u32 ^ 0xA5A5);
        let drift = (v.time * 8.0 + k as f32 * 3.0) % r.height();
        let at = Pos2::new(
            r.left() + h * r.width(),
            r.bottom() - (h2 * r.height() + drift) % r.height(),
        );
        p.rect_filled(
            Rect::from_center_size(at, Vec2::splat(3.0)),
            0.0,
            rgba(ink(v), 0.18),
        );
    }
    let name = p.layout(
        rec.card.name.clone(),
        FontId::monospace(15.0 * f),
        rgba(ink(v), 0.3),
        r.width() - 16.0 * f,
    );
    p.galley(
        r.left_top() + Vec2::new(8.0 * f, 24.0 * f),
        name,
        rgba(ink(v), 0.3),
    );
    p.text(
        r.center() + Vec2::new(0.0, 0.18 * r.height()),
        Align2::CENTER_CENTER,
        "FADED",
        FontId::monospace(22.0 * f),
        rgba(ink(v), 0.35),
    );
    p.text(
        r.center() + Vec2::new(0.0, 0.18 * r.height() + 26.0 * f),
        Align2::CENTER_CENTER,
        format!("+{} dust", crate::cards::fade_dust(rec.card.rarity)),
        FontId::monospace(16.0 * f),
        rgba([255, 210, 80], 0.7),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_reveal_in_order_and_the_pack_finishes() {
        assert_eq!(reveal_stage(0, 3, 0.0), 0);
        assert_eq!(reveal_stage(0, 3, 0.7), 1);
        assert_eq!(reveal_stage(0, 3, 1.2), 2);
        assert_eq!(reveal_stage(1, 3, 1.2), 0, "second card waits its turn");
        assert!(!reveal_done(3, 1.0));
        assert!(reveal_done(3, 0.6 + 2.0 * REVEAL_STEP + 0.36));
        assert!(reveal_done(0, 0.0));
        assert!(reveal_done(5, f32::MAX), "skip reveals everything");
    }

    #[test]
    fn any_run_length_fits_on_screen() {
        for screen in [Vec2::new(1280.0, 720.0), Vec2::new(1920.0, 1080.0)] {
            let area = pack_area(screen);
            for n in 1..=300usize {
                let l = pack_layout(n, area);
                let (w, h) = (l.size.x + l.gap, l.size.y + l.gap);
                let rows = n.min(l.shown).div_ceil(l.per_row);
                assert!(
                    l.per_row as f32 * w - l.gap <= area.x + 0.5,
                    "n={n} too wide"
                );
                assert!(rows as f32 * h - l.gap <= area.y + 0.5, "n={n} too tall");
                assert!(l.shown >= n.min(l.per_row), "shows at least a row");
                if n <= 6 {
                    assert!(
                        !l.compact && l.size == CARD_SIZE * 0.62,
                        "short runs look as before"
                    );
                }
            }
            assert!(
                pack_layout(30, area).size.x < CARD_SIZE.x * 0.62,
                "30 dreams shrink the cards"
            );
            assert!(pack_layout(300, area).compact, "300 dreams become chips");
        }
    }

    #[test]
    fn long_packs_reveal_faster_but_in_order() {
        assert_eq!(reveal_step(3), REVEAL_STEP, "short packs keep their pace");
        let total = |n: usize| 0.6 + reveal_step(n) * n as f32;
        for n in [10, 40, 150] {
            assert!(
                total(n) <= REVEAL_BUDGET + 0.7,
                "{n} cards take {}",
                total(n)
            );
        }
        assert!(reveal_stage(0, 40, 0.7) >= 1);
        assert_eq!(
            reveal_stage(39, 40, 0.7),
            0,
            "last card still waits its turn"
        );
        assert!(reveal_done(40, 0.6 + REVEAL_BUDGET + 0.4));
    }
}
